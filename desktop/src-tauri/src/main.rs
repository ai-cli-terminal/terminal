#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    env,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
const DEFAULT_UBUNTU_DISTRO: &str = "Ubuntu";
const DEFAULT_DOCKER_IMAGE: &str = "ubuntu:24.04";
const DOCKER_WORKSPACE_TARGET: &str = "/workspace";
const MANAGED_NPM_PREFIX: &str = "$HOME/.local/share/ai-terminal/npm-global";

type SharedSession = Arc<Mutex<TerminalSession>>;
type SessionMap = Arc<Mutex<HashMap<String, SharedSession>>>;

#[derive(Clone, Default)]
struct TerminalState {
    sessions: SessionMap,
}

struct TerminalSession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

#[derive(Clone, Serialize)]
struct TerminalData {
    id: String,
    data: String,
}

#[derive(Clone, Serialize)]
struct TerminalExit {
    id: String,
    status: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrontendSmokeConfig {
    delay_milliseconds: u32,
    selection_text: String,
    paste_text: String,
    paste_expected_output: String,
    scrollback_lines: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInventory {
    checked_at_epoch_seconds: u64,
    probes: Vec<RuntimeProbe>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeProbe {
    id: String,
    label: String,
    status: String,
    detail: String,
    version: Option<String>,
    path: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DockerAppProbe {
    id: String,
    label: String,
    image: String,
    status: String,
    detail: String,
    shell: Vec<String>,
}

#[derive(Clone)]
struct DockerAppDefinition {
    id: &'static str,
    label: &'static str,
    image: String,
    shell: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AptPackageProbe {
    id: String,
    label: String,
    package_name: String,
    status: String,
    detail: String,
    version: Option<String>,
}

#[derive(Clone)]
struct AptPackageDefinition {
    id: &'static str,
    label: &'static str,
    package_name: &'static str,
}

struct ProbeOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

#[tauri::command]
fn terminal_open(
    app: AppHandle,
    state: State<'_, TerminalState>,
    rows: u16,
    cols: u16,
) -> Result<String, String> {
    let mut command = CommandBuilder::new(resolve_ash_program(&app).map_err(display_error)?);
    command.env("AI_TERMINAL_GUI", "1");
    open_terminal_session(app, state, rows, cols, command, true)
}

#[tauri::command]
fn terminal_open_runtime(
    app: AppHandle,
    state: State<'_, TerminalState>,
    rows: u16,
    cols: u16,
    runtime: String,
    workspace_dir: Option<String>,
) -> Result<String, String> {
    match runtime.as_str() {
        "ash" => terminal_open(app, state, rows, cols),
        "ubuntu" => {
            let mut command = wsl_ubuntu_command()?;
            command.env("TERM", "xterm-256color");
            open_terminal_session(app, state, rows, cols, command, false)
        }
        "docker" => {
            let mut command = docker_runtime_command(workspace_dir.as_deref())?;
            command.env("TERM", "xterm-256color");
            open_terminal_session(app, state, rows, cols, command, false)
        }
        "codex" | "claude" | "gemini" => {
            let mut command = ai_cli_runtime_command(&runtime)?;
            command.env("TERM", "xterm-256color");
            open_terminal_session(app, state, rows, cols, command, false)
        }
        _ => Err(format!("unknown runtime: {runtime}")),
    }
}

#[tauri::command]
fn terminal_open_docker_app(
    app: AppHandle,
    state: State<'_, TerminalState>,
    rows: u16,
    cols: u16,
    app_id: String,
    workspace_dir: Option<String>,
) -> Result<String, String> {
    let mut command = docker_app_runtime_command(&app_id, workspace_dir.as_deref())?;
    command.env("TERM", "xterm-256color");
    open_terminal_session(app, state, rows, cols, command, false)
}

fn open_terminal_session(
    app: AppHandle,
    state: State<'_, TerminalState>,
    rows: u16,
    cols: u16,
    command: CommandBuilder,
    enable_smoke_hooks: bool,
) -> Result<String, String> {
    let id = format!("term-{}", NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed));
    let size = sanitize_size(rows, cols);
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(size).map_err(display_error)?;
    let mut reader = pair.master.try_clone_reader().map_err(display_error)?;
    let writer = pair.master.take_writer().map_err(display_error)?;

    let child = pair.slave.spawn_command(command).map_err(display_error)?;
    drop(pair.slave);

    let session = Arc::new(Mutex::new(TerminalSession {
        master: pair.master,
        writer,
        child,
    }));

    state
        .sessions
        .lock()
        .map_err(|_| "terminal session table is poisoned".to_string())?
        .insert(id.clone(), session);

    if enable_smoke_hooks {
        schedule_smoke_ash_integration(state.sessions.clone(), id.clone());
        schedule_smoke_ctrl_c(state.sessions.clone(), id.clone());
        schedule_smoke_ctrl_d(state.sessions.clone(), id.clone());
    }

    spawn_reader_thread(
        app,
        state.sessions.clone(),
        id.clone(),
        Box::new(move |buffer| reader.read(buffer)),
    );

    Ok(id)
}

#[tauri::command]
fn terminal_write(state: State<'_, TerminalState>, id: String, data: String) -> Result<(), String> {
    let session = get_session(&state, &id)?;
    let mut session = session
        .lock()
        .map_err(|_| format!("terminal session {id} is poisoned"))?;
    session
        .writer
        .write_all(data.as_bytes())
        .map_err(display_error)?;
    session.writer.flush().map_err(display_error)?;
    Ok(())
}

#[tauri::command]
fn terminal_resize(
    state: State<'_, TerminalState>,
    id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let session = get_session(&state, &id)?;
    let session = session
        .lock()
        .map_err(|_| format!("terminal session {id} is poisoned"))?;
    session
        .master
        .resize(sanitize_size(rows, cols))
        .map_err(display_error)
}

#[tauri::command]
fn terminal_kill(state: State<'_, TerminalState>, id: String) -> Result<(), String> {
    kill_session(&state.sessions, &id)
}

#[tauri::command]
fn terminal_eof(state: State<'_, TerminalState>, id: String) -> Result<(), String> {
    let session = get_session(&state, &id)?;
    let mut session = session
        .lock()
        .map_err(|_| format!("terminal session {id} is poisoned"))?;
    session.writer.write_all(b"exit\r").map_err(display_error)?;
    session.writer.flush().map_err(display_error)
}

#[tauri::command]
fn terminal_kill_all(state: State<'_, TerminalState>) -> Result<(), String> {
    let ids = state
        .sessions
        .lock()
        .map_err(|_| "terminal session table is poisoned".to_string())?
        .keys()
        .cloned()
        .collect::<Vec<_>>();

    for id in ids {
        kill_session(&state.sessions, &id)?;
    }
    Ok(())
}

#[tauri::command]
fn terminal_smoke_command() -> Option<String> {
    env::var("AI_TERMINAL_GUI_SMOKE_COMMAND")
        .ok()
        .filter(|command| !command.trim().is_empty())
}

#[tauri::command]
fn terminal_smoke_ctrl_d_delay_ms() -> Option<u32> {
    smoke_ctrl_d_delay_ms().map(|delay| delay as u32)
}

#[tauri::command]
fn terminal_smoke_frontend_config() -> Option<FrontendSmokeConfig> {
    env::var_os("AI_TERMINAL_GUI_SMOKE_FRONTEND_EVIDENCE")?;
    Some(FrontendSmokeConfig {
        delay_milliseconds: smoke_frontend_delay_ms(),
        selection_text: env::var("AI_TERMINAL_GUI_SMOKE_SELECTION_TEXT")
            .unwrap_or_else(|_| "AI_TERMINAL_GUI_SMOKE_SELECTION_TEXT".to_string()),
        paste_text: env::var("AI_TERMINAL_GUI_SMOKE_PASTE_TEXT")
            .unwrap_or_else(|_| "print AI_TERMINAL_GUI_SMOKE_PASTE_OK\r".to_string()),
        paste_expected_output: env::var("AI_TERMINAL_GUI_SMOKE_PASTE_EXPECTED_OUTPUT")
            .unwrap_or_else(|_| "AI_TERMINAL_GUI_SMOKE_PASTE_OK".to_string()),
        scrollback_lines: env::var("AI_TERMINAL_GUI_SMOKE_SCROLLBACK_LINES")
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|lines| (20..=2_000).contains(lines))
            .unwrap_or(120),
    })
}

#[tauri::command]
fn terminal_write_smoke_frontend_evidence(evidence: String) -> Result<(), String> {
    let path = env::var_os("AI_TERMINAL_GUI_SMOKE_FRONTEND_EVIDENCE")
        .map(PathBuf::from)
        .ok_or_else(|| "frontend smoke evidence path is not configured".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(display_error)?;
    }
    fs::write(path, evidence).map_err(display_error)
}

#[tauri::command]
fn runtime_inventory(app: AppHandle, workspace_dir: Option<String>) -> RuntimeInventory {
    RuntimeInventory {
        checked_at_epoch_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default(),
        probes: vec![
            probe_ash(&app),
            probe_wsl_ubuntu(),
            probe_docker(workspace_dir.as_deref()),
            probe_managed_ai_cli("codex", "Codex"),
            probe_managed_ai_cli("claude", "Claude"),
            probe_managed_ai_cli("gemini", "Gemini"),
        ],
    }
}

#[tauri::command]
fn wsl_ubuntu_install() -> Result<String, String> {
    let distro = preferred_ubuntu_distro();
    let mut command = Command::new("wsl.exe");
    command.args(["--install", "-d", &distro]);
    configure_probe_command(&mut command);
    let child = command.spawn().map_err(display_error)?;
    Ok(format!(
        "Started WSL Ubuntu install for {distro} (pid {}). Refresh runtimes after it completes.",
        child.id()
    ))
}

#[tauri::command]
fn apt_package_catalog() -> Vec<AptPackageProbe> {
    let distro = resolve_ubuntu_distro();
    apt_package_definitions()
        .into_iter()
        .map(|definition| match &distro {
            Ok(distro) => {
                let version = apt_package_version(definition.package_name);
                AptPackageProbe {
                    id: definition.id.to_string(),
                    label: definition.label.to_string(),
                    package_name: definition.package_name.to_string(),
                    status: if version.is_some() {
                        "ready"
                    } else {
                        "missing"
                    }
                    .to_string(),
                    detail: if version.is_some() {
                        format!(
                            "{} is installed in managed Ubuntu distro {distro}.",
                            definition.label
                        )
                    } else {
                        format!(
                            "{} is not installed in managed Ubuntu distro {distro}.",
                            definition.label
                        )
                    },
                    version,
                }
            }
            Err(error) => AptPackageProbe {
                id: definition.id.to_string(),
                label: definition.label.to_string(),
                package_name: definition.package_name.to_string(),
                status: "unavailable".to_string(),
                detail: format!("Managed Ubuntu is not ready: {error}"),
                version: None,
            },
        })
        .collect()
}

#[tauri::command]
fn apt_update() -> Result<String, String> {
    let output = run_wsl_bash_probe("sudo -n apt-get update")?;
    if output.success {
        Ok("Updated Ubuntu apt package index.".to_string())
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| "apt-get update failed".to_string()))
    }
}

#[tauri::command]
fn apt_package_install(package_id: String) -> Result<String, String> {
    let definition = apt_package_definition(&package_id)?;
    let script = format!("sudo -n apt-get install -y {}", definition.package_name);
    let output = run_wsl_bash_probe(&script)?;
    if output.success {
        Ok(format!(
            "Installed Ubuntu apt package: {} ({})",
            definition.label, definition.package_name
        ))
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| format!("apt-get install failed for {}", definition.package_name)))
    }
}

