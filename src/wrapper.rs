//! Native Wrapper: 영속 PTY 셸 + probe 기반 built-in 상태 동기화 (설계 §30-1, §7.4).
//!
//! `ai exec`/`ai tui`는 명령마다 새 PTY를 써 `cd` 등 built-in 상태가 유지되지 않는다.
//! 본 모듈은 **영속 세션**에서 명령을 실행하고, 실행 후 cwd를 marker(`\x1f`)로 방출·파싱해
//! 세션 컨텍스트를 동기화한다(§7.4 probe). 바운디드 MVP: cwd만, 라인 단위(인터셉트 제외).

/// probe 마커 — unit separator(U+001F). 일반 출력에 거의 등장하지 않아 경계로 안전하다.
pub const PROBE: char = '\u{1f}';

/// 영속 세션 셸을 띄울 때 줄 인자.
///
/// **왜 필요한가**: PROBE(`\x1f` = Ctrl-_)는 bash readline에서 `undo` 키바인딩이다.
/// 라인 에디터가 켜진 인터랙티브 셸에 `\x1f`를 입력하면 readline이 이를 편집 명령으로
/// 가로채 명령줄을 망가뜨려 probe가 출력에 **전혀 도달하지 못한다**. 그러면
/// [`crate::pty::PtySession::read_chunk`]의 블로킹 read가 마커를 영원히 기다리며 멈춘다.
/// bash는 `--noediting`으로 라인 에디터를 끄면 `\x1f`가 리터럴로 통과한다(검증 완료).
/// 사용자 rc(별칭/PATH)는 보존하기 위해 `--norc`/`--noprofile`은 붙이지 않는다.
/// bash 외 셸은 라인 에디터 비활성 플래그가 제각각이라 인자를 비워 둔다(MVP 범위: bash).
pub fn session_shell_args(shell: &str) -> Vec<String> {
    let name = std::path::Path::new(shell)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(shell);
    if name == "bash" {
        vec!["--noediting".to_string()]
    } else {
        Vec::new()
    }
}

/// 사용자 명령 실행 후 `\x1f$PWD\x1f`를 방출하도록 감싼 셸 명령을 만든다.
/// 사용자 명령의 종료 코드와 무관하게 probe가 방출되도록 `;`로 잇는다.
pub fn probe_command(user_command: &str) -> String {
    format!("{user_command}\nprintf '{PROBE}%s{PROBE}' \"$PWD\"\n")
}

/// PTY 출력에서 probe 마커쌍 사이의 cwd 값들을 추출한다(순수). 마커 밖 텍스트와
/// 닫히지 않은(홀수) 마커는 무시한다.
pub fn parse_probe_cwds(output: &str) -> Vec<String> {
    let parts: Vec<&str> = output.split(PROBE).collect();
    // parts: [밖, cwd, 밖, cwd, ...]. cwd는 홀수 인덱스이며, **뒤에 닫는 마커가 있어야**
    // 완결된 쌍이다(마지막 조각이 홀수면 닫히지 않은 dangling → 제외).
    let last = parts.len().saturating_sub(1);
    parts
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 1 && *i < last)
        .map(|(_, s)| s.to_string())
        .collect()
}

/// 표시용으로 마커 구간(`\x1f...\x1f`)을 제거한 출력을 만든다.
pub fn strip_probes(output: &str) -> String {
    let parts: Vec<&str> = output.split(PROBE).collect();
    // 짝수 인덱스(마커 밖)만 이어 붙인다. 홀수 인덱스는 cwd 페이로드라 버린다.
    parts
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, s)| *s)
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_command_emits_pwd_marker() {
        let c = probe_command("ls -al");
        assert!(c.contains("ls -al"));
        assert!(c.contains("$PWD"));
        assert!(c.contains(PROBE));
    }

    #[test]
    fn parse_probe_cwds_extracts_marker_pairs() {
        let out = format!("hello{PROBE}/tmp{PROBE}world{PROBE}/home/u{PROBE}");
        assert_eq!(parse_probe_cwds(&out), vec!["/tmp", "/home/u"]);
    }

    #[test]
    fn parse_probe_cwds_ignores_unclosed_and_plain() {
        assert!(parse_probe_cwds("no markers here").is_empty());
        // 닫히지 않은 마커(홀수) — 마지막 조각은 페이로드가 아니므로 버려진다.
        let out = format!("a{PROBE}/x{PROBE}b{PROBE}dangling");
        assert_eq!(parse_probe_cwds(&out), vec!["/x"]);
    }

    #[test]
    fn strip_probes_removes_marker_payload() {
        let out = format!("line1\n{PROBE}/tmp{PROBE}line2\n");
        assert_eq!(strip_probes(&out), "line1\nline2\n");
    }

    /// 영속 PtySession에서 cd가 다음 명령에 유지되고 probe가 cwd를 보고함을 검증한다(WSL).
    #[cfg(unix)]
    #[test]
    fn persistent_session_keeps_cwd_and_probe_reports_it() {
        use crate::pty::PtySession;

        let args = session_shell_args("bash");
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut s = PtySession::spawn("bash", &arg_refs).unwrap();
        // 회귀(예: 라인 에디터가 다시 켜져 probe가 안 나오는 경우) 시 블로킹 read가 무한
        // 대기하지 않도록 워치독으로 5s 뒤 자식을 kill → read가 EOF로 풀려 fail-fast 된다.
        let mut killer = s.killer();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(5));
            let _ = killer.kill();
        });
        // 1) cd 후 probe.
        s.write_input(&probe_command("cd /tmp")).unwrap();
        // 2) 같은 세션에서 pwd 후 probe — cd가 유지되면 /tmp가 보고된다.
        s.write_input(&probe_command("true")).unwrap();
        let mut acc = String::new();
        for _ in 0..100 {
            acc.push_str(&s.read_chunk().unwrap());
            if acc.matches(PROBE).count() >= 4 {
                break;
            }
        }
        let _ = s.kill();
        let cwds = parse_probe_cwds(&acc);
        assert!(
            cwds.iter().any(|c| c.contains("/tmp")),
            "probe가 cd된 cwd(/tmp)를 보고해야 함(영속성): {cwds:?} / raw={acc:?}"
        );
    }
}
