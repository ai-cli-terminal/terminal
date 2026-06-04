//! M0 인터셉트 대화형 e2e — 생성 hook이 armed에서 위험 명령을 실행 전 차단함을 증명.
//! unix 전용(대화형 셸·PTY). zsh 미설치 시 zsh 케이스는 건너뛴다.
//!
//! 차단 메커니즘: bash extdebug DEBUG trap / zsh ZLE accept-line 위젯(설계 spike 실증).
//! 검증 명령 `rm -rf <abs>/target`은 High(55) → armed(allow_high=false)에서 Block.
#![cfg(unix)]

use std::time::Duration;

use ai_terminal::pty::PtySession;
use ai_terminal::shell::{self, Shell};

/// 셸별 e2e: armed 시 rm -rf 차단(대상 생존), 안전 명령 실행. None=셸 미설치 스킵.
fn run_case(shell: Shell, bin: &str) -> Option<(bool, bool)> {
    which(bin)?; // 미설치 스킵

    let tmp = std::env::temp_dir().join(format!("ra_e2e_{bin}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // 격리된 XDG_CONFIG_HOME + armed 파일(allow_high=false → High 차단).
    let xdg = tmp.join("cfg");
    std::fs::create_dir_all(xdg.join("ai-terminal")).unwrap();
    std::fs::write(xdg.join("ai-terminal").join("armed"), "allow_high=false\n").unwrap();

    // rc 파일 = 생성된 hook.
    let rc = tmp.join("rc");
    std::fs::write(&rc, shell::hook_script(shell)).unwrap();

    let target = tmp.join("target");
    std::fs::create_dir_all(&target).unwrap();
    let safe = tmp.join("safe");

    // ai 바이너리를 PATH 앞에 추가(hook의 ai __gate 가 해석되도록).
    let ai_bin = env!("CARGO_BIN_EXE_ai");
    let ai_dir = std::path::Path::new(ai_bin).parent().unwrap();
    let path_env = format!(
        "{}:{}",
        ai_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let argv: Vec<&str> = match shell {
        // bash: readline 무관(차단은 DEBUG trap). --noediting으로 결정성↑.
        Shell::Bash => vec!["--norc", "--noprofile", "--noediting", "-i"],
        // zsh: ZLE 위젯이 차단하므로 ZLE(인터랙티브)를 켠 채로. -f=rc 무시.
        Shell::Zsh => vec!["-f", "-i"],
    };
    let mut s = PtySession::spawn(bin, &argv).unwrap();
    // 무한 행 방지 워치독(회귀 안전장치): 8s 뒤 자식 kill → read가 EOF로 풀림.
    let mut killer = s.killer();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(8));
        let _ = killer.kill();
    });

    let send = |s: &mut PtySession, line: String| {
        s.write_input(&line).unwrap();
        std::thread::sleep(Duration::from_millis(300));
    };
    send(&mut s, format!("export PATH='{path_env}'\n"));
    send(
        &mut s,
        format!("export XDG_CONFIG_HOME='{}'\n", xdg.display()),
    );
    send(&mut s, format!("source '{}'\n", rc.display()));
    send(&mut s, format!("rm -rf '{}'\n", target.display())); // High → 차단
    send(&mut s, format!("echo hi > '{}'\n", safe.display())); // Low → 실행
    send(&mut s, "exit\n".to_string());

    // EOF까지 드레인(워치독이 상한 보장).
    for _ in 0..2000 {
        match s.read_chunk() {
            Ok(chunk) if chunk.is_empty() => break, // EOF
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    let _ = s.kill();

    let target_survived = target.is_dir();
    let safe_executed = safe.is_file();
    let _ = std::fs::remove_dir_all(&tmp);
    Some((target_survived, safe_executed))
}

/// 최소 which: PATH에서 실행 파일을 찾는다.
fn which(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|d| {
        let p = d.join(bin);
        if p.is_file() {
            Some(p)
        } else {
            None
        }
    })
}

#[test]
fn bash_armed_blocks_rm_rf_allows_safe() {
    match run_case(Shell::Bash, "bash") {
        Some((survived, safe)) => {
            assert!(
                survived,
                "armed bash: rm -rf 가 차단되어 대상이 살아남아야 함"
            );
            assert!(safe, "armed bash: 안전 명령은 실행되어야 함");
        }
        None => eprintln!("skip bash e2e: not installed"),
    }
}

#[test]
fn zsh_armed_blocks_rm_rf_allows_safe() {
    match run_case(Shell::Zsh, "zsh") {
        Some((survived, safe)) => {
            assert!(
                survived,
                "armed zsh: rm -rf 가 차단되어 대상이 살아남아야 함"
            );
            assert!(safe, "armed zsh: 안전 명령은 실행되어야 함");
        }
        None => eprintln!("skip zsh e2e: not installed"),
    }
}
