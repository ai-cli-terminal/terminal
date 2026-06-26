//! ash 외부 실행 안전 게이트 (데스크톱 호스트 계층).
//! `shellcore::external::ExternalRunner`를 구현해 risk/policy/preview/undo/pipeline 게이트를
//! ash 외부 명령 앞단에 결선한다. shellcore는 이 모듈을 모른다(경계 유지).

use crate::pipeline::ExecOutcome;
use crate::shellcore::value::Value;

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
