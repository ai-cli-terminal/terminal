# S4 — ash File-Backed History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist the reedline editor history to `<config_dir>/ash_history` (recall across sessions), with capacity from config and commands containing secrets/PII excluded via mask.

**Architecture:** A `FilteringHistory` wrapper over reedline's `FileBackedHistory` implements `reedline::History`, delegating all methods to the inner backend except `save`, which drops items where `is_sensitive_command` (mask) trips. `ReedlineReader::with_history(path, capacity)` builds it (fail-soft to in-memory on a corrupt file); `ash` passes the config-derived path + `history_limit`. All in `src/line_editor.rs` (desktop); `shellcore`/android unchanged.

**Tech Stack:** Rust, reedline 0.48 (`FileBackedHistory`, `History`, `HistoryItem`, `SearchQuery`), `crate::mask::Masker`.

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-06-27-windows-ash-s4-history-design.md`.
- **shellcore purity:** changes live in `src/line_editor.rs` (`cfg(not(target_os = "android"))`) and `src/bin/ash.rs`. `shellcore`/`LineReader` unchanged. Verified by android cdylib check.
- **fail-soft:** a corrupt/unreadable history file → in-memory history + a warning; never aborts `ash`.
- **Build env (WSL only):** `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **Exit-code detection:** NEVER `cmd | tail && echo OK` (pipe masks exit). Use `if cmd >/tmp/log 2>&1; then echo PASS; else echo FAIL; tail /tmp/log; fi` or `set -o pipefail`.
- **fmt:** run actual `cargo fmt --all` (not just `--check`) before committing; **then re-run `cargo clippy --all-targets --features "storage tls remote" -- -D warnings` and confirm it exits 0** (a prior slice's implementer misreported clippy — verify, don't trust).
- **Verification gate (each compiling task):** `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"` green.

---

## File Structure

- `src/line_editor.rs` (modify) — add `is_sensitive_command`, `FilteringHistory` (impl `reedline::History`), change `ReedlineReader::new()` → `with_history(path, capacity)`.
- `src/bin/ash.rs` (modify) — build the history path + capacity and call `with_history`.

---

## Task 1: `is_sensitive_command` predicate (`src/line_editor.rs`)

**Files:**
- Modify: `src/line_editor.rs` (add fn + tests).

