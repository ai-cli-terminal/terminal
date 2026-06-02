//! Undo / Transaction 계층 (설계 §31.6, M3/W10). best-effort 파일 롤백.
//!
//! 파괴적/수정 명령 실행 전 대상 파일을 백업하고, `ai undo last`로 복구한다.
//! 백업 상한을 초과하면 거부(Refused)하여 호출측이 사전 고지·중단하게 한다(§31.6 DoD).
//! 미지원: 삭제 전체 복구 보장, DB/패키지/서비스/네트워크/클라우드 변경.

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 백업 상한(§31.6).
#[derive(Debug, Clone, Copy)]
pub struct UndoLimits {
    pub max_backup_size_mb: u64,
    pub max_file_count: usize,
    pub max_file_size_mb: u64,
    pub backup_ttl_days: u64,
}

impl UndoLimits {
    /// 기본값: 500MB / 1000 files / 파일 20MB / TTL 7일.
    pub fn defaults() -> UndoLimits {
        UndoLimits {
            max_backup_size_mb: 500,
            max_file_count: 1000,
            max_file_size_mb: 20,
            backup_ttl_days: 7,
        }
    }
}

/// 백업 시도 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupOutcome {
    /// 백업 성공(undo_id).
    Created(String),
    /// 상한 초과 등으로 거부(사유). 호출측은 위험 명령을 중단해야 한다.
    Refused(String),
}

#[derive(Serialize, Deserialize)]
struct FileEntry {
    original: String,
    backup: String,
}

#[derive(Serialize, Deserialize)]
struct UndoMeta {
    undo_id: String,
    created_at: String,
    files: Vec<FileEntry>,
}

/// 기본 undo 디렉터리: `<data>/ai-terminal/undo`.
pub fn default_undo_dir() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(|h| PathBuf::from(h).join(".local").join("share"))
        })
        .ok_or_else(|| anyhow::anyhow!("데이터 디렉터리를 찾을 수 없습니다"))?;
    Ok(base.join("ai-terminal").join("undo"))
}

/// 대상 파일들을 백업한다. 상한 초과 시 `Refused`.
pub fn create_backup(base: &Path, files: &[PathBuf], limits: &UndoLimits) -> Result<BackupOutcome> {
    if files.len() > limits.max_file_count {
        return Ok(BackupOutcome::Refused(format!(
            "파일 수 {} > 상한 {}",
            files.len(),
            limits.max_file_count
        )));
    }
    let max_file = limits.max_file_size_mb * 1024 * 1024;
    let max_total = limits.max_backup_size_mb * 1024 * 1024;
    let mut total: u64 = 0;
    for f in files {
        let meta = std::fs::metadata(f)?;
        if !meta.is_file() {
            return Ok(BackupOutcome::Refused(format!(
                "{} 는 일반 파일이 아님",
                f.display()
            )));
        }
        if meta.len() > max_file {
            return Ok(BackupOutcome::Refused(format!(
                "{} ({}B) > 파일 상한 {}B",
                f.display(),
                meta.len(),
                max_file
            )));
        }
        total += meta.len();
    }
    if total > max_total {
        return Ok(BackupOutcome::Refused(format!(
            "백업 총량 {total}B > 상한 {max_total}B"
        )));
    }

    let undo_id = gen_id();
    let files_dir = base.join(&undo_id).join("files");
    std::fs::create_dir_all(&files_dir)?;
    let mut entries = Vec::new();
    for (i, f) in files.iter().enumerate() {
        let name = f
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let backup = files_dir.join(format!("{i}_{name}"));
        std::fs::copy(f, &backup)?;
        entries.push(FileEntry {
            original: f.display().to_string(),
            backup: backup.display().to_string(),
        });
    }
    let meta = UndoMeta {
        undo_id: undo_id.clone(),
        created_at: now_ts(),
        files: entries,
    };
    std::fs::write(
        base.join(&undo_id).join("metadata.toml"),
        toml::to_string(&meta)?,
    )?;
    Ok(BackupOutcome::Created(undo_id))
}

/// 백업을 원본 위치로 복구하고 복구한 파일 수를 반환한다.
pub fn restore(base: &Path, undo_id: &str) -> Result<usize> {
    let meta_path = base.join(undo_id).join("metadata.toml");
    let meta: UndoMeta = toml::from_str(&std::fs::read_to_string(meta_path)?)?;
    let mut n = 0;
    for e in &meta.files {
        std::fs::copy(&e.backup, &e.original)?;
        n += 1;
    }
    Ok(n)
}

/// 가장 최근 백업 id(없으면 None).
pub fn latest(base: &Path) -> Option<String> {
    let mut ids: Vec<String> = std::fs::read_dir(base)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n.starts_with("undo_"))
        .collect();
    ids.sort();
    ids.pop()
}

fn gen_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("undo_{nanos:039}_{n:06}")
}

fn now_ts() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn uniq(tag: &str) -> PathBuf {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ai_undo_{}_{}_{}", std::process::id(), tag, n));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn backup_and_restore_roundtrip() {
        let work = uniq("work");
        std::fs::create_dir_all(&work).unwrap();
        let f = work.join("a.txt");
        std::fs::write(&f, "orig").unwrap();

        let base = uniq("store");
        let out = create_backup(&base, std::slice::from_ref(&f), &UndoLimits::defaults()).unwrap();
        let id = match out {
            BackupOutcome::Created(id) => id,
            BackupOutcome::Refused(r) => panic!("unexpected refuse: {r}"),
        };

        std::fs::write(&f, "changed").unwrap();
        let n = restore(&base, &id).unwrap();
        assert_eq!(n, 1);
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "orig");
    }

    #[test]
    fn refuses_when_too_many_files() {
        let work = uniq("many");
        std::fs::create_dir_all(&work).unwrap();
        let f1 = work.join("1");
        let f2 = work.join("2");
        std::fs::write(&f1, "a").unwrap();
        std::fs::write(&f2, "b").unwrap();
        let limits = UndoLimits {
            max_file_count: 1,
            ..UndoLimits::defaults()
        };
        let out = create_backup(&uniq("s"), &[f1, f2], &limits).unwrap();
        assert!(matches!(out, BackupOutcome::Refused(_)), "{out:?}");
    }

    #[test]
    fn refuses_when_file_too_large() {
        let work = uniq("big");
        std::fs::create_dir_all(&work).unwrap();
        let f = work.join("big.bin");
        std::fs::write(&f, vec![0u8; 100]).unwrap();
        let limits = UndoLimits {
            max_file_size_mb: 0,
            ..UndoLimits::defaults()
        };
        let out = create_backup(&uniq("s"), &[f], &limits).unwrap();
        assert!(matches!(out, BackupOutcome::Refused(_)), "{out:?}");
    }

    #[test]
    fn latest_returns_most_recent() {
        let work = uniq("lw");
        std::fs::create_dir_all(&work).unwrap();
        let f = work.join("x");
        std::fs::write(&f, "1").unwrap();
        let base = uniq("ls");
        let first = match create_backup(&base, std::slice::from_ref(&f), &UndoLimits::defaults())
            .unwrap()
        {
            BackupOutcome::Created(id) => id,
            _ => panic!(),
        };
        let second = match create_backup(&base, std::slice::from_ref(&f), &UndoLimits::defaults())
            .unwrap()
        {
            BackupOutcome::Created(id) => id,
            _ => panic!(),
        };
        assert_ne!(first, second);
        assert_eq!(latest(&base).as_deref(), Some(second.as_str()));
    }
}
