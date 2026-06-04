//! 명령 환각 검증 게이트 (설계 §29.2, W8). 존재하지 않는 바이너리를 사전 식별한다.
//!
//! AI(또는 사용자)가 제시한 명령의 실행 프로그램이 PATH에 실재하는지 확인한다.
//! MVP는 바이너리 존재 검증까지(플래그 검증은 Phase 2, §13 hallucination_guard).

use std::path::PathBuf;

/// 바이너리 검증 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryStatus {
    /// PATH(또는 경로)에서 실행 파일을 찾음.
    Found(PathBuf),
    /// 셸 빌트인/키워드(별도 바이너리 불필요).
    Builtin,
    /// 찾을 수 없음(환각 가능성).
    Unknown,
}

const BUILTINS: &[&str] = &[
    "cd", "pushd", "popd", "export", "unset", "alias", "unalias", "source", ".", "set", "eval",
    "exec", "return", "local", "declare", "read", "true", "false", "test", ":", "history", "jobs",
    "fg", "bg", "wait", "type", "hash", "umask", "ulimit", "trap", "echo", "printf", "let",
];

fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&name)
}

/// 명령에서 실제 실행 프로그램 토큰을 추출한다(선행 래퍼·`VAR=` 건너뜀).
/// 파싱 규칙은 [`crate::cmdparse`] 단일 진실원에 위임한다.
pub fn extract_program(command: &str) -> Option<&str> {
    crate::cmdparse::program_token(command)
}

#[cfg(windows)]
fn exe_exts() -> Vec<String> {
    vec![
        String::new(),
        ".exe".into(),
        ".cmd".into(),
        ".bat".into(),
        ".com".into(),
    ]
}

#[cfg(not(windows))]
fn exe_exts() -> Vec<String> {
    vec![String::new()]
}

/// 주어진 디렉터리 목록에서 프로그램 실행 파일을 찾는다.
pub fn resolve_in_dirs(program: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    // 경로가 포함되면 그 경로를 직접 확인한다.
    if program.contains('/') || program.contains('\\') {
        let p = PathBuf::from(program);
        return p.is_file().then_some(p);
    }
    for dir in dirs {
        for ext in exe_exts() {
            let cand = dir.join(format!("{program}{ext}"));
            if cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

fn path_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default()
}

/// 명령의 실행 바이너리 상태를 판정한다(현재 환경 PATH 기준).
pub fn check_binary(command: &str) -> BinaryStatus {
    let program = match extract_program(command) {
        Some(p) => p,
        None => return BinaryStatus::Unknown,
    };
    if is_builtin(program) {
        return BinaryStatus::Builtin;
    }
    match resolve_in_dirs(program, &path_dirs()) {
        Some(p) => BinaryStatus::Found(p),
        None => BinaryStatus::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_program_skips_sudo_and_env() {
        assert_eq!(extract_program("sudo rm -rf /"), Some("rm"));
        assert_eq!(extract_program("FOO=bar ls -al"), Some("ls"));
        assert_eq!(extract_program("env X=1 git status"), Some("git"));
        assert_eq!(extract_program("ls"), Some("ls"));
        assert_eq!(extract_program("./build.sh"), Some("./build.sh"));
        assert_eq!(extract_program("   "), None);
    }

    #[test]
    fn builtin_is_recognized() {
        assert_eq!(check_binary("cd /tmp"), BinaryStatus::Builtin);
        assert_eq!(check_binary("export PATH=/x"), BinaryStatus::Builtin);
    }

    #[test]
    fn unknown_binary_is_flagged() {
        assert_eq!(
            check_binary("definitely_not_a_real_cmd_xyz123 --foo"),
            BinaryStatus::Unknown
        );
    }

    #[test]
    fn resolve_finds_file_in_dir() {
        let dir = std::env::temp_dir().join(format!("ai_verify_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let tool = dir.join("mytool");
        std::fs::write(&tool, b"#!/bin/sh\n").unwrap();
        let found = resolve_in_dirs("mytool", std::slice::from_ref(&dir));
        assert!(found.is_some(), "should resolve mytool in {dir:?}");
        assert!(resolve_in_dirs("nope_nope", std::slice::from_ref(&dir)).is_none());
    }
}
