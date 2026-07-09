//! AI CLI 통합 리눅스 터미널 — `ai` 진입점 (스켈레톤).
//!
//! 설계 정본: `../document/`(v3.3). 본 골격은 M0 부트스트랩 단계의 최소 구현으로,
//! CLI 표면(§9, §31)을 잡아두고 이후 마일스톤(M1~M4)에서 채워 넣는다.
//!
//! 불변식(자세히는 `docs/RULES.md`):
//! - AI 기능 장애가 일반 셸 사용을 막아서는 안 된다(§3-3).
//! - AI 생성 명령은 자동 실행하지 않는다(§3-11).
//! - 로컬 정책/위험도 평가가 먼저 수행된다(§3-9).

mod cli;

use std::path::PathBuf;

use ai_terminal::config;
use ai_terminal::context;
use ai_terminal::dispatch;
use ai_terminal::gateway;
use ai_terminal::index;
use ai_terminal::intent;
use ai_terminal::mcp;
use ai_terminal::planner;
use ai_terminal::shell;
use ai_terminal::skill;
use ai_terminal::ui;
use ai_terminal::undo;
#[cfg(feature = "storage")]
use ai_terminal::usage;
use ai_terminal::verify_agent;
use clap::Parser;
use cli::command::{Cli, Command, InitMode, InitTarget, PolicyAction, RemoteAction};
use cli::doctor::run_doctor;
use cli::gate::{run_gate, run_gate_daemon};
use cli::hooks::{plan_init_shell, resolve_rc, resolve_shell};
#[cfg(feature = "storage")]
use cli::hooks::{record_hook_chpwd, record_hook_precmd, record_hook_preexec};
use cli::inspect::{
    describe_profile, format_mask, format_preview, format_risk, resolve_profile, run_explain,
};
use cli::remote::{
    run_remote_approval_url, run_remote_approval_verify, run_remote_devices, run_remote_pair,
    run_remote_relay_setup, run_remote_transport,
};
use cli::run::{cache_badge, run_dispatch, run_exec};

/// 현재 콘솔에 attach된 프로세스 수(Windows). 비-Windows·감지 실패 시 None.
#[cfg(windows)]
fn console_process_count() -> Option<u32> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleProcessList(lpdwProcessList: *mut u32, dwProcessCount: u32) -> u32;
    }
    let mut buf = [0u32; 4];
    // SAFETY: 유효한 버퍼 포인터와 길이로 호출. 반환값은 콘솔에 attach된 프로세스 수(0=실패).
    let n = unsafe { GetConsoleProcessList(buf.as_mut_ptr(), buf.len() as u32) };
    if n == 0 {
        None
    } else {
        Some(n)
    }
}

#[cfg(not(windows))]
fn console_process_count() -> Option<u32> {
    None
}

