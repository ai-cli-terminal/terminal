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

/// 실행 추상화(W2 스트리밍 심). `PtyExecutor`는 PTY 출력을 청크 단위로 sink에 스트리밍한다.
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
        crate::pty::run_in_pty_streaming(&self.shell, command, |c| sink.write(c))
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

    let plan = preview::classify_preview(command);
    let targets = backup_targets(command);
    let backup_files: Vec<String> = targets.iter().map(|p| p.display().to_string()).collect();

    if matches!(decision, Decision::Confirm | Decision::StrongConfirm) {
        let req = ConfirmRequest {
            command: command.to_string(),
            level: assessment.level,
            decision,
            factors: factors.clone(),
            preview: plan,
            backup_files: backup_files.clone(),
        };
        if !confirmer.confirm(&req) {
            return Ok(ExecOutcome::Declined);
        }
    }

    let mut undo_id = None;
    if !targets.is_empty() {
        match undo::create_backup(cfg.undo_dir, &targets, &cfg.limits)? {
            BackupOutcome::Created(id) => undo_id = Some(id),
            BackupOutcome::Refused(reason) => return Ok(ExecOutcome::BackupRefused(reason)),
        }
    }

    let exit_code = executor.run(command, sink)?;
    Ok(ExecOutcome::Ran { exit_code, undo_id })
}

/// 백업 대상 파일을 산출한다. 삭제/덮어쓰기/in-place 편집 명령의 인자 경로와
/// 리다이렉트 대상 중 **기존 일반 파일**만. 권한 변경(chmod/chown/chgrp)은
/// 내용 백업이 무의미하므로 제외한다.
fn backup_targets(command: &str) -> Vec<PathBuf> {
    let toks: Vec<&str> = command.split_whitespace().collect();
    let prog = crate::cmdparse::program_token(command);
    let in_place =
        matches!(prog, Some("sed") | Some("perl")) && toks.iter().any(|t| t.starts_with("-i"));
    let prog_backupable = matches!(
        prog,
        Some("rm")
            | Some("unlink")
            | Some("shred")
            | Some("cp")
            | Some("mv")
            | Some("tee")
            | Some("touch")
    );

    let mut cands: Vec<String> = Vec::new();
    if prog_backupable || in_place {
        cands.extend(candidate_paths(&toks));
    }
    cands.extend(redirect_targets(&toks));

    let mut seen = std::collections::HashSet::new();
    cands
        .into_iter()
        .filter(|c| seen.insert(c.clone()))
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect()
}

/// 플래그/숫자/옵션/리다이렉트 연산자를 제외한 경로 후보(선행 래퍼·프로그램 토큰 제외).
fn candidate_paths(toks: &[&str]) -> Vec<String> {
    let mut it = toks.iter().copied();
    for t in it.by_ref() {
        if crate::cmdparse::is_wrapper_token(t) || crate::cmdparse::is_env_assignment(t) {
            continue;
        }
        break;
    }
    it.filter(|t| {
        !t.starts_with('-')
            && !t.chars().all(|c| c.is_ascii_digit())
            && !t.contains('=')
            && !matches!(*t, "|" | "&&" | ";")
            && strip_redirect_op(t).is_none()
    })
    .map(String::from)
    .collect()
}

/// 토큰이 리다이렉트 연산자로 시작하면 연산자 뒤 나머지(대상; 분리형이면 "")를 반환한다.
/// 인식: 선택적 fd 접두(`[0-9]*` 또는 단일 `&`) + `>` + 선택적 `>`(append).
fn strip_redirect_op(tok: &str) -> Option<&str> {
    let bytes = tok.as_bytes();
    let mut j = 0;
    // 선택적 fd 접두: 단일 '&' 또는 숫자들
    if j < bytes.len() && bytes[j] == b'&' {
        j += 1;
    } else {
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
    }
    // 반드시 '>' 가 와야 한다
    if j >= bytes.len() || bytes[j] != b'>' {
        return None;
    }
    j += 1;
    // append '>>'
    if j < bytes.len() && bytes[j] == b'>' {
        j += 1;
    }
    Some(&tok[j..])
}

