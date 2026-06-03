//! 중앙 실행 파이프라인 (설계 §5/§16.2, 그룹 C 키스톤).
//!
//! 위험도 → 정책 게이트 → preview → undo 백업(W10) → 실행(W2)을 하나의
//! 오케스트레이터로 묶는다. I/O는 트레이트로 주입해 코어를 순수하게 유지한다
//! (PTY 없이 단위 테스트 가능). 스토리지 기록은 호출측(CLI)에서 수행한다.

use std::path::{Path, PathBuf};

use crate::policy::{Decision, PolicyProfile};
use crate::preview::{self, PreviewPlan};
use crate::risk::{self, RiskLevel};
use crate::undo::{self, BackupOutcome, UndoLimits};

/// 출력 싱크(W2 스트리밍 심). CLI=stdout, TUI=히스토리, 테스트=수집.
pub trait OutputSink {
    fn write(&mut self, chunk: &str);
}

/// 실행 추상화(W2 스트리밍 심). 지금은 동기 PtyExecutor, 후속에 스트리밍 impl.
pub trait Executor {
    /// 명령을 실행하고 출력을 sink로 흘려보낸 뒤 종료 코드를 반환한다.
    fn run(&self, command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32>;
}

/// 확인 게이트에 노출할 정보(감사/설명용, RULES §2).
pub struct ConfirmRequest {
    pub command: String,
    pub level: RiskLevel,
    pub decision: Decision,
    pub factors: Vec<String>,
    pub preview: PreviewPlan,
    pub backup_files: Vec<String>,
}

/// 확인 게이트 주입. CLI=stdin/--yes, TUI=모달, 테스트=스크립트.
pub trait Confirmer {
    fn confirm(&mut self, req: &ConfirmRequest) -> bool;
}

/// 파이프라인 설정.
pub struct ExecConfig<'a> {
    pub profile: &'a PolicyProfile,
    pub undo_dir: &'a Path,
    pub limits: UndoLimits,
}

/// 실행 결과.
#[derive(Debug, PartialEq, Eq)]
pub enum ExecOutcome {
    /// 정책상 차단(실행 안 함).
    Blocked {
        level: RiskLevel,
        factors: Vec<String>,
    },
    /// 사용자가 확인을 거부(실행 안 함).
    Declined,
    /// 백업이 상한 등으로 거부됨(위험 명령 중단).
    BackupRefused(String),
    /// 실행됨.
    Ran {
        exit_code: i32,
        undo_id: Option<String>,
    },
}

/// PTY 기반 실행기(실제 셸 경로).
pub struct PtyExecutor {
    pub shell: String,
}

impl Executor for PtyExecutor {
    fn run(&self, command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
        let out = crate::pty::run_in_pty(&self.shell, command)?;
        sink.write(&out.output);
        Ok(out.exit_code as i32)
    }
}

/// 중앙 실행 파이프라인. 게이트를 통과한 경우에만 executor를 호출한다.
pub fn execute(
    command: &str,
    cfg: &ExecConfig,
    executor: &dyn Executor,
    confirmer: &mut dyn Confirmer,
    sink: &mut dyn OutputSink,
) -> anyhow::Result<ExecOutcome> {
    let _ = confirmer; // Task 3에서 사용
    let assessment = risk::assess(command);
    let decision = cfg.profile.decide(assessment.level);
    let factors: Vec<String> = assessment
        .factors
        .iter()
        .map(|f| format!("{} ({:+})", f.label, f.delta))
        .collect();

    if decision == Decision::Block {
        return Ok(ExecOutcome::Blocked {
            level: assessment.level,
            factors,
        });
    }

    let exit_code = executor.run(command, sink)?;
    Ok(ExecOutcome::Ran {
        exit_code,
        undo_id: None,
    })
}

