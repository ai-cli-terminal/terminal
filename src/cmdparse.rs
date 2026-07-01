//! 셸 명령 파싱 공용 헬퍼 (설계 §8). risk/preview/pipeline/verify가 공유한다.
//!
//! 선행 권한/환경 래퍼(`sudo`/`doas`/`env`/`nohup`/`nice`)와 `VAR=value` 환경 할당을
//! 건너뛰고 실제 프로그램 토큰·인자를 식별한다. 단일 진실원으로 모듈 간 동작을 일치시킨다.

/// 명령 앞에 올 수 있는 권한/환경 래퍼 토큰인지(`sudo`/`doas`/`env`/`nohup`/`nice`).
pub fn is_wrapper_token(tok: &str) -> bool {
    matches!(tok, "sudo" | "doas" | "env" | "nohup" | "nice")
}

/// `VAR=value` 형태의 환경 할당 토큰인지. 경로(`/`·`.` 시작)는 할당으로 보지 않는다.
pub fn is_env_assignment(tok: &str) -> bool {
    tok.contains('=') && !tok.starts_with('/') && !tok.starts_with('.')
}

/// 선행 래퍼/환경 할당을 건너뛴 실제 프로그램 토큰.
pub fn program_token(command: &str) -> Option<&str> {
    command
        .split_whitespace()
        .find(|t| !is_wrapper_token(t) && !is_env_assignment(t))
}

/// 선행 래퍼/환경 할당 + 프로그램 토큰까지 건너뛴 **나머지 인자** 이터레이터.
pub fn args_after_program(command: &str) -> impl Iterator<Item = &str> {
    let mut it = command.split_whitespace();
    for t in it.by_ref() {
        if is_wrapper_token(t) || is_env_assignment(t) {
            continue;
        }
        break; // 프로그램 토큰 소비
    }
    it
}

/// 토큰이 리다이렉트 연산자로 시작하면 연산자 뒤 나머지(대상; 분리형이면 "")를 반환한다.
/// 인식: 선택적 fd 접두(`[0-9]*` 또는 단일 `&`) + `>` + 선택적 `>`(append).
pub fn strip_redirect_op(tok: &str) -> Option<&str> {
    let bytes = tok.as_bytes();
    let mut j = 0;
    if j < bytes.len() && bytes[j] == b'&' {
        j += 1;
    } else {
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
    }
    if j >= bytes.len() || bytes[j] != b'>' {
        return None;
    }
    j += 1;
    if j < bytes.len() && bytes[j] == b'>' {
        j += 1;
    }
    Some(&tok[j..])
}

/// 리다이렉트 대상 파일명들을 추출한다. 붙은 형태는 토큰에서, 분리형(`> f`)은 다음 토큰에서.
pub fn redirect_targets<'a, I>(tokens: I) -> Vec<String>
where
    I: IntoIterator<Item = &'a str>,
{
    let toks: Vec<&str> = tokens.into_iter().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if let Some(rest) = strip_redirect_op(toks[i]) {
            if !rest.is_empty() {
                out.push(rest.to_string());
            } else if i + 1 < toks.len() {
                out.push(toks[i + 1].to_string());
                i += 1;
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_token_skips_wrappers_and_env() {
        assert_eq!(program_token("ls -al"), Some("ls"));
        assert_eq!(program_token("sudo rm -rf x"), Some("rm"));
        assert_eq!(
            program_token("env FOO=1 BAR=2 python app.py"),
            Some("python")
        );
        assert_eq!(
            program_token("nice nohup doas systemctl restart"),
            Some("systemctl")
        );
        assert_eq!(program_token(""), None);
    }

    #[test]
    fn env_assignment_excludes_paths() {
        assert!(is_env_assignment("FOO=bar"));
        assert!(!is_env_assignment("/usr/bin/x"));
        assert!(!is_env_assignment("./a=b")); // 경로는 할당 아님
        assert!(!is_env_assignment("plainarg"));
    }

    #[test]
    fn args_after_program_drops_leading_wrappers_and_prog() {
        let args: Vec<&str> = args_after_program("sudo FOO=1 rm -rf a b").collect();
        assert_eq!(args, vec!["-rf", "a", "b"]);
        let none: Vec<&str> = args_after_program("ls").collect();
        assert!(none.is_empty());
    }

    #[test]
    fn strip_redirect_op_recognizes_forms() {
        assert_eq!(strip_redirect_op(">out"), Some("out"));
        assert_eq!(strip_redirect_op(">>log"), Some("log"));
        assert_eq!(strip_redirect_op("2>err"), Some("err"));
        assert_eq!(strip_redirect_op("&>all"), Some("all"));
        assert_eq!(strip_redirect_op("2>>log"), Some("log"));
        assert_eq!(strip_redirect_op(">"), Some(""));
        assert_eq!(strip_redirect_op(">>"), Some(""));
        assert_eq!(strip_redirect_op("2>"), Some(""));
        assert_eq!(strip_redirect_op("123"), None);
        assert_eq!(strip_redirect_op("-i"), None);
        assert_eq!(strip_redirect_op("a=b"), None);
        assert_eq!(strip_redirect_op("file"), None);
    }

    #[test]
    fn redirect_targets_extracts_attached_and_detached() {
        assert_eq!(
            redirect_targets(["echo", "hi", ">out.txt"]),
            vec!["out.txt".to_string()]
        );
        assert_eq!(
            redirect_targets(["cmd", ">", "out.txt"]),
            vec!["out.txt".to_string()]
        );
        assert_eq!(
            redirect_targets(["cmd", "2>err", ">>log"]),
            vec!["err".to_string(), "log".to_string()]
        );
        assert!(redirect_targets(["cmd", ">"]).is_empty());
        assert!(redirect_targets(["ls", "-al"]).is_empty());
    }
}
