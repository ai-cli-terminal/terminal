//! SQLite 스토리지 (설계 §31.2, M1/W4). `storage` feature 필요(rusqlite, C 컴파일러).
//!
//! 확정값(§31.2): 데몬 없음. 단일 `ai-terminal.db`(WAL) + advisory 파일 락 + stale cleanup.
//! 스키마(6 데이터 테이블 + locks) + PRAGMA + CRUD + locks 레지스트리(register/reclaim) +
//! audit 기록을 제공한다. 파일 락 프리미티브는 [`crate::lock`], 2층 결합은 상위 오케스트레이션.

use std::path::Path;

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};

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

/// 컨텍스트 스냅샷 입력(부분 필드; 나머지 컬럼은 NULL).
#[derive(Debug, Clone)]
pub struct NewContext {
    pub session_id: String,
    pub context_type: String,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
}

/// 조회용 컨텍스트 행(요약 필드).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextRow {
    pub context_type: String,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
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

    /// 직전(가장 최근) 종료 코드 미정 명령에 종료 코드를 반영한다(precmd 시점, §31.1).
    ///
    /// `preexec`에서 명령은 `exit_code = NULL`로 기록되고, 명령이 끝난 뒤 `precmd`가
    /// 실제 종료 코드를 채운다. 갱신 대상(미정 명령)이 있었으면 `true`.
    pub fn update_last_exit(&self, session_id: &str, exit_code: i64) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE commands SET exit_code = ?1, ended_at = ?2
             WHERE rowid = (
                 SELECT rowid FROM commands
                 WHERE session_id = ?3 AND exit_code IS NULL
                 ORDER BY rowid DESC LIMIT 1
             )",
            rusqlite::params![exit_code, now_ts(), session_id],
        )?;
        Ok(n > 0)
    }

    /// 세션에서 가장 최근의 실패(0이 아닌 종료 코드) 명령을 반환한다(`ai explain last-error`용).
    pub fn last_error(&self, session_id: &str) -> Result<Option<CommandRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, command_text, exit_code, risk_level, risk_score
             FROM commands
             WHERE session_id = ?1 AND exit_code IS NOT NULL AND exit_code != 0
             ORDER BY rowid DESC LIMIT 1",
        )?;
        let row = stmt
            .query_row([session_id], |r| {
                Ok(CommandRow {
                    id: r.get(0)?,
                    command_text: r.get(1)?,
                    exit_code: r.get(2)?,
                    risk_level: r.get(3)?,
                    risk_score: r.get(4)?,
                })
            })
            .optional()?;
        Ok(row)
    }

    /// 컨텍스트 스냅샷을 기록한다(chpwd/startup 등 상태 갱신 시점, §31.10).
    pub fn record_context_snapshot(&self, c: &NewContext) -> Result<String> {
        let id = gen_id("ctx");
        self.conn.execute(
            "INSERT INTO context_snapshots
             (id, session_id, created_at, context_type, cwd, git_branch, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                id,
                c.session_id,
                now_ts(),
                c.context_type,
                c.cwd,
                c.git_branch,
                "{}",
            ],
        )?;
        Ok(id)
    }

    /// 세션의 가장 최근 컨텍스트 스냅샷을 반환한다.
    pub fn latest_context(&self, session_id: &str) -> Result<Option<ContextRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT context_type, cwd, git_branch FROM context_snapshots
             WHERE session_id = ?1 ORDER BY rowid DESC LIMIT 1",
        )?;
        let row = stmt
            .query_row([session_id], |r| {
                Ok(ContextRow {
                    context_type: r.get(0)?,
                    cwd: r.get(1)?,
                    git_branch: r.get(2)?,
                })
            })
            .optional()?;
        Ok(row)
    }

    /// 세션의 현재 cwd를 갱신한다(chpwd 시점). 갱신 대상이 있었으면 true.
    pub fn update_session_cwd(&self, session_id: &str, cwd: &str) -> Result<bool> {
        let n = self.conn.execute(
            "UPDATE sessions SET cwd = ?1 WHERE id = ?2",
            rusqlite::params![cwd, session_id],
        )?;
        Ok(n > 0)
    }

    /// 테이블 행 수.
    pub fn count(&self, table: &str) -> Result<i64> {
        // table은 내부 고정 식별자만 전달된다(사용자 입력 아님).
        let sql = format!("SELECT COUNT(*) FROM {table}");
        Ok(self.conn.query_row(&sql, [], |r| r.get(0))?)
    }

    /// 락 레지스트리에 등록한다(§31.2 `locks` 테이블). `ttl_secs`로 만료 시각 계산.
    pub fn register_lock(
        &self,
        name: &str,
        owner_pid: i64,
        owner_session: Option<&str>,
        ttl_secs: i64,
    ) -> Result<()> {
        let now = now_ms() as i64;
        let expires = now + ttl_secs * 1000;
        self.conn.execute(
            "INSERT OR REPLACE INTO locks
             (name, owner_pid, owner_session_id, acquired_at, expires_at, heartbeat_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?4)",
            rusqlite::params![
                name,
                owner_pid,
                owner_session,
                now.to_string(),
                expires.to_string()
            ],
        )?;
        Ok(())
    }

    /// 락 소유자(pid, expires_at)를 조회한다.
    pub fn lock_owner(&self, name: &str) -> Result<Option<(i64, String)>> {
        let row = self
            .conn
            .query_row(
                "SELECT owner_pid, expires_at FROM locks WHERE name = ?1",
                [name],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .ok();
        Ok(row)
    }

    /// 락 레지스트리에서 제거한다.
    pub fn release_lock(&self, name: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM locks WHERE name = ?1", [name])?;
        Ok(())
    }

    /// AI 요청 usage event를 기록한다(§31.7).
    #[allow(clippy::too_many_arguments)]
    pub fn record_usage(
        &self,
        provider: &str,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        cached_tokens: i64,
        cost_usd: f64,
        session_id: Option<&str>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO usage_events
             (id, created_at, provider, model, input_tokens, output_tokens, cached_tokens,
              estimated_cost_usd, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                gen_id("usage"),
                now_ms().to_string(),
                provider,
                model,
                input_tokens,
                output_tokens,
                cached_tokens,
                cost_usd,
                session_id
            ],
        )?;
        Ok(())
    }

    /// 누적 비용(USD)을 반환한다. `session_id` 지정 시 해당 세션만.
    pub fn total_cost(&self, session_id: Option<&str>) -> Result<f64> {
        let cost = match session_id {
            Some(sid) => self.conn.query_row(
                "SELECT COALESCE(SUM(estimated_cost_usd),0) FROM usage_events WHERE session_id = ?1",
                [sid],
                |r| r.get(0),
            )?,
            None => self.conn.query_row(
                "SELECT COALESCE(SUM(estimated_cost_usd),0) FROM usage_events",
                [],
                |r| r.get(0),
            )?,
        };
        Ok(cost)
    }

    /// 감사 이벤트를 기록한다(§31.2 `audit_events`). 민감 정보는 payload에 넣지 않는다.
    pub fn record_audit(
        &self,
        event_type: &str,
        risk_level: Option<&str>,
        policy_profile: Option<&str>,
        payload_json: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO audit_events
             (id, created_at, event_type, risk_level, policy_profile, payload_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                gen_id("audit"),
                now_ms().to_string(),
                event_type,
                risk_level,
                policy_profile,
                payload_json
            ],
        )?;
        Ok(())
    }

    /// 레지스트리의 락이 만료됐으면 audit 기록 후 제거한다(§31.2 stale 처리).
    pub fn reclaim_if_stale(&self, name: &str) -> Result<bool> {
        if let Some((pid, expires)) = self.lock_owner(name)? {
            let exp: u128 = expires.parse().unwrap_or(0);
            if now_ms() > exp {
                self.record_audit(
                    "lock_stale_reclaimed",
                    None,
                    None,
                    &format!("{{\"name\":\"{name}\",\"owner_pid\":{pid}}}"),
                )?;
                self.release_lock(name)?;
                return Ok(true);
            }
        }
        Ok(false)
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

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
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

    /// preexec(미정) → precmd(종료코드 반영) 라이프사이클.
    #[test]
    fn update_last_exit_fills_pending_command() {
        let s = Store::open_in_memory().unwrap();
        let sid = sample_session(&s);
        let mut c = cmd(&sid, "false");
        c.exit_code = None; // preexec 시점
        s.record_command(&c).unwrap();

        let updated = s.update_last_exit(&sid, 1).unwrap();
        assert!(updated, "미정 명령이 갱신되어야 한다");
        assert_eq!(s.recent_commands(1).unwrap()[0].exit_code, Some(1));
    }

    /// 이미 종료 코드가 채워졌으면 더 이상 갱신 대상이 없다.
    #[test]
    fn update_last_exit_only_targets_pending() {
        let s = Store::open_in_memory().unwrap();
        let sid = sample_session(&s);
        let mut c = cmd(&sid, "true");
        c.exit_code = None;
        s.record_command(&c).unwrap();
        assert!(s.update_last_exit(&sid, 0).unwrap());
        assert!(
            !s.update_last_exit(&sid, 7).unwrap(),
            "채워진 명령은 다시 갱신되지 않는다"
        );
    }

    #[test]
    fn last_error_returns_most_recent_failure() {
        let s = Store::open_in_memory().unwrap();
        let sid = sample_session(&s);
        let mut ok1 = cmd(&sid, "ls");
        ok1.exit_code = Some(0);
        s.record_command(&ok1).unwrap();
        let mut fail = cmd(&sid, "cat missing");
        fail.exit_code = Some(1);
        s.record_command(&fail).unwrap();
        let mut ok2 = cmd(&sid, "pwd");
        ok2.exit_code = Some(0);
        s.record_command(&ok2).unwrap();

        let e = s
            .last_error(&sid)
            .unwrap()
            .expect("실패 명령이 있어야 한다");
        assert_eq!(e.command_text, "cat missing");
        assert_eq!(e.exit_code, Some(1));
    }

    #[test]
    fn last_error_none_when_all_succeed() {
        let s = Store::open_in_memory().unwrap();
        let sid = sample_session(&s);
        s.record_command(&cmd(&sid, "ls")).unwrap(); // exit 0
        assert!(s.last_error(&sid).unwrap().is_none());
    }

    #[test]
    fn context_snapshot_record_and_latest() {
        let s = Store::open_in_memory().unwrap();
        let sid = sample_session(&s);
        s.record_context_snapshot(&NewContext {
            session_id: sid.clone(),
            context_type: "chpwd".into(),
            cwd: Some("/work/proj".into()),
            git_branch: Some("main".into()),
        })
        .unwrap();
        let c = s
            .latest_context(&sid)
            .unwrap()
            .expect("스냅샷이 있어야 한다");
        assert_eq!(c.context_type, "chpwd");
        assert_eq!(c.cwd.as_deref(), Some("/work/proj"));
        assert_eq!(c.git_branch.as_deref(), Some("main"));
    }

    #[test]
    fn update_session_cwd_targets_existing_only() {
        let s = Store::open_in_memory().unwrap();
        let sid = sample_session(&s);
        assert!(s.update_session_cwd(&sid, "/new/dir").unwrap());
        assert!(!s.update_session_cwd("no-such-session", "/x").unwrap());
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
    fn lock_registry_register_query_release() {
        let s = Store::open_in_memory().unwrap();
        s.register_lock("db.lock", 123, Some("sess-1"), 10).unwrap();
        assert!(matches!(s.lock_owner("db.lock").unwrap(), Some((123, _))));
        s.release_lock("db.lock").unwrap();
        assert!(s.lock_owner("db.lock").unwrap().is_none());
    }

    #[test]
    fn usage_record_and_total_cost() {
        let s = Store::open_in_memory().unwrap();
        s.record_usage("p", "m", 100, 50, 0, 0.004, Some("sess-1"))
            .unwrap();
        s.record_usage("p", "m", 200, 80, 0, 0.006, Some("sess-1"))
            .unwrap();
        assert!((s.total_cost(Some("sess-1")).unwrap() - 0.010).abs() < 1e-9);
        assert!((s.total_cost(None).unwrap() - 0.010).abs() < 1e-9);
    }

    #[test]
    fn record_audit_is_stored() {
        let s = Store::open_in_memory().unwrap();
        s.record_audit("test_event", Some("High"), Some("balanced"), "{}")
            .unwrap();
        assert_eq!(s.count("audit_events").unwrap(), 1);
    }

    #[test]
    fn stale_lock_reclaim_writes_audit() {
        let s = Store::open_in_memory().unwrap();
        s.register_lock("idx.lock", 999, None, -1).unwrap(); // 즉시 만료
        assert!(s.reclaim_if_stale("idx.lock").unwrap());
        assert!(s.lock_owner("idx.lock").unwrap().is_none());
        assert_eq!(s.count("audit_events").unwrap(), 1);

        s.register_lock("p.lock", 1, None, 100).unwrap(); // 신선
        assert!(!s.reclaim_if_stale("p.lock").unwrap());
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
