#[cfg(feature = "storage")]
use ai_terminal::config;

/// `ai shell` — 영속 PTY 셸(Native Wrapper, FU-3). 하나의 `PtySession`을 재사용해 `cd`가
/// 다음 명령에 유지되며(영속성), 각 명령 뒤 probe로 cwd를 동기화한다(§7.4). 라인 단위
/// REPL이며 입력 인터셉트·분류는 범위 외(라인 게이트는 `ai exec`/`ai tui`).
pub(crate) fn run_persistent_shell() -> anyhow::Result<()> {
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
pub(crate) fn sync_wrapper_cwd(cwd: &str) {
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
