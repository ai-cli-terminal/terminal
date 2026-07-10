use ai_terminal::config;
use ai_terminal::guardrails;
use ai_terminal::shell;

/// `ai doctor`용 config 진단 텍스트(순수 포매터).
pub(crate) fn format_config_diagnostics(loaded: &config::LoadedConfig) -> String {
    use std::fmt::Write as _;
    let source = match &loaded.source {
        config::ConfigSource::File(p) => format!("file: {}", p.display()),
        config::ConfigSource::Default => "default (no file)".to_string(),
    };
    let shell = loaded
        .config
        .general
        .default_shell
        .as_deref()
        .unwrap_or("<unset>");
    let mut out = String::new();
    let _ = writeln!(out, "config: {source}");
    let _ = writeln!(
        out,
        "  general.history_limit = {}",
        loaded.config.general.history_limit
    );
    let _ = write!(out, "  general.default_shell = {shell}");
    if let Some(w) = &loaded.warning {
        let _ = write!(out, "\n  warning: {w}");
    }
    out
}

/// `ai doctor` — 현재 환경/플랫폼 capability를 표시한다.
///
/// MVP에서는 정적 분석·preview·timeout 등 baseline guardrails를 모든 플랫폼에서
/// 보장하고, 동적 감시(seccomp/cgroups 등)는 플랫폼별로 다르다(§31.11).
pub(crate) fn run_doctor(show_guardrails: bool) -> anyhow::Result<()> {
    println!("AI Terminal doctor");
    println!("  version : {}", env!("CARGO_PKG_VERSION"));
    println!("{}", format_config_diagnostics(&config::load()));
    println!("  os      : {}", std::env::consts::OS);
    println!("  arch    : {}", std::env::consts::ARCH);

    // 통합 모드(§30-1): hook 마커가 현재 셸에 있으면 hook, 아니면 wrapper fallback.
    let hook_on = shell::hook_active(|k| std::env::var(k).ok());
    let mode = shell::resolve_integration_mode(shell::ConfiguredMode::Auto, hook_on);
    match mode {
        shell::IntegrationMode::Hook => println!("  shell   : hook 통합 활성"),
        shell::IntegrationMode::Wrapper => {
            println!("  shell   : wrapper fallback (hook 미감지)");
            println!(
                "            명령을 `ai exec \"<cmd>\"`로 실행하면 컨텍스트가 기록됩니다. hook 설치: `ai init shell`."
            );
        }
    }

    if show_guardrails {
        let platform = guardrails::detect();
        println!("\nplatform : {platform:?}  (정본 §31.11)");
        println!("baseline guardrails (모든 플랫폼):");
        for g in guardrails::baseline() {
            println!("  - {g}");
        }
        println!("platform-specific (동적 감시):");
        for c in guardrails::capabilities(platform) {
            println!("  - {:<28} {:?}", c.name, c.support);
        }
        if guardrails::dynamic_monitoring_limited(platform) {
            println!(
                "\n[!] 동적 감시가 제한되는 플랫폼입니다. High 이상 명령 확인을 강화합니다(§31.11)."
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_diagnostics_show_file_source_and_values() {
        let loaded = ai_terminal::config::LoadedConfig {
            config: ai_terminal::config::Config {
                general: ai_terminal::config::General {
                    default_shell: Some("/bin/bash".to_string()),
                    history_limit: 123,
                },
                ai: Default::default(),
            },
            source: ai_terminal::config::ConfigSource::File(std::path::PathBuf::from(
                "/cfg/config.toml",
            )),
            warning: None,
        };
        let out = format_config_diagnostics(&loaded);
        assert!(out.contains("file: /cfg/config.toml"), "{out}");
        assert!(out.contains("general.history_limit = 123"), "{out}");
        assert!(out.contains("general.default_shell = /bin/bash"), "{out}");
    }

    #[test]
    fn config_diagnostics_show_default_and_warning() {
        let loaded = ai_terminal::config::LoadedConfig {
            config: ai_terminal::config::Config::default(),
            source: ai_terminal::config::ConfigSource::Default,
            warning: Some("boom".to_string()),
        };
        let out = format_config_diagnostics(&loaded);
        assert!(out.contains("default (no file)"), "{out}");
        assert!(out.contains("general.default_shell = <unset>"), "{out}");
        assert!(out.contains("warning: boom"), "{out}");
    }
}
