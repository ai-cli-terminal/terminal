use std::{
    env,
    fs::OpenOptions,
    io::{Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::runtimes::{
    ai_cli_runtime_command, display_error, docker_app_runtime_command, docker_runtime_command,
    wsl_ubuntu_command,
};
use crate::smoke::{schedule_smoke_ash_integration, schedule_smoke_ctrl_c, schedule_smoke_ctrl_d};
use crate::types::{SessionMap, SharedSession, TerminalData, TerminalExit, TerminalSession, TerminalState};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

#[tauri::command]
pub(crate) fn terminal_open(
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
pub(crate) fn terminal_open_runtime(
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
            let mut command = wsl_ubuntu_command(workspace_dir.as_deref())?;
            command.env("TERM", "xterm-256color");
            open_terminal_session(app, state, rows, cols, command, false)
        }
        "docker" => {
            let mut command = docker_runtime_command(workspace_dir.as_deref())?;
            command.env("TERM", "xterm-256color");
            open_terminal_session(app, state, rows, cols, command, false)
        }
        "codex" | "claude" | "gemini" => {
            let mut command = ai_cli_runtime_command(&runtime, workspace_dir.as_deref())?;
            command.env("TERM", "xterm-256color");
            open_terminal_session(app, state, rows, cols, command, false)
        }
        _ => Err(format!("unknown runtime: {runtime}")),
    }
}

#[tauri::command]
pub(crate) fn terminal_open_docker_app(
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
pub(crate) fn terminal_write(state: State<'_, TerminalState>, id: String, data: String) -> Result<(), String> {
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
pub(crate) fn terminal_resize(
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
pub(crate) fn terminal_kill(state: State<'_, TerminalState>, id: String) -> Result<(), String> {
    kill_session(&state.sessions, &id)
}

#[tauri::command]
pub(crate) fn terminal_eof(state: State<'_, TerminalState>, id: String) -> Result<(), String> {
    let session = get_session(&state, &id)?;
    let mut session = session
        .lock()
        .map_err(|_| format!("terminal session {id} is poisoned"))?;
    session.writer.write_all(b"exit\r").map_err(display_error)?;
    session.writer.flush().map_err(display_error)
}

#[tauri::command]
pub(crate) fn terminal_kill_all(state: State<'_, TerminalState>) -> Result<(), String> {
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

fn sanitize_size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows: rows.clamp(2, 300),
        cols: cols.clamp(20, 500),
        pixel_width: 0,
        pixel_height: 0,
    }
}

pub(crate) fn resolve_ash_program(app: &AppHandle) -> anyhow::Result<PathBuf> {
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
