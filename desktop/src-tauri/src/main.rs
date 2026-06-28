#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    env,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

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

#[tauri::command]
fn terminal_open(
    app: AppHandle,
    state: State<'_, TerminalState>,
    rows: u16,
    cols: u16,
) -> Result<String, String> {
    let id = format!("term-{}", NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed));
    let size = sanitize_size(rows, cols);
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(size).map_err(display_error)?;
    let mut reader = pair.master.try_clone_reader().map_err(display_error)?;
    let writer = pair.master.take_writer().map_err(display_error)?;

    let mut command = CommandBuilder::new(resolve_ash_program(&app).map_err(display_error)?);
    command.env("AI_TERMINAL_GUI", "1");
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

    schedule_smoke_ash_integration(state.sessions.clone(), id.clone());
    schedule_smoke_ctrl_c(state.sessions.clone(), id.clone());
    schedule_smoke_ctrl_d(state.sessions.clone(), id.clone());

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

fn main() {
    tauri::Builder::default()
        .manage(TerminalState::default())
        .invoke_handler(tauri::generate_handler![
            terminal_open,
            terminal_write,
            terminal_resize,
            terminal_kill,
            terminal_eof,
            terminal_kill_all,
            terminal_smoke_command,
            terminal_smoke_ctrl_d_delay_ms,
            terminal_smoke_frontend_config,
            terminal_write_smoke_frontend_evidence
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

fn display_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
