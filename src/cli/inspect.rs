use ai_terminal::explain;
use ai_terminal::mask;
use ai_terminal::policy::PolicyProfile;
use ai_terminal::preview;
use ai_terminal::risk;
use ai_terminal::verify::{self, BinaryStatus};

/// `ai explain` 출력 문자열을 만든다.
pub(crate) fn format_explain(command: &str, exit: i32, stderr: &str) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let e = explain::explain(&explain::ErrorContext {
        command: command.into(),
        exit_code: exit,
        stderr: stderr.into(),
        cwd,
    });
    let mut s = format!("원인     : {}\n", e.summary);
    if !e.suggestions.is_empty() {
        s.push_str("제안     :\n");
        for sug in &e.suggestions {
            s.push_str(&format!("  - {sug}\n"));
        }
    }
    s
}

/// `ai explain` 실행: 주어진 명령(또는 `--last-error`로 저장소의 직전 실패 명령)을 분석한다.
pub(crate) fn run_explain(
    command: Option<String>,
    exit: i32,
    stderr: String,
    last_error: bool,
) -> anyhow::Result<()> {
    if last_error {
        #[cfg(feature = "storage")]
        {
            let store = ai_terminal::store::Store::open_default()?;
            match store.last_error("sess-default")? {
                Some(row) => {
                    let ec = row.exit_code.unwrap_or(1) as i32;
                    print!("{}", format_explain(&row.command_text, ec, ""));
                }
                None => println!("최근 실패한 명령이 없습니다."),
            }
            return Ok(());
        }
        #[cfg(not(feature = "storage"))]
        anyhow::bail!("--last-error 는 storage feature 빌드에서만 사용할 수 있습니다.");
    }
    let command = command
        .ok_or_else(|| anyhow::anyhow!("분석할 명령을 지정하거나 --last-error 를 사용하세요."))?;
    print!("{}", format_explain(&command, exit, &stderr));
    Ok(())
}

/// `ai preview` 출력 문자열을 만든다(실제 diff/content-at-risk).
pub(crate) fn format_preview(command: &str) -> String {
    use preview::PreviewRender;
    let mut s = String::new();
    for r in preview::render_preview(command) {
        match r {
            PreviewRender::Diff(d) => {
                s.push_str("preview  : 변경 diff (적용 전)\n");
                s.push_str(&d);
                if !d.ends_with('\n') {
                    s.push('\n');
                }
            }
            PreviewRender::ContentAtRisk {
                path,
                lines,
                bytes,
                head,
            } => {
                s.push_str(&format!(
                    "preview  : 손실 예정 {path} ({lines}줄, {bytes} bytes)\n"
                ));
                for line in head.lines() {
                    s.push_str(&format!("  | {line}\n"));
                }
            }
            PreviewRender::Info(m) => {
                s.push_str(&format!("preview  : {m}\n"));
            }
        }
    }
    if s.is_empty() {
        s.push_str("preview  : (출력 없음)\n");
    }
    s
}

/// `ai mask` 출력 문자열을 만든다.
pub(crate) fn format_mask(input: &str) -> String {
    let out = mask::Masker::baseline().mask(input);
    let redacted = if out.redactions.is_empty() {
        "(none)".to_string()
    } else {
        out.redactions.join(", ")
    };
    let remote = if out.blocked {
        format!("BLOCKED ({})", out.block_reason.unwrap_or_default())
    } else {
        "eligible".to_string()
    };
    format!(
        "masked   : {}\nredacted : {}\nremote   : {}\n",
        out.text, redacted, remote
    )
}

/// `ai risk` 출력 문자열을 만든다. (stdout 분리로 테스트 가능하게)
pub(crate) fn format_risk(command: &str, profile: &PolicyProfile) -> String {
    let a = risk::assess(command);
    let decision = profile.decide(a.level);
    let binary = match verify::check_binary(command) {
        BinaryStatus::Found(_) => "found".to_string(),
        BinaryStatus::Builtin => "builtin".to_string(),
        BinaryStatus::Unknown => "UNKNOWN (hallucination?)".to_string(),
    };
    let mut out = format!(
        "command  : {command}\nrisk     : {:?} ({}/100)\npolicy   : {} -> {:?}\nbinary   : {}\n",
        a.level, a.score, profile.name, decision, binary
    );
    if !a.factors.is_empty() {
        out.push_str("factors  :\n");
        for f in &a.factors {
            out.push_str(&format!("  {:+4}  {}\n", f.delta, f.label));
        }
    }
    out
}