#[tauri::command]
fn docker_desktop_install() -> Result<String, String> {
    let mut command = Command::new("winget");
    command.args([
        "install",
        "--exact",
        "--id",
        "Docker.DockerDesktop",
        "--accept-package-agreements",
        "--accept-source-agreements",
    ]);
    configure_probe_command(&mut command);
    let child = command.spawn().map_err(display_error)?;
    Ok(format!(
        "Started Docker Desktop install through winget (pid {}). Refresh runtimes after it completes.",
        child.id()
    ))
}

#[tauri::command]
fn docker_image_pull() -> Result<String, String> {
    let image = preferred_docker_image();
    let output = run_probe("docker", &["pull", &image])?;
    if output.success {
        Ok(format!("Pulled Docker image: {image}"))
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| format!("docker pull failed for {image}")))
    }
}

#[tauri::command]
fn docker_app_catalog(workspace_dir: Option<String>) -> Vec<DockerAppProbe> {
    let engine_ready = docker_engine_ready();
    let workspace_detail = docker_workspace_detail(workspace_dir.as_deref());
    docker_app_definitions()
        .into_iter()
        .map(|definition| {
            let image_ready = engine_ready && docker_image_exists(&definition.image);
            let status = if !engine_ready {
                "unavailable"
            } else if image_ready {
                "ready"
            } else {
                "missing"
            };
            DockerAppProbe {
                id: definition.id.to_string(),
                label: definition.label.to_string(),
                image: definition.image.clone(),
                status: status.to_string(),
                detail: if !engine_ready {
                    format!(
                        "Docker Engine is not reachable. Start Docker Desktop before pulling {}. {workspace_detail}",
                        definition.image,
                    )
                } else if image_ready {
                    format!(
                        "Docker app image is ready: {}. {workspace_detail}",
                        definition.image
                    )
                } else {
                    format!(
                        "Docker app image is missing: {}. {workspace_detail}",
                        definition.image
                    )
                },
                shell: definition.shell,
            }
        })
        .collect()
}

