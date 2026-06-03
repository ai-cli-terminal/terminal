//! 셸 통합: hook 스크립트 생성 + rc 삽입 블록 관리 (설계 §31.1, M1/W3).
//!
//! 확정값(§31.1): Hook 기반 기본 + Native Wrapper fallback, rc 수정은 **명시적 opt-in**.
//! MVP 필수 셸: bash, zsh.
//!
//! 불변식(`docs/RULES.md` §1):
//! - rc 파일은 자동 수정하지 않는다(명시적 `ai init shell`로만).
//! - `--uninstall`은 우리가 삽입한 블록만 제거하고 사용자 라인은 건드리지 않는다.
//! - Hook 실패가 일반 셸 사용을 중단시키지 않는다(`command -v ai` 가드 + 에러 무시).

/// 지원 셸(§31.1 MVP 필수).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
}

/// rc 삽입 블록의 시작/끝 마커. `--uninstall`이 이 범위만 제거한다.
pub const BEGIN_MARKER: &str = "# >>> AI Terminal integration >>>";
pub const END_MARKER: &str = "# <<< AI Terminal integration <<<";

impl Shell {
    /// 문자열에서 셸을 파싱한다. 경로(`/bin/zsh`)도 허용한다.
    pub fn parse(s: &str) -> Option<Shell> {
        let name = s.rsplit(['/', '\\']).next().unwrap_or(s);
        match name {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            _ => None,
        }
    }

    /// `bash` / `zsh` 식별자.
    pub fn as_str(&self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
        }
    }

    /// 기본 rc 파일명(`.bashrc` / `.zshrc`).
    pub fn default_rc_filename(&self) -> &'static str {
        match self {
            Shell::Bash => ".bashrc",
            Shell::Zsh => ".zshrc",
        }
    }
}

/// `ai shell-hook <shell>` 출력. preexec/precmd 등 §31.1 이벤트를 수집하는 hook.
///
/// 모든 외부 호출은 `command -v ai` 가드 + `2>/dev/null || true`로 감싸 hook 실패가
/// 일반 셸 사용을 막지 않게 한다(§31.1 수용 기준). 상태 보고는 내부 `ai __hook`로 보낸다
/// (현재 no-op, 기록은 W4 스토리지에서 구현).
pub fn hook_script(shell: Shell) -> String {
    match shell {
        Shell::Bash => BASH_HOOK.to_string(),
        Shell::Zsh => ZSH_HOOK.to_string(),
    }
}

/// rc 파일에 삽입할 가드 블록. `command -v ai` 가드로 감싸 미설치 환경에서도 무해하다.
pub fn rc_block(shell: Shell) -> String {
    format!(
        "{BEGIN_MARKER}\nif command -v ai >/dev/null 2>&1; then\n  eval \"$(ai shell-hook {})\"\nfi\n{END_MARKER}\n",
        shell.as_str()
    )
}

/// rc 내용에 우리 삽입 블록이 이미 있는지.
pub fn is_installed(content: &str) -> bool {
    content.contains(BEGIN_MARKER)
}

/// rc 내용에 가드 블록을 추가한다. 이미 설치돼 있으면 변경하지 않는다(idempotent).
pub fn apply_install(content: &str, shell: Shell) -> String {
    if is_installed(content) {
        return content.to_string();
    }
    let mut out = String::from(content);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&rc_block(shell));
    out
}

/// rc 내용에서 우리 블록(마커 사이)만 제거한다. 사용자 라인은 보존한다.
pub fn apply_uninstall(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let begin = lines.iter().position(|l| l.trim() == BEGIN_MARKER);
    let end = lines.iter().position(|l| l.trim() == END_MARKER);
    let (b, e) = match (begin, end) {
        (Some(b), Some(e)) if e >= b => (b, e),
        _ => return content.to_string(),
    };
    let kept: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < b || *i > e)
        .map(|(_, l)| *l)
        .collect();
    let mut out = kept.join("\n");
    if !out.is_empty() && content.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// 변경 전/후의 최소 unified diff를 만든다(공통 prefix/suffix 기반).
pub fn unified_diff(old: &str, new: &str, path: &str) -> String {
    let o: Vec<&str> = old.lines().collect();
    let n: Vec<&str> = new.lines().collect();
    let max_pre = o.len().min(n.len());
    let mut pre = 0;
    while pre < max_pre && o[pre] == n[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < (o.len() - pre).min(n.len() - pre) && o[o.len() - 1 - suf] == n[n.len() - 1 - suf] {
        suf += 1;
    }
    let removed = &o[pre..o.len() - suf];
    let added = &n[pre..n.len() - suf];
    let mut out = format!("--- {path}\n+++ {path}\n");
    out.push_str(&format!(
        "@@ -{},{} +{},{} @@\n",
        pre + 1,
        removed.len(),
        pre + 1,
        added.len()
    ));
    for l in removed {
        out.push_str(&format!("-{l}\n"));
    }
    for l in added {
        out.push_str(&format!("+{l}\n"));
    }
    out
}

const BASH_HOOK: &str = r#"# ai-terminal bash hook
__ai_preexec() {
  [ -n "$COMP_LINE" ] && return
  command -v ai >/dev/null 2>&1 && ai __hook preexec "cmd=$BASH_COMMAND" "cwd=$PWD" >/dev/null 2>&1 || true
}
__ai_precmd() {
  local __ai_ec=$?
  command -v ai >/dev/null 2>&1 && ai __hook precmd "exit=$__ai_ec" "cwd=$PWD" >/dev/null 2>&1 || true
  return $__ai_ec
}
trap '__ai_preexec' DEBUG
case "$PROMPT_COMMAND" in
  *__ai_precmd*) ;;
  *) PROMPT_COMMAND="__ai_precmd${PROMPT_COMMAND:+; $PROMPT_COMMAND}" ;;
