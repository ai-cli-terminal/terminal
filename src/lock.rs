//! Advisory 파일 락 + stale 정리 (설계 §31.2, M1/W4). 데몬 없는 프로세스 간 직렬화.
//!
//! 락 파일을 원자적으로 생성(`create_new`)해 상호 배제하고, 파일에 `pid`/`timestamp`를
//! 기록한다. 소유 프로세스가 죽어 락이 남으면 TTL 초과(또는 PID 부재) 시 다음 실행이
//! 회수한다(§31.2 stale 처리). [`LockGuard`] drop 시 락 파일을 제거한다.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Result;

/// 획득한 락. drop되면 락 파일을 제거한다(RAII).
pub struct LockGuard {
    path: PathBuf,
}

impl LockGuard {
    /// 락 파일 경로.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// `dir/<name>.lock`을 획득한다. 살아있는 다른 소유자가 있으면 Err, stale이면 회수한다.
pub fn acquire(dir: &Path, name: &str, ttl: Duration) -> Result<LockGuard> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("{name}.lock"));

    // 최대 2회 시도: stale 회수 후 1회 재시도.
    for _ in 0..2 {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                writeln!(f, "{}", std::process::id())?;
                writeln!(f, "{}", now_ms())?;
                return Ok(LockGuard { path });
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                if is_stale(&path, ttl) {
                    // §31.2: stale 확인 → 제거 → 재시도. (audit 기록은 store 연동 시.)
                    let _ = std::fs::remove_file(&path);
                    continue;
                }
                anyhow::bail!("lock '{name}' is held by another live process");
            }
            Err(e) => return Err(e.into()),
        }
    }
    anyhow::bail!("failed to acquire lock '{name}' after stale cleanup")
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// 락 파일이 stale인지(TTL 초과 / PID 부재 / 파싱 불가).
fn is_stale(path: &Path, ttl: Duration) -> bool {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return true,
    };
    let mut lines = content.lines();
    let pid = lines.next().and_then(|l| l.trim().parse::<u32>().ok());
    let ts = lines.next().and_then(|l| l.trim().parse::<u128>().ok());
    match ts {
        Some(ts) if now_ms().saturating_sub(ts) > ttl.as_millis() => return true,
        None => return true,
        _ => {}
    }
    match pid {
        Some(p) => !process_alive(p),
        None => true,
    }
}

#[cfg(target_os = "linux")]
fn process_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

#[cfg(not(target_os = "linux"))]
fn process_alive(_pid: u32) -> bool {
    // 비-Linux에서는 PID 생존 확인을 생략하고 TTL에 의존한다.
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn unique_dir() -> PathBuf {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!("ai_lock_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn acquire_release_then_reacquire() {
        let dir = unique_dir();
        {
            let _g = acquire(&dir, "db", Duration::from_secs(10)).unwrap();
        } // drop releases
        let _g2 = acquire(&dir, "db", Duration::from_secs(10)).unwrap();
    }

    #[test]
    fn held_lock_blocks_second_acquire() {
        let dir = unique_dir();
        let _g = acquire(&dir, "usage", Duration::from_secs(10)).unwrap();
        assert!(acquire(&dir, "usage", Duration::from_secs(10)).is_err());
    }

    #[test]
    fn stale_lock_is_reclaimed() {
        let dir = unique_dir();
        std::fs::create_dir_all(&dir).unwrap();
        // pid 999999 + ts=0(아주 오래됨) → stale.
        std::fs::write(dir.join("idx.lock"), "999999\n0\n").unwrap();
        let _g = acquire(&dir, "idx", Duration::from_secs(10)).unwrap();
    }
}
