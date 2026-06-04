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
}
