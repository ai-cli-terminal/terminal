//! 에러 분석 (설계 §4.3 `ai explain last-error`, M3/W12).
//!
//! 직전 명령·종료 코드·stderr·cwd로 원인과 해결책을 제안한다. MVP는 규칙 기반
//! 휴리스틱(결정성)이며, 이후 AI 보조 신호를 결합한다(로컬 우선, `docs/RULES.md`).

/// 분석 입력 컨텍스트.
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub command: String,
    pub exit_code: i32,
    pub stderr: String,
    pub cwd: String,
}

/// 분석 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    pub summary: String,
    pub suggestions: Vec<String>,
}

/// 컨텍스트로부터 원인/해결책을 추론한다(규칙 기반, deterministic).
pub fn explain(ctx: &ErrorContext) -> Explanation {
    let lower = ctx.stderr.to_lowercase();
    let program = ctx
        .command
        .split_whitespace()
        .find(|t| !matches!(*t, "sudo" | "doas" | "env"))
        .unwrap_or("");

    if ctx.exit_code == 0 && ctx.stderr.trim().is_empty() {
        return Explanation {
            summary: "오류 없음 (종료 코드 0)".into(),
            suggestions: vec![],
        };
    }

    if ctx.exit_code == 127 || lower.contains("command not found") {
        return Explanation {
            summary: format!("명령을 찾을 수 없습니다: {program}"),
            suggestions: vec![
                "명령 철자를 확인하세요".into(),
                format!("`command -v {program}` 로 설치 여부를 확인하세요"),
                "필요하면 패키지를 설치하세요(apt/brew 등)".into(),
            ],
        };
    }

    if ctx.exit_code == 126 || lower.contains("permission denied") {
        return Explanation {
            summary: "권한이 거부되었습니다".into(),
            suggestions: vec![
                "`ls -l` 로 권한/소유자를 확인하세요".into(),
                format!("실행 권한이 필요하면 `chmod +x {program}`"),
                "시스템 변경이면 신중히 `sudo` 사용을 검토하세요".into(),
            ],
        };
    }

    if lower.contains("no such file or directory") {
        return Explanation {
            summary: "파일 또는 경로가 없습니다".into(),
            suggestions: vec![
                format!("현재 디렉터리({})와 경로 오타를 확인하세요", ctx.cwd),
                "`ls` 로 대상 존재를 확인하세요".into(),
            ],
        };
    }

    let tail: String = ctx.stderr.lines().last().unwrap_or("").to_string();
    Explanation {
        summary: format!("명령이 종료 코드 {}로 실패했습니다", ctx.exit_code),
        suggestions: if tail.is_empty() {
            vec!["stderr 출력을 확인하세요".into()]
        } else {
            vec![format!("stderr: {tail}")]
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(command: &str, exit_code: i32, stderr: &str) -> ErrorContext {
        ErrorContext {
            command: command.into(),
            exit_code,
            stderr: stderr.into(),
            cwd: "/home/u".into(),
        }
    }

    #[test]
    fn success_has_no_error() {
        let e = explain(&ctx("ls", 0, ""));
        assert!(e.summary.contains("오류 없음"), "{}", e.summary);
        assert!(e.suggestions.is_empty());
    }

    #[test]
    fn command_not_found_is_detected() {
        let e = explain(&ctx(
            "frobnicate",
            127,
            "bash: frobnicate: command not found",
        ));
        assert!(e.summary.contains("찾을 수 없"), "{}", e.summary);
        assert!(!e.suggestions.is_empty());
    }

    #[test]
    fn permission_denied_is_detected() {
        let e = explain(&ctx("./run.sh", 126, "bash: ./run.sh: Permission denied"));
        assert!(e.summary.contains("권한"), "{}", e.summary);
        assert!(e
            .suggestions
            .iter()
            .any(|s| s.contains("chmod") || s.contains("sudo")));
    }

    #[test]
    fn no_such_file_is_detected() {
        let e = explain(&ctx("cat x", 1, "cat: x: No such file or directory"));
        assert!(
            e.summary.contains("파일") || e.summary.contains("경로"),
            "{}",
            e.summary
        );
    }

    #[test]
    fn generic_failure_reports_exit_code() {
        let e = explain(&ctx("make", 2, "make: *** error"));
        assert!(e.summary.contains('2'), "{}", e.summary);
    }
}
