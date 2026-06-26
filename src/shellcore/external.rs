//! 외부 명령 실행 어댑터.
//!
//! `shellcore`는 파서/평가기/구조화 값 모델을 모바일 UI에도 임베드해야 하므로,
//! OS process spawn은 trait 경계 뒤에 둔다. 데스크톱 `ash`는 기본 runner를 쓰고,
//! 모바일/테스트는 `DisabledRunner`로 외부 실행을 명시적으로 막는다.

use anyhow::{bail, Result};
use std::path::Path;

use crate::shellcore::value::Value;

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

/// argv를 cwd/현재 env로 stdio 상속 spawn하고 exit code를 반환한다(None=시그널 종료).
/// 출력하지 않는다(호출측이 결과 처리). NotFound는 "command not found"로 bail.
#[cfg(not(windows))]
pub fn spawn_inherit(name: &str, args: &[String], cwd: &Path) -> Result<Option<i32>> {
    use std::process::Command;
    match Command::new(name).args(args).current_dir(cwd).status() {
        Ok(st) => Ok(st.code()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("command not found: {name}")
        }
        Err(e) => bail!("failed to run {name}: {e}"),
    }
}

/// Windows: winexec로 .exe/.cmd/.ps1 해석 후 argv 직접 spawn(stdio 상속).
#[cfg(windows)]
pub fn spawn_inherit(name: &str, args: &[String], cwd: &Path) -> Result<Option<i32>> {
    use crate::shellcore::winexec;
    use std::process::Command;
    let path_dirs = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    let pathext_raw = std::env::var("PATHEXT").ok();
    let pathext = winexec::split_pathext(pathext_raw.as_deref());
    let invocation = winexec::resolve_windows_invocation(name, cwd, &path_dirs, &pathext)
        .ok_or_else(|| anyhow::anyhow!("command not found: {name}"))?;
    let plan = winexec::spawn_plan(invocation, args);
    let mut cmd = Command::new(plan.program);
    cmd.args(plan.args);
    match cmd.current_dir(cwd).status() {
        Ok(st) => Ok(st.code()),
        Err(e) => bail!("failed to run {name}: {e}"),
    }
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
        let args: Vec<String> = command.args.iter().map(|v| v.coerce_string()).collect();
        match spawn_inherit(command.name, &args, command.cwd)? {
            Some(0) => {}
            Some(code) => eprintln!("[{}: exit {code}]", command.name),
            None => eprintln!("[{}: exit signal]", command.name),
        }
        Ok(Value::Nothing)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn spawn_inherit_returns_exit_code() {
        let cwd = std::env::temp_dir();
        let code =
            spawn_inherit("/bin/sh", &["-c".to_string(), "exit 7".to_string()], &cwd).unwrap();
        assert_eq!(code, Some(7));
    }

    #[cfg(not(windows))]
    #[test]
    fn spawn_inherit_missing_command_errs() {
        let cwd = std::env::temp_dir();
        assert!(spawn_inherit("definitely_not_a_real_cmd_zzz", &[], &cwd).is_err());
    }
}
