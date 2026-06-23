//! 외부 명령 실행 어댑터.
//!
//! `shellcore`는 파서/평가기/구조화 값 모델을 모바일 UI에도 임베드해야 하므로,
//! OS process spawn은 trait 경계 뒤에 둔다. 데스크톱 `ash`는 기본 runner를 쓰고,
//! 모바일/테스트는 `DisabledRunner`로 외부 실행을 명시적으로 막는다.

use anyhow::{bail, Result};
use std::path::Path;

use crate::shellcore::value::Value;
#[cfg(windows)]
use crate::shellcore::winexec::{self, WindowsInvocation};

/// 플랫폼 실행 capability. 이 값은 제품/문서 매트릭스와 같은 의미를 갖는다.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionCapabilities {
    pub can_spawn: bool,
    pub has_pty: bool,
    pub has_conpty: bool,
    pub has_userland: bool,
    pub can_write_workspace: bool,
    pub can_network: bool,
}

impl ExecutionCapabilities {
    pub const fn pure_core() -> Self {
        Self {
            can_spawn: false,
            has_pty: false,
            has_conpty: false,
            has_userland: false,
            can_write_workspace: false,
            can_network: false,
        }
    }

    pub const fn desktop_process() -> Self {
        Self {
            can_spawn: true,
            has_pty: false,
            has_conpty: false,
            has_userland: true,
            can_write_workspace: true,
            can_network: true,
        }
    }
}

pub struct ExternalCommand<'a> {
    pub name: &'a str,
    pub args: &'a [Value],
    pub cwd: &'a Path,
}

pub trait ExternalRunner {
    fn capabilities(&self) -> ExecutionCapabilities;
    fn run(&self, command: ExternalCommand<'_>) -> Result<Value>;
}

#[derive(Debug, Default)]
pub struct DesktopRunner;

impl ExternalRunner for DesktopRunner {
    fn capabilities(&self) -> ExecutionCapabilities {
        ExecutionCapabilities::desktop_process()
    }

    /// 외부 명령을 셸 cwd·현재 env로 실행한다. stdout/stderr는 터미널로 통과.
    /// 반환은 Nothing. 비0 종료는 안내만 하고 에러로 만들지 않는다(REPL 지속).
    fn run(&self, command: ExternalCommand<'_>) -> Result<Value> {
        run_desktop_command(command)
    }
}

#[cfg(not(windows))]
fn run_desktop_command(command: ExternalCommand<'_>) -> Result<Value> {
    use std::process::Command;
    let arg_strs: Vec<String> = command.args.iter().map(|v| v.coerce_string()).collect();
    match Command::new(command.name)
        .args(&arg_strs)
        .current_dir(command.cwd)
        .status()
    {
        Ok(st) => {
            if !st.success() {
                let code = st
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into());
                eprintln!("[{}: exit {code}]", command.name);
            }
            Ok(Value::Nothing)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("command not found: {}", command.name)
        }
        Err(e) => bail!("failed to run {}: {e}", command.name),
    }
}

#[cfg(windows)]
fn run_desktop_command(command: ExternalCommand<'_>) -> Result<Value> {
    use std::process::Command;

    let arg_strs: Vec<String> = command.args.iter().map(|v| v.coerce_string()).collect();
    let path_dirs = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    let pathext_raw = std::env::var("PATHEXT").ok();
    let pathext = winexec::split_pathext(pathext_raw.as_deref());
    let invocation =
        winexec::resolve_windows_invocation(command.name, command.cwd, &path_dirs, &pathext)
            .ok_or_else(|| anyhow::anyhow!("command not found: {}", command.name))?;

    let mut cmd = match invocation {
        WindowsInvocation::Direct(path) => {
            let mut cmd = Command::new(path);
            cmd.args(&arg_strs);
            cmd
        }
        WindowsInvocation::CmdScript(path) => {
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/d").arg("/c").arg(path);
            cmd.args(&arg_strs);
            cmd
        }
        WindowsInvocation::PowerShellScript(path) => {
            let mut cmd = Command::new("powershell.exe");
            cmd.arg("-NoProfile")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-File")
                .arg(path);
            cmd.args(&arg_strs);
            cmd
        }
    };

    match cmd.current_dir(command.cwd).status() {
        Ok(st) => {
            if !st.success() {
                let code = st
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into());
                eprintln!("[{}: exit {code}]", command.name);
            }
            Ok(Value::Nothing)
        }
        Err(e) => bail!("failed to run {}: {e}", command.name),
    }
}

#[derive(Debug, Default)]
pub struct DisabledRunner;

impl ExternalRunner for DisabledRunner {
    fn capabilities(&self) -> ExecutionCapabilities {
        ExecutionCapabilities::pure_core()
    }

    fn run(&self, command: ExternalCommand<'_>) -> Result<Value> {
        bail!("external execution disabled: {}", command.name)
    }
}
