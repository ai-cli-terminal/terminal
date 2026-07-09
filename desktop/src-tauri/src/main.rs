#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod runtimes;
mod smoke;
mod terminal_cmds;
mod types;

use crate::types::TerminalState;

fn main() {
    tauri::Builder::default()
        .manage(TerminalState::default())
        .invoke_handler(tauri::generate_handler![
            terminal_cmds::terminal_open,
            terminal_cmds::terminal_open_runtime,
            terminal_cmds::terminal_open_docker_app,
            terminal_cmds::terminal_write,
            terminal_cmds::terminal_resize,
            terminal_cmds::terminal_kill,
            terminal_cmds::terminal_eof,
            terminal_cmds::terminal_kill_all,
            smoke::terminal_smoke_command,
            smoke::terminal_smoke_ctrl_d_delay_ms,
            smoke::terminal_smoke_frontend_config,
            smoke::terminal_write_smoke_frontend_evidence,
            runtimes::runtime_inventory,
            runtimes::workspace_probe,
            runtimes::wsl_ubuntu_install,
            runtimes::apt_package_catalog,
            runtimes::apt_update,
            runtimes::apt_package_install,
            runtimes::docker_desktop_install,
            runtimes::docker_image_pull,
            runtimes::docker_app_catalog,
            runtimes::docker_app_pull,
            runtimes::ai_cli_install,
            runtimes::ai_cli_update
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AI Terminal");
}
