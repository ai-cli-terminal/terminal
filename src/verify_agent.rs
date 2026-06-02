//! Verification Agent (설계 §5 Agent Pipeline verify, Phase 2).
//!
//! AI가 제안한 명령을 실행/제시 전에 종합 검증한다: 바이너리 실재(환각)·위험도·정책
//! 결정·secret 포함 여부. 로컬 규칙이 우선이며 결과는 사용자 확인 게이트의 근거가 된다.

use crate::mask::Masker;
use crate::policy::{Decision, PolicyProfile};
use crate::risk::{self, RiskLevel};
use crate::verify::{self, BinaryStatus};

/// 검증 판정.
#[derive(Debug, Clone)]
pub struct Verdict {
    pub risk: RiskLevel,
    pub decision: Decision,
    pub binary: BinaryStatus,
    pub issues: Vec<String>,
    /// 차단되지 않고 바이너리가 실재할 때만 true(제시/실행 후보).
    pub safe_to_suggest: bool,
}

/// 명령을 종합 검증한다.
pub fn verify_command(command: &str, profile: &PolicyProfile) -> Verdict {
    let risk = risk::assess(command).level;
    let decision = profile.decide(risk);
    let binary = verify::check_binary(command);

    let mut issues = Vec::new();
    if matches!(binary, BinaryStatus::Unknown) {
        issues.push("존재하지 않는 바이너리(환각 가능성)".to_string());
    }
    if matches!(decision, Decision::Block) {
        issues.push("정책상 차단된 명령".to_string());
    }
    if matches!(risk, RiskLevel::High | RiskLevel::Critical) {
        issues.push("고위험 명령(실행 전 확인 강화)".to_string());
    }
    if !Masker::baseline().mask(command).redactions.is_empty() {
        issues.push("명령에 secret/민감정보 포함".to_string());
    }

    let safe_to_suggest =
        !matches!(decision, Decision::Block) && !matches!(binary, BinaryStatus::Unknown);

    Verdict {
        risk,
        decision,
        binary,
        issues,
        safe_to_suggest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_builtin_command() {
        let v = verify_command("cd /tmp", &PolicyProfile::balanced());
        assert_eq!(v.binary, BinaryStatus::Builtin);
        assert!(v.safe_to_suggest);
        assert!(v.issues.is_empty(), "{:?}", v.issues);
    }

    #[test]
    fn hallucinated_binary_flagged() {
        let v = verify_command("frobnicate_xyz --do", &PolicyProfile::balanced());
        assert_eq!(v.binary, BinaryStatus::Unknown);
        assert!(!v.safe_to_suggest);
        assert!(v.issues.iter().any(|i| i.contains("바이너리")));
    }

    #[test]
    fn critical_is_blocked_and_unsafe() {
        let v = verify_command("rm -rf /", &PolicyProfile::balanced());
        assert_eq!(v.decision, Decision::Block);
        assert!(!v.safe_to_suggest);
        assert!(v.issues.iter().any(|i| i.contains("차단")));
    }

    #[test]
    fn secret_in_command_is_flagged() {
        let v = verify_command("echo Bearer abc.def-123", &PolicyProfile::balanced());
        assert!(v
            .issues
            .iter()
            .any(|i| i.contains("secret") || i.contains("민감")));
    }
}
