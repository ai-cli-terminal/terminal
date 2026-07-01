//! Context Consistency Manager (설계 §31.10, M4/W13).
//!
//! cwd·셸·git 상태 등 핵심 컨텍스트를 추적한다. env 수집은 allowlist 기반이며
//! denylist(TOKEN/SECRET/KEY/PASSWORD)와 PATH hash-only로 **secret을 저장하지 않는다**
//! (`docs/RULES.md` §2). 정확성보다 안정성 우선 — 전체 셸 상태를 복제하지 않는다.

use std::path::{Path, PathBuf};

/// 추적하는 세션 컨텍스트(§31.10 필수 항목).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionContext {
    pub cwd: String,
    pub shell: String,
    pub user: String,
    pub hostname: String,
    pub git_branch: Option<String>,
}

/// 원격 승인 TOCTOU 검증에 쓰는 컨텍스트 스냅샷.
///
/// secret 원문은 포함하지 않는다. env는 allowlist 후 hash-only 정책을 통과한 값만,
/// target은 명령 인자에서 보수적으로 뽑은 경로를 realpath 우선으로 정규화한 값만 담는다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemoteContextSnapshot {
    pub cwd_realpath: String,
    pub shell: String,
    pub user: String,
    pub hostname: String,
    pub git_branch: Option<String>,
    pub env: Vec<(String, String)>,
    pub targets: Vec<String>,
}

/// `ai __gate`가 데몬에 넘기는 origin context. 원격 승인 해시는 데몬 프로세스가
/// 아니라 이 셸 origin을 기준으로 계산해야 한다.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RemoteContextOrigin {
    pub cwd: String,
    pub env: Vec<(String, String)>,
}

impl RemoteContextOrigin {
    /// 현재 `ai __gate` 프로세스가 상속한 셸 cwd/env에서 origin을 만든다. env 값은 이미
    /// allowlist/hash-only 정책을 통과한 값만 담으므로 IPC payload에 secret 원문을 싣지 않는다.
    pub fn gather() -> Self {
        let policy = EnvPolicy::defaults();
        let mut env: Vec<(String, String)> = std::env::vars()
            .filter_map(|(k, v)| filter_env_var(&policy, &k, &v).map(|filtered| (k, filtered)))
            .collect();
        env.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            env,
        }
    }
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
        return Some(stable_hash_hex([value]));
    }
    Some(value.to_string())
}

/// 원격 AI 컨텍스트에 이 파일을 포함해도 되는지(§31.8 fail-closed).
///
/// 민감 경로(`.env`/`.pem`/`.key`/`id_rsa`/`credentials` 등)는 제외한다. 패턴은
/// [`crate::mask::is_sensitive_path`] 한 곳에서만 정의(단일 진실원). 이는 경로
/// 기준 1차 방어이며, 포함된 파일 본문은 추가로 마스킹(2차 방어)을 거친다.
pub fn allow_file_in_context(path: &str) -> bool {
    !crate::mask::is_sensitive_path(path)
}