/// 탐색기 더블클릭으로 자기 콘솔을 단독 점유해 실행됐는지 추정한다(순수).
/// attach 프로세스가 자기 자신 1개뿐이면 double-click; 터미널 실행은 부모 셸도 attach 되어
/// 2 이상. None(비-Windows/감지 실패)은 false(보수적 — 일시정지하지 않음).
fn is_double_click_launch(console_process_count: Option<u32>) -> bool {
    console_process_count == Some(1)
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Doctor { guardrails }) => run_doctor(guardrails),
        Some(Command::Risk { command, profile }) => {
            let p = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
            print!("{}", format_risk(&command, &p));
            Ok(())
        }
        Some(Command::Policy { action }) => match action {
            PolicyAction::Show { profile } => {
                let p = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
                print!("{}", describe_profile(&p));
                Ok(())
            }
            PolicyAction::Set { profile } => {
                let p = resolve_profile(&profile)?;
                config::set_active_profile(p.name)?;
                println!("활성 정책 프로파일을 '{}'(으)로 설정했습니다.", p.name);
                Ok(())
            }
        },
        Some(Command::ShellHook { shell }) => {
            let sh = resolve_shell(Some(&shell))?;
            print!("{}", shell::hook_script(sh));
            Ok(())
        }
        Some(Command::Tui { profile }) => {
            let p = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
            ui::run(p.name)
        }
        Some(Command::Shell {}) => run_persistent_shell(),
        Some(Command::Remote { action }) => {
            use ai_terminal::gate;
            let path = gate::armed_path()?;
            match action {
                RemoteAction::Arm { allow_high } => {
                    gate::arm_at(&path, allow_high)?;
                    println!(
                        "원격 게이트 armed{}.",
                        if allow_high {
                            " (High opt-in 허용)"
                        } else {
                            ""
                        }
                    );
                }
                RemoteAction::Disarm {} => {
                    gate::disarm_at(&path)?;
                    println!("원격 게이트 disarmed.");
                }
                RemoteAction::Status {} => match gate::load_arm_state(&path) {
                    Some(st) => println!(
                        "armed (allow_high={}). 위험 명령이 인터셉트됩니다.",
                        st.allow_high
                    ),
                    None => println!("disarmed. 인터셉트 미개입."),
                },
                RemoteAction::Daemon {
                    device_id,
                    transport,
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    relay_ttl_seconds,
                } => run_gate_daemon(
                    device_id,
                    transport,
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    relay_ttl_seconds,
                )?,
                RemoteAction::Devices {} => run_remote_devices()?,
                RemoteAction::Transport {} => run_remote_transport()?,
                RemoteAction::RelaySetup {
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    device_id,
                    ttl_seconds,
                } => run_remote_relay_setup(
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    device_id,
                    ttl_seconds,
                )?,
                RemoteAction::Pair {
                    device_id,
                    code,
                    noise_pubkey_hex,
                    approval_pubkey_hex,
                    ttl_seconds,
                    pwa_url,
                } => run_remote_pair(
                    device_id,
                    code,
                    noise_pubkey_hex,
                    approval_pubkey_hex,
                    ttl_seconds,
                    pwa_url,
                )?,
                RemoteAction::ApprovalUrl {
                    request_json,
                    pwa_url,
                } => run_remote_approval_url(request_json, pwa_url)?,
                RemoteAction::ApprovalVerify {
                    request_json,
                    response_json,
                    device_id,
                    approval_pubkey_hex,
                    device_epoch,
                    now,
                    context_hash,
                } => run_remote_approval_verify(
                    request_json,
                    response_json,
                    device_id,
                    approval_pubkey_hex,
                    device_epoch,
                    now,
                    context_hash,
                )?,
            }
            Ok(())
        }
        Some(Command::Mask { text }) => {
            print!("{}", format_mask(&text));
            Ok(())
        }
        Some(Command::Preview { command }) => {
            print!("{}", format_preview(&command));
            Ok(())
        }
        Some(Command::Verify { command }) => {
            let profile = resolve_profile(&config::get_active_profile())?;
            let v = verify_agent::verify_command(&command, &profile);
            println!("binary   : {:?}", v.binary);
            println!("risk     : {:?} -> {:?}", v.risk, v.decision);
            println!("safe     : {}", v.safe_to_suggest);
            if !v.issues.is_empty() {
                println!("issues   :");
                for i in &v.issues {
                    println!("  - {i}");
                }
            }
            Ok(())
        }
        Some(Command::Classify { input }) => {
            println!("{:?}", intent::classify(&input));
            Ok(())
        }
        Some(Command::Plan { request }) => {
            for (i, step) in planner::plan(&request).steps.iter().enumerate() {
                let cmd = step.command.as_deref().unwrap_or("(AI 위임)");
                println!("{}. {} — {}", i + 1, step.description, cmd);
            }
            Ok(())
        }
        Some(Command::Index { query, root }) => {
            let idx = index::FileIndex::build(&root);
            let results = idx.search(&query, 10);
            if results.is_empty() {
                println!("(매칭 파일 없음, {} 파일 인덱싱)", idx.len());
            }
            for (path, score) in results {
                println!("  {score:>3}  {}", path.display());
            }
            Ok(())
        }
        Some(Command::Route { input }) => {
            let profile = resolve_profile(&config::get_active_profile())?;
            match dispatch::dispatch(&input, &profile) {
                dispatch::Route::Empty => println!("(빈 입력)"),
                dispatch::Route::Shell {
                    command,
                    risk,
                    decision,
                } => {
                    println!("route    : Shell");
                    println!("command  : {command}");
                    println!("risk     : {risk:?} -> {decision:?}");
                }
                dispatch::Route::Ai { prompt } => {
                    println!("route    : AI");
                    println!("prompt   : {prompt}");
                }
            }
            Ok(())
        }
        Some(Command::Ask {
            prompt,
            backend,
            model,
            ollama_url,
            openai_url,
        }) => {
            let ai_cfg = config::Ai {
                provider: backend.clone(),
                model: model.clone(),
                ollama_url: ollama_url.clone(),
                openai_url: openai_url.clone(),
            };
            let cap = ai_terminal::provider::Provider::mock().models[0].clone();
            let gw = match ai_cfg.provider.as_str() {
                "ollama" => {
                    let b = ai_terminal::ollama::OllamaBackend::new(
                        ai_terminal::http::TcpTransport,
                        &ai_cfg.ollama_url,
                        &ai_cfg.model,
                    );
                    gateway::Gateway::new(Box::new(b), cap)
                }
                "openai" => {
                    let api_key = std::env::var("OPENAI_API_KEY").ok();
                    let b = ai_terminal::openai::OpenAiBackend::new(
                        ai_terminal::http::TcpTransport,
                        &ai_cfg.openai_url,
                        &ai_cfg.model,
                        api_key,
                    );
                    gateway::Gateway::new(Box::new(b), cap)
                }
                _ => gateway::Gateway::mock(),
            };
            // 예산 게이트(§31.7) — storage가 있으면 누적 지출을 읽어 주입한다. 초과 시
            // 게이트웨이가 원격 백엔드 호출 전에 차단한다. default 빌드는 영속 지출을
            // 모르므로 미적용(현행 동작 보존).
            #[cfg(feature = "storage")]
            let gw = match ai_terminal::store::Store::open_default() {
                Ok(store) => {
                    let spent = store.total_cost(None).unwrap_or(0.0);
                    gw.with_budget(spent, usage::BudgetConfig::defaults())
                }
                Err(_) => gw,
            };
            let ctx = context::gather();
            // AI 호출을 타임아웃·Ctrl+C 취소와 함께 실행한다(§16.2, Graceful Recovery).
            let ctx_str = format!("cwd={}", ctx.cwd);
            let timeout = ai_terminal::aitask::Timeouts::defaults().request;
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let result = rt.block_on(async {
                let cancel = std::sync::Arc::new(tokio::sync::Notify::new());
                ai_terminal::aitask::cancel_on_ctrl_c(cancel.clone());
                gw.ask_cancellable(&prompt, &ctx_str, timeout, cancel).await
            });
            match result {
                Ok(gateway::GatewayOutcome::Answered {
                    text,
                    input_tokens,
                    output_tokens,
                    source,
                }) => {
                    println!("{text}");
                    let usage = ai_terminal::ai_usage::summarize(
                        &ai_cfg,
                        source,
                        input_tokens,
                        output_tokens,
                    );
                    let cost_badge = if usage.estimated { " estimated" } else { "" };
                    println!(
                        "(tokens ~ in:{input_tokens} out:{output_tokens} · cost ~ ${:.4}{cost_badge}){}",
                        usage.cost_usd,
                        cache_badge(source)
                    );
                    #[cfg(feature = "storage")]
                    let _ = ai_terminal::ai_usage::record(&usage, None);
                }
                Ok(gateway::GatewayOutcome::Blocked(reason)) => {
                    println!("[차단] 원격 전송 불가(fail-closed): {reason}");
                }
                Err(e) => {
                    // AI 장애는 셸로 전파되지 않는다(§3-3). 친절히 고지하고 정상 종료.
                    println!("[AI 사용 불가] {e}");
                }
            }
            Ok(())
        }
        Some(Command::Skill { query }) => {
            let mut paths = vec![PathBuf::from("./.ai-terminal/skills")];
            if let Ok(cd) = config::config_dir() {
                paths.push(cd.join("skills"));
            }
            let skills = skill::discover(&paths);
            let shown: Vec<&skill::Skill> = match &query {
                Some(q) => skill::match_skills(&skills, q, 5),
                None => skills.iter().collect(),
            };
            if shown.is_empty() {
                println!("(스킬 없음 — {:?})", paths);
            }
            for s in shown {
                println!("- {} — {}", s.name, s.description);
            }
            Ok(())
        }
        Some(Command::Mcp { config: cfg }) => {
            let path = match cfg {
                Some(p) => p,
                None => config::config_dir()?.join("mcp.json"),
            };
            match std::fs::read_to_string(&path) {
                Ok(json) => {
                    let servers = mcp::parse_servers(&json)?;
                    if servers.is_empty() {
                        println!("(등록된 MCP 서버 없음)");
                    }
                    for s in &servers {
                        println!("- {} : {} {}", s.name, s.command, s.args.join(" "));
                    }
                    println!("(auto_connect=false — 부작용 도구는 컨센트·감사 필요, §27)");
                }
                Err(_) => println!("mcp.json 없음: {}", path.display()),
            }
            Ok(())
        }
        Some(Command::Context {}) => {
            let c = context::gather();
            println!("cwd      : {}", c.cwd);
            println!("shell    : {}", c.shell);
            println!("user     : {}", c.user);
            println!("hostname : {}", c.hostname);
            println!("git      : {}", c.git_branch.as_deref().unwrap_or("-"));
            Ok(())
        }
        Some(Command::Explain {
            command,
            exit,
            stderr,
            last_error,
        }) => run_explain(command, exit, stderr, last_error),
        Some(Command::Undo { target }) => {
            if target != "last" {
                anyhow::bail!("지원하지 않는 undo 대상: {target} (last만 지원)");
            }
            let dir = undo::default_undo_dir()?;
            match undo::latest(&dir) {
                Some(id) => {
                    let n = undo::restore(&dir, &id)?;
                    println!("복구 완료: {n}개 파일 ({id})");
                }
                None => println!("복구할 백업이 없습니다."),
            }
            Ok(())
        }
        Some(Command::Init { target }) => match target {
            InitTarget::Shell {
                shell,
                rc,
                dry_run,
                diff,
                uninstall,
            } => {
                let sh = resolve_shell(shell.as_deref())?;
                let path = resolve_rc(rc, sh)?;
                let old = std::fs::read_to_string(&path).unwrap_or_default();
                let mode = if uninstall {
                    InitMode::Uninstall
                } else if diff {
                    InitMode::Diff
                } else if dry_run {
                    InitMode::DryRun
                } else {
                    InitMode::Install
                };
                let plan = plan_init_shell(&old, sh, mode, &path.display().to_string());
                if plan.write {
                    std::fs::write(&path, &plan.new_content)?;
                }
                print!("{}", plan.message);
                Ok(())
            }
        },
        #[cfg(feature = "storage")]
        Some(Command::Usage {}) => {
            let store = ai_terminal::store::Store::open_default()?;
            let spent = store.total_cost(None)?;
            let cfg = usage::BudgetConfig::defaults();
            let action = usage::evaluate(spent, cfg.session_usd, cfg.warn_pct, cfg.block_pct);
            println!("usage    : ${spent:.4} 사용");
            println!(
                "budget   : 세션 ${:.2} / 월 ${:.2} (경고 {}% / 차단 {}%)",
                cfg.session_usd, cfg.monthly_usd, cfg.warn_pct, cfg.block_pct
            );
            println!("status   : {action:?}");
            Ok(())
        }
        #[cfg(feature = "storage")]
        Some(Command::History { limit }) => {
            let store = ai_terminal::store::Store::open_default()?;
            let rows = store.recent_commands(limit)?;
            if rows.is_empty() {
                println!("(아직 기록된 명령이 없습니다)");
            }
            for r in rows.iter().rev() {
                let lvl = r.risk_level.as_deref().unwrap_or("-");
                println!("[{lvl:<8}] {}", r.command_text);
            }
            Ok(())
        }
        Some(Command::Exec {
            command,
            yes,
            profile,
        }) => run_exec(&command, yes, profile),
        Some(Command::Dispatch {
            input,
            yes,
            profile,
        }) => run_dispatch(&input, yes, profile),
        Some(Command::Gate { command }) => {
            let cmd = command.join(" ");
            let code = run_gate(&cmd);
            std::process::exit(code);
        }
        Some(Command::Hook { event, rest }) => {
            // hook 실패가 셸을 막지 않도록 항상 Ok 반환(best-effort).
            #[cfg(feature = "storage")]
            match event.as_str() {
                // preexec: 명령을 위험도와 함께 기록(종료 코드 미정).
                "preexec" => {
                    if let Err(e) = record_hook_preexec(&rest) {
                        tracing::debug!("hook preexec record failed (ignored): {e}");
                    }
                }
                // precmd: 직전 명령의 실제 종료 코드를 반영.
                "precmd" => {
                    if let Err(e) = record_hook_precmd(&rest) {
                        tracing::debug!("hook precmd record failed (ignored): {e}");
                    }
                }
                // chpwd: 작업 디렉터리 변경 → 세션 cwd + git branch 컨텍스트 스냅샷.
                "chpwd" => {
                    if let Err(e) = record_hook_chpwd(&rest) {
                        tracing::debug!("hook chpwd record failed (ignored): {e}");
                    }
                }
                _ => {}
            }
            tracing::trace!(event, ?rest, "shell hook event");
            Ok(())
        }
        None => {
            // TODO(M1): 인터랙티브 터미널(REPL/TUI) 진입. 현재는 사용법 안내.
            println!(
                "ai {} — `ai doctor` 로 환경 진단, `ai --help` 로 사용법 확인.",
                env!("CARGO_PKG_VERSION")
            );
            // 탐색기 더블클릭(자기 콘솔 단독 점유)이면 콘솔이 즉시 닫혀 안내를 못 본다.
            // 이 도구는 CLI다 — 사용법을 보여주고 Enter 입력까지 창을 유지한다(터미널 실행엔 무영향).
            if is_double_click_launch(console_process_count()) {
                use std::io::Write;
                println!();
                println!("이 파일은 AI Terminal GUI 앱이 아니라 명령줄(CLI) 도구 `ai.exe`입니다.");
                println!("Windows GUI 터미널은 릴리즈의 `ai-terminal-windows-*.zip` 또는 설치 파일을 받아 `ai-terminal.exe`를 실행하세요.");
                println!();
                println!("CLI로 사용할 때는 더블클릭이 아니라 터미널에서 실행하세요:");
                println!("  ai doctor                 # 환경 진단");
                println!("  ai risk \"rm -rf /tmp/x\"   # 위험도 평가");
                println!("  ai --help                 # 전체 명령");
                println!(
                    "CLI PATH 등록: scripts/install.ps1 · 문서: https://github.com/ai-cli-terminal/terminal"
                );
                print!("\n계속하려면 Enter 키를 누르세요... ");
                let _ = std::io::stdout().flush();
                let mut _line = String::new();
                let _ = std::io::stdin().read_line(&mut _line);
            }
            Ok(())
        }
    }
}

