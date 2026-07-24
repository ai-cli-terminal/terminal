use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::runtimes::display_error;
use crate::types::{FrontendSmokeConfig, SessionMap};

#[tauri::command]
pub(crate) fn terminal_smoke_command() -> Option<String> {
    env::var("AI_TERMINAL_GUI_SMOKE_COMMAND")
        .ok()
        .filter(|command| !command.trim().is_empty())
}

#[tauri::command]
pub(crate) fn terminal_smoke_runtime() -> Result<Option<String>, String> {
    let Some(runtime) = env::var("AI_TERMINAL_GUI_SMOKE_RUNTIME")
        .ok()
        .map(|runtime| runtime.trim().to_string())
        .filter(|runtime| !runtime.is_empty())
    else {
        return Ok(None);
    };

    match runtime.as_str() {
        "ash" | "ubuntu" | "powershell" | "docker" | "codex" | "claude" | "gemini" => {
            Ok(Some(runtime))
        }
        _ => Err(format!("unsupported GUI smoke runtime: {runtime}")),
    }
}

#[tauri::command]
pub(crate) fn terminal_smoke_ctrl_d_delay_ms() -> Option<u32> {
    smoke_ctrl_d_delay_ms().map(|delay| delay as u32)
}

#[tauri::command]
pub(crate) fn terminal_smoke_frontend_config() -> Option<FrontendSmokeConfig> {
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
pub(crate) fn terminal_write_smoke_frontend_evidence(evidence: String) -> Result<(), String> {
    let path = env::var_os("AI_TERMINAL_GUI_SMOKE_FRONTEND_EVIDENCE")
        .map(PathBuf::from)
        .ok_or_else(|| "frontend smoke evidence path is not configured".to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(display_error)?;
    }
    fs::write(path, evidence).map_err(display_error)
}

pub(crate) fn schedule_smoke_ctrl_d(sessions: SessionMap, id: String) {
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

pub(crate) fn schedule_smoke_ash_integration(sessions: SessionMap, id: String) {
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

pub(crate) fn schedule_smoke_ctrl_c(sessions: SessionMap, id: String) {
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
