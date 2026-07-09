use std::path::PathBuf;

#[cfg(feature = "storage")]
use ai_terminal::risk;
use ai_terminal::shell::{self, Shell};
use crate::cli::command::InitMode;

/// rc 수정 계획(파일 I/O와 분리해 테스트 가능하게).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InitPlan {
    pub(crate) new_content: String,
    pub(crate) write: bool,
    pub(crate) message: String,
}

/// rc 내용·셸·모드로부터 수정 계획을 산출한다(순수 함수).
pub(crate) fn plan_init_shell(old: &str, shell: Shell, mode: InitMode, path: &str) -> InitPlan {
    match mode {
        InitMode::Install => {
            if shell::is_installed(old) {
                InitPlan {
                    new_content: old.to_string(),
                    write: false,
                    message: format!("이미 설치됨: {path}\n"),
                }
            } else {
                let new_content = shell::apply_install(old, shell);
                InitPlan {
                    new_content,
                    write: true,
                    message: format!("설치 완료: {path} ({} hook)\n", shell.as_str()),
                }
            }
        }
        InitMode::DryRun => {
            let new_content = shell::apply_install(old, shell);
            InitPlan {
                new_content,
                write: false,
                message: format!(
                    "[dry-run] {path} 를 수정하지 않았습니다. 변경 내용은 `--diff`로 확인하세요.\n"
                ),
            }
        }
        InitMode::Diff => {
            let new_content = shell::apply_install(old, shell);
            let message = shell::unified_diff(old, &new_content, path);
            InitPlan {
                new_content,
                write: false,
                message,
            }
        }
        InitMode::Uninstall => {
            if shell::is_installed(old) {
                let new_content = shell::apply_uninstall(old);
                InitPlan {
                    new_content,
                    write: true,
                    message: format!("제거 완료: {path} 에서 통합 블록 삭제\n"),
                }
            } else {
                InitPlan {
                    new_content: old.to_string(),
                    write: false,
                    message: format!("제거할 통합 블록이 없습니다: {path}\n"),
                }
            }
        }
    }
}

/// `--shell` 또는 `$SHELL`에서 셸을 결정한다(기본 bash).
pub(crate) fn resolve_shell(opt: Option<&str>) -> anyhow::Result<Shell> {
    if let Some(s) = opt {
        return Shell::parse(s).ok_or_else(|| anyhow::anyhow!("unsupported shell: {s} (bash|zsh)"));
    }
    if let Ok(env_shell) = std::env::var("SHELL") {
        if let Some(s) = Shell::parse(&env_shell) {
            return Ok(s);
        }
    }
    Ok(Shell::Bash)
}

/// rc 파일 경로를 결정한다(`--rc` 또는 `$HOME/<기본 rc>`).
pub(crate) fn resolve_rc(opt: Option<PathBuf>, shell: Shell) -> anyhow::Result<PathBuf> {
    if let Some(p) = opt {
        return Ok(p);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("HOME not set; use --rc <path>"))?;
    Ok(home.join(shell.default_rc_filename()))
}

/// 셸 hook의 `preexec` 이벤트를 받아 명령을 기본 세션에 기록한다(best-effort).
///
/// 위험도를 함께 산출해 저장한다. 재진입(자기 자신의 `ai __hook` 호출)은 건너뛴다.
#[cfg(feature = "storage")]
pub(crate) fn record_hook_preexec(rest: &[String]) -> anyhow::Result<()> {
    use ai_terminal::store::{NewCommand, NewSession, Store};

    let kv = |k: &str| -> Option<String> {
        let pre = format!("{k}=");
        rest.iter()
            .find_map(|s| s.strip_prefix(&pre))
            .map(String::from)
    };
    let cmd_text = kv("cmd").unwrap_or_default();
    if cmd_text.is_empty() || cmd_text.starts_with("ai __hook") {
        return Ok(());
    }

    let store = Store::open_default()?;
    let session_id = "sess-default";
    store.get_or_create_session(
        session_id,
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: kv("cwd").unwrap_or_default(),
            policy_profile: "balanced".into(),
        },
    )?;

    let a = risk::assess(&cmd_text);
    store.record_command(&NewCommand {
        session_id: session_id.into(),
        command_text: cmd_text,
        source: "shell".into(),
        cwd: kv("cwd"),
        exit_code: None,
        risk_level: Some(format!("{:?}", a.level)),
        risk_score: Some(a.score as i64),
        ai_generated: false,
        confirmed: true,
    })?;
    Ok(())
}

