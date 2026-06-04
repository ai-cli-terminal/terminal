//! Native Wrapper: 영속 PTY 셸 + probe 기반 built-in 상태 동기화 (설계 §30-1, §7.4).
//!
//! `ai exec`/`ai tui`는 명령마다 새 PTY를 써 `cd` 등 built-in 상태가 유지되지 않는다.
//! 본 모듈은 **영속 세션**에서 명령을 실행하고, 실행 후 cwd를 marker(`\x1f`)로 방출·파싱해
//! 세션 컨텍스트를 동기화한다(§7.4 probe). 바운디드 MVP: cwd만, 라인 단위(인터셉트 제외).

/// probe 마커 — unit separator(U+001F). 일반 출력에 거의 등장하지 않아 경계로 안전하다.
pub const PROBE: char = '\u{1f}';

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

        let mut s = PtySession::spawn("bash", &[]).unwrap();
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