#[tauri::command]
fn docker_app_pull(app_id: String) -> Result<String, String> {
    let definition = docker_app_definition(&app_id)?;
    let output = run_probe("docker", &["pull", &definition.image])?;
    if output.success {
        Ok(format!(
            "Pulled Docker app image for {}: {}",
            definition.label, definition.image
        ))
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| format!("docker pull failed for {}", definition.image)))
    }
}

#[tauri::command]
fn ai_cli_install() -> Result<String, String> {
    run_managed_ai_cli_script(
        &env::var("AI_TERMINAL_AI_CLI_INSTALL_SCRIPT")
            .unwrap_or_else(|_| managed_ai_cli_npm_script("Installing managed AI CLIs")),
        "Installed AI CLIs in managed Ubuntu.",
    )
}

#[tauri::command]
fn ai_cli_update() -> Result<String, String> {
    run_managed_ai_cli_script(
        &env::var("AI_TERMINAL_AI_CLI_UPDATE_SCRIPT")
            .or_else(|_| env::var("AI_TERMINAL_AI_CLI_INSTALL_SCRIPT"))
            .unwrap_or_else(|_| managed_ai_cli_npm_script("Updating managed AI CLIs")),
        "Updated AI CLIs in managed Ubuntu.",
    )
}

fn main() {
    tauri::Builder::default()
        .manage(TerminalState::default())
        .invoke_handler(tauri::generate_handler![
            terminal_open,
            terminal_open_runtime,
            terminal_open_docker_app,
            terminal_write,
            terminal_resize,
            terminal_kill,
            terminal_eof,
            terminal_kill_all,
            terminal_smoke_command,
            terminal_smoke_ctrl_d_delay_ms,
            terminal_smoke_frontend_config,
            terminal_write_smoke_frontend_evidence,
            runtime_inventory,
            wsl_ubuntu_install,
            apt_package_catalog,
            apt_update,
            apt_package_install,
            docker_desktop_install,
            docker_image_pull,
            docker_app_catalog,
            docker_app_pull,
            ai_cli_install,
            ai_cli_update
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AI Terminal");
}

fn get_session(state: &TerminalState, id: &str) -> Result<SharedSession, String> {
    state
        .sessions
        .lock()
        .map_err(|_| "terminal session table is poisoned".to_string())?
        .get(id)
        .cloned()
        .ok_or_else(|| format!("terminal session {id} is not active"))
}

fn kill_session(sessions: &SessionMap, id: &str) -> Result<(), String> {
    let session = sessions
        .lock()
        .map_err(|_| "terminal session table is poisoned".to_string())?
        .remove(id);

    if let Some(session) = session {
        let mut session = session
            .lock()
            .map_err(|_| format!("terminal session {id} is poisoned"))?;
        session.child.kill().map_err(display_error)?;
    }

    Ok(())
}

fn spawn_reader_thread(
    app: AppHandle,
    sessions: SessionMap,
    id: String,
    mut read: Box<dyn FnMut(&mut [u8]) -> std::io::Result<usize> + Send>,
) {
    let thread_id = id.clone();
    let _ = std::thread::Builder::new()
        .name(format!("terminal-reader-{thread_id}"))
        .spawn(move || {
            let mut buffer = [0_u8; 8192];
            let mut status = "exited".to_string();
            let mut transcript = env::var_os("AI_TERMINAL_GUI_SMOKE_TRANSCRIPT")
                .map(PathBuf::from)
                .and_then(|path| {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    OpenOptions::new().create(true).append(true).open(path).ok()
                });
            loop {
                match read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Some(file) = transcript.as_mut() {
                            let _ = file.write_all(&buffer[..n]);
                            let _ = file.flush();
                        }
                        let payload = TerminalData {
                            id: id.clone(),
                            data: String::from_utf8_lossy(&buffer[..n]).into_owned(),
                        };
                        if app.emit("terminal-data", payload).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        status = format!("read error: {error}");
                        break;
                    }
                }
            }

            let _ = sessions.lock().map(|mut sessions| sessions.remove(&id));
            let _ = app.emit("terminal-exit", TerminalExit { id, status });
        });
}

