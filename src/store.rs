//! SQLite 스토리지 (설계 §31.2, M1/W4). `storage` feature 필요(rusqlite, C 컴파일러).
//!
//! 확정값(§31.2): 데몬 없음. 단일 `ai-terminal.db`(WAL) + advisory 파일 락 + stale cleanup.
//! 본 모듈은 스키마(6 데이터 테이블 + locks) + PRAGMA + 기본 CRUD를 제공한다.
//! (2층 락/stale 정리는 후속 — 현재는 locks 테이블 생성까지.)

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

/// §31.2 테이블 DDL.
const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY, started_at TEXT NOT NULL, ended_at TEXT,
  shell TEXT, hostname TEXT, cwd TEXT,
  policy_profile TEXT NOT NULL, status TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS commands (
  id TEXT PRIMARY KEY, session_id TEXT NOT NULL,
  started_at TEXT NOT NULL, ended_at TEXT, cwd TEXT,
  command_text TEXT NOT NULL, command_hash TEXT NOT NULL,
  source TEXT NOT NULL, exit_code INTEGER,
  risk_level TEXT, risk_score INTEGER,
  confirmed INTEGER NOT NULL DEFAULT 0,
  ai_generated INTEGER NOT NULL DEFAULT 0,
  preview_id TEXT, undo_id TEXT,
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);
CREATE TABLE IF NOT EXISTS ai_requests (
  id TEXT PRIMARY KEY, session_id TEXT NOT NULL, command_id TEXT,
  created_at TEXT NOT NULL, provider TEXT, model TEXT, intent TEXT,
  status TEXT NOT NULL, input_tokens INTEGER, output_tokens INTEGER,
  estimated_cost_usd REAL, context_hash TEXT,
  cancelled INTEGER NOT NULL DEFAULT 0, error_code TEXT,
  FOREIGN KEY(session_id) REFERENCES sessions(id),
  FOREIGN KEY(command_id) REFERENCES commands(id)
);
CREATE TABLE IF NOT EXISTS usage_events (
  id TEXT PRIMARY KEY, created_at TEXT NOT NULL,
  provider TEXT NOT NULL, model TEXT NOT NULL,
  input_tokens INTEGER NOT NULL DEFAULT 0,
  output_tokens INTEGER NOT NULL DEFAULT 0,
  cached_tokens INTEGER NOT NULL DEFAULT 0,
  estimated_cost_usd REAL NOT NULL DEFAULT 0,
  session_id TEXT, request_id TEXT
);
CREATE TABLE IF NOT EXISTS audit_events (
  id TEXT PRIMARY KEY, created_at TEXT NOT NULL,
  session_id TEXT, command_id TEXT, event_type TEXT NOT NULL,
  risk_level TEXT, policy_profile TEXT, payload_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS context_snapshots (
  id TEXT PRIMARY KEY, session_id TEXT NOT NULL, created_at TEXT NOT NULL,
  context_type TEXT NOT NULL, hostname TEXT, cwd TEXT, shell TEXT,
  git_root TEXT, git_branch TEXT, env_hash TEXT, alias_hash TEXT,
  payload_json TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES sessions(id)
);
CREATE TABLE IF NOT EXISTS locks (
  name TEXT PRIMARY KEY, owner_pid INTEGER NOT NULL, owner_session_id TEXT,
  acquired_at TEXT NOT NULL, expires_at TEXT NOT NULL, heartbeat_at TEXT NOT NULL
);
"#;

/// 새 세션 메타.
#[derive(Debug, Clone)]
pub struct NewSession {
    pub shell: String,
    pub hostname: String,
    pub cwd: String,
    pub policy_profile: String,
}

/// 기록할 명령.
#[derive(Debug, Clone)]
pub struct NewCommand {
    pub session_id: String,
    pub command_text: String,
    pub source: String,
    pub cwd: Option<String>,
    pub exit_code: Option<i64>,
    pub risk_level: Option<String>,
    pub risk_score: Option<i64>,
    pub ai_generated: bool,
    pub confirmed: bool,
}

/// 조회용 명령 행(요약 필드).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRow {
    pub id: String,
    pub command_text: String,
    pub exit_code: Option<i64>,
    pub risk_level: Option<String>,
    pub risk_score: Option<i64>,
}