/// 리다이렉트 대상 파일명들을 추출한다. 붙은 형태는 토큰에서, 분리형(`> f`)은 다음 토큰에서.
fn redirect_targets(toks: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if let Some(rest) = strip_redirect_op(toks[i]) {
            if !rest.is_empty() {
                out.push(rest.to_string());
            } else if i + 1 < toks.len() {
                out.push(toks[i + 1].to_string());
                i += 1;
            }
        }
        i += 1;
    }
    out
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

    #[test]
    fn high_command_declined_when_confirmer_says_no() {
        // `sudo systemctl restart` → High → balanced: StrongConfirm
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 0);
        let mut sink = Sink(String::new());
        let mut conf = No;
        let out = execute(
            "sudo systemctl restart nginx",
            &cfg(&prof, &undo),
            &exec,
            &mut conf,
            &mut sink,
        )
        .unwrap();
        assert_eq!(out, ExecOutcome::Declined);
        assert_eq!(*exec.calls.borrow(), 0);
    }

    #[test]
    fn high_command_runs_when_confirmed() {
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("done\n", 0);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let out = execute(
            "sudo systemctl restart nginx",
            &cfg(&prof, &undo),
            &exec,
            &mut conf,
            &mut sink,
        )
        .unwrap();
        assert!(
            matches!(out, ExecOutcome::Ran { exit_code: 0, .. }),
            "{out:?}"
        );
        assert_eq!(*exec.calls.borrow(), 1);
    }

    #[test]
    fn deletion_backs_up_existing_file_before_running() {
        let prof = PolicyProfile::balanced();
        let work = tmp("w");
        std::fs::create_dir_all(&work).unwrap();
        let f = work.join("data.txt");
        std::fs::write(&f, "original").unwrap();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 0);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let cmd = format!("rm {}", f.display());
        let out = execute(&cmd, &cfg(&prof, &undo), &exec, &mut conf, &mut sink).unwrap();
        let id = match out {
            ExecOutcome::Ran {
                undo_id: Some(id), ..
            } => id,
            other => panic!("expected Ran with undo_id, got {other:?}"),
        };
        // 백업으로 복구 가능해야 한다
        std::fs::write(&f, "changed").unwrap();
        undo::restore(&undo, &id).unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "original");
    }

    #[test]
    fn backup_refused_aborts_execution() {
        let prof = PolicyProfile::balanced();
        let work = tmp("w");
        std::fs::create_dir_all(&work).unwrap();
        let f = work.join("data.txt");
        std::fs::write(&f, vec![0u8; 1024]).unwrap();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 0);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let limits = UndoLimits {
            max_file_size_mb: 0,
            ..UndoLimits::defaults()
        };
        let config = ExecConfig {
            profile: &prof,
            undo_dir: &undo,
            limits,
        };
        let cmd = format!("rm {}", f.display());
        let out = execute(&cmd, &config, &exec, &mut conf, &mut sink).unwrap();
        assert!(matches!(out, ExecOutcome::BackupRefused(_)), "{out:?}");
        assert_eq!(*exec.calls.borrow(), 0, "refused backup must not execute");
    }

    #[test]
    fn exit_code_is_propagated() {
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 3);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let out = execute("ls", &cfg(&prof, &undo), &exec, &mut conf, &mut sink).unwrap();
        assert_eq!(
            out,
            ExecOutcome::Ran {
                exit_code: 3,
                undo_id: None
            }
        );
    }

    #[test]
    fn strip_redirect_op_recognizes_forms() {
        assert_eq!(strip_redirect_op(">out"), Some("out"));
        assert_eq!(strip_redirect_op(">>log"), Some("log"));
        assert_eq!(strip_redirect_op("2>err"), Some("err"));
        assert_eq!(strip_redirect_op("&>all"), Some("all"));
        assert_eq!(strip_redirect_op("2>>log"), Some("log"));
        assert_eq!(strip_redirect_op(">"), Some(""));
        assert_eq!(strip_redirect_op(">>"), Some(""));
        assert_eq!(strip_redirect_op("2>"), Some(""));
        assert_eq!(strip_redirect_op("123"), None);
        assert_eq!(strip_redirect_op("-i"), None);
        assert_eq!(strip_redirect_op("a=b"), None);
        assert_eq!(strip_redirect_op("file"), None);
    }

    #[test]
    fn redirect_targets_extracts_attached_and_detached() {
        assert_eq!(
            redirect_targets(&["echo", "hi", ">out.txt"]),
            vec!["out.txt".to_string()]
        );
        assert_eq!(
            redirect_targets(&["cmd", ">", "out.txt"]),
            vec!["out.txt".to_string()]
        );
        assert_eq!(
            redirect_targets(&["cmd", "2>err", ">>log"]),
            vec!["err".to_string(), "log".to_string()]
        );
        assert!(redirect_targets(&["cmd", ">"]).is_empty());
        assert!(redirect_targets(&["ls", "-al"]).is_empty());
    }

    #[test]
    fn backup_targets_picks_up_redirect_overwrite() {
        let work = tmp("rt");
        std::fs::create_dir_all(&work).unwrap();
        let f = work.join("out.txt");
        std::fs::write(&f, "x").unwrap();
        let cmd = format!("echo hi >{}", f.display());
        let t = backup_targets(&cmd);
        assert!(t.contains(&f), "attached redirect target missing: {t:?}");
        let cmd2 = format!("echo hi > {}", f.display());
        let t2 = backup_targets(&cmd2);
        assert!(t2.contains(&f), "detached redirect target missing: {t2:?}");
    }

    #[test]
    fn backup_targets_skips_new_redirect_file_and_chmod() {
        let work = tmp("rt2");
        std::fs::create_dir_all(&work).unwrap();
        let missing = work.join("new.txt");
        let cmd = format!("echo hi >{}", missing.display());
        assert!(
            backup_targets(&cmd).is_empty(),
            "new file should not be backed up"
        );
        let existing = work.join("e.txt");
        std::fs::write(&existing, "x").unwrap();
        let cmd2 = format!("chmod 755 {}", existing.display());
        assert!(
            backup_targets(&cmd2).is_empty(),
            "chmod must be excluded: {:?}",
            backup_targets(&cmd2)
        );
    }
}