fn schedule_smoke_ctrl_d(sessions: SessionMap, id: String) {
    let Some(delay_ms) = smoke_ctrl_d_delay_ms() else {
        return;
    };

    let _ = std::thread::Builder::new()
        .name(format!("terminal-smoke-ctrl-d-{id}"))
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            let session = sessions
                .lock()
                .ok()
                .and_then(|sessions| sessions.get(&id).cloned());
            let Some(session) = session else {
                return;
            };
            let locked = session.lock();
            if let Ok(mut session) = locked {
                let _ = session.writer.write_all(b"exit\r");
                let _ = session.writer.flush();
            }
        });
}

fn schedule_smoke_ash_integration(sessions: SessionMap, id: String) {
    let Some(delay_ms) = smoke_ash_integration_delay_ms() else {
        return;
    };
    let commands = env::var("AI_TERMINAL_GUI_SMOKE_ASH_INTEGRATION_COMMANDS")
        .ok()
        .map(|value| {
            value
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|commands| !commands.is_empty());
    let Some(commands) = commands else {
        return;
    };
    let interval_ms = smoke_ash_integration_interval_ms();

    let _ = std::thread::Builder::new()
        .name(format!("terminal-smoke-ash-integration-{id}"))
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            let session = sessions
                .lock()
                .ok()
                .and_then(|sessions| sessions.get(&id).cloned());
            let Some(session) = session else {
                return;
            };
            let locked = session.lock();
            if let Ok(mut session) = locked {
                for command in commands {
                    let _ = session.writer.write_all(command.as_bytes());
                    let _ = session.writer.write_all(b"\r");
                    let _ = session.writer.flush();
                    std::thread::sleep(std::time::Duration::from_millis(interval_ms));
                }
            }
        });
}

fn schedule_smoke_ctrl_c(sessions: SessionMap, id: String) {
    let Some(delay_ms) = smoke_ctrl_c_delay_ms() else {
        return;
    };
    let input = env::var("AI_TERMINAL_GUI_SMOKE_CTRL_C_INPUT")
        .unwrap_or_else(|_| "AI_TERMINAL_GUI_SMOKE_CTRL_C_PENDING".to_string());
    let recovery_command = env::var("AI_TERMINAL_GUI_SMOKE_CTRL_C_RECOVERY_COMMAND")
        .unwrap_or_else(|_| "print AI_TERMINAL_GUI_SMOKE_CTRL_C_OK".to_string());

    let _ = std::thread::Builder::new()
        .name(format!("terminal-smoke-ctrl-c-{id}"))
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            let session = sessions
                .lock()
                .ok()
                .and_then(|sessions| sessions.get(&id).cloned());
            let Some(session) = session else {
                return;
            };
            let locked = session.lock();
            if let Ok(mut session) = locked {
                let _ = session.writer.write_all(input.as_bytes());
                let _ = session.writer.flush();
                std::thread::sleep(std::time::Duration::from_millis(250));
                let _ = session.writer.write_all(b"\x03");
                let _ = session.writer.flush();
                std::thread::sleep(std::time::Duration::from_millis(250));
                let _ = session.writer.write_all(recovery_command.as_bytes());
                let _ = session.writer.write_all(b"\r");
                let _ = session.writer.flush();
            }
        });
}