**Interfaces:**
- Produces: `pub(crate) fn is_sensitive_command(cmd: &str) -> bool`.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` in `src/line_editor.rs`:

```rust
    #[test]
    fn sensitive_command_detects_secrets() {
        assert!(is_sensitive_command("git push ghp_aaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(is_sensitive_command("export PASSWORD=hunter2"));
    }

    #[test]
    fn sensitive_command_allows_plain_commands() {
        assert!(!is_sensitive_command("ls -al"));
        assert!(!is_sensitive_command("echo hi"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib line_editor 2>&1 | tail -20'`
Expected: FAIL — `cannot find function is_sensitive_command`.

- [ ] **Step 3: Write the implementation**

Add to `src/line_editor.rs` (top-level, near `map_signal`):

```rust
/// 명령 텍스트에 secret/PII가 탐지되면 true → history 저장에서 제외한다.
pub(crate) fn is_sensitive_command(cmd: &str) -> bool {
    !crate::mask::Masker::baseline().mask(cmd).redactions.is_empty()
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib line_editor 2>&1 | tail -8'`
Expected: PASS.

- [ ] **Step 5: fmt + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
git add src/line_editor.rs
git commit -m "feat(line-editor): is_sensitive_command via mask detection"
```
(append `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`)

---

## Task 2: `FilteringHistory` wrapper (`src/line_editor.rs`)

**Files:**
- Modify: `src/line_editor.rs` (imports + `FilteringHistory` + impl `reedline::History` + test).

**Interfaces:**
- Consumes: `is_sensitive_command` (Task 1).
- Produces: `struct FilteringHistory { inner: reedline::FileBackedHistory }` implementing `reedline::History`.

- [ ] **Step 1: Write the failing test (persistence + filtering via a real file)**

Add to the `#[cfg(test)] mod tests` in `src/line_editor.rs`:

```rust
    #[test]
    fn filtering_history_persists_only_nonsensitive() {
        use reedline::{FileBackedHistory, History, HistoryItem, SearchDirection, SearchQuery};
        let path = std::env::temp_dir().join(format!("ash_hist_test_{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let mut fh = FilteringHistory {
                inner: FileBackedHistory::with_file(100, path.clone()).unwrap(),
            };
            fh.save(HistoryItem::new("ls -al".to_string())).unwrap();
            fh.save(HistoryItem::new("git push ghp_aaaaaaaaaaaaaaaaaaaaaaaa".to_string()))
                .unwrap();
            fh.sync().unwrap();
        }
        let reloaded = FileBackedHistory::with_file(100, path.clone()).unwrap();
        let all = reloaded
            .search(SearchQuery::everything(SearchDirection::Backward, None))
            .unwrap();
        let cmds: Vec<String> = all.into_iter().map(|i| i.command_line).collect();
        assert!(cmds.iter().any(|c| c == "ls -al"), "{cmds:?}");
        assert!(!cmds.iter().any(|c| c.contains("ghp_")), "{cmds:?}");
        let _ = std::fs::remove_file(&path);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib line_editor 2>&1 | tail -20'`
Expected: FAIL — `cannot find type FilteringHistory`.

- [ ] **Step 3: Add imports + the wrapper**

In `src/line_editor.rs`, extend the reedline import line to include the history types and add `PathBuf`:

```rust
use std::path::PathBuf;

use reedline::{
    FileBackedHistory, History, HistoryItem, HistoryItemId, HistorySessionId, Prompt,
    PromptEditMode, PromptHistorySearch, Reedline, SearchQuery, Signal,
};
```
(merge with the existing `use reedline::{...}` line — do not duplicate `Prompt`/`Reedline`/`Signal`/`PromptEditMode`/`PromptHistorySearch`.)

Add the wrapper (top-level):

```rust
/// reedline History 래퍼: 민감명령은 저장에서 제외하고 나머지는 inner에 위임한다.
struct FilteringHistory {
    inner: FileBackedHistory,
}

impl History for FilteringHistory {
    fn save(&mut self, h: HistoryItem) -> reedline::Result<HistoryItem> {
        if is_sensitive_command(&h.command_line) {
            Ok(h) // 영속하지 않음(추가 안 함)
        } else {
            self.inner.save(h)
        }
    }
    fn load(&self, id: HistoryItemId) -> reedline::Result<HistoryItem> {
        self.inner.load(id)
    }
    fn count(&self, query: SearchQuery) -> reedline::Result<i64> {
        self.inner.count(query)
    }
    fn count_all(&self) -> reedline::Result<i64> {
        self.inner.count_all()
    }
    fn search(&self, query: SearchQuery) -> reedline::Result<Vec<HistoryItem>> {
        self.inner.search(query)
    }
    fn update(
        &mut self,
        id: HistoryItemId,
        updater: &dyn Fn(HistoryItem) -> HistoryItem,
    ) -> reedline::Result<()> {
        self.inner.update(id, updater)
    }
    fn clear(&mut self) -> reedline::Result<()> {
        self.inner.clear()
    }
    fn delete(&mut self, h: HistoryItemId) -> reedline::Result<()> {
        self.inner.delete(h)
    }
    fn sync(&mut self) -> std::io::Result<()> {
        self.inner.sync()
    }
    fn session(&self) -> Option<HistorySessionId> {
        self.inner.session()
    }
}
```

> Note: reedline's `History` trait surface is version-specific. The signatures above match reedline 0.48; if the compiler reports a different method set or signature, adjust the delegations to match (every non-`save` method delegates straight to `self.inner`). Keep the `save` filter exactly: sensitive → return the item without persisting; otherwise delegate.

- [ ] **Step 4: Run test to verify it passes**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib line_editor 2>&1 | tail -8'`
Expected: PASS — `filtering_history_persists_only_nonsensitive` green.

- [ ] **Step 5: fmt + clippy + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; cargo clippy --lib --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1 && echo CLIPPY_CLEAN || { echo CLIPPY_FAIL; tail -20 /tmp/c.log; }'
git add src/line_editor.rs
git commit -m "feat(line-editor): FilteringHistory drops sensitive commands"
```
(append the Co-Authored-By line; only commit if CLIPPY_CLEAN)

---

## Task 3: Wire history into `ReedlineReader` + `ash` (`src/line_editor.rs`, `src/bin/ash.rs`)

**Files:**
- Modify: `src/line_editor.rs` (`ReedlineReader::new` → `with_history`).
- Modify: `src/bin/ash.rs` (path + capacity, call `with_history`).

**Interfaces:**
- Consumes: `FilteringHistory` (Task 2), `is_sensitive_command` (Task 1), `config::config_dir` (existing).
- Produces: `pub fn ReedlineReader::with_history(path: PathBuf, capacity: usize) -> anyhow::Result<Self>` (replaces `new`).

- [ ] **Step 1: Replace `ReedlineReader::new` with `with_history`**

In `src/line_editor.rs`, replace the `impl ReedlineReader { pub fn new() ... }` block with:

```rust
impl ReedlineReader {
    /// 파일 영속 history로 에디터를 만든다. capacity=0이면 영속 없이 메모리 history만.
    /// history 파일 로드 실패는 fail-soft(메모리 history + 경고).
    pub fn with_history(path: PathBuf, capacity: usize) -> anyhow::Result<Self> {
        let editor = if capacity == 0 {
            Reedline::create()
        } else {
            match FileBackedHistory::with_file(capacity, path) {
                Ok(fbh) => {
                    Reedline::create().with_history(Box::new(FilteringHistory { inner: fbh }))
                }
                Err(e) => {
                    eprintln!("ash: history 파일 로드 실패({e}) — 메모리 history 사용");
                    Reedline::create()
                        .with_history(Box::new(FilteringHistory { inner: FileBackedHistory::new() }))
                }
            }
        };
        Ok(Self { editor })
    }
}
```

- [ ] **Step 2: Wire `ash.rs`**

In `src/bin/ash.rs`, replace the `ReedlineReader::new()` call inside the TTY branch with the history-backed construction:

```rust
        let history_path = ai_terminal::config::config_dir()
            .map(|d| d.join("ash_history"))
            .unwrap_or_else(|_| std::env::temp_dir().join("ash_history"));
        match ai_terminal::line_editor::ReedlineReader::with_history(
            history_path,
            loaded.config.general.history_limit,
        ) {
            Ok(r) => Box::new(r),
            Err(e) => {
                eprintln!("ash: 라인에디터 초기화 실패({e}) — 기본 입력 사용");
                Box::new(ai_terminal::shellcore::repl::StdinLineReader)
            }
        }
```
(`loaded` is the `config::load()` result already in `main`; keep the `else { Box::new(StdinLineReader) }` non-TTY branch and the `repl::run(settings, runner, reader)` call below unchanged.)

- [ ] **Step 3: Build both binaries**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo build --bins >/tmp/b.log 2>&1; then echo BINS_OK; else echo BINS_FAIL; tail -20 /tmp/b.log; fi'`
Expected: `BINS_OK`.

- [ ] **Step 4: Android boundary**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo check --lib --target aarch64-linux-android >/tmp/a.log 2>&1; then echo ANDROID_OK; else echo ANDROID_FAIL; tail -15 /tmp/a.log; fi'`
Expected: `ANDROID_OK`.

- [ ] **Step 5: e2e — non-TTY path still works**

Write `/mnt/d/workspace/terminal-project/terminal/.git/sdd/s4_e2e.sh`:

```sh
source ~/.cargo/env
cd /mnt/d/workspace/terminal-project/terminal
export CARGO_TARGET_DIR=$HOME/targets/ai-terminal
ASH="$CARGO_TARGET_DIR/debug/ash"
cargo build --bin ash >/tmp/b.log 2>&1 && echo BINS_OK || { echo BINS_FAIL; tail -10 /tmp/b.log; }
printf 'echo hi\nexit\n' | "$ASH" >/tmp/s.out 2>/dev/null; cat /tmp/s.out
printf 'rm -rf /\nexit\n' | "$ASH" >/tmp/blk.out 2>/tmp/blk.err
if grep -qi '차단' /tmp/blk.out /tmp/blk.err; then echo BLOCKED_OK; else echo BLOCKED_MISSING; cat /tmp/blk.err; fi
```
Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/d/workspace/terminal-project/terminal/.git/sdd/s4_e2e.sh`
Expected: `BINS_OK`; `echo hi` prints `hi`; `BLOCKED_OK`. (History persistence is non-TTY-invisible since piped stdin uses StdinLineReader; it is covered by the Task 2 file test + manual verification below.)

- [ ] **Step 6: Full verification gate**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; if cargo fmt --all -- --check >/dev/null 2>&1 && cargo clippy --all-targets --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1 && cargo test --features "storage tls remote" >/tmp/t.log 2>&1; then echo GATE_OK; else echo GATE_FAIL; tail -20 /tmp/c.log /tmp/t.log; fi'`
Expected: `GATE_OK`.

- [ ] **Step 7: Commit**

```bash
git add src/line_editor.rs src/bin/ash.rs
git commit -m "feat(ash): persist editor history to <config_dir>/ash_history"
```
(append the Co-Authored-By line)

> **Manual verification (cannot be scripted):** in a real terminal, run `ash`, enter `echo one` then `git push ghp_aaaaaaaaaaaaaaaaaaaaaaaa`, exit, then start `ash` again: ↑ should recall `echo one` but NOT the `ghp_` command; `<config_dir>/ash_history` should contain `echo one` and not the token.

---

## Self-Review

**Spec coverage:**
- §3 `is_sensitive_command` → Task 1. §4 `FilteringHistory` (save filter + delegation) → Task 2. §5 `ReedlineReader::with_history` (capacity 0 + corrupt fallback) → Task 3. §6 ash wiring (config_dir/ash_history + history_limit) → Task 3. §7 fail-soft → Task 3 Step 1 (Err fallback). §8 tests → Tasks 1–2 (is_sensitive_command, file persistence+filter) + Task 3 e2e + manual note. §2 boundary → Task 3 Step 4. §9 acceptance → all + Task 3 Step 6. All covered.

**Placeholder scan:** No TBD/TODO. The History-trait version note is a concrete compiler-guided instruction, not a placeholder.

**Type consistency:** `is_sensitive_command` (Task 1) used in Task 2's `save`. `FilteringHistory { inner: FileBackedHistory }` (Task 2) built in Task 3's `with_history`. `with_history(path: PathBuf, capacity: usize)` (Task 3 line_editor) called with the same arg order/types in Task 3 ash. reedline types (`HistoryItem`, `SearchQuery`, `SearchDirection`, `History`) consistent with context7-confirmed reedline 0.48 surface.

**Note for implementer:** the reedline `History` trait and `HistoryItem`/`SearchQuery` are version-specific (reedline 0.48 resolved in S3). If the compiler flags a signature mismatch, adjust the delegating methods to the resolved surface; the `save` filter logic and the persistence test assertions must stay as written.