/// 정책 프로파일의 주요 필드를 표시한다 (§31.3).
pub(crate) fn describe_profile(p: &PolicyProfile) -> String {
    format!(
        "profile   : {}\n\
         confirm   : {:?}\n\
         block     : critical={} high={}\n\
         remote_ai : {}\n\
         sudo_ai   : {}\n\
         masking   : secrets={} pii={} fail_closed={}\n\
         preview   : {}\n\
         auto_exec : {}\n\
         healing   : {} (max {})\n",
        p.name,
        p.confirm_level,
        p.block_critical,
        p.block_high_risk,
        p.allow_remote_ai,
        p.allow_sudo_ai_commands,
        p.mask_secrets,
        p.mask_pii,
        p.block_on_masking_failure,
        p.preview_file_modifications,
        p.auto_execute,
        p.auto_healing,
        p.auto_healing_max_attempts
    )
}

/// 프로파일 이름을 조회하고, 없으면 명확한 오류를 반환한다.
pub(crate) fn resolve_profile(name: &str) -> anyhow::Result<PolicyProfile> {
    PolicyProfile::by_name(name)
        .ok_or_else(|| anyhow::anyhow!("unknown profile: {name} (balanced|paranoid)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_risk_reports_score_level_and_decision() {
        let out = format_risk("rm -rf /", &PolicyProfile::balanced());
        assert!(out.contains("Critical"), "should report level: {out}");
        assert!(out.contains("100"), "should report score: {out}");
        assert!(out.contains("Block"), "balanced must Block critical: {out}");
    }

    #[test]
    fn describe_profile_shows_remote_ai_setting() {
        let out = describe_profile(&PolicyProfile::paranoid());
        assert!(out.contains("paranoid"), "{out}");
        assert!(out.to_lowercase().contains("remote_ai"), "{out}");
    }

    #[test]
    fn format_risk_flags_unknown_binary() {
        let out = format_risk("definitely_not_real_xyz123 foo", &PolicyProfile::balanced());
        assert!(
            out.contains("UNKNOWN"),
            "unknown binary should be flagged: {out}"
        );
    }

    #[test]
    fn format_risk_marks_builtin() {
        let out = format_risk("cd /tmp", &PolicyProfile::balanced());
        assert!(out.contains("builtin"), "{out}");
    }

    #[test]
    fn format_explain_reports_cause_and_suggestions() {
        let out = format_explain("frob", 127, "command not found");
        assert!(out.contains("찾을 수 없"), "{out}");
        assert!(out.contains("제안"), "{out}");
    }

    #[test]
    fn format_preview_lists_delete_targets() {
        let out = format_preview("rm -rf ./build");
        assert!(out.contains("preview  :"), "{out}");
        assert!(out.contains("./build"), "{out}");
    }

    #[test]
    fn format_preview_flags_not_available() {
        let out = format_preview("sudo systemctl restart nginx");
        assert!(out.contains("불가"), "{out}");
    }

    #[test]
    fn format_mask_redacts_and_blocks() {
        let out = format_mask("-----BEGIN RSA PRIVATE KEY-----");
        assert!(out.contains("PRIVATE_KEY_REDACTED"), "{out}");
        assert!(out.contains("BLOCKED"), "{out}");
    }

    #[test]
    fn format_mask_eligible_for_clean_text() {
        let out = format_mask("ls -al");
        assert!(out.contains("eligible"), "{out}");
        assert!(out.contains("(none)"), "{out}");
    }

    #[test]
    fn resolve_profile_rejects_unknown() {
        assert!(resolve_profile("nonexistent").is_err());
        assert!(resolve_profile("balanced").is_ok());
    }
}