/// `ai-terminal.db` 핸들.
pub struct Store {
    #[allow(dead_code)]
    conn: Connection,
}

impl Store {
    /// 인메모리 DB(테스트용).
    pub fn open_in_memory() -> Result<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    /// 기본 데이터 디렉터리의 `ai-terminal.db`를 연다(없으면 생성).
    pub fn open_default() -> Result<Store> {
        let dir = data_dir()?;
        std::fs::create_dir_all(&dir)?;
        Self::open(&dir.join("ai-terminal.db"))
    }

    /// 파일 DB를 열거나 생성한다.
    pub fn open(path: &Path) -> Result<Store> {
        Self::init(Connection::open(path)?)
    }

    fn init(conn: Connection) -> Result<Store> {
        // WAL은 execute_batch로 설정(반환값 무시). foreign_keys/busy_timeout 포함.
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;\
             PRAGMA synchronous=NORMAL;\
             PRAGMA foreign_keys=ON;\
             PRAGMA busy_timeout=5000;",
        )?;
        conn.execute_batch(SCHEMA)?;
        Ok(Store { conn })
    }

    /// 현재 journal_mode (WAL 기대; 인메모리는 'memory').
    pub fn journal_mode(&self) -> Result<String> {
        Ok(self
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))?)
    }

    /// 생성된 테이블 이름(정렬).
    pub fn table_names(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
        let names = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(names)
    }

    /// 세션을 생성하고 id를 반환한다.
    pub fn create_session(&self, s: &NewSession) -> Result<String> {
        let id = gen_id("sess");
        self.conn.execute(
            "INSERT INTO sessions (id, started_at, shell, hostname, cwd, policy_profile, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active')",
            rusqlite::params![id, now_ts(), s.shell, s.hostname, s.cwd, s.policy_profile],
        )?;
        Ok(id)
    }

    /// 지정 id의 세션이 없으면 생성한다(idempotent). 셸 hook의 기본 세션 확보용.
    pub fn get_or_create_session(&self, id: &str, s: &NewSession) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO sessions
             (id, started_at, shell, hostname, cwd, policy_profile, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active')",
            rusqlite::params![id, now_ts(), s.shell, s.hostname, s.cwd, s.policy_profile],
        )?;
        Ok(())
    }

    /// 명령을 기록하고 id를 반환한다.
    pub fn record_command(&self, c: &NewCommand) -> Result<String> {
        let id = gen_id("cmd");
        self.conn.execute(
            "INSERT INTO commands
             (id, session_id, started_at, cwd, command_text, command_hash, source,
              exit_code, risk_level, risk_score, confirmed, ai_generated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            rusqlite::params![
                id,
                c.session_id,
                now_ts(),
                c.cwd,
                c.command_text,
                hash_hex(&c.command_text),
                c.source,
                c.exit_code,
                c.risk_level,
                c.risk_score,
                c.confirmed as i64,
                c.ai_generated as i64,
            ],
        )?;
        Ok(id)
    }

    /// 최근 명령을 신규순(삽입 역순)으로 반환한다.
    pub fn recent_commands(&self, limit: u32) -> Result<Vec<CommandRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, command_text, exit_code, risk_level, risk_score
             FROM commands ORDER BY rowid DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([limit], |r| {
                Ok(CommandRow {
                    id: r.get(0)?,
                    command_text: r.get(1)?,
                    exit_code: r.get(2)?,
                    risk_level: r.get(3)?,
                    risk_score: r.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// 테이블 행 수.
    pub fn count(&self, table: &str) -> Result<i64> {
        // table은 내부 고정 식별자만 전달된다(사용자 입력 아님).
        let sql = format!("SELECT COUNT(*) FROM {table}");
        Ok(self.conn.query_row(&sql, [], |r| r.get(0))?)
    }

    /// `PRAGMA integrity_check` 결과가 "ok"인지(무결성 확인).
    pub fn integrity_ok(&self) -> Result<bool> {
        let res: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        Ok(res == "ok")
    }
}

/// 데이터 디렉터리(§31.2): `$XDG_DATA_HOME/ai-terminal` 또는 `$HOME/.local/share/ai-terminal`.
pub fn data_dir() -> Result<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(xdg).join("ai-terminal"));
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .ok_or_else(|| anyhow::anyhow!("HOME/XDG_DATA_HOME not set"))?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("ai-terminal"))
}

fn gen_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{nanos}_{n}")
}