fn smoke_ctrl_c_delay_ms() -> Option<u64> {
    env::var("AI_TERMINAL_GUI_SMOKE_CTRL_C_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|delay| *delay <= 60_000)
}

fn smoke_ctrl_d_delay_ms() -> Option<u64> {
    env::var("AI_TERMINAL_GUI_SMOKE_CTRL_D_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|delay| *delay <= 60_000)
}

fn smoke_ash_integration_delay_ms() -> Option<u64> {
    env::var("AI_TERMINAL_GUI_SMOKE_ASH_INTEGRATION_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|delay| *delay <= 60_000)
}

fn smoke_ash_integration_interval_ms() -> u64 {
    env::var("AI_TERMINAL_GUI_SMOKE_ASH_INTEGRATION_INTERVAL_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|delay| (50..=10_000).contains(delay))
        .unwrap_or(1_200)
}

fn smoke_frontend_delay_ms() -> u32 {
    env::var("AI_TERMINAL_GUI_SMOKE_FRONTEND_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|delay| *delay <= 60_000)
        .unwrap_or(4_200)
}

fn sanitize_size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows: rows.clamp(2, 300),
        cols: cols.clamp(20, 500),
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn resolve_ash_program(app: &AppHandle) -> anyhow::Result<PathBuf> {
    if let Some(path) = env::var_os("AI_TERMINAL_ASH_PATH") {
        return Ok(PathBuf::from(path));
    }

    for base_dir in ash_search_dirs(app) {
        let sidecar = base_dir.join(ash_binary_name());
        if sidecar.exists() {
            return Ok(sidecar);
        }
    }

    Ok(PathBuf::from(ash_binary_name()))
}

fn ash_search_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(exe) = env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            dirs.push(exe_dir.to_path_buf());
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        dirs.push(resource_dir.clone());
        dirs.push(resource_dir.join("bin"));
    }

    dirs
}

fn ash_binary_name() -> &'static str {
    if cfg!(windows) {
        "ash.exe"
    } else {
        "ash"
    }
}

fn preferred_ubuntu_distro() -> String {
    env::var("AI_TERMINAL_UBUNTU_DISTRO")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_UBUNTU_DISTRO.to_string())
}

fn resolve_ubuntu_distro() -> Result<String, String> {
    let preferred = preferred_ubuntu_distro();
    let output = run_probe("wsl.exe", &["--list", "--quiet"])?;
    if !output.success {
        return Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| "wsl.exe did not list installed distributions".to_string()));
    }

    let distros = output
        .stdout
        .lines()
        .map(clean_wsl_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    if distros
        .iter()
        .any(|distro| distro.eq_ignore_ascii_case(&preferred))
    {
        return Ok(preferred);
    }

    distros
        .into_iter()
        .find(|distro| distro.to_ascii_lowercase().contains("ubuntu"))
        .ok_or_else(|| {
            format!("No Ubuntu WSL distribution is installed. Install {preferred} first.")
        })
}

fn wsl_ubuntu_command() -> Result<CommandBuilder, String> {
    let distro = resolve_ubuntu_distro()?;
    let mut command = CommandBuilder::new("wsl.exe");
    command.args(["-d", &distro, "--exec", "bash", "-l"]);
    Ok(command)
}

fn wsl_bash_command(script: &str) -> Result<CommandBuilder, String> {
    let distro = resolve_ubuntu_distro()?;
    let mut command = CommandBuilder::new("wsl.exe");
    command.args(["-d", &distro, "--exec", "bash", "-lc", script]);
    Ok(command)
}

fn run_wsl_bash_probe(script: &str) -> Result<ProbeOutput, String> {
    let distro = resolve_ubuntu_distro()?;
    run_probe("wsl.exe", &["-d", &distro, "--exec", "bash", "-lc", script])
}

fn ai_cli_runtime_command(runtime: &str) -> Result<CommandBuilder, String> {
    let label = match runtime {
        "codex" => "Codex",
        "claude" => "Claude",
        "gemini" => "Gemini",
        _ => return Err(format!("unknown AI CLI runtime: {runtime}")),
    };
    let script = format!(
        r#"export PATH="{MANAGED_NPM_PREFIX}/bin:$PATH"
if ! command -v {runtime} >/dev/null 2>&1; then
  echo "{label} CLI is not installed in managed Ubuntu. Use Install AI CLIs from the ribbon." >&2
  exec bash -l
fi
exec {runtime}
"#
    );
    wsl_bash_command(&script)
}

fn managed_ai_cli_npm_script(action: &str) -> String {
    format!(
        r##"set -e
echo "{action}"
if ! command -v npm >/dev/null 2>&1; then
  echo "npm is missing; installing nodejs/npm through Ubuntu apt"
  sudo -n apt-get update
  sudo -n apt-get install -y nodejs npm
fi
mkdir -p "{MANAGED_NPM_PREFIX}"
npm config set prefix "{MANAGED_NPM_PREFIX}"
export PATH="{MANAGED_NPM_PREFIX}/bin:$PATH"
profile="$HOME/.profile"
start="# >>> ai-terminal ai-cli path >>>"
end="# <<< ai-terminal ai-cli path <<<"
if [ -f "$profile" ] && grep -Fq "$start" "$profile"; then
  :
else
  {{
    printf '\n%s\n' "$start"
    printf 'export PATH="{MANAGED_NPM_PREFIX}/bin:$PATH"\n'
    printf '%s\n' "$end"
  }} >> "$profile"
fi
npm install -g @openai/codex@latest @anthropic-ai/claude-code@latest @google/gemini-cli@latest
codex --version || true
claude --version || true
gemini --version || true
"##
    )
}

fn run_managed_ai_cli_script(script: &str, success_message: &str) -> Result<String, String> {
    let output = run_wsl_bash_probe(script)?;
    if output.success {
        let detail = first_non_empty(&output.stdout, &output.stderr)
            .map(|line| format!(" {line}"))
            .unwrap_or_default();
        Ok(format!("{success_message}{detail}"))
    } else {
        Err(first_non_empty(&output.stderr, &output.stdout)
            .unwrap_or_else(|| "managed AI CLI script failed".to_string()))
    }
}

