//! Windows external-command resolution for `ash`.
//!
//! This module is pure so Linux/WSL CI can lock down Windows rules before the
//! native adapter is exercised on a Windows runner.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowsInvocation {
    Direct(PathBuf),
    CmdScript(PathBuf),
    PowerShellScript(PathBuf),
}

impl WindowsInvocation {
    pub fn path(&self) -> &Path {
        match self {
            Self::Direct(p) | Self::CmdScript(p) | Self::PowerShellScript(p) => p,
        }
    }
}

pub fn default_pathext() -> Vec<String> {
    vec![
        ".COM".into(),
        ".EXE".into(),
        ".BAT".into(),
        ".CMD".into(),
        ".PS1".into(),
    ]
}

pub fn split_pathext(raw: Option<&str>) -> Vec<String> {
    let mut out = raw
        .unwrap_or(".COM;.EXE;.BAT;.CMD")
        .split(';')
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else if s.starts_with('.') {
                Some(s.to_ascii_uppercase())
            } else {
                Some(format!(".{}", s.to_ascii_uppercase()))
            }
        })
        .collect::<Vec<_>>();
    if out.is_empty() {
        out = default_pathext();
    }
    out
}

pub fn classify_windows_path(path: PathBuf) -> WindowsInvocation {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext.as_str() {
        "cmd" | "bat" => WindowsInvocation::CmdScript(path),
        "ps1" => WindowsInvocation::PowerShellScript(path),
        _ => WindowsInvocation::Direct(path),
    }
}

pub fn resolve_windows_invocation(
    name: &str,
    cwd: &Path,
    path_dirs: &[PathBuf],
    pathext: &[String],
) -> Option<WindowsInvocation> {
    resolve_windows_invocation_by(name, cwd, path_dirs, pathext, |p| p.is_file())
}

pub fn resolve_windows_invocation_by(
    name: &str,
    cwd: &Path,
    path_dirs: &[PathBuf],
    pathext: &[String],
    is_file: impl Fn(&Path) -> bool,
) -> Option<WindowsInvocation> {
    if has_path_separator(name) {
        let raw = PathBuf::from(name);
        let base = if raw.is_absolute() {
            raw
        } else {
            cwd.join(raw)
        };
        return resolve_candidate(base, pathext, is_file);
    }

    let search_dirs = std::iter::once(cwd.to_path_buf()).chain(path_dirs.iter().cloned());
    for dir in search_dirs {
        if let Some(invocation) = resolve_candidate(dir.join(name), pathext, &is_file) {
            return Some(invocation);
        }
    }
    None
}

fn resolve_candidate(
    base: PathBuf,
    pathext: &[String],
    is_file: impl Fn(&Path) -> bool,
) -> Option<WindowsInvocation> {
    if has_extension(&base) {
        return is_file(&base).then(|| classify_windows_path(base));
    }
    if is_file(&base) {
        return Some(classify_windows_path(base));
    }
    for ext in pathext {
        let candidate = PathBuf::from(format!("{}{}", base.display(), ext));
        if is_file(&candidate) {
            return Some(classify_windows_path(candidate));
        }
    }
    None
}

fn has_extension(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some()
}

fn has_path_separator(name: &str) -> bool {
    name.contains('/') || name.contains('\\')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn exists(paths: &[&str]) -> impl Fn(&Path) -> bool {
        let set = paths.iter().map(PathBuf::from).collect::<HashSet<_>>();
        move |p| set.contains(p)
    }

    #[test]
    fn pathext_normalizes_case_and_dots() {
        assert_eq!(
            split_pathext(Some("exe;.cmd;;Ps1")),
            vec![".EXE", ".CMD", ".PS1"]
        );
        assert_eq!(
            split_pathext(Some("")),
            vec![".COM", ".EXE", ".BAT", ".CMD", ".PS1"]
        );
    }

    #[test]
    fn classifies_invocation_kind_by_extension() {
        assert_eq!(
            classify_windows_path(PathBuf::from("tool.exe")),
            WindowsInvocation::Direct(PathBuf::from("tool.exe"))
        );
        assert_eq!(
            classify_windows_path(PathBuf::from("tool.cmd")),
            WindowsInvocation::CmdScript(PathBuf::from("tool.cmd"))
        );
        assert_eq!(
            classify_windows_path(PathBuf::from("tool.ps1")),
            WindowsInvocation::PowerShellScript(PathBuf::from("tool.ps1"))
        );
    }

    #[test]
    fn resolves_path_and_pathext_order() {
        let cwd = PathBuf::from("C:/work");
        let path = vec![PathBuf::from("C:/bin")];
        let pathext = split_pathext(Some(".EXE;.CMD;.PS1"));
        let found = resolve_windows_invocation_by(
            "tool",
            &cwd,
            &path,
            &pathext,
            exists(&["C:/bin/tool.CMD", "C:/bin/tool.PS1"]),
        );
        assert_eq!(
            found,
            Some(WindowsInvocation::CmdScript(PathBuf::from(
                "C:/bin/tool.CMD"
            )))
        );
    }

    #[test]
    fn cwd_wins_before_path() {
        let cwd = PathBuf::from("C:/work");
        let path = vec![PathBuf::from("C:/bin")];
        let pathext = split_pathext(Some(".EXE"));
        let found = resolve_windows_invocation_by(
            "tool",
            &cwd,
            &path,
            &pathext,
            exists(&["C:/work/tool.EXE", "C:/bin/tool.EXE"]),
        );
        assert_eq!(
            found,
            Some(WindowsInvocation::Direct(PathBuf::from("C:/work/tool.EXE")))
        );
    }

    #[test]
    fn relative_path_uses_cwd_and_pathext() {
        let cwd = PathBuf::from("C:/work");
        let pathext = split_pathext(Some(".PS1;.EXE"));
        let found = resolve_windows_invocation_by(
            "./scripts/build",
            &cwd,
            &[],
            &pathext,
            exists(&["C:/work/./scripts/build.PS1"]),
        );
        assert_eq!(
            found,
            Some(WindowsInvocation::PowerShellScript(PathBuf::from(
                "C:/work/./scripts/build.PS1"
            )))
        );
    }
}
