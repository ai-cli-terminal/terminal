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
) -> Result<String, String> {
    match runtime.as_str() {
        "ash" => terminal_open(app, state, rows, cols),
        "ubuntu" => {
            let mut command = wsl_ubuntu_command()?;
            command.env("TERM", "xterm-256color");
            open_terminal_session(app, state, rows, cols, command, false)
        }
        "docker" | "codex" | "claude" | "gemini" => Err(format!(
            "{runtime} runtime execution is not wired yet; Ubuntu runtime is the current S3 slice"
        )),
        _ => Err(format!("unknown runtime: {runtime}")),
    }
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
fn runtime_inventory(app: AppHandle) -> RuntimeInventory {
    RuntimeInventory {
        checked_at_epoch_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default(),
        probes: vec![
            probe_ash(&app),
            probe_wsl_ubuntu(),
            probe_docker(),
            probe_cli("codex", "Codex"),
            probe_cli("claude", "Claude"),
            probe_cli("gemini", "Gemini"),
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

fn main() {
    tauri::Builder::default()
        .manage(TerminalState::default())
        .invoke_handler(tauri::generate_handler![
            terminal_open,
            terminal_open_runtime,
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
            wsl_ubuntu_install
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

fn probe_docker() -> RuntimeProbe {
    let path = find_program_path("docker");
    match run_probe("docker", &["--version"]) {
        Ok(output) if output.success => {
            let version = first_non_empty(&output.stdout, &output.stderr);
            RuntimeProbe {
                id: "docker".to_string(),
                label: "Docker".to_string(),
                status: "ready".to_string(),
                detail: "Docker CLI is available.".to_string(),
                version,
                path,
            }
        }
        Ok(output) => RuntimeProbe {
            id: "docker".to_string(),
            label: "Docker".to_string(),
            status: "missing".to_string(),
            detail: first_non_empty(&output.stderr, &output.stdout)
                .unwrap_or_else(|| "Docker CLI did not return a version.".to_string()),
            version: None,
            path,
        },
        Err(error) => RuntimeProbe {
            id: "docker".to_string(),
            label: "Docker".to_string(),
            status: "missing".to_string(),
            detail: error,
            version: None,
            path,
        },
    }
}

fn probe_cli(command: &str, label: &str) -> RuntimeProbe {
    let path = find_program_path(command);
    match run_probe(command, &["--version"]) {
        Ok(output) if output.success => RuntimeProbe {
            id: command.to_string(),
            label: label.to_string(),
            status: "ready".to_string(),
            detail: format!("{label} CLI is available on the host PATH."),
            version: first_non_empty(&output.stdout, &output.stderr),
            path,
        },
        Ok(output) => RuntimeProbe {
            id: command.to_string(),
            label: label.to_string(),
            status: "missing".to_string(),
            detail: first_non_empty(&output.stderr, &output.stdout)
                .unwrap_or_else(|| format!("{label} CLI did not return a version.")),
            version: None,
            path,
        },
        Err(error) => RuntimeProbe {
            id: command.to_string(),
            label: label.to_string(),
            status: "missing".to_string(),
            detail: format!("{label} CLI not found on host PATH: {error}"),
            version: None,
            path,
        },
    }
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