fn apt_package_definitions() -> Vec<AptPackageDefinition> {
    vec![
        AptPackageDefinition {
            id: "git",
            label: "Git",
            package_name: "git",
        },
        AptPackageDefinition {
            id: "curl",
            label: "curl",
            package_name: "curl",
        },
        AptPackageDefinition {
            id: "build-essential",
            label: "Build Essential",
            package_name: "build-essential",
        },
        AptPackageDefinition {
            id: "python3",
            label: "Python 3",
            package_name: "python3",
        },
        AptPackageDefinition {
            id: "nodejs",
            label: "Node.js",
            package_name: "nodejs",
        },
        AptPackageDefinition {
            id: "npm",
            label: "npm",
            package_name: "npm",
        },
    ]
}

fn apt_package_definition(package_id: &str) -> Result<AptPackageDefinition, String> {
    apt_package_definitions()
        .into_iter()
        .find(|definition| definition.id == package_id)
        .ok_or_else(|| format!("unknown apt package: {package_id}"))
}

fn apt_package_version(package_name: &str) -> Option<String> {
    let script = format!("dpkg-query -W -f='${{Version}}' {package_name}");
    run_wsl_bash_probe(&script)
        .ok()
        .filter(|output| output.success)
        .and_then(|output| first_non_empty(&output.stdout, &output.stderr))
}

fn preferred_docker_image() -> String {
    env::var("AI_TERMINAL_DOCKER_IMAGE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_DOCKER_IMAGE.to_string())
}

fn preferred_docker_shell() -> Vec<String> {
    env::var("AI_TERMINAL_DOCKER_SHELL")
        .ok()
        .map(|value| {
            value
                .split_whitespace()
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|parts| !parts.is_empty())
        .unwrap_or_else(|| vec!["bash".to_string(), "-l".to_string()])
}

fn docker_image_exists(image: &str) -> bool {
    run_probe(
        "docker",
        &["image", "inspect", image, "--format", "{{.Id}}"],
    )
    .map(|output| output.success)
    .unwrap_or(false)
}

fn docker_runtime_command(workspace_dir: Option<&str>) -> Result<CommandBuilder, String> {
    let image = preferred_docker_image();
    if !docker_engine_ready() {
        return Err("Docker Engine is not reachable. Start Docker Desktop first.".to_string());
    }
    if !docker_image_exists(&image) {
        return Err(format!(
            "Docker image {image} is not present. Use Pull Image before starting Docker runtime."
        ));
    }

    let shell = preferred_docker_shell();
    let mut command = CommandBuilder::new("docker");
    command.args(["run", "--rm", "-it"]);
    add_docker_workspace_args(&mut command, workspace_dir)?;
    command.arg(&image);
    command.args(shell);
    Ok(command)
}

fn docker_app_definitions() -> Vec<DockerAppDefinition> {
    vec![
        DockerAppDefinition {
            id: "ubuntu-base",
            label: "Ubuntu Base",
            image: preferred_docker_image(),
            shell: vec!["bash".to_string(), "-l".to_string()],
        },
        DockerAppDefinition {
            id: "node-dev",
            label: "Node.js Dev",
            image: env::var("AI_TERMINAL_DOCKER_APP_NODE_IMAGE")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "node:22-bookworm".to_string()),
            shell: vec!["bash".to_string(), "-l".to_string()],
        },
        DockerAppDefinition {
            id: "python-dev",
            label: "Python Dev",
            image: env::var("AI_TERMINAL_DOCKER_APP_PYTHON_IMAGE")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "python:3.12-bookworm".to_string()),
            shell: vec!["bash".to_string(), "-l".to_string()],
        },
        DockerAppDefinition {
            id: "rust-dev",
            label: "Rust Dev",
            image: env::var("AI_TERMINAL_DOCKER_APP_RUST_IMAGE")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "rust:1-bookworm".to_string()),
            shell: vec!["bash".to_string(), "-l".to_string()],
        },
    ]
}

fn docker_app_definition(app_id: &str) -> Result<DockerAppDefinition, String> {
    docker_app_definitions()
        .into_iter()
        .find(|definition| definition.id == app_id)
        .ok_or_else(|| format!("unknown Docker app: {app_id}"))
}

fn docker_app_runtime_command(
    app_id: &str,
    workspace_dir: Option<&str>,
) -> Result<CommandBuilder, String> {
    let definition = docker_app_definition(app_id)?;
    if !docker_engine_ready() {
        return Err("Docker Engine is not reachable. Start Docker Desktop first.".to_string());
    }
    if !docker_image_exists(&definition.image) {
        return Err(format!(
            "Docker app image {} is not present. Use Pull App before starting {}.",
            definition.image, definition.label
        ));
    }

    let mut command = CommandBuilder::new("docker");
    command.args(["run", "--rm", "-it"]);
    add_docker_workspace_args(&mut command, workspace_dir)?;
    command.arg(&definition.image);
    command.args(definition.shell);
    Ok(command)
}

fn add_docker_workspace_args(
    command: &mut CommandBuilder,
    workspace_dir: Option<&str>,
) -> Result<(), String> {
    let Some(source) = docker_workspace_source(workspace_dir)? else {
        return Ok(());
    };

    command.arg("--mount");
    command.arg(format!(
        "type=bind,source={},target={DOCKER_WORKSPACE_TARGET}",
        source.display()
    ));
    command.args(["--workdir", DOCKER_WORKSPACE_TARGET]);
    Ok(())
}