fn now_ts() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn hash_hex(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_creates_all_tables() {
        let s = Store::open_in_memory().unwrap();
        let tables = s.table_names().unwrap();
        for t in [
            "ai_requests",
            "audit_events",
            "commands",
            "context_snapshots",
            "locks",
            "sessions",
            "usage_events",
        ] {
            assert!(
                tables.contains(&t.to_string()),
                "missing table {t}: {tables:?}"
            );
        }
    }

    #[test]
    fn wal_enabled_on_disk() {
        let dir = std::env::temp_dir().join("ai_terminal_store_test");
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("wal-probe.db");
        let _ = std::fs::remove_file(&db);
        let s = Store::open(&db).unwrap();
        assert_eq!(s.journal_mode().unwrap().to_lowercase(), "wal");
    }

    fn sample_session(store: &Store) -> String {
        store
            .create_session(&NewSession {
                shell: "bash".into(),
                hostname: "host".into(),
                cwd: "/home/u".into(),
                policy_profile: "balanced".into(),
            })
            .unwrap()
    }

    fn cmd(session_id: &str, text: &str) -> NewCommand {
        NewCommand {
            session_id: session_id.into(),
            command_text: text.into(),
            source: "shell".into(),
            cwd: Some("/home/u".into()),
            exit_code: Some(0),
            risk_level: Some("Low".into()),
            risk_score: Some(0),
            ai_generated: false,
            confirmed: false,
        }
    }

    #[test]
    fn session_command_roundtrip() {
        let s = Store::open_in_memory().unwrap();
        let sid = sample_session(&s);
        s.record_command(&cmd(&sid, "ls -al")).unwrap();
        s.record_command(&cmd(&sid, "git status")).unwrap();
        assert_eq!(s.count("commands").unwrap(), 2);
        assert_eq!(s.count("sessions").unwrap(), 1);
        let recent = s.recent_commands(10).unwrap();
        assert_eq!(recent.len(), 2);
    }

    #[test]
    fn recent_orders_newest_first() {
        let s = Store::open_in_memory().unwrap();
        let sid = sample_session(&s);
        s.record_command(&cmd(&sid, "first")).unwrap();
        s.record_command(&cmd(&sid, "second")).unwrap();
        let recent = s.recent_commands(1).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].command_text, "second");
    }

    #[test]
    fn foreign_key_rejects_orphan_command() {
        let s = Store::open_in_memory().unwrap();
        let err = s.record_command(&cmd("no_such_session", "ls"));
        assert!(err.is_err(), "FK should reject orphan command");
    }

    #[test]
    fn concurrent_connections_no_corruption() {
        // 동시 터미널 2개 모사: 같은 파일 DB에 두 연결이 교대로 write (WAL + busy_timeout).
        let dir = std::env::temp_dir().join("ai_store_concurrent");
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join(format!("c_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&db);

        let a = Store::open(&db).unwrap();
        let b = Store::open(&db).unwrap();
        a.get_or_create_session(
            "s",
            &NewSession {
                shell: "bash".into(),
                hostname: "h".into(),
                cwd: "/".into(),
                policy_profile: "balanced".into(),
            },
        )
        .unwrap();

        for _ in 0..15 {
            a.record_command(&cmd("s", "from_a")).unwrap();
            b.record_command(&cmd("s", "from_b")).unwrap();
        }

        assert_eq!(a.count("commands").unwrap(), 30);
        assert!(a.integrity_ok().unwrap(), "db integrity must hold");
        assert!(b.integrity_ok().unwrap());
    }

    #[test]
    fn get_or_create_session_is_idempotent() {
        let s = Store::open_in_memory().unwrap();
        let ns = NewSession {
            shell: "bash".into(),
            hostname: "h".into(),
            cwd: "/".into(),
            policy_profile: "balanced".into(),
        };
        s.get_or_create_session("sess-default", &ns).unwrap();
        s.get_or_create_session("sess-default", &ns).unwrap();
        assert_eq!(s.count("sessions").unwrap(), 1);
        // 같은 세션에 명령 기록 가능(FK 충족).
        s.record_command(&cmd("sess-default", "echo hi")).unwrap();
        assert_eq!(s.count("commands").unwrap(), 1);
    }
}
