//! 원격 승인 게이트 결정 + armed 상태 (M0, §30-13 경계).
//!
//! armed 상태에서 명령을 **실행 전** 통과/차단 결정한다(순수, deterministic).
//! §30-13 정본 경계: Low/Medium 통과, High 기본 차단(opt-in 시 통과), Critical 항상 차단.
//! armed가 아니면 게이트는 개입하지 않는다(항상 통과). 크립토·원격 왕복은 M1+.

use std::path::{Path, PathBuf};

use crate::risk::{self, RiskLevel};

/// 게이트 결정.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Allow,
    Block { reason: String },
}

/// armed/allow_high 상태에서 명령의 통과/차단을 결정한다(순수).
pub fn decide_gate(command: &str, armed: bool, allow_high: bool) -> GateDecision {
    if !armed {
        return GateDecision::Allow;
    }
    let a = risk::assess(command);
    match a.level {
        RiskLevel::Low | RiskLevel::Medium => GateDecision::Allow,
        RiskLevel::High if allow_high => GateDecision::Allow,
        RiskLevel::High => GateDecision::Block {
            reason: format!(
                "High 위험(score {}) — 원격 승인 opt-in 필요(§30-13)",
                a.score
            ),
        },
        RiskLevel::Critical => GateDecision::Block {
            reason: format!(
                "Critical 위험(score {}) — 원격 승인 불가, 로컬 터미널에서 실행(§30-13)",
                a.score
            ),
        },
    }
}

/// armed 상태(파일 존재 = armed). 내용으로 opt-in 플래그를 표현.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmState {
    pub allow_high: bool,
}

/// armed 파일 경로: `<config_dir>/armed`. 셸 hook의 경로와 일치해야 한다
/// (`${XDG_CONFIG_HOME:-$HOME/.config}/ai-terminal/armed`).
pub fn armed_path() -> anyhow::Result<PathBuf> {
    Ok(crate::config::config_dir()?.join("armed"))
}

/// armed 파일 내용을 파싱(순수). `allow_high=true` 라인이 있으면 opt-in.
pub fn parse_arm_file(content: &str) -> ArmState {
    ArmState {
        allow_high: content.lines().any(|l| l.trim() == "allow_high=true"),
    }
}

/// armed 파일 내용을 생성(순수).
pub fn render_arm_file(allow_high: bool) -> String {
    format!("allow_high={allow_high}\n")
}

/// armed 상태를 읽는다. 파일이 없으면 `None`(=armed 아님).
pub fn load_arm_state(path: &Path) -> Option<ArmState> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(parse_arm_file(&content))
}

/// armed 파일을 기록(상위 디렉터리 생성).
pub fn arm_at(path: &Path, allow_high: bool) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, render_arm_file(allow_high))?;
    Ok(())
}

/// armed 파일을 제거(없으면 무해).
pub fn disarm_at(path: &Path) -> anyhow::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_armed_always_allows() {
        assert_eq!(decide_gate("rm -rf /", false, false), GateDecision::Allow);
    }

    #[test]
    fn armed_allows_low_and_medium() {
        // 실측 등급: "ls -al"=Low(0), "rm file.txt"=Medium(35) → 둘 다 통과.
        assert_eq!(decide_gate("ls -al", true, false), GateDecision::Allow);
        assert_eq!(decide_gate("rm file.txt", true, false), GateDecision::Allow);
    }

    #[test]
    fn armed_blocks_critical_always() {
        match decide_gate("rm -rf /", true, true) {
            GateDecision::Block { reason } => assert!(reason.contains("Critical")),
            d => panic!("expected Block, got {d:?}"),
        }
    }

    #[test]
    fn armed_high_blocks_unless_optin() {
        // 실측 등급: "rm -rf build"=High(65). 기본 차단, opt-in 시 통과.
        let cmd = "rm -rf build";
        match decide_gate(cmd, true, false) {
            GateDecision::Block { reason } => assert!(reason.contains("opt-in")),
            d => panic!("expected Block default, got {d:?}"),
        }
        assert_eq!(decide_gate(cmd, true, true), GateDecision::Allow);
    }

    #[test]
    fn arm_file_roundtrip() {
        assert!(parse_arm_file(&render_arm_file(true)).allow_high);
        assert!(!parse_arm_file(&render_arm_file(false)).allow_high);
        assert!(!parse_arm_file("garbage\n").allow_high);
    }

    #[test]
    fn arm_disarm_load_cycle() {
        let dir = std::env::temp_dir().join(format!("ra_gate_{}", std::process::id()));
        let path = dir.join("armed");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(load_arm_state(&path).is_none(), "초기엔 armed 아님");
        arm_at(&path, true).unwrap();
        assert!(load_arm_state(&path).unwrap().allow_high);
        disarm_at(&path).unwrap();
        assert!(load_arm_state(&path).is_none(), "disarm 후 armed 아님");
        disarm_at(&path).unwrap(); // 재호출 무해
        let _ = std::fs::remove_dir_all(&dir);
    }
}
