//! ash 외부 실행 안전 게이트 (데스크톱 호스트 계층).
//! `shellcore::external::ExternalRunner`를 구현해 risk/policy/preview/undo/pipeline 게이트를
//! ash 외부 명령 앞단에 결선한다. shellcore는 이 모듈을 모른다(경계 유지).

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config;
use crate::pipeline::ExecOutcome;
use crate::pipeline::{self, ConfirmRequest, Confirmer, ExecConfig, Executor, OutputSink};
use crate::policy::PolicyProfile;
use crate::shellcore::external::{
    spawn_inherit, ExecutionCapabilities, ExternalCommand, ExternalRunner,
};
use crate::shellcore::value::Value;
use crate::undo::{self, UndoLimits};

/// argv를 분석용 명령 문자열로 재구성한다. 실행엔 쓰지 않는다(argv 직접 spawn).
/// 한계: 공백/특수문자가 든 인자는 분석 토크나이즈가 부정확할 수 있다(안전 분석은 보수적).
pub(crate) fn command_string(name: &str, args: &[Value]) -> String {
    let mut s = String::from(name);
    for a in args {
        s.push(' ');
        s.push_str(&a.coerce_string());
    }
    s
}

/// 확인 응답 판정: y/yes(대소문자 무시)만 승인.
pub(crate) fn decide_confirm(answer: &str) -> bool {
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// ExecOutcome → (stderr 메시지, REPL 반환 Value). 비-Ran은 미실행을 뜻한다.
pub(crate) fn outcome_message(outcome: &ExecOutcome, name: &str) -> (Option<String>, Value) {
    match outcome {
        ExecOutcome::Blocked { level, factors } => (
            Some(format!(
                "ash: 정책상 차단됨 [{level:?}] — {}",
                factors.join(", ")
            )),
            Value::Nothing,
        ),
        ExecOutcome::Declined => (Some("ash: 취소됨".to_string()), Value::Nothing),
        ExecOutcome::BackupRefused(reason) => (
            Some(format!("ash: 백업 거부({reason}) — 실행 중단")),
            Value::Nothing,
        ),
        ExecOutcome::Ran { exit_code, .. } => {
            if *exit_code == 0 {
                (None, Value::Nothing)
            } else {
                (Some(format!("[{name}: exit {exit_code}]")), Value::Nothing)
            }
        }
    }
}

/// 출력 싱크 무시(stdio는 상속 spawn으로 직접 터미널에 감).
struct NullSink;
impl OutputSink for NullSink {
    fn write(&mut self, _chunk: &str) {}
}

/// 원본 argv를 직접 spawn하는 Executor. 파이프라인이 넘기는 문자열/sink는 무시한다.
struct ArgvExecutor<'a> {
    name: &'a str,
    args: Vec<String>,
    cwd: &'a Path,
}
impl Executor for ArgvExecutor<'_> {
    fn run(&self, _command: &str, _sink: &mut dyn OutputSink) -> Result<i32> {
        Ok(spawn_inherit(self.name, &self.args, self.cwd)?.unwrap_or(-1))
    }
}

/// stdin 기반 확인. 비-TTY는 fail-closed(거부).
struct StdinConfirmer {
    is_tty: bool,
}
impl Confirmer for StdinConfirmer {
    fn confirm(&mut self, req: &ConfirmRequest) -> bool {
        if !self.is_tty {
            eprintln!("ash: 비대화형 입력 — 확인 불가로 거부: {}", req.command);
            return false;
        }
        eprintln!("⚠ 확인 필요: {}", req.command);
        eprintln!(
            "  위험도: {:?}  요인: {}",
            req.level,
            req.factors.join(", ")
        );
        if !req.backup_files.is_empty() {
            eprintln!("  백업 대상: {}", req.backup_files.join(", "));
        }
        eprint!("  실행할까요? [y/N] ");
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        if std::io::stdin().read_line(&mut answer).is_err() {
            return false;
        }
        decide_confirm(&answer)
    }
}

/// ash 외부 실행을 안전 게이트로 감싸는 runner.
pub struct GatedRunner {
    profile: PolicyProfile,
    undo_dir: PathBuf,
    limits: UndoLimits,
    is_tty: bool,
}

impl GatedRunner {
    /// config의 활성 profile + 기본 undo dir/limits로 구성한다. 실패는 fail-soft.
    pub fn from_environment() -> Self {
        let name = config::get_active_profile();
        let profile = PolicyProfile::by_name(&name).unwrap_or_else(PolicyProfile::balanced);
        let undo_dir = undo::default_undo_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("ai-terminal-undo"));
        Self {
            profile,
            undo_dir,
            limits: UndoLimits::defaults(),
            is_tty: std::io::stdin().is_terminal(),
        }
    }
}

impl ExternalRunner for GatedRunner {
    fn capabilities(&self) -> ExecutionCapabilities {
        ExecutionCapabilities::desktop_process()
    }

    fn run(&self, command: ExternalCommand<'_>) -> Result<Value> {
        let cmd = command_string(command.name, command.args);
        let cfg = ExecConfig {
            profile: &self.profile,
            undo_dir: &self.undo_dir,
            limits: self.limits,
        };
        let args: Vec<String> = command.args.iter().map(|v| v.coerce_string()).collect();
        let executor = ArgvExecutor {
            name: command.name,
            args,
            cwd: command.cwd,
        };
        let mut confirmer = StdinConfirmer {
            is_tty: self.is_tty,
        };
        let mut sink = NullSink;
        let outcome = pipeline::execute(&cmd, &cfg, &executor, &mut confirmer, &mut sink)?;
        let (msg, value) = outcome_message(&outcome, command.name);
        if let Some(m) = msg {
            eprintln!("{m}");
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::ExecOutcome;
    use crate::risk::RiskLevel;
    use crate::shellcore::value::Value;

    #[test]
    fn command_string_joins_argv() {
        let args = vec![Value::String("-rf".into()), Value::String("/x".into())];
        assert_eq!(command_string("rm", &args), "rm -rf /x");
        assert_eq!(command_string("ls", &[]), "ls");
    }

    #[test]
    fn decide_confirm_only_yes() {
        assert!(decide_confirm("y"));
        assert!(decide_confirm("Y"));
        assert!(decide_confirm("yes"));
        assert!(decide_confirm("YES"));
        assert!(!decide_confirm(""));
        assert!(!decide_confirm("n"));
        assert!(!decide_confirm("no"));
        assert!(!decide_confirm("maybe"));
    }

    #[test]
    fn outcome_message_maps_variants() {
        let (m, _) = outcome_message(
            &ExecOutcome::Blocked {
                level: RiskLevel::Critical,
                factors: vec!["x".into()],
            },
            "rm",
        );
        assert!(m.unwrap().contains("차단"));
        assert_eq!(
            outcome_message(&ExecOutcome::Declined, "rm").0.as_deref(),
            Some("ash: 취소됨")
        );
        assert!(
            outcome_message(&ExecOutcome::BackupRefused("big".into()), "rm")
                .0
                .unwrap()
                .contains("백업 거부")
        );
        assert!(outcome_message(
            &ExecOutcome::Ran {
                exit_code: 0,
                undo_id: None
            },
            "ls"
        )
        .0
        .is_none());
        assert_eq!(
            outcome_message(
                &ExecOutcome::Ran {
                    exit_code: 3,
                    undo_id: None
                },
                "ls"
            )
            .0
            .as_deref(),
            Some("[ls: exit 3]")
        );
    }
}
