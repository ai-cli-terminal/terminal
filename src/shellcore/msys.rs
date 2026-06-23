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

#[cfg(test)]
mod tests {
    use super::*;

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
