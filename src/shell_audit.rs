//! 셸 실행 게이트 결과의 storage 기록(audit/command). `ai exec`와 `ash`가 공유한다.
//! storage feature 미빌드 시 기록 함수는 no-op이다.

use crate::pipeline::ExecOutcome;

/// 비-Ran 게이트 결과를 audit 레코드로 매핑한 결과.
pub struct AuditRecord {
    pub event_type: &'static str,
    pub level: String,
    pub payload_json: String,
}

/// 비-Ran ExecOutcome → AuditRecord(순수). Ran은 None(별도 command 기록).
pub fn shell_outcome_audit(
    command: &str,
    source: &str,
    outcome: &ExecOutcome,
) -> Option<AuditRecord> {
    let (event_type, level, mut payload) = match outcome {
        ExecOutcome::Ran { .. } => return None,
        ExecOutcome::Blocked { level, factors } => (
            "command_blocked",
            format!("{level:?}"),
            serde_json::json!({ "factors": factors }),
        ),
        ExecOutcome::Declined => (
            "command_declined",
            format!("{:?}", crate::risk::assess(command).level),
            serde_json::json!({}),
        ),
        ExecOutcome::BackupRefused(reason) => (
            "command_backup_refused",
            format!("{:?}", crate::risk::assess(command).level),
            serde_json::json!({ "reason": reason }),
        ),
    };
    let masked = crate::mask::Masker::baseline().mask(command).text;
    let map = payload
        .as_object_mut()
        .expect("audit payload must be a JSON object");
    map.insert("command".into(), serde_json::Value::String(masked));
    map.insert(
        "source".into(),
        serde_json::Value::String(source.to_owned()),
    );
    Some(AuditRecord {
        event_type,
        level,
        payload_json: payload.to_string(),
    })
}

/// 실행 성공/실패 audit payload를 비-Ran outcome과 같은 JSON 형식으로 만든다.
pub fn ran_command_audit_payload(command: &str, source: &str, exit_code: i32) -> String {
    let masked = crate::mask::Masker::baseline().mask(command).text;
    serde_json::json!({
        "command": masked,
        "source": source,
        "exit": exit_code,
    })
    .to_string()
}

/// audit 레코드를 영속화한다(storage feature, best-effort). 실패는 조용히 무시.
#[cfg(feature = "storage")]
pub fn record_outcome_audit(rec: &AuditRecord) {
    use crate::store::Store;
    let Ok(store) = Store::open_default() else {
        return;
    };
    let _ = store.record_audit(
        rec.event_type,
        Some(&rec.level),
        Some(&crate::config::get_active_profile()),
        &rec.payload_json,
    );
}

#[cfg(not(feature = "storage"))]
pub fn record_outcome_audit(_rec: &AuditRecord) {}

/// 실행된 명령을 commands + command_executed audit으로 기록한다(storage feature, best-effort).
#[cfg(feature = "storage")]
pub fn record_ran_command(command: &str, exit_code: i32, source: &str) {
    use crate::store::{NewCommand, NewSession, Store};
    let Ok(store) = Store::open_default() else {
        return;
    };
    let a = crate::risk::assess(command);
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .ok();
    let _ = store.get_or_create_session(
        "sess-default",
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: cwd.clone().unwrap_or_default(),
            policy_profile: crate::config::get_active_profile(),
        },
    );
    let _ = store.record_command(&NewCommand {
        session_id: "sess-default".into(),
        command_text: command.into(),
        source: source.into(),
        cwd,
        exit_code: Some(exit_code as i64),
        risk_level: Some(format!("{:?}", a.level)),
        risk_score: Some(a.score as i64),
        ai_generated: false,
        confirmed: true,
    });
    let _ = store.record_audit(
        "command_executed",
        Some(&format!("{:?}", a.level)),
        Some(&crate::config::get_active_profile()),
        &ran_command_audit_payload(command, source, exit_code),
    );
}

#[cfg(not(feature = "storage"))]
pub fn record_ran_command(_command: &str, _exit_code: i32, _source: &str) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::ExecOutcome;

    #[test]
    fn ran_outcome_has_no_audit() {
        let out = ExecOutcome::Ran {
            exit_code: 0,
            undo_id: None,
        };
        assert!(shell_outcome_audit("ls -al", "ash", &out).is_none());
    }

    #[test]
    fn blocked_maps_to_command_blocked() {
        let out = ExecOutcome::Blocked {
            level: crate::risk::RiskLevel::Critical,
            factors: vec!["x".into()],
        };
        let rec = shell_outcome_audit("rm -rf /", "ash", &out).expect("blocked → Some");
        assert_eq!(rec.event_type, "command_blocked");
        assert!(
            rec.payload_json.contains("\"source\":\"ash\""),
            "{}",
            rec.payload_json
        );
        assert!(rec.payload_json.contains("command"), "{}", rec.payload_json);
    }

    #[test]
    fn declined_and_backup_refused_map() {
        let d = shell_outcome_audit("rm -rf /", "ash", &ExecOutcome::Declined)
            .expect("declined → Some");
        assert_eq!(d.event_type, "command_declined");
        let b = shell_outcome_audit(
            "rm /tmp/x",
            "ash",
            &ExecOutcome::BackupRefused("big".into()),
        )
        .expect("refused → Some");
        assert_eq!(b.event_type, "command_backup_refused");
        assert!(b.payload_json.contains("big"), "{}", b.payload_json);
    }

    #[test]
    fn ran_payload_matches_audit_shape_and_masks_command() {
        let payload =
            ran_command_audit_payload("echo ghp_1234567890abcdef1234567890abcdef1234", "ash", 7);
        assert!(payload.contains("\"source\":\"ash\""), "{payload}");
        assert!(payload.contains("\"exit\":7"), "{payload}");
        assert!(payload.contains("\"command\""), "{payload}");
        assert!(
            !payload.contains("ghp_1234567890abcdef1234567890abcdef1234"),
            "{payload}"
        );
    }
}