/// 셸 hook의 `precmd` 이벤트를 받아 직전 명령의 종료 코드를 반영한다(best-effort).
///
/// `preexec`에서 종료 코드 미정으로 기록된 명령에 실제 `$?`를 채운다(§31.1).
/// `last-error` 분석(`ai explain --last-error`)의 입력이 된다.
#[cfg(feature = "storage")]
pub(crate) fn record_hook_precmd(rest: &[String]) -> anyhow::Result<()> {
    use ai_terminal::store::Store;

    let exit = rest
        .iter()
        .find_map(|s| s.strip_prefix("exit="))
        .and_then(|s| s.parse::<i64>().ok());
    let Some(exit) = exit else { return Ok(()) };

    let store = Store::open_default()?;
    store.update_last_exit("sess-default", exit)?;
    Ok(())
}

/// 셸 hook의 `chpwd` 이벤트를 받아 세션 cwd와 git branch 컨텍스트를 갱신한다(§31.10).
///
/// `cd`/`git switch` 등으로 작업 디렉터리가 바뀌면 세션 cwd를 갱신하고,
/// 해당 경로의 git branch를 컨텍스트 스냅샷으로 남긴다(best-effort).
#[cfg(feature = "storage")]
pub(crate) fn record_hook_chpwd(rest: &[String]) -> anyhow::Result<()> {
    use ai_terminal::store::{NewContext, NewSession, Store};

    let cwd = rest.iter().find_map(|s| s.strip_prefix("cwd="));
    let Some(cwd) = cwd else { return Ok(()) };

    let store = Store::open_default()?;
    let session_id = "sess-default";
    store.get_or_create_session(
        session_id,
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: cwd.to_string(),
            policy_profile: "balanced".into(),
        },
    )?;

    let branch = ai_terminal::context::git_branch(std::path::Path::new(cwd));
    store.update_session_cwd(session_id, cwd)?;
    store.record_context_snapshot(&NewContext {
        session_id: session_id.into(),
        context_type: "chpwd".into(),
        cwd: Some(cwd.to_string()),
        git_branch: branch,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::command::{Cli, Command, InitTarget};
    use clap::Parser;

    #[test]
    fn cli_parses_shell_hook() {
        let cli = Cli::try_parse_from(["ai", "shell-hook", "zsh"]).unwrap();
        match cli.command {
            Some(Command::ShellHook { shell }) => assert_eq!(shell, "zsh"),
            _ => panic!("expected shell-hook"),
        }
    }

    #[test]
    fn cli_parses_init_shell_dry_run() {
        let cli = Cli::try_parse_from(["ai", "init", "shell", "--dry-run"]).unwrap();
        match cli.command {
            Some(Command::Init {
                target: InitTarget::Shell { dry_run, .. },
            }) => assert!(dry_run),
            _ => panic!("expected init shell"),
        }
    }

    #[test]
    fn plan_dry_run_never_writes() {
        let p = plan_init_shell(
            "export X=1\n",
            Shell::Bash,
            InitMode::DryRun,
            "/tmp/.bashrc",
        );
        assert!(!p.write, "dry-run must not write");
        assert!(p.message.contains("dry-run"));
    }

    #[test]
    fn plan_diff_shows_diff_without_writing() {
        let p = plan_init_shell("export X=1\n", Shell::Bash, InitMode::Diff, "/tmp/.bashrc");
        assert!(!p.write, "diff is preview-only");
        assert!(
            p.message.contains('+'),
            "should show added lines: {}",
            p.message
        );
    }

    #[test]
    fn plan_install_writes_block_then_idempotent() {
        let p = plan_init_shell(
            "export X=1\n",
            Shell::Bash,
            InitMode::Install,
            "/tmp/.bashrc",
        );
        assert!(p.write);
        assert!(shell::is_installed(&p.new_content));
        let again = plan_init_shell(
            &p.new_content,
            Shell::Bash,
            InitMode::Install,
            "/tmp/.bashrc",
        );
        assert!(!again.write, "second install must be a no-op");
    }

    #[test]
    fn plan_uninstall_writes_removal() {
        let installed = shell::apply_install("export X=1\n", Shell::Zsh);
        let p = plan_init_shell(&installed, Shell::Zsh, InitMode::Uninstall, "/tmp/.zshrc");
        assert!(p.write);
        assert!(!shell::is_installed(&p.new_content));
    }
}