/// 백업 대상 파일을 산출한다. 삭제/덮어쓰기/in-place 편집 명령의 **기존 일반 파일**만.
/// 권한 변경(chmod/chown/chgrp)은 내용 백업이 무의미하므로 제외한다.
fn backup_targets(command: &str) -> Vec<PathBuf> {
    let toks: Vec<&str> = command.split_whitespace().collect();
    let prog = program_token(&toks);
    let in_place =
        matches!(prog, Some("sed") | Some("perl")) && toks.iter().any(|t| t.starts_with("-i"));
    let backupable = matches!(
        prog,
        Some("rm")
            | Some("unlink")
            | Some("shred")
            | Some("cp")
            | Some("mv")
            | Some("tee")
            | Some("touch")
    ) || in_place
        || command.contains('>');
    if !backupable {
        return Vec::new();
    }
    candidate_paths(&toks)
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect()
}

/// 선행 sudo/env/`VAR=` 를 건너뛴 프로그램 토큰.
fn program_token<'a>(toks: &[&'a str]) -> Option<&'a str> {
    for &t in toks {
        if matches!(t, "sudo" | "doas" | "env" | "nohup" | "nice") {
            continue;
        }
        if t.contains('=') && !t.starts_with('/') && !t.starts_with('.') {
            continue;
        }
        return Some(t);
    }
    None
}

/// 플래그/숫자/옵션/리다이렉트 연산자를 제외한 경로 후보.
fn candidate_paths(toks: &[&str]) -> Vec<String> {
    let mut it = toks.iter().copied();
    for t in it.by_ref() {
        if matches!(t, "sudo" | "doas" | "env" | "nohup" | "nice") {
            continue;
        }
        if t.contains('=') && !t.starts_with('/') && !t.starts_with('.') {
            continue;
        }
        break;
    }
    it.filter(|t| {
        !t.starts_with('-')
            && !t.chars().all(|c| c.is_ascii_digit())
            && !t.contains('=')
            && !matches!(*t, ">" | ">>" | "|" | "&&" | ";")
    })
    .map(String::from)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockExecutor {
        calls: RefCell<u32>,
        output: String,
        exit: i32,
    }
    impl MockExecutor {
        fn new(output: &str, exit: i32) -> Self {
            Self {
                calls: RefCell::new(0),
                output: output.into(),
                exit,
            }
        }
    }
    impl Executor for MockExecutor {
        fn run(&self, _command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
            *self.calls.borrow_mut() += 1;
            sink.write(&self.output);
            Ok(self.exit)
        }
    }

    struct Sink(String);
    impl OutputSink for Sink {
        fn write(&mut self, c: &str) {
            self.0.push_str(c);
        }
    }

    struct Yes;
    impl Confirmer for Yes {
        fn confirm(&mut self, _: &ConfirmRequest) -> bool {
            true
        }
    }
    struct No;
    impl Confirmer for No {
        fn confirm(&mut self, _: &ConfirmRequest) -> bool {
            false
        }
    }

    fn tmp(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ai_pipe_{}_{}_{}", std::process::id(), tag, n));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn cfg<'a>(profile: &'a PolicyProfile, undo: &'a Path) -> ExecConfig<'a> {
        ExecConfig {
            profile,
            undo_dir: undo,
            limits: UndoLimits::defaults(),
        }
    }

    #[test]
    fn allow_command_runs_without_confirm() {
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("hi\n", 0);
        let mut sink = Sink(String::new());
        let mut conf = No; // Allow는 확인을 호출하지 않으므로 No여도 실행된다
        let out = execute("ls -al", &cfg(&prof, &undo), &exec, &mut conf, &mut sink).unwrap();
        assert_eq!(
            out,
            ExecOutcome::Ran {
                exit_code: 0,
                undo_id: None
            }
        );
        assert_eq!(*exec.calls.borrow(), 1);
        assert_eq!(sink.0, "hi\n");
    }

    #[test]
    fn critical_command_is_blocked() {
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 0);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let out = execute("rm -rf /", &cfg(&prof, &undo), &exec, &mut conf, &mut sink).unwrap();
        assert!(matches!(out, ExecOutcome::Blocked { .. }), "{out:?}");
        assert_eq!(*exec.calls.borrow(), 0, "blocked must not execute");
    }
}