/// 컨텍스트 후보 파일 경로에서 민감 경로를 제거하고 순서를 보존해 반환한다.
///
/// **계약**: 향후 파일 본문 수집기는 원격 전송 전 반드시 이 게이트를 통과시켜
/// `.env` 등의 유출을 막는다(§31.8).
pub fn filter_context_paths<I, S>(paths: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    paths
        .into_iter()
        .filter(|p| allow_file_in_context(p.as_ref()))
        .map(|p| p.as_ref().to_string())
        .collect()
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

/// 원격 승인 요청에 묶을 컨텍스트 해시를 계산한다.
pub fn remote_context_hash(command: &str) -> String {
    remote_context_hash_from_snapshot(&remote_context_snapshot(command))
}

/// 셸 origin context를 기준으로 원격 승인 요청 해시를 계산한다.
pub fn remote_context_hash_for_origin(command: &str, origin: &RemoteContextOrigin) -> String {
    remote_context_hash_from_snapshot(&remote_context_snapshot_for_origin(command, origin))
}

/// 테스트와 재검증용: 스냅샷 구조에서 안정적인 64-bit hex 해시를 만든다.
pub fn remote_context_hash_from_snapshot(snapshot: &RemoteContextSnapshot) -> String {
    let mut parts = vec![
        "v1".to_string(),
        snapshot.cwd_realpath.clone(),
        snapshot.shell.clone(),
        snapshot.user.clone(),
        snapshot.hostname.clone(),
        snapshot.git_branch.clone().unwrap_or_default(),
    ];
    for (k, v) in &snapshot.env {
        parts.push(format!("env:{k}={v}"));
    }
    for target in &snapshot.targets {
        parts.push(format!("target:{target}"));
    }
    stable_hash_hex(parts.iter().map(String::as_str))
}

/// 현재 프로세스 상태와 명령 대상 경로를 모아 원격 승인 스냅샷을 만든다.
pub fn remote_context_snapshot(command: &str) -> RemoteContextSnapshot {
    let origin = RemoteContextOrigin::gather();
    remote_context_snapshot_for_origin(command, &origin)
}

/// 셸 origin context를 기준으로 원격 승인 스냅샷을 만든다. 응답 직전에도 같은 origin으로
/// 다시 호출하면 target realpath 변경은 재계산되고, cwd/env는 승인 요청 당시 셸 origin에 묶인다.
pub fn remote_context_snapshot_for_origin(
    command: &str,
    origin: &RemoteContextOrigin,
) -> RemoteContextSnapshot {
    let cwd = std::env::current_dir().unwrap_or_default();
    let cwd = if origin.cwd.is_empty() {
        cwd
    } else {
        PathBuf::from(&origin.cwd)
    };
    let cwd_realpath = canonical_or_display(&cwd);
    let git = git_branch(&cwd);
    let mut env = origin.env.clone();
    env.sort_by(|a, b| a.0.cmp(&b.0));
    let targets = command_target_paths(command, &cwd);
    RemoteContextSnapshot {
        cwd_realpath,
        shell: env_value(&env, "SHELL").unwrap_or_default(),
        user: env_value(&env, "USER")
            .or_else(|| env_value(&env, "USERNAME"))
            .unwrap_or_default(),
        hostname: env_value(&env, "HOSTNAME")
            .or_else(|| env_value(&env, "COMPUTERNAME"))
            .unwrap_or_default(),
        git_branch: git,
        env,
        targets,
    }
}

fn env_value(env: &[(String, String)], key: &str) -> Option<String> {
    env.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn command_target_paths(command: &str, cwd: &Path) -> Vec<String> {
    let program = crate::cmdparse::program_token(command).unwrap_or_default();
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let redirects = crate::cmdparse::redirect_targets(tokens.iter().copied());
    let mut out: Vec<String> = redirects
        .into_iter()
        .map(|p| resolve_command_path(cwd, &p))
        .collect();

    let mut skip_next_redirect_target = false;
    let mut seen_chown_owner = false;
    for arg in crate::cmdparse::args_after_program(command) {
        if skip_next_redirect_target {
            skip_next_redirect_target = false;
            continue;
        }
        if let Some(rest) = crate::cmdparse::strip_redirect_op(arg) {
            if rest.is_empty() {
                skip_next_redirect_target = true;
            }
            continue;
        }
        if is_option_token(arg) {
            continue;
        }
        if program == "chmod" && looks_like_chmod_mode(arg) {
            continue;
        }
        if matches!(program, "chown" | "chgrp") && !seen_chown_owner {
            seen_chown_owner = true;
            continue;
        }
        if should_track_arg_as_target(program, arg) {
            out.push(resolve_command_path(cwd, arg));
        }
    }
    out.sort();
    out.dedup();
    out
}

fn should_track_arg_as_target(program: &str, arg: &str) -> bool {
    if looks_like_path(arg) {
        return true;
    }
    matches!(
        program,
        "rm" | "rmdir"
            | "mv"
            | "cp"
            | "chmod"
            | "chown"
            | "chgrp"
            | "truncate"
            | "tee"
            | "touch"
            | "mkdir"
    )
}

fn is_option_token(arg: &str) -> bool {
    arg.starts_with('-') && !matches!(arg, "-" | "./-" | "../-")
}

fn looks_like_path(arg: &str) -> bool {
    matches!(arg, "." | "..")
        || arg.starts_with('/')
        || arg.starts_with("./")
        || arg.starts_with("../")
        || arg.starts_with('~')
        || arg.contains('/')
}

fn looks_like_chmod_mode(arg: &str) -> bool {
    !arg.is_empty()
        && arg.chars().all(|c| {
            c.is_ascii_digit()
                || matches!(
                    c,
                    'u' | 'g'
                        | 'o'
                        | 'a'
                        | 'r'
                        | 'w'
                        | 'x'
                        | 'X'
                        | 's'
                        | 't'
                        | '+'
                        | '-'
                        | '='
                        | ','
                )
        })
}

fn resolve_command_path(cwd: &Path, raw: &str) -> String {
    let expanded = if raw == "~" || raw.starts_with("~/") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| {
                if raw == "~" {
                    home
                } else {
                    home.join(&raw[2..])
                }
            })
            .unwrap_or_else(|| PathBuf::from(raw))
    } else {
        PathBuf::from(raw)
    };
    let absolute = if expanded.is_absolute() {
        expanded
    } else {
        cwd.join(expanded)
    };
    canonical_or_display(&absolute)
}

