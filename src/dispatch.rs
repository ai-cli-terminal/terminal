//! Hybrid Mode dispatcher (설계 §5 Input Handler, Phase 2).
//!
//! 입력을 [`crate::intent`]로 분류해 **일반 셸 경로**와 **AI 경로**로 분기한다.
//! 셸 경로는 위험도·정책 게이트를 함께 산출한다(로컬 우선, `docs/RULES.md`).

use crate::intent::{self, Intent};
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn natural_language_routes_to_ai() {
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
