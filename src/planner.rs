//! Tool Use Planner (설계 §5 Agent Pipeline plan, Phase 2).
//!
//! 자연어 요청을 후보 명령 단계로 분해한다. MVP는 결정성 규칙 기반 매핑이며, 매칭이 없으면
//! AI 위임 단계(command=None)로 떨어진다. 다단계 AI 계획은 provider 연동 후(Phase 2+).
//! 생성된 명령은 그대로 실행하지 않고 위험도·정책·확인 게이트를 거친다(RULES §1).

/// 계획의 한 단계.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStep {
    pub description: String,
    /// 제안 명령(없으면 AI 위임).
    pub command: Option<String>,
}

/// 요청에 대한 실행 계획.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub steps: Vec<PlanStep>,
}

struct Rule {
    triggers: &'static [&'static str],
    description: &'static str,
    command: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        triggers: &["big file", "large file", "큰 파일", "대용량"],
        description: "큰 파일 검색",
        command: "find . -type f -size +100M",
    },
    Rule {
        triggers: &["disk usage", "디스크 사용", "용량 확인"],
        description: "디스크 사용량",
        command: "du -sh *",
    },
    Rule {
        triggers: &["list file", "파일 목록", "목록 보"],
        description: "파일 목록",
        command: "ls -al",
    },
    Rule {
        triggers: &["process", "프로세스"],
        description: "실행 중 프로세스",
        command: "ps aux",
    },
    Rule {
        triggers: &["git status", "깃 상태", "git 상태"],
        description: "git 상태",
        command: "git status",
    },
];

/// 요청을 계획으로 변환한다(규칙 매칭, 없으면 AI 위임).
pub fn plan(request: &str) -> Plan {
    let lower = request.to_lowercase();
    let mut steps: Vec<PlanStep> = RULES
        .iter()
        .filter(|r| r.triggers.iter().any(|t| lower.contains(t)))
        .map(|r| PlanStep {
            description: r.description.to_string(),
            command: Some(r.command.to_string()),
        })
        .collect();
    if steps.is_empty() {
        steps.push(PlanStep {
            description: "AI에게 위임(규칙 매칭 없음)".to_string(),
            command: None,
        });
    }
    Plan { steps }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_request_to_command() {
        let p = plan("큰 파일 찾아줘");
        assert_eq!(p.steps.len(), 1);
        let cmd = p.steps[0].command.as_deref().unwrap();
        assert!(cmd.contains("find") && cmd.contains("-size"), "{cmd}");
    }

    #[test]
    fn maps_english_request() {
        let p = plan("show me disk usage please");
        assert_eq!(p.steps[0].command.as_deref(), Some("du -sh *"));
    }

    #[test]
    fn compound_request_yields_multiple_steps() {
        let p = plan("list files and show processes");
        assert_eq!(p.steps.len(), 2);
    }

    #[test]
    fn unknown_request_delegates_to_ai() {
        let p = plan("explain quantum entanglement");
        assert_eq!(p.steps.len(), 1);
        assert_eq!(p.steps[0].command, None);
    }
}