fn canonical_or_display(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

fn stable_hash_hex<'a, I>(parts: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let mut h = 0xcbf29ce484222325u64;
    for part in parts {
        for b in part.as_bytes().iter().chain(std::iter::once(&0)) {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{h:016x}")
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
    fn sensitive_paths_excluded_from_context() {
        // 민감 경로는 원격 컨텍스트 포함 불가.
        for p in [
            "/home/u/project/.env",
            ".env.local",
            "secrets.pem",
            "deploy.key",
            "/root/.ssh/id_rsa",
            "aws/credentials",
        ] {
            assert!(!allow_file_in_context(p), "must exclude: {p}");
        }
        // 일반 소스는 포함 가능.
        for p in ["src/main.rs", "README.md", "/srv/app/config.toml"] {
            assert!(allow_file_in_context(p), "must allow: {p}");
        }
    }

    #[test]
    fn filter_context_paths_drops_sensitive_preserves_order() {
        let input = vec![
            "src/main.rs".to_string(),
            ".env".to_string(),
            "README.md".to_string(),
            "tls/server.pem".to_string(),
        ];
        let kept = filter_context_paths(input);
        assert_eq!(kept, vec!["src/main.rs", "README.md"]);
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

    #[test]
    fn remote_context_hash_covers_env_and_targets() {
        let base = RemoteContextSnapshot {
            cwd_realpath: "/repo".into(),
            shell: "/bin/bash".into(),
            user: "alice".into(),
            hostname: "host".into(),
            git_branch: Some("main".into()),
            env: vec![("PATH".into(), "hash-a".into())],
            targets: vec!["/repo/src/main.rs".into()],
        };
        let same = RemoteContextSnapshot {
            env: vec![("PATH".into(), "hash-a".into())],
            targets: vec!["/repo/src/main.rs".into()],
            ..base.clone()
        };
        assert_eq!(
            remote_context_hash_from_snapshot(&base),
            remote_context_hash_from_snapshot(&same)
        );

        let mut env_changed = base.clone();
        env_changed.env = vec![("PATH".into(), "hash-b".into())];
        assert_ne!(
            remote_context_hash_from_snapshot(&base),
            remote_context_hash_from_snapshot(&env_changed)
        );

        let mut target_changed = base.clone();
        target_changed.targets = vec!["/repo/src/lib.rs".into()];
        assert_ne!(
            remote_context_hash_from_snapshot(&base),
            remote_context_hash_from_snapshot(&target_changed)
        );
    }

    #[test]
    fn remote_context_snapshot_tracks_command_targets() {
        let snap = remote_context_snapshot("chmod -R 777 . > out.log");
        let cwd = canonical_or_display(&std::env::current_dir().unwrap());
        assert!(
            snap.targets.iter().any(|p| p.ends_with("out.log")),
            "redirect output target should be tracked: {:?}",
            snap.targets
        );
        assert!(
            snap.targets.iter().any(|p| p == &cwd),
            "dot target should resolve to current dir: {:?}",
            snap.targets
        );
    }

    #[test]
    fn remote_context_hash_uses_origin_cwd_and_env() {
        let dir_a = uniq("origin_a");
        let dir_b = uniq("origin_b");
        std::fs::create_dir_all(&dir_a).unwrap();
        std::fs::create_dir_all(&dir_b).unwrap();
        let origin_a = RemoteContextOrigin {
            cwd: dir_a.display().to_string(),
            env: vec![
                ("PATH".into(), "hash-a".into()),
                ("USER".into(), "alice".into()),
            ],
        };
        let origin_b = RemoteContextOrigin {
            cwd: dir_b.display().to_string(),
            env: vec![
                ("PATH".into(), "hash-a".into()),
                ("USER".into(), "alice".into()),
            ],
        };
        assert_ne!(
            remote_context_hash_for_origin("chmod 777 .", &origin_a),
            remote_context_hash_for_origin("chmod 777 .", &origin_b)
        );

        let origin_env_changed = RemoteContextOrigin {
            cwd: dir_a.display().to_string(),
            env: vec![
                ("PATH".into(), "hash-b".into()),
                ("USER".into(), "alice".into()),
            ],
        };
        assert_ne!(
            remote_context_hash_for_origin("chmod 777 .", &origin_a),
            remote_context_hash_for_origin("chmod 777 .", &origin_env_changed)
        );
        let _ = std::fs::remove_dir_all(&dir_a);
        let _ = std::fs::remove_dir_all(&dir_b);
    }
}