esac
"#;

const ZSH_HOOK: &str = r#"# ai-terminal zsh hook
autoload -Uz add-zsh-hook
__ai_preexec() {
  command -v ai >/dev/null 2>&1 && ai __hook preexec "cmd=$1" "cwd=$PWD" >/dev/null 2>&1 || true
}
__ai_precmd() {
  local __ai_ec=$?
  command -v ai >/dev/null 2>&1 && ai __hook precmd "exit=$__ai_ec" "cwd=$PWD" >/dev/null 2>&1 || true
}
__ai_chpwd() {
  command -v ai >/dev/null 2>&1 && ai __hook chpwd "cwd=$PWD" >/dev/null 2>&1 || true
}
add-zsh-hook preexec __ai_preexec
add-zsh-hook precmd __ai_precmd
add-zsh-hook chpwd __ai_chpwd
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_names_and_paths() {
        assert_eq!(Shell::parse("bash"), Some(Shell::Bash));
        assert_eq!(Shell::parse("zsh"), Some(Shell::Zsh));
        assert_eq!(Shell::parse("/usr/bin/zsh"), Some(Shell::Zsh));
        assert_eq!(Shell::parse("/bin/bash"), Some(Shell::Bash));
        assert_eq!(Shell::parse("fish"), None);
    }

    #[test]
    fn default_rc_filenames() {
        assert_eq!(Shell::Bash.default_rc_filename(), ".bashrc");
        assert_eq!(Shell::Zsh.default_rc_filename(), ".zshrc");
    }

    #[test]
    fn bash_hook_wires_preexec_and_precmd_safely() {
        let h = hook_script(Shell::Bash);
        assert!(
            h.contains("command -v ai"),
            "must guard on ai presence: {h}"
        );
        assert!(
            h.contains("PROMPT_COMMAND") || h.contains("precmd"),
            "must hook precmd: {h}"
        );
        assert!(
            h.contains("DEBUG") || h.contains("preexec"),
            "must hook preexec: {h}"
        );
    }

    #[test]
    fn zsh_hook_uses_add_zsh_hook() {
        let h = hook_script(Shell::Zsh);
        assert!(
            h.contains("add-zsh-hook"),
            "zsh hook must use add-zsh-hook: {h}"
        );
        assert!(h.contains("precmd"));
        assert!(h.contains("preexec"));
        assert!(h.contains("command -v ai"));
    }

    #[test]
    fn rc_block_is_marker_wrapped_and_guarded() {
        let b = rc_block(Shell::Bash);
        assert!(b.contains(BEGIN_MARKER), "must have begin marker");
        assert!(b.contains(END_MARKER), "must have end marker");
        assert!(b.contains("command -v ai"), "must guard");
        assert!(b.contains("ai shell-hook bash"), "must eval the bash hook");
    }

    #[test]
    fn install_is_idempotent() {
        let rc = "export PATH=$HOME/bin:$PATH\n";
        let once = apply_install(rc, Shell::Bash);
        assert!(is_installed(&once));
        assert!(once.contains("export PATH=$HOME/bin:$PATH"));
        let twice = apply_install(&once, Shell::Bash);
        assert_eq!(once, twice, "installing twice must not duplicate the block");
    }

    #[test]
    fn uninstall_removes_only_our_block() {
        let rc = "line1\nexport X=1\nalias g=git\n";
        let installed = apply_install(rc, Shell::Zsh);
        assert_ne!(installed, rc);
        assert!(is_installed(&installed));
        let removed = apply_uninstall(&installed);
        assert_eq!(removed, rc, "uninstall must restore user content exactly");
    }

    #[test]
    fn uninstall_is_noop_when_absent() {
        let rc = "just user lines\n";
        assert_eq!(apply_uninstall(rc), rc);
    }

    #[test]
    fn diff_shows_added_block_lines() {
        let rc = "export X=1\n";
        let new = apply_install(rc, Shell::Bash);
        let d = unified_diff(rc, &new, "~/.bashrc");
        assert!(d.contains("~/.bashrc"), "diff must name the file: {d}");
        assert!(
            d.lines()
                .any(|l| l.starts_with('+') && l.contains("AI Terminal")),
            "diff must show added block: {d}"
        );
    }

    /// 생성한 hook 스크립트가 실제 셸 문법으로 유효한지 검증한다(WSL에서 실행).
    #[cfg(unix)]
    #[test]
    fn generated_hooks_pass_syntax_check() {
        use std::io::Write;
        use std::process::{Command, Stdio};
        for (shell, bin) in [(Shell::Bash, "bash"), (Shell::Zsh, "zsh")] {
            let script = hook_script(shell);
            let mut child = match Command::new(bin).arg("-n").stdin(Stdio::piped()).spawn() {
                Ok(c) => c,
                // 셸 바이너리가 없는 환경(예: zsh 미설치 CI 러너)에서는 해당 셸 검사를 건너뛴다.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    eprintln!("skip {bin} syntax check: not installed");
                    continue;
                }
                Err(e) => panic!("spawn {bin}: {e}"),
            };
            child
                .stdin
                .take()
                .unwrap()
                .write_all(script.as_bytes())
                .unwrap();
            let status = child.wait().unwrap();
            assert!(
                status.success(),
                "{bin} -n rejected generated hook:\n{script}"
            );
        }
    }
}
