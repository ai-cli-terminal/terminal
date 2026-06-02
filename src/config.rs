//! 설정/상태 영속화 (설계 §31.3 정책 활성화, §10.5 디렉터리).
//!
//! 활성 정책 프로파일을 `~/.config/ai-terminal/active_profile`에 저장한다.
//! config.toml(사용자 편집 정본)을 재작성하지 않기 위해 작은 상태 파일을 별도로 둔다.

use std::path::{Path, PathBuf};

use anyhow::Result;

/// 설정 디렉터리: `$XDG_CONFIG_HOME/ai-terminal` 또는 `$HOME/.config/ai-terminal`.
pub fn config_dir() -> Result<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(x).join("ai-terminal"));
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| anyhow::anyhow!("HOME/XDG_CONFIG_HOME not set"))?;
    Ok(PathBuf::from(home).join(".config").join("ai-terminal"))
}

/// 활성 프로파일 상태 파일 경로.
pub fn active_profile_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("active_profile"))
}

/// 파일에서 활성 프로파일을 읽는다. 없거나 비면 `balanced`.
pub fn read_active_profile_from(path: &Path) -> String {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "balanced".to_string())
}

/// 활성 프로파일을 파일에 기록한다(상위 디렉터리 생성).
pub fn write_active_profile_to(path: &Path, name: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{name}\n"))?;
    Ok(())
}

/// 현재 활성 프로파일(기본 위치).
pub fn get_active_profile() -> String {
    active_profile_path()
        .map(|p| read_active_profile_from(&p))
        .unwrap_or_else(|_| "balanced".to_string())
}

/// 활성 프로파일을 설정(기본 위치).
pub fn set_active_profile(name: &str) -> Result<()> {
    let path = active_profile_path()?;
    write_active_profile_to(&path, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ai_cfg_{}_{}", std::process::id(), tag))
    }

    #[test]
    fn defaults_to_balanced_when_missing() {
        let p = tmp_file("missing");
        let _ = std::fs::remove_file(&p);
        assert_eq!(read_active_profile_from(&p), "balanced");
    }

    #[test]
    fn write_then_read_roundtrip() {
        let p = tmp_file("rt");
        write_active_profile_to(&p, "paranoid").unwrap();
        assert_eq!(read_active_profile_from(&p), "paranoid");
        let _ = std::fs::remove_file(&p);
    }
}
