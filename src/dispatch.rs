//! Hybrid Mode dispatcher (설계 §5 Input Handler, Phase 2).
//!
//! 입력을 [`crate::intent`]로 분류해 **일반 셸 경로**와 **AI 경로**로 분기한다.
//! 셸 경로는 위험도·정책 게이트를 함께 산출한다(로컬 우선, `docs/RULES.md`).

use crate::cache::CacheSource;
use crate::intent::{self, Intent};
use crate::pipeline::{self, ExecConfig, ExecOutcome, OutputSink};
use crate::policy::{Decision, PolicyProfile};
use crate::risk::{self, RiskLevel};

/// 분기 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// 빈 입력.
    Empty,
    /// 일반 셸 경로(위험도·정책 결정 동반).
    Shell {
        command: String,
        risk: RiskLevel,
        decision: Decision,
    },
    /// AI 경로(질의/인라인).
    Ai { prompt: String },
}

/// 입력을 경로로 분기한다.
pub fn dispatch(input: &str, profile: &PolicyProfile) -> Route {
    match intent::classify(input) {
        Intent::Empty => Route::Empty,
        Intent::Shell => {
            let command = input.trim().to_string();
            let assessment = risk::assess(&command);
            Route::Shell {
                command,
                risk: assessment.level,
                decision: profile.decide(assessment.level),
            }
        }
        Intent::AiQuery => Route::Ai {
            prompt: input.trim().to_string(),
        },
        Intent::AiInline => {
            // 선행 "ai " 트리거 제거 후 나머지를 프롬프트로.
            let prompt = input
                .trim()
                .strip_prefix("ai")
                .map(|r| r.trim_start().to_string())
                .unwrap_or_default();
            Route::Ai { prompt }
        }
    }
}

/// AI 핸들러 추상화(Executor/Confirmer/OutputSink와 같은 결의 심).
/// 컨텍스트(cwd 등)는 실제 구현이 내부에서 모은다.
pub trait AiResponder {
    fn respond(&mut self, prompt: &str, sink: &mut dyn OutputSink) -> anyhow::Result<AiOutcome>;
}

/// AI 응답 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiOutcome {
    /// 응답 성공(text는 sink에도 기록됨).
    Answered {
        text: String,
        input_tokens: usize,
        output_tokens: usize,
        source: CacheSource,
    },
    /// 마스킹 fail-closed 등으로 원격 전송 차단.
    Blocked(String),
    /// 장애·타임아웃·취소(§3-3: 셸을 막지 않음).
    Unavailable(String),
}

/// 통합 실행 결과.
#[derive(Debug, PartialEq, Eq)]
pub enum Handled {
    Empty,
    Shell(ExecOutcome),
    Ai(AiOutcome),
}

/// 주입 핸들러 묶음. sink는 셸/AI가 공유한다.
pub struct Handlers<'a> {
    pub executor: &'a dyn pipeline::Executor,
    pub confirmer: &'a mut dyn pipeline::Confirmer,
    pub ai: &'a mut dyn AiResponder,
    pub sink: &'a mut dyn OutputSink,
}

