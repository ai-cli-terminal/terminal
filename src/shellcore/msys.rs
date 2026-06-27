//! Git Bash/MSYS profile contract for Windows `ash`.
//!
//! MSYS is not the same target as native Windows and is not WSL. This module is
//! pure so the profile boundary can be tested before a bridge runner exists.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsShellProfile {
    NativeWindows,
    MsysBridge,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileSelection {
    Selected(WindowsShellProfile),
    Unsupported(String),
    MsysRequestedOutsideMsys,
}

pub const PROFILE_ENV: &str = "AI_TERMINAL_WINDOWS_PROFILE";
pub const PROFILE_NATIVE: &str = "native";
pub const PROFILE_MSYS: &str = "msys";

/// Select the Windows shell profile.
///
/// Defaults to native Windows even when launched from Git Bash/MSYS. The MSYS
/// bridge is an explicit profile because path conversion, POSIX tool discovery,
/// and PTY expectations differ from native `ash.exe`.
pub fn select_profile(
    profile_env: Option<&str>,
    msystem: Option<&str>,
    msystem_prefix: Option<&str>,
) -> ProfileSelection {
    let requested = profile_env.unwrap_or(PROFILE_NATIVE).trim();
    if requested.is_empty() || requested.eq_ignore_ascii_case(PROFILE_NATIVE) {
        return ProfileSelection::Selected(WindowsShellProfile::NativeWindows);
    }
    if requested.eq_ignore_ascii_case(PROFILE_MSYS) {
        if is_msys_environment(msystem, msystem_prefix) {
            return ProfileSelection::Selected(WindowsShellProfile::MsysBridge);
        }
        return ProfileSelection::MsysRequestedOutsideMsys;
    }
    ProfileSelection::Unsupported(requested.to_string())
}

pub fn is_msys_environment(msystem: Option<&str>, msystem_prefix: Option<&str>) -> bool {
    let has_msystem = msystem.is_some_and(|v| !v.trim().is_empty());
    let has_prefix = msystem_prefix.is_some_and(|v| !v.trim().is_empty());
    has_msystem || has_prefix
}

/// MSYS POSIX host 호출을 구성한다. `sh`가 PATH에서 POSIX tool을 찾고 POSIX path를
/// 해석하므로 ash는 path 변환/tool 스캔을 하지 않는다. (host는 `sh` 고정)
pub fn bridge_invocation(command: &str) -> (String, Vec<String>) {
    (
        "sh".to_string(),
        vec!["-lc".to_string(), command.to_string()],
    )
}

/// 현재 활성 Windows 셸 profile. env(`AI_TERMINAL_WINDOWS_PROFILE`/`MSYSTEM`/
/// `MSYSTEM_PREFIX`)를 읽어 `select_profile`로 판정한다. Selected가 아니면 native로
/// 안전 폴백(비-Windows/MSYS밖/미지 profile 포함).
pub fn active_profile() -> WindowsShellProfile {
    let profile_env = std::env::var(PROFILE_ENV).ok();
    let msystem = std::env::var("MSYSTEM").ok();
    let prefix = std::env::var("MSYSTEM_PREFIX").ok();
    match select_profile(
        profile_env.as_deref(),
        msystem.as_deref(),
        prefix.as_deref(),
    ) {
        ProfileSelection::Selected(profile) => profile,
        _ => WindowsShellProfile::NativeWindows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_invocation_wraps_in_posix_host() {
        let (prog, args) = bridge_invocation("ls -al /c/Users");
        assert_eq!(prog, "sh");
        assert_eq!(args, vec!["-lc".to_string(), "ls -al /c/Users".to_string()]);
    }

    #[test]
    fn defaults_to_native_even_inside_msys() {
        assert_eq!(
            select_profile(None, Some("MINGW64"), Some("/mingw64")),
            ProfileSelection::Selected(WindowsShellProfile::NativeWindows)
        );
    }

    #[test]
    fn explicit_native_wins_inside_msys() {
        assert_eq!(
            select_profile(Some("native"), Some("MINGW64"), Some("/mingw64")),
            ProfileSelection::Selected(WindowsShellProfile::NativeWindows)
        );
    }

    #[test]
    fn explicit_msys_requires_msys_environment() {
        assert_eq!(
            select_profile(Some("msys"), Some("MINGW64"), Some("/mingw64")),
            ProfileSelection::Selected(WindowsShellProfile::MsysBridge)
        );
        assert_eq!(
            select_profile(Some("msys"), None, None),
            ProfileSelection::MsysRequestedOutsideMsys
        );
    }

    #[test]
    fn unsupported_profile_is_rejected() {
        assert_eq!(
            select_profile(Some("wsl"), Some("MINGW64"), Some("/mingw64")),
            ProfileSelection::Unsupported("wsl".to_string())
        );
    }
}
