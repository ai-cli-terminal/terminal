use ai_terminal::config;
use ai_terminal::undo;

use crate::cli::inspect::resolve_profile;
use crate::cli::io::{AutoYes, StdinConfirmer, StdoutSink};

pub(crate) fn run_exec(command: &str, yes: bool, profile: Option<String>) -> anyhow::Result<()> {
    use ai_terminal::pipeline::{self, ExecConfig};

    let prof = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let undo_dir = undo::default_undo_dir()?;
    let cfg = ExecConfig {
        profile: &prof,
        undo_dir: &undo_dir,
        limits: undo::UndoLimits::defaults(),
    };
    let executor = pipeline::PtyExecutor { shell };
    let mut sink = StdoutSink;
    let mut confirmer: Box<dyn pipeline::Confirmer> = if yes {
        Box::new(AutoYes)
    } else {
        Box::new(StdinConfirmer)
    };

    let outcome = pipeline::execute(command, &cfg, &executor, confirmer.as_mut(), &mut sink)?;
    flush_stdout();
    finish_shell_outcome(command, "exec", outcome)
}

pub(crate) fn run_dispatch(input: &str, yes: bool, profile: Option<String>) -> anyhow::Result<()> {
    use ai_terminal::dispatch::{self, AiOutcome, Handled, Handlers};
    use ai_terminal::pipeline::{self, ExecConfig};

    let prof = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let undo_dir = undo::default_undo_dir()?;
    let cfg = ExecConfig {
        profile: &prof,
        undo_dir: &undo_dir,
        limits: undo::UndoLimits::defaults(),
    };
    let executor = pipeline::PtyExecutor { shell };
    let mut confirmer: Box<dyn pipeline::Confirmer> = if yes {
        Box::new(AutoYes)
    } else {
        Box::new(StdinConfirmer)
    };
    let mut ai = ai_terminal::responder::GatewayResponder::mock()?;
    let mut sink = StdoutSink;

    let mut h = Handlers {
        executor: &executor,
        confirmer: confirmer.as_mut(),
        ai: &mut ai,
        sink: &mut sink,
    };
    let handled = dispatch::run(input, &prof, &cfg, &mut h)?;
    flush_stdout();

    match handled {
        Handled::Empty => Ok(()),
        Handled::Shell(outcome) => finish_shell_outcome(input, "dispatch", outcome),
        Handled::Ai(AiOutcome::Answered {
            input_tokens,
            output_tokens,
            source,
            ..
        }) => {
            let usage = ai_terminal::ai_usage::summarize(
                &config::Ai {
                    provider: "mock".into(),
                    model: "mock-model".into(),
                    ..Default::default()
                },
                source,
                input_tokens,
                output_tokens,
            );
            #[cfg(feature = "storage")]
            let _ = ai_terminal::ai_usage::record(&usage, None);
            // 답변 본문은 이미 sink(stdout)로 출력됨. 토큰 요약만 덧붙인다.
            let cost_badge = if usage.estimated { " estimated" } else { "" };
            println!(
                "\n(tokens ~ in:{input_tokens} out:{output_tokens} · cost ~ ${:.4}{cost_badge}){}",
                usage.cost_usd,
                cache_badge(source)
            );
            Ok(())
        }
        Handled::Ai(AiOutcome::Blocked(r)) => {
            println!("[차단] 원격 전송 불가(fail-closed): {r}");
            Ok(())
        }
        Handled::Ai(AiOutcome::Unavailable(e)) => {
            println!("[AI 사용 불가] {e}");
            Ok(())
        }
    }
}

pub(crate) fn flush_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

/// 셸 실행 결과를 마무리한다: audit 기록 + 사용자 안내 + 프로세스 종료(항상 발산).
/// `run_exec`·`run_dispatch`가 공유한다. `command`는 기록/안내에 쓸 명령 텍스트.
pub(crate) fn finish_shell_outcome(
    command: &str,
    source: &str,
    outcome: ai_terminal::pipeline::ExecOutcome,
) -> ! {
    use ai_terminal::pipeline::ExecOutcome;

    if let ExecOutcome::Ran { exit_code, undo_id } = &outcome {
        if let Some(id) = undo_id {
            eprintln!("(백업 생성: {id} — 되돌리려면 `ai undo last`)");
        }
        ai_terminal::shell_audit::record_ran_command(command, *exit_code, source);
        std::process::exit(*exit_code);
    }

    // 비-Ran: audit 기록 후 안내 + exit 1.
    if let Some(rec) = ai_terminal::shell_audit::shell_outcome_audit(command, source, &outcome) {
        ai_terminal::shell_audit::record_outcome_audit(&rec);
    }
    match outcome {
        ExecOutcome::Blocked { level, factors } => {
            eprintln!("차단됨: 위험 등급 {level:?} (정책상 실행 불가)");
            for f in &factors {
                eprintln!("  - {f}");
            }
        }
        ExecOutcome::Declined => eprintln!("실행을 취소했습니다."),
        ExecOutcome::BackupRefused(r) => eprintln!("백업 거부로 실행 중단: {r}"),
        ExecOutcome::Ran { .. } => unreachable!("Ran 은 위에서 처리됨"),
    }
    std::process::exit(1);
}

/// 캐시 출처 배지(Backend는 무배지). `ai ask`·`ai dispatch` 공용.
pub(crate) fn cache_badge(source: ai_terminal::cache::CacheSource) -> &'static str {
    use ai_terminal::cache::CacheSource;
    match source {
        CacheSource::Backend => "",
        CacheSource::Exact => " [cache: exact]",
        CacheSource::Semantic => " [cache: semantic ~근사]",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_badge_labels() {
        use ai_terminal::cache::CacheSource;
        assert_eq!(cache_badge(CacheSource::Backend), "");
        assert!(cache_badge(CacheSource::Exact).contains("exact"));
        assert!(cache_badge(CacheSource::Semantic).contains("semantic"));
    }
}