fn docker_workspace_source(workspace_dir: Option<&str>) -> Result<Option<PathBuf>, String> {
    if !docker_workspace_mount_enabled() {
        return Ok(None);
    }

    let source = workspace_dir
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::var_os("AI_TERMINAL_WORKSPACE_DIR").map(PathBuf::from))
        .map(Ok)
        .unwrap_or_else(env::current_dir)
        .map_err(display_error)?;
    let source = if source.is_absolute() {
        source
    } else {
        env::current_dir().map_err(display_error)?.join(source)
    };
    if !source.is_dir() {
        return Err(format!(
            "Docker workspace source is not a directory: {}",
            source.display()
        ));
    }
    Ok(Some(source))
}

fn docker_workspace_mount_enabled() -> bool {
    env::var("AI_TERMINAL_DOCKER_WORKSPACE")
        .ok()
        .map(|value| {
            !matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "no" | "off"
            )
        })
        .unwrap_or(true)
}

fn docker_workspace_detail(workspace_dir: Option<&str>) -> String {
    match docker_workspace_source(workspace_dir) {
        Ok(Some(source)) => format!(
            "Workspace mount: {} -> {DOCKER_WORKSPACE_TARGET}.",
            source.display()
        ),
        Ok(None) => "Workspace mount disabled.".to_string(),
        Err(error) => format!("Workspace mount unavailable: {error}"),
    }
}

fn probe_ash(app: &AppHandle) -> RuntimeProbe {
    match resolve_ash_program(app) {
        Ok(path) => {
            let exists = path.exists();
            RuntimeProbe {
                id: "ash".to_string(),
                label: "ash".to_string(),
                status: if exists { "ready" } else { "unknown" }.to_string(),
                detail: if exists {
                    "Bundled ash sidecar was found.".to_string()
                } else {
                    "ash will be resolved from PATH when the terminal opens.".to_string()
                },
                version: None,
                path: Some(path.display().to_string()),
            }
        }
        Err(error) => RuntimeProbe {
            id: "ash".to_string(),
            label: "ash".to_string(),
            status: "unavailable".to_string(),
            detail: error.to_string(),
            version: None,
            path: None,
        },
    }
}

fn probe_wsl_ubuntu() -> RuntimeProbe {
    let path = find_program_path("wsl.exe");
    let preferred = preferred_ubuntu_distro();
    let status = match run_probe("wsl.exe", &["--status"]) {
        Ok(output) if output.success => output,
        Ok(output) => {
            return RuntimeProbe {
                id: "ubuntu".to_string(),
                label: "Ubuntu".to_string(),
                status: "unavailable".to_string(),
                detail: first_non_empty(&output.stderr, &output.stdout).unwrap_or_else(|| {
                    "wsl.exe is installed but did not report a usable status.".to_string()
                }),
                version: None,
                path,
            };
        }
        Err(error) => {
            return RuntimeProbe {
                id: "ubuntu".to_string(),
                label: "Ubuntu".to_string(),
                status: "unavailable".to_string(),
                detail: error,
                version: None,
                path,
            };
        }
    };

    let distros = run_probe("wsl.exe", &["--list", "--verbose"]).ok();
    let ubuntu_line = distros
        .as_ref()
        .and_then(|output| find_ubuntu_distro_line(&output.stdout));
    let distro_detail = ubuntu_line.as_ref().map(|line| {
        if line
            .to_ascii_lowercase()
            .contains(&preferred.to_ascii_lowercase())
        {
            format!("Managed distro ready: {line}")
        } else {
            format!("Ubuntu distro ready: {line}")
        }
    });
    RuntimeProbe {
        id: "ubuntu".to_string(),
        label: "Ubuntu".to_string(),
        status: if ubuntu_line.is_some() {
            "ready"
        } else {
            "missing"
        }
        .to_string(),
        detail: distro_detail.unwrap_or_else(|| {
            first_non_empty(&status.stdout, &status.stderr)
                .map(|line| {
                    format!(
                        "WSL is available, but no Ubuntu distro was found. Preferred distro: {preferred}. {line}"
                    )
                })
                .unwrap_or_else(|| {
                    format!(
                        "WSL is available, but no Ubuntu distro was found. Preferred distro: {preferred}."
                    )
                })
        }),
        version: extract_wsl_version(&status.stdout),
        path,
    }
}