/// `ai shell` — 영속 PTY 셸(Native Wrapper, FU-3). 하나의 `PtySession`을 재사용해 `cd`가
/// 다음 명령에 유지되며(영속성), 각 명령 뒤 probe로 cwd를 동기화한다(§7.4). 라인 단위
/// REPL이며 입력 인터셉트·분류는 범위 외(라인 게이트는 `ai exec`/`ai tui`).
fn run_persistent_shell() -> anyhow::Result<()> {
    use std::io::{BufRead, Write};

    use ai_terminal::pty::PtySession;
    use ai_terminal::wrapper::{self, PROBE};

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    // 라인 에디터가 probe 마커(\x1f)를 가로채면 cwd 동기화가 멈추므로 셸별 안전 인자를 준다.
    let shell_args = wrapper::session_shell_args(&shell);
    let shell_arg_refs: Vec<&str> = shell_args.iter().map(String::as_str).collect();
    let mut session = PtySession::spawn(&shell, &shell_arg_refs)?;
    println!("ai shell — 영속 셸 (exit/quit/Ctrl-D 종료). cwd는 probe로 동기화됩니다.");

    let stdin = std::io::stdin();
    let mut last_cwd = String::new();
    loop {
        print!("ai> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break; // EOF(Ctrl-D)
        }
        let cmd = line.trim_end();
        if cmd == "exit" || cmd == "quit" {
            break;
        }
        if cmd.trim().is_empty() {
            continue;
        }

        session.write_input(&wrapper::probe_command(cmd))?;
        // probe 쌍(이번 명령의 cwd 방출)을 볼 때까지 출력을 모은다. 인터랙티브 echo로
        // 마커가 더 보일 수 있으나, 마지막 파싱 cwd가 실제값이다.
        let mut acc = String::new();
        for _ in 0..2000 {
            acc.push_str(&session.read_chunk()?);
            if acc.matches(PROBE).count() >= 2 {
                break;
            }
        }
        print!("{}", wrapper::strip_probes(&acc));
        let _ = std::io::stdout().flush();

        if let Some(cwd) = wrapper::parse_probe_cwds(&acc).into_iter().last() {
            if cwd != last_cwd && cwd.starts_with('/') {
                last_cwd = cwd.clone();
                #[cfg(feature = "storage")]
                sync_wrapper_cwd(&cwd);
            }
        }
    }
    let _ = session.kill();
    Ok(())
}