/// 입력을 분류해 셸 파이프라인 또는 AI 핸들러로 보낸다(설계 §3·§4).
pub fn run(
    input: &str,
    profile: &PolicyProfile,
    exec_cfg: &ExecConfig,
    h: &mut Handlers,
) -> anyhow::Result<Handled> {
    match dispatch(input, profile) {
        Route::Empty => Ok(Handled::Empty),
        Route::Shell { command, .. } => {
            // risk/decision은 pipeline::execute 내부에서 재산출되므로 여기서는 버린다.
            let out = pipeline::execute(&command, exec_cfg, h.executor, h.confirmer, h.sink)?;
            Ok(Handled::Shell(out))
        }
        Route::Ai { prompt } => {
            let out = h.ai.respond(&prompt, h.sink)?;
            Ok(Handled::Ai(out))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::cache::CacheSource;
    use crate::undo::UndoLimits;
    use std::cell::RefCell;
    use std::path::PathBuf;

    struct CollectSink(String);
    impl OutputSink for CollectSink {
        fn write(&mut self, c: &str) {
            self.0.push_str(c);
        }
    }

    struct MockExec {
        out: String,
        exit: i32,
        calls: RefCell<u32>,
    }
    impl pipeline::Executor for MockExec {
        fn run(&self, _cmd: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
            *self.calls.borrow_mut() += 1;
            sink.write(&self.out);
            Ok(self.exit)
        }
    }

    struct YesConfirm;
    impl pipeline::Confirmer for YesConfirm {
        fn confirm(&mut self, _: &pipeline::ConfirmRequest) -> bool {
            true
        }
    }

    struct MockAi {
        answer: String,
        calls: u32,
    }
    impl AiResponder for MockAi {
        fn respond(
            &mut self,
            _prompt: &str,
            sink: &mut dyn OutputSink,
        ) -> anyhow::Result<AiOutcome> {
            self.calls += 1;
            sink.write(&self.answer);
            Ok(AiOutcome::Answered {
                text: self.answer.clone(),
                input_tokens: 1,
                output_tokens: 2,
                source: CacheSource::Backend,
            })
        }
    }

    /// undo 디렉터리 경로만 만든다(실제 생성 안 함). 테스트 경로는 백업 대상이 없어
    /// pipeline이 디렉터리를 만들지 않으므로 정리 대상이 없다.
    fn undo_tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("ai_disp_{}_{}", std::process::id(), n))
    }

    fn run_with(input: &str, exec: &MockExec, ai: &mut MockAi, sink: &mut CollectSink) -> Handled {
        let prof = PolicyProfile::balanced();
        let undo = undo_tmp();
        let cfg = ExecConfig {
            profile: &prof,
            undo_dir: &undo,
            limits: UndoLimits::defaults(),
        };
        let mut conf = YesConfirm;
        let mut h = Handlers {
            executor: exec,
            confirmer: &mut conf,
            ai,
            sink,
        };
        run(input, &prof, &cfg, &mut h).unwrap()
    }

    #[test]
    fn empty_routes_to_handled_empty() {
        let exec = MockExec {
            out: String::new(),
            exit: 0,
            calls: RefCell::new(0),
        };
        let mut ai = MockAi {
            answer: "x".into(),
            calls: 0,
        };
        let mut sink = CollectSink(String::new());
        assert_eq!(run_with("   ", &exec, &mut ai, &mut sink), Handled::Empty);
        assert_eq!(*exec.calls.borrow(), 0);
        assert_eq!(ai.calls, 0);
    }

    #[test]
    fn shell_command_runs_through_pipeline() {
        let exec = MockExec {
            out: "hi\n".into(),
            exit: 0,
            calls: RefCell::new(0),
        };
        let mut ai = MockAi {
            answer: "x".into(),
            calls: 0,
        };
        let mut sink = CollectSink(String::new());
        let out = run_with("ls -al", &exec, &mut ai, &mut sink);
        assert_eq!(
            out,
            Handled::Shell(ExecOutcome::Ran {
                exit_code: 0,
                undo_id: None
            })
        );
        assert_eq!(*exec.calls.borrow(), 1);
        assert_eq!(sink.0, "hi\n");
    }

    #[test]
    fn critical_shell_is_blocked_not_executed() {
        let exec = MockExec {
            out: String::new(),
            exit: 0,
            calls: RefCell::new(0),
        };
        let mut ai = MockAi {
            answer: "x".into(),
            calls: 0,
        };
        let mut sink = CollectSink(String::new());
        let out = run_with("rm -rf /", &exec, &mut ai, &mut sink);
        assert!(
            matches!(out, Handled::Shell(ExecOutcome::Blocked { .. })),
            "{out:?}"
        );
        assert_eq!(*exec.calls.borrow(), 0);
    }

    #[test]
    fn natural_language_routes_to_ai() {
        let exec = MockExec {
            out: String::new(),
            exit: 0,
            calls: RefCell::new(0),
        };
        let mut ai = MockAi {
            answer: "answer-text".into(),
            calls: 0,
        };
        let mut sink = CollectSink(String::new());
        let out = run_with("how do I undo a commit?", &exec, &mut ai, &mut sink);
        assert_eq!(
            out,
            Handled::Ai(AiOutcome::Answered {
                text: "answer-text".into(),
                input_tokens: 1,
                output_tokens: 2,
                source: CacheSource::Backend,
            })
        );
        assert_eq!(ai.calls, 1);
        assert_eq!(*exec.calls.borrow(), 0);
        assert_eq!(sink.0, "answer-text");
    }

    #[test]
    fn ai_inline_routes_to_ai() {
        let exec = MockExec {
            out: String::new(),
            exit: 0,
            calls: RefCell::new(0),
        };
        let mut ai = MockAi {
            answer: "a".into(),
            calls: 0,
        };
        let mut sink = CollectSink(String::new());
        let out = run_with("ai explain last-error", &exec, &mut ai, &mut sink);
        assert!(
            matches!(out, Handled::Ai(AiOutcome::Answered { .. })),
            "{out:?}"
        );
        assert_eq!(ai.calls, 1);
    }

    #[test]
    fn empty_routes_to_empty() {
        assert_eq!(dispatch("   ", &PolicyProfile::balanced()), Route::Empty);
    }

    #[test]
    fn shell_command_routes_with_gate() {
        let r = dispatch("ls -al", &PolicyProfile::balanced());
        assert_eq!(
            r,
            Route::Shell {
                command: "ls -al".into(),
                risk: RiskLevel::Low,
                decision: Decision::Allow,
            }
        );
    }

    #[test]
    fn critical_shell_is_blocked() {
        let r = dispatch("rm -rf /", &PolicyProfile::balanced());
        match r {
            Route::Shell { risk, decision, .. } => {
                assert_eq!(risk, RiskLevel::Critical);
                assert_eq!(decision, Decision::Block);
            }
            other => panic!("expected shell, got {other:?}"),
        }
    }

    #[test]
    fn dispatch_natural_language_routes_to_ai() {
        assert_eq!(
            dispatch("how do I undo a commit?", &PolicyProfile::balanced()),
            Route::Ai {
                prompt: "how do I undo a commit?".into()
            }
        );
    }

    #[test]
    fn ai_inline_strips_prefix() {
        assert_eq!(
            dispatch("ai explain last-error", &PolicyProfile::balanced()),
            Route::Ai {
                prompt: "explain last-error".into()
            }
        );
    }
}