fn probe_docker(workspace_dir: Option<&str>) -> RuntimeProbe {
    let path = find_program_path("docker");
    let image = preferred_docker_image();
    let workspace_detail = docker_workspace_detail(workspace_dir);
    match run_probe("docker", &["--version"]) {
        Ok(output) if output.success => {
            let version = first_non_empty(&output.stdout, &output.stderr);
            let engine = docker_engine_version();
            if engine.is_none() {
                return RuntimeProbe {
                    id: "docker".to_string(),
                    label: "Docker".to_string(),
                    status: "unavailable".to_string(),
                    detail: format!(
                        "Docker CLI is available, but Docker Engine is not reachable. Start Docker Desktop. Managed image: {image}. {workspace_detail}"
                    ),
                    version,
                    path,
                };
            }

            let image_ready = docker_image_exists(&image);
            RuntimeProbe {
                id: "docker".to_string(),
                label: "Docker".to_string(),
                status: if image_ready { "ready" } else { "missing" }.to_string(),
                detail: if image_ready {
                    format!(
                        "Docker Engine is reachable. Managed image is ready: {image}. {workspace_detail}"
                    )
                } else {
                    format!(
                        "Docker Engine is reachable. Managed image is missing: {image}. {workspace_detail}"
                    )
                },
                version: engine.or(version),
                path,
            }
        }
        Ok(output) => RuntimeProbe {
            id: "docker".to_string(),
            label: "Docker".to_string(),
            status: "unavailable".to_string(),
            detail: first_non_empty(&output.stderr, &output.stdout)
                .unwrap_or_else(|| "Docker CLI did not return a version.".to_string()),
            version: None,
            path,
        },
        Err(error) => RuntimeProbe {
            id: "docker".to_string(),
            label: "Docker".to_string(),
            status: "unavailable".to_string(),
            detail: error,
            version: None,
            path,
        },
    }
}

fn docker_engine_ready() -> bool {
    docker_engine_version().is_some()
}

fn docker_engine_version() -> Option<String> {
    run_probe("docker", &["info", "--format", "{{.ServerVersion}}"])
        .ok()
        .filter(|output| output.success)
        .and_then(|output| first_non_empty(&output.stdout, &output.stderr))
}

fn probe_managed_ai_cli(command: &str, label: &str) -> RuntimeProbe {
    let distro = match resolve_ubuntu_distro() {
        Ok(distro) => distro,
        Err(error) => {
            return RuntimeProbe {
                id: command.to_string(),
                label: label.to_string(),
                status: "missing".to_string(),
                detail: format!("Managed Ubuntu is not ready: {error}"),
                version: None,
                path: None,
            };
        }
    };

    let script = format!(
        r#"export PATH="{MANAGED_NPM_PREFIX}/bin:$PATH"
if command -v {command} >/dev/null 2>&1; then
  printf '__AI_TERMINAL_PATH__%s\n' "$(command -v {command})"
  {command} --version
else
  exit 127
fi
"#
    );

    match run_probe("wsl.exe", &["-d", &distro, "--exec", "bash", "-lc", &script]) {
        Ok(output) if output.success => RuntimeProbe {
            id: command.to_string(),
            label: label.to_string(),
            status: "ready".to_string(),
            detail: format!("{label} CLI is installed in managed Ubuntu distro {distro}."),
            version: ai_cli_probe_version(&output.stdout).or_else(|| first_non_empty(&output.stderr, "")),
            path: ai_cli_probe_path(&output.stdout),
        },
        Ok(output) => RuntimeProbe {
            id: command.to_string(),
            label: label.to_string(),
            status: "missing".to_string(),
            detail: first_non_empty(&output.stderr, &output.stdout).unwrap_or_else(|| {
                format!(
                    "{label} CLI is not installed in managed Ubuntu distro {distro}. Use Install AI CLIs."
                )
            }),
            version: None,
            path: None,
        },
        Err(error) => RuntimeProbe {
            id: command.to_string(),
            label: label.to_string(),
            status: "missing".to_string(),
            detail: format!("{label} CLI probe failed in managed Ubuntu distro {distro}: {error}"),
            version: None,
            path: None,
        },
    }
}

fn ai_cli_probe_path(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        line.strip_prefix("__AI_TERMINAL_PATH__")
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty())
    })
}

fn ai_cli_probe_version(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("__AI_TERMINAL_PATH__"))
        .map(ToOwned::to_owned)
}

fn run_probe(program: &str, args: &[&str]) -> Result<ProbeOutput, String> {
    let mut command = Command::new(program);
    command.args(args);
    configure_probe_command(&mut command);
    let output = command.output().map_err(display_error)?;
    Ok(ProbeOutput {
        success: output.status.success(),
        stdout: decode_process_output(&output.stdout),
        stderr: decode_process_output(&output.stderr),
    })
}

#[cfg(windows)]
fn configure_probe_command(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_probe_command(_command: &mut Command) {}

fn find_program_path(program: &str) -> Option<String> {
    let finder = if cfg!(windows) { "where.exe" } else { "which" };
    run_probe(finder, &[program])
        .ok()
        .filter(|output| output.success)
        .and_then(|output| first_non_empty(&output.stdout, &output.stderr))
}

fn decode_process_output(bytes: &[u8]) -> String {
    if bytes.len() >= 4 {
        let zeros = bytes.iter().filter(|byte| **byte == 0).count();
        if zeros > bytes.len() / 4 {
            let utf16 = bytes
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<_>>();
            return String::from_utf16_lossy(&utf16);
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn first_non_empty(primary: &str, fallback: &str) -> Option<String> {
    primary
        .lines()
        .chain(fallback.lines())
        .map(clean_wsl_line)
        .find(|line| !line.is_empty())
}

fn find_ubuntu_distro_line(output: &str) -> Option<String> {
    output
        .lines()
        .map(clean_wsl_line)
        .map(|line| line.trim_start_matches('*').trim().to_string())
        .find(|line| line.to_ascii_lowercase().contains("ubuntu"))
}

fn clean_wsl_line(line: &str) -> String {
    line.trim().trim_matches('\0').to_string()
}

fn extract_wsl_version(output: &str) -> Option<String> {
    output
        .lines()
        .map(clean_wsl_line)
        .find(|line| line.to_ascii_lowercase().contains("version"))
}

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