/// probe로 관측한 cwd를 세션 컨텍스트에 동기화한다(§7.4, storage feature).
#[cfg(feature = "storage")]
fn sync_wrapper_cwd(cwd: &str) {
    use ai_terminal::store::{NewContext, NewSession, Store};
    let Ok(store) = Store::open_default() else {
        return;
    };
    let session_id = "sess-default";
    let _ = store.get_or_create_session(
        session_id,
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: cwd.to_string(),
            policy_profile: config::get_active_profile(),
        },
    );
    let branch = ai_terminal::context::git_branch(std::path::Path::new(cwd));
    let _ = store.update_session_cwd(session_id, cwd);
    let _ = store.record_context_snapshot(&NewContext {
        session_id: session_id.into(),
        context_type: "wrapper_probe".into(),
        cwd: Some(cwd.to_string()),
        git_branch: branch,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_click_launch_only_when_sole_console_process() {
        // 자기 콘솔 단독 점유(1) = 더블클릭. 터미널 실행(부모 셸 attach, 2+)·감지 실패(None)는 아님.
        assert!(is_double_click_launch(Some(1)));
        assert!(!is_double_click_launch(Some(2)));
        assert!(!is_double_click_launch(Some(5)));
        assert!(!is_double_click_launch(None));
    }

    #[test]
    fn parses_exec_command() {
        let cli = Cli::parse_from(["ai", "exec", "rm -rf build", "--yes"]);
        match cli.command {
            Some(Command::Exec {
                command,
                yes,
                profile,
            }) => {
                assert_eq!(command, "rm -rf build");
                assert!(yes);
                assert!(profile.is_none());
            }
            other => panic!("expected Exec, got {other:?}"),
        }
    }

    #[test]
    fn parses_dispatch_command() {
        let cli = Cli::parse_from(["ai", "dispatch", "ls -al", "--yes"]);
        match cli.command {
            Some(Command::Dispatch {
                input,
                yes,
                profile,
            }) => {
                assert_eq!(input, "ls -al");
                assert!(yes);
                assert!(profile.is_none());
            }
            other => panic!("expected Dispatch, got {other:?}"),
        }
    }
}
