# S2 follow-on — ash Gate Audit Recording Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Record ash's external-execution gate outcomes to storage (ai exec parity) by extracting the audit logic from `main.rs` into a shared lib module and wiring `GatedRunner` to use it.

**Architecture:** Move `AuditRecord` + `shell_outcome_audit` + `record_outcome_audit` + (renamed) `record_ran_command` from `src/main.rs` into a new lib module `src/shell_audit.rs` (desktop, storage-gated). `main.rs` (`ai exec`/`ai dispatch`) and `GatedRunner` (ash) both call it — DRY. Recording is best-effort and a no-op without the `storage` feature.

**Tech Stack:** Rust, `crate::store` (SQLite, storage feature), `crate::risk`, `crate::mask`, `serde_json`.

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-06-27-windows-ash-s2-audit-followup-design.md`.
- **shellcore/android:** `src/shell_audit.rs` is `cfg(not(target_os = "android"))` (uses desktop `pipeline`/`store`). `shellcore`/android cdylib unchanged. Verified by android cdylib check.
- **Recording is storage-gated + best-effort:** without `storage`, the record fns are no-ops; failures are silently ignored (`let _ =`); never affect the gate/shell.
- **Behavior unchanged for `ai exec`/`ai dispatch`** after the move (same logic, relocated).
- **Build env (WSL only):** `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **Exit-code detection:** NEVER `cmd | tail && echo OK` (pipe masks exit). Use `if cmd >/tmp/log 2>&1; then echo PASS; else echo FAIL; tail /tmp/log; fi` or `set -o pipefail`.
- **fmt + clippy:** run actual `cargo fmt --all`, then verify `cargo clippy --all-targets --features "storage tls remote" -- -D warnings` exits 0 (re-run; don't trust).
- **Verification gate:** `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"` AND default `cargo test` green.

---

## File Structure

- `src/shell_audit.rs` (create) — `AuditRecord`, `shell_outcome_audit`, `record_outcome_audit`, `record_ran_command` (+ tests).
- `src/lib.rs` (modify) — register `shell_audit` (cfg not android).
- `src/main.rs` (modify) — delete the moved items + their tests; update `finish_shell_outcome` to call `ai_terminal::shell_audit::*`.
- `src/gated_runner.rs` (modify) — record the outcome after `pipeline::execute`.

---

## Task 1: extract audit logic to `src/shell_audit.rs` + migrate `main.rs`

**Files:**
- Create: `src/shell_audit.rs`.
- Modify: `src/lib.rs`, `src/main.rs`.

**Interfaces:**
- Produces:
  - `pub struct AuditRecord { pub event_type: &'static str, pub level: String, pub payload_json: String }`
  - `pub fn shell_outcome_audit(command: &str, source: &str, outcome: &crate::pipeline::ExecOutcome) -> Option<AuditRecord>`
  - `pub fn record_outcome_audit(rec: &AuditRecord)` (storage-gated; no-op otherwise)
  - `pub fn record_ran_command(command: &str, exit_code: i32, source: &str)` (storage-gated; no-op otherwise)

- [ ] **Step 1: Register the module**

In `src/lib.rs`, add (near `shell`, keep the cfg):

```rust
#[cfg(not(target_os = "android"))]
pub mod shell_audit;
```

- [ ] **Step 2: Create `src/shell_audit.rs`**

```rust
//! 셸 실행 게이트 결과의 storage 기록(audit/command). `ai exec`와 `ash`가 공유한다.
//! storage feature 미빌드 시 기록 함수는 no-op이다.

use crate::pipeline::ExecOutcome;

/// 비-Ran 게이트 결과를 audit 레코드로 매핑한 결과.
pub struct AuditRecord {
    pub event_type: &'static str,
    pub level: String,
    pub payload_json: String,
}

/// 비-Ran ExecOutcome → AuditRecord(순수). Ran은 None(별도 command 기록).
pub fn shell_outcome_audit(
    command: &str,
    source: &str,
    outcome: &ExecOutcome,
) -> Option<AuditRecord> {
    let (event_type, level, mut payload) = match outcome {
        ExecOutcome::Ran { .. } => return None,
        ExecOutcome::Blocked { level, factors } => (
            "command_blocked",
            format!("{level:?}"),
            serde_json::json!({ "factors": factors }),
        ),
        ExecOutcome::Declined => (
            "command_declined",
            format!("{:?}", crate::risk::assess(command).level),
            serde_json::json!({}),
        ),
        ExecOutcome::BackupRefused(reason) => (
            "command_backup_refused",
            format!("{:?}", crate::risk::assess(command).level),
            serde_json::json!({ "reason": reason }),
        ),
    };
    let masked = crate::mask::Masker::baseline().mask(command).text;
    let map = payload
        .as_object_mut()
        .expect("audit payload must be a JSON object");
    map.insert("command".into(), serde_json::Value::String(masked));
    map.insert("source".into(), serde_json::Value::String(source.to_owned()));
    Some(AuditRecord {
        event_type,
        level,
        payload_json: payload.to_string(),
    })
}

/// audit 레코드를 영속화한다(storage feature, best-effort). 실패는 조용히 무시.
#[cfg(feature = "storage")]
pub fn record_outcome_audit(rec: &AuditRecord) {
    use crate::store::Store;
    let Ok(store) = Store::open_default() else {
        return;
    };
    let _ = store.record_audit(
        rec.event_type,
        Some(&rec.level),
        Some(&crate::config::get_active_profile()),
        &rec.payload_json,
    );
}

#[cfg(not(feature = "storage"))]
pub fn record_outcome_audit(_rec: &AuditRecord) {}

/// 실행된 명령을 commands + command_executed audit으로 기록한다(storage feature, best-effort).
#[cfg(feature = "storage")]
pub fn record_ran_command(command: &str, exit_code: i32, source: &str) {
    use crate::store::{NewCommand, NewSession, Store};
    let Ok(store) = Store::open_default() else {
        return;
    };
    let a = crate::risk::assess(command);
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .ok();
    let _ = store.get_or_create_session(
        "sess-default",
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: cwd.clone().unwrap_or_default(),
            policy_profile: crate::config::get_active_profile(),
        },
    );
    let _ = store.record_command(&NewCommand {
        session_id: "sess-default".into(),
        command_text: command.into(),
        source: source.into(),
        cwd,
        exit_code: Some(exit_code as i64),
        risk_level: Some(format!("{:?}", a.level)),
        risk_score: Some(a.score as i64),
        ai_generated: false,
        confirmed: true,
    });
    let _ = store.record_audit(
        "command_executed",
        Some(&format!("{:?}", a.level)),
        Some(&crate::config::get_active_profile()),
        &format!("{{\"exit\":{exit_code}}}"),
    );
}

#[cfg(not(feature = "storage"))]
pub fn record_ran_command(_command: &str, _exit_code: i32, _source: &str) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::ExecOutcome;

    #[test]
    fn ran_outcome_has_no_audit() {
        let out = ExecOutcome::Ran { exit_code: 0, undo_id: None };
        assert!(shell_outcome_audit("ls -al", "ash", &out).is_none());
    }

    #[test]
    fn blocked_maps_to_command_blocked() {
        let out = ExecOutcome::Blocked { level: crate::risk::RiskLevel::Critical, factors: vec!["x".into()] };
        let rec = shell_outcome_audit("rm -rf /", "ash", &out).expect("blocked → Some");
        assert_eq!(rec.event_type, "command_blocked");
        assert!(rec.payload_json.contains("\"source\":\"ash\""), "{}", rec.payload_json);
        assert!(rec.payload_json.contains("command"), "{}", rec.payload_json);
    }

    #[test]
    fn declined_and_backup_refused_map() {
        let d = shell_outcome_audit("rm -rf /", "ash", &ExecOutcome::Declined).expect("declined → Some");
        assert_eq!(d.event_type, "command_declined");
        let b = shell_outcome_audit("rm /tmp/x", "ash", &ExecOutcome::BackupRefused("big".into()))
            .expect("refused → Some");
        assert_eq!(b.event_type, "command_backup_refused");
        assert!(b.payload_json.contains("big"), "{}", b.payload_json);
    }
}
```

- [ ] **Step 3: Delete the moved code from `main.rs` and update call sites**

In `src/main.rs`:
1. **Delete** these items (they now live in `shell_audit`): `struct AuditRecord` (~line 1417), both `fn record_exec` cfg variants (~1375 + ~1413), `fn shell_outcome_audit` (~1425), both `fn record_outcome_audit` cfg variants (~1470 + ~1483), and the `shell_outcome_audit` tests in `mod tests` (~1610–1660: the `ran/blocked/declined/backup_refused`-audit test fns).
2. In `fn finish_shell_outcome` (~1488), update the three calls:

```rust
        record_ran_command(command, *exit_code, source);   // was record_exec(...)
```
```rust
    if let Some(rec) = ai_terminal::shell_audit::shell_outcome_audit(command, source, &outcome) {
        ai_terminal::shell_audit::record_outcome_audit(&rec);
    }
```
And add `use ai_terminal::shell_audit::record_ran_command;` (or call `ai_terminal::shell_audit::record_ran_command(...)` fully-qualified) near `finish_shell_outcome`.

> Only `finish_shell_outcome` calls these (call sites at main.rs ~1297 and ~1332 go through it). Do not change `run_exec`/`run_dispatch` signatures.

- [ ] **Step 4: Run tests (lib + default) + build**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib --features storage shell_audit 2>&1 | tail -8; if cargo build --bins >/tmp/b.log 2>&1; then echo BINS_OK; else echo BINS_FAIL; tail -20 /tmp/b.log; fi; if cargo build --features "storage tls remote" --bins >/tmp/b2.log 2>&1; then echo BINS_STORAGE_OK; else echo BINS_STORAGE_FAIL; tail -20 /tmp/b2.log; fi'`
Expected: shell_audit tests PASS; `BINS_OK` (default, no-op recording); `BINS_STORAGE_OK` (storage build, main.rs uses moved fns).

- [ ] **Step 5: fmt + clippy + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; if cargo clippy --all-targets --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1; then echo CLIPPY_CLEAN; else echo CLIPPY_FAIL; tail -20 /tmp/c.log; fi'
git add src/lib.rs src/shell_audit.rs src/main.rs
git commit -m "refactor(audit): extract shell gate audit into shared lib module"
```
(append `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`; only commit if CLIPPY_CLEAN)

---

## Task 2: record ash gate outcomes in `GatedRunner` (`src/gated_runner.rs`)

**Files:**
- Modify: `src/gated_runner.rs` (record after `pipeline::execute`).

**Interfaces:**
- Consumes: `crate::shell_audit::{shell_outcome_audit, record_outcome_audit, record_ran_command}` (Task 1); `pipeline::ExecOutcome`.

> No new unit test: recording is best-effort + storage-gated; the mapper is unit-tested in Task 1, and the storage path is covered by the e2e (sqlite query). The wiring is thin.

- [ ] **Step 1: Record the outcome in `GatedRunner::run`**

In `src/gated_runner.rs`, in `GatedRunner::run`, insert recording between `pipeline::execute` and `outcome_message`:

```rust
        let outcome = pipeline::execute(&cmd, &cfg, &executor, &mut confirmer, &mut sink)?;
        match &outcome {
            crate::pipeline::ExecOutcome::Ran { exit_code, .. } => {
                crate::shell_audit::record_ran_command(&cmd, *exit_code, "ash");
            }
            other => {
                if let Some(rec) = crate::shell_audit::shell_outcome_audit(&cmd, "ash", other) {
                    crate::shell_audit::record_outcome_audit(&rec);
                }
            }
        }
        let (msg, value) = outcome_message(&outcome, command.name);
```
(leave the rest of `run` unchanged.)

- [ ] **Step 2: Build + android boundary**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo build --bins >/tmp/b.log 2>&1; then echo BINS_OK; else echo BINS_FAIL; tail -20 /tmp/b.log; fi; if cargo check --lib --target aarch64-linux-android >/tmp/a.log 2>&1; then echo ANDROID_OK; else echo ANDROID_FAIL; tail -15 /tmp/a.log; fi'`
Expected: `BINS_OK`, `ANDROID_OK`.

- [ ] **Step 3: e2e — storage build records ash gate outcomes**

Write `/mnt/d/workspace/terminal-project/terminal/.git/sdd/audit_e2e.sh`:

```sh
source ~/.cargo/env
cd /mnt/d/workspace/terminal-project/terminal
export CARGO_TARGET_DIR=$HOME/targets/ai-terminal
cargo build --features storage --bin ash >/tmp/b.log 2>&1 && echo BINS_OK || { echo BINS_FAIL; tail -10 /tmp/b.log; }
ASH="$CARGO_TARGET_DIR/debug/ash"
D=$(mktemp -d)
printf 'echo hi\nrm -rf /\nexit\n' | XDG_CONFIG_HOME="$D" HOME="$D" "$ASH" >/tmp/o.out 2>/tmp/o.err
DB="$D/.config/ai-terminal/ai-terminal.db"
echo "db: $DB"
python3 - "$DB" <<'PY'
import sqlite3, sys
db = sys.argv[1]
try:
    con = sqlite3.connect(db)
    cur = con.cursor()
    ev = [r[0] for r in cur.execute("select event_type from audit_events").fetchall()]
    cmds = cur.execute("select count(*) from commands").fetchone()[0]
    print("audit_events:", ev)
    print("commands:", cmds)
    print("BLOCKED_AUDIT_OK" if "command_blocked" in ev else "BLOCKED_AUDIT_MISSING")
    print("RAN_RECORDED_OK" if cmds >= 1 else "RAN_RECORDED_MISSING")
except Exception as e:
    print("DB_ERROR", e)
PY
rm -rf "$D"
```
Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/d/workspace/terminal-project/terminal/.git/sdd/audit_e2e.sh`
Expected: `BINS_OK`; `BLOCKED_AUDIT_OK` (`rm -rf /` → `command_blocked` row); `RAN_RECORDED_OK` (`echo hi` → a `commands` row). (The DB lives under the storage default dir; the script points `HOME`/`XDG_CONFIG_HOME` at a temp dir so the run is isolated. If the default db path differs, find it with `find "$D" -name 'ai-terminal.db'`.)

- [ ] **Step 4: Full verification gate (default + storage)**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; if cargo fmt --all -- --check >/dev/null 2>&1 && cargo clippy --all-targets --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1 && cargo test --features "storage tls remote" >/tmp/t.log 2>&1 && cargo test >/tmp/t2.log 2>&1; then echo GATE_OK; else echo GATE_FAIL; tail -20 /tmp/c.log /tmp/t.log /tmp/t2.log; fi'`
Expected: `GATE_OK` (storage features AND default both green).

- [ ] **Step 5: Commit**

```bash
git add src/gated_runner.rs
git commit -m "feat(ash): record gate outcomes to storage (audit/command)"
```
(append the Co-Authored-By line)

---

## Self-Review

**Spec coverage:**
- §2 shared lib module (move + main.rs migration) → Task 1. §3 GatedRunner wiring (Ran→command, non-Ran→audit, source="ash") → Task 2. §4 best-effort/no-op → Task 1 (cfg variants) + Task 2 (uses them). §5 boundary/tests → Task 1 (mapper tests, default+storage build) + Task 2 (android, storage e2e). §6 acceptance → all + Task 2 Step 4. All covered.

**Placeholder scan:** No TBD/TODO. The main.rs deletion step references exact items/approx lines (a relocation of existing code); the moved tests are re-written in Task 1 Step 2 (not left as "move tests").

**Type consistency:** `AuditRecord`/`shell_outcome_audit`/`record_outcome_audit`/`record_ran_command` (Task 1) consumed by Task 2 and by main.rs's `finish_shell_outcome`. `NewCommand`/`NewSession`/`Store::record_command`/`record_audit`/`get_or_create_session` match the read-confirmed store signatures. `ExecOutcome` variants match `pipeline`.

**Note for implementer:** the move must be behavior-preserving — `ai exec`/`ai dispatch` keep recording exactly as before (only the functions' home changed, with `crate::` prefixes for the lib). Verify both the storage build and the default build compile (Task 1 Step 4). Do not change `record_ran_command`'s raw (unmasked) command_text or the `command_executed` payload format — keeping ai-exec parity; the masked-vs-raw inconsistency is a separate deferred follow-up.
