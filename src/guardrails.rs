//! Execution Guardrails Engine — capability matrix (설계 §31.11, M4/W14).
//!
//! Baseline guardrails는 모든 플랫폼에서 보장하고, 동적 감시(seccomp/cgroups 등)는
//! 플랫폼별로 다르다. 미지원 기능은 **조용히 실패하지 않고 명시**한다(§31.11 DoD).
//! 동적 감시가 제한된 플랫폼에서는 High 이상 명령 확인을 강화한다.

/// 실행 플랫폼.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Wsl,
    MacOs,
    Windows,
    Other,
}

/// 기능 지원 수준.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Support {
    Supported,
    Partial,
    Unsupported,
}

/// 플랫폼별 기능 지원 항목.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability {
    pub name: &'static str,
    pub support: Support,
}

/// 현재 플랫폼을 감지한다.
pub fn detect() -> Platform {
    #[cfg(target_os = "linux")]
    {
        if is_wsl() {
            Platform::Wsl
        } else {
            Platform::Linux
        }
    }
    #[cfg(target_os = "macos")]
    {
        Platform::MacOs
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Platform::Other
    }
}

#[cfg(target_os = "linux")]
fn is_wsl() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false)
}

/// 모든 플랫폼이 보장하는 baseline guardrails(§31.11).
pub fn baseline() -> Vec<&'static str> {
    vec![
        "static risk analysis",
        "risk scoring",
        "preview/diff",
        "dry-run",
        "timeout",
        "confirmation prompt",
        "secret/PII masking",
        "policy enforcement",
    ]
}

/// 플랫폼별 동적 guardrails 지원 매트릭스.
pub fn capabilities(platform: Platform) -> Vec<Capability> {
    use Support::{Partial, Supported, Unsupported};
    let cap = |name, support| Capability { name, support };
    let linux = platform == Platform::Linux;
    let wsl = platform == Platform::Wsl;
    let macos = platform == Platform::MacOs;
    let windows = platform == Platform::Windows;
    vec![
        cap(
            "process group termination",
            if macos || linux { Supported } else { Partial },
        ),
        cap(
            "PTY / ConPTY terminal",
            if linux || wsl || macos || windows {
                Supported
            } else {
                Unsupported
            },
        ),
        cap(
            "Windows ConPTY",
            if windows { Supported } else { Unsupported },
        ),
        cap("file count pre-scan", Supported),
        cap(
            "cgroups CPU/mem limit",
            if linux {
                Supported
            } else if wsl {
                Partial
            } else {
                Unsupported
            },
        ),
        cap(
            "seccomp / fanotify",
            if linux {
                Supported
            } else if wsl {
                Partial
            } else {
                Unsupported
            },
        ),
        cap(
            "inotify",
            if linux || wsl { Supported } else { Unsupported },
        ),
        cap("FSEvents", if macos { Supported } else { Unsupported }),
        cap("Docker sandbox", if linux { Supported } else { Partial }),
        cap("bubblewrap", if linux { Supported } else { Unsupported }),
        cap("gVisor / eBPF", if linux { Partial } else { Unsupported }),
    ]
}

/// 동적 감시가 제한되는 플랫폼인지(완전 Linux가 아니면 true). High+ 확인 강화 근거.
pub fn dynamic_monitoring_limited(platform: Platform) -> bool {
    platform != Platform::Linux
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_includes_core_guardrails() {
        let b = baseline();
        for needed in [
            "static risk analysis",
            "preview/diff",
            "timeout",
            "secret/PII masking",
            "policy enforcement",
        ] {
            assert!(b.contains(&needed), "baseline missing {needed}: {b:?}");
        }
    }

    #[test]
    fn linux_supports_more_than_macos() {
        let linux = capabilities(Platform::Linux);
        let macos = capabilities(Platform::MacOs);
        let find = |caps: &[Capability], name: &str| {
            caps.iter().find(|c| c.name == name).map(|c| c.support)
        };
        assert_eq!(find(&linux, "seccomp / fanotify"), Some(Support::Supported));
        assert_eq!(
            find(&macos, "seccomp / fanotify"),
            Some(Support::Unsupported)
        );
    }

    #[test]
    fn dynamic_monitoring_limited_off_linux() {
        assert!(!dynamic_monitoring_limited(Platform::Linux));
        assert!(dynamic_monitoring_limited(Platform::Wsl));
        assert!(dynamic_monitoring_limited(Platform::MacOs));
        assert!(dynamic_monitoring_limited(Platform::Windows));
    }

    #[test]
    fn windows_reports_conpty_but_not_linux_sandboxing() {
        let caps = capabilities(Platform::Windows);
        let find = |name: &str| caps.iter().find(|c| c.name == name).map(|c| c.support);
        assert_eq!(find("Windows ConPTY"), Some(Support::Supported));
        assert_eq!(find("seccomp / fanotify"), Some(Support::Unsupported));
    }
}
