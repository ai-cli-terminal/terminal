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

/// config.toml 경로(기본 위치): config_dir()/config.toml.
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// `[general]` 섹션(최소 스키마). 누락 필드는 기본값.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct General {
    pub default_shell: Option<String>,
    pub history_limit: usize,
}

impl Default for General {
    fn default() -> Self {
        Self {
            default_shell: None,
            history_limit: 10_000,
        }
    }
}

/// 사용자 config.toml 타입 모델(최소). 후속 슬라이스가 섹션을 확장한다.
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
}

/// 로드된 config의 출처(진단용).
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSource {
    Default,
    File(PathBuf),
}

/// fail-soft 로드 결과.
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub config: Config,
    pub source: ConfigSource,
    pub warning: Option<String>,
}

/// 주어진 경로에서 fail-soft 로드한다. 부재→기본값, 파싱오류→기본값+경고.
/// 에러를 전파하지 않는다(세션 비중단).
pub fn load_from(path: &Path) -> LoadedConfig {
    match std::fs::read_to_string(path) {
        Err(_) => LoadedConfig {
            config: Config::default(),
            source: ConfigSource::Default,
            warning: None,
        },
        Ok(text) => match toml::from_str::<Config>(&text) {
            Ok(config) => LoadedConfig {
                config,
                source: ConfigSource::File(path.to_path_buf()),
                warning: None,
            },
            Err(e) => LoadedConfig {
                config: Config::default(),
                source: ConfigSource::Default,
                warning: Some(format!("config.toml 파싱 실패({e}) — 기본값 사용")),
            },
        },
    }
}

/// 기본 위치에서 로드한다. 경로 해석 실패도 fail-soft.
pub fn load() -> LoadedConfig {
    match config_path() {
        Ok(p) => load_from(&p),
        Err(_) => LoadedConfig {
            config: Config::default(),
            source: ConfigSource::Default,
            warning: None,
        },
    }
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

    #[test]
    fn load_missing_file_yields_defaults() {
        let p = tmp_file("cfg_missing.toml");
        let _ = std::fs::remove_file(&p);
        let loaded = load_from(&p);
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.source, ConfigSource::Default);
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.config.general.history_limit, 10_000);
    }

    #[test]
    fn load_valid_file_reads_values() {
        let p = tmp_file("cfg_valid.toml");
        std::fs::write(&p, "[general]\ndefault_shell = \"/bin/bash\"\nhistory_limit = 42\n").unwrap();
        let loaded = load_from(&p);
        assert_eq!(loaded.config.general.history_limit, 42);
        assert_eq!(loaded.config.general.default_shell.as_deref(), Some("/bin/bash"));
        assert_eq!(loaded.source, ConfigSource::File(p.clone()));
        assert!(loaded.warning.is_none());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn load_partial_file_fills_missing_with_defaults() {
        let p = tmp_file("cfg_partial.toml");
        std::fs::write(&p, "[general]\nhistory_limit = 5\n").unwrap();
        let loaded = load_from(&p);
        assert_eq!(loaded.config.general.history_limit, 5);
        assert_eq!(loaded.config.general.default_shell, None);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn load_corrupt_file_is_failsoft() {
        let p = tmp_file("cfg_corrupt.toml");
        std::fs::write(&p, "this is = = not valid toml [[[").unwrap();
        let loaded = load_from(&p);
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.source, ConfigSource::Default);
        assert!(loaded.warning.is_some());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn load_unknown_keys_are_ignored() {
        let p = tmp_file("cfg_unknown.toml");
        std::fs::write(&p, "[general]\nhistory_limit = 7\nfuture_field = true\n[unknown_section]\nx = 1\n").unwrap();
        let loaded = load_from(&p);
        assert_eq!(loaded.config.general.history_limit, 7);
        assert!(loaded.warning.is_none());
        let _ = std::fs::remove_file(&p);
    }
}
