//! 정책 엔진 + 프로파일 (설계 §31.3, 액션 매핑 §31.4).
//!
//! 핵심 불변식(`docs/RULES.md` §1·§2):
//! - 로컬 정책이 AI 분류보다 우선한다(위험 등급은 [`crate::risk`]의 로컬 점수에서 나온다).
//! - 두 프로파일 모두 Critical을 차단한다.
//! - AI 생성 명령은 자동 실행하지 않는다(`auto_execute=false` 고정).

use crate::risk::RiskLevel;

/// 위험 등급에 대한 정책 결정(§31.4 액션 매핑).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// 그대로 허용.
    Allow,
    /// 사용자 확인 요청.
    Confirm,
    /// 강한 확인 + (가능 시) sandbox/preview.
    StrongConfirm,
    /// 실행 차단.
    Block,
}

/// 확인 수준(§31.3 `confirm_level`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmLevel {
    /// Medium 이상에서 확인 (balanced).
    MediumAndAbove,
    /// 모든 AI 명령에서 확인 (paranoid).
    AllAi,
}

/// 정책 프로파일(§31.3 권위값). MVP 필수: `balanced`(기본), `paranoid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyProfile {
    pub name: &'static str,
    pub confirm_level: ConfirmLevel,
    pub block_critical: bool,
    pub block_high_risk: bool,
    pub preview_file_modifications: bool,
    pub auto_execute: bool,
    pub auto_healing: bool,
    pub auto_healing_max_attempts: u8,
    pub allow_remote_ai: bool,
    pub allow_sudo_ai_commands: bool,
    pub mask_pii: bool,
    pub mask_secrets: bool,
    pub block_on_masking_failure: bool,
    pub remote_approval: bool,
}

impl PolicyProfile {
    /// 기본 프로파일(§31.3 `[profiles.balanced]`).
    pub fn balanced() -> PolicyProfile {
        PolicyProfile {
            name: "balanced",
            confirm_level: ConfirmLevel::MediumAndAbove,
            block_critical: true,
            block_high_risk: false,
            preview_file_modifications: true,
            auto_execute: false,
            auto_healing: true,
            auto_healing_max_attempts: 1,
            allow_remote_ai: true,
            allow_sudo_ai_commands: false,
            mask_pii: true,
            mask_secrets: true,
            block_on_masking_failure: true,
            remote_approval: false,
        }
    }

    /// 강화 프로파일(§31.3 `[profiles.paranoid]`).
    pub fn paranoid() -> PolicyProfile {
        PolicyProfile {
            name: "paranoid",
            confirm_level: ConfirmLevel::AllAi,
            block_critical: true,
            block_high_risk: true,
            preview_file_modifications: true,
            auto_execute: false,
            auto_healing: false,
            auto_healing_max_attempts: 0,
            allow_remote_ai: false,
            allow_sudo_ai_commands: false,
            mask_pii: true,
            mask_secrets: true,
            block_on_masking_failure: true,
            remote_approval: false,
        }
    }

    /// 이름으로 프로파일을 조회한다. MVP는 balanced/paranoid만 지원한다.
    pub fn by_name(name: &str) -> Option<PolicyProfile> {
        match name {
            "balanced" => Some(Self::balanced()),
            "paranoid" => Some(Self::paranoid()),
            _ => None,
        }
    }

    /// 위험 등급에 대한 정책 결정을 반환한다(§31.4 액션 매핑).
    pub fn decide(&self, level: RiskLevel) -> Decision {
        match level {
            RiskLevel::Critical if self.block_critical => Decision::Block,
            RiskLevel::Critical => Decision::StrongConfirm,
            RiskLevel::High if self.block_high_risk => Decision::Block,
            RiskLevel::High => Decision::StrongConfirm,
            RiskLevel::Medium => Decision::Confirm,
            RiskLevel::Low => match self.confirm_level {
                ConfirmLevel::AllAi => Decision::Confirm,
                ConfirmLevel::MediumAndAbove => Decision::Allow,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_action_mapping() {
        let p = PolicyProfile::balanced();
        assert_eq!(p.decide(RiskLevel::Low), Decision::Allow);
        assert_eq!(p.decide(RiskLevel::Medium), Decision::Confirm);
        assert_eq!(p.decide(RiskLevel::High), Decision::StrongConfirm);
        assert_eq!(p.decide(RiskLevel::Critical), Decision::Block);
    }

    #[test]
    fn paranoid_action_mapping() {
        let p = PolicyProfile::paranoid();
        assert_eq!(p.decide(RiskLevel::Low), Decision::Confirm);
        assert_eq!(p.decide(RiskLevel::Medium), Decision::Confirm);
        assert_eq!(p.decide(RiskLevel::High), Decision::Block);
        assert_eq!(p.decide(RiskLevel::Critical), Decision::Block);
    }

    #[test]
    fn both_profiles_block_critical() {
        assert_eq!(
            PolicyProfile::balanced().decide(RiskLevel::Critical),
            Decision::Block
        );
        assert_eq!(
            PolicyProfile::paranoid().decide(RiskLevel::Critical),
            Decision::Block
        );
    }

    #[test]
    fn paranoid_blocks_remote_ai_but_balanced_allows() {
        assert!(!PolicyProfile::paranoid().allow_remote_ai);
        assert!(PolicyProfile::balanced().allow_remote_ai);
    }

    #[test]
    fn no_profile_auto_executes_or_allows_sudo() {
        for p in [PolicyProfile::balanced(), PolicyProfile::paranoid()] {
            assert!(!p.auto_execute, "{} must not auto-execute", p.name);
            assert!(
                !p.allow_sudo_ai_commands,
                "{} must not allow sudo AI",
                p.name
            );
            assert!(p.block_on_masking_failure, "{} must fail-closed", p.name);
        }
    }

    #[test]
    fn by_name_resolves_known_profiles_only() {
        assert_eq!(PolicyProfile::by_name("balanced").unwrap().name, "balanced");
        assert_eq!(PolicyProfile::by_name("paranoid").unwrap().name, "paranoid");
        assert!(PolicyProfile::by_name("nonexistent").is_none());
    }
}
