//! Context Consistency Manager (설계 §31.10, M4/W13).
//!
//! cwd·셸·git 상태 등 핵심 컨텍스트를 추적한다. env 수집은 allowlist 기반이며
//! denylist(TOKEN/SECRET/KEY/PASSWORD)와 PATH hash-only로 **secret을 저장하지 않는다**
//! (`docs/RULES.md` §2). 정확성보다 안정성 우선 — 전체 셸 상태를 복제하지 않는다.

use std::path::Path;

/// 추적하는 세션 컨텍스트(§31.10 필수 항목).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionContext {
    pub cwd: String,
    pub shell: String,
    pub user: String,
    pub hostname: String,
    pub git_branch: Option<String>,
}

/// env 수집 정책(§31.10 `[context.env]`).
#[derive(Debug, Clone)]
pub struct EnvPolicy {
    pub allowlist: Vec<String>,
    pub denylist_substrings: Vec<String>,
    pub hash_only: Vec<String>,
}

impl EnvPolicy {
    /// 기본값: allowlist 핵심 변수 / denylist 시크릿 패턴 / PATH는 hash-only.
    pub fn defaults() -> EnvPolicy {
        let s = |v: &str| v.to_string();
        EnvPolicy {
            allowlist: [
                "PATH",
                "SHELL",
                "USER",
                "HOME",
                "PWD",
                "VIRTUAL_ENV",
                "CONDA_DEFAULT_ENV",
                "NODE_ENV",
            ]
            .iter()
            .map(|v| s(v))
            .collect(),
            denylist_substrings: ["TOKEN", "SECRET", "KEY", "PASSWORD"]
                .iter()
                .map(|v| s(v))
                .collect(),
            hash_only: vec![s("PATH")],
        }
    }
}

/// env 한 항목을 정책에 따라 변환한다. 제외 대상은 None, PATH류는 해시 반환.
pub fn filter_env_var(policy: &EnvPolicy, key: &str, value: &str) -> Option<String> {
    let upper = key.to_uppercase();
    if policy
        .denylist_substrings
        .iter()
        .any(|d| upper.contains(d.as_str()))
    {
        return None;
    }
    if !policy.allowlist.iter().any(|a| a == key) {
        return None;
    }
    if policy.hash_only.iter().any(|h| h == key) {
        return Some(hash_hex(value));
    }
    Some(value.to_string())
}

/// 컨텍스트를 바꾸는 built-in 명령인지(§31.10 트리거).
pub fn is_context_changing(command: &str) -> bool {
    let toks: Vec<&str> = command.split_whitespace().collect();
    let first = toks.first().copied().unwrap_or("");
    if matches!(
        first,
        "cd" | "pushd" | "popd" | "export" | "unset" | "alias" | "unalias" | "source" | "."
    ) {
        return true;
    }
    if first == "git" {
        if let Some(sub) = toks.get(1) {
            return matches!(
                *sub,
                "checkout" | "switch" | "pull" | "reset" | "merge" | "rebase"
            );
        }
    }
    false
}

/// AI 컨텍스트와 실제 상태가 어긋나 refresh가 필요한지.
pub fn needs_refresh(
    ai_cwd: &str,
    actual_cwd: &str,
    ai_branch: Option<&str>,
    actual_branch: Option<&str>,
) -> bool {
    ai_cwd != actual_cwd || ai_branch != actual_branch
}

/// `dir`에서 위로 올라가며 `.git/HEAD`를 찾아 현재 브랜치를 읽는다(분리 HEAD는 "(detached)").
pub fn git_branch(dir: &Path) -> Option<String> {
    for ancestor in dir.ancestors() {
        let head = ancestor.join(".git").join("HEAD");
        if let Ok(content) = std::fs::read_to_string(&head) {
            let trimmed = content.trim();
            return Some(match trimmed.strip_prefix("ref: refs/heads/") {
                Some(branch) => branch.to_string(),
                None => "(detached)".to_string(),
            });
        }
    }
    None
}

fn hash_hex(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// 현재 환경에서 세션 컨텍스트를 수집한다(secret 미포함).
pub fn gather() -> SessionContext {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let git = git_branch(Path::new(&cwd));
    SessionContext {
        cwd: cwd.clone(),
        shell: std::env::var("SHELL").unwrap_or_default(),
        user: std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_default(),
        hostname: std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_default(),
        git_branch: git,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn uniq(tag: &str) -> PathBuf {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ai_ctx_{}_{}_{}", std::process::id(), tag, n));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn env_filter_drops_secrets_and_hashes_path() {
        let p = EnvPolicy::defaults();
        assert_eq!(filter_env_var(&p, "USER", "alice"), Some("alice".into()));
        assert!(filter_env_var(&p, "MY_SECRET_TOKEN", "abc").is_none());
        assert!(filter_env_var(&p, "AWS_SECRET_KEY", "xyz").is_none());
        assert!(filter_env_var(&p, "RANDOM_VAR", "v").is_none()); // allowlist 외
        let path = filter_env_var(&p, "PATH", "/usr/bin:/bin").unwrap();
        assert_ne!(path, "/usr/bin:/bin", "PATH는 해시로만 저장");
    }

    #[test]
    fn detects_context_changing_builtins() {
        assert!(is_context_changing("cd /tmp"));
        assert!(is_context_changing("export A=1"));
        assert!(is_context_changing("git switch feature"));
        assert!(is_context_changing("git pull"));
        assert!(!is_context_changing("git status"));
        assert!(!is_context_changing("ls -al"));
    }

    #[test]
    fn refresh_when_cwd_or_branch_diverges() {
        assert!(needs_refresh("/a", "/b", Some("main"), Some("main")));
        assert!(needs_refresh("/a", "/a", Some("main"), Some("dev")));
        assert!(!needs_refresh("/a", "/a", Some("main"), Some("main")));
    }

    #[test]
    fn reads_git_branch_from_head() {
        let repo = uniq("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
        assert_eq!(git_branch(&repo).as_deref(), Some("main"));

        let sub = repo.join("src");
        std::fs::create_dir_all(&sub).unwrap();
        assert_eq!(
            git_branch(&sub).as_deref(),
            Some("main"),
            "하위 디렉터리에서도 탐색"
        );

        assert_eq!(git_branch(&uniq("nogit")), None);
    }
}
