# S3 — ash Line Editor (reedline) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give interactive `ash` rich line editing, in-session history, Ctrl-C (cancel) and Ctrl-D (EOF) via reedline, behind a `LineReader` abstraction so `shellcore` stays reedline-free and the android cdylib build is unchanged.

**Architecture:** `shellcore::repl` gains a `LineReader` trait (`ReadOutcome`: Line/Eof/Interrupted) with a std-only `StdinLineReader` default. A desktop module `src/line_editor.rs` provides `ReedlineReader` (reedline, target-gated on crossterm) with an `AshPrompt`. `ash` picks `ReedlineReader` on a TTY (fail-soft to `StdinLineReader`) and `StdinLineReader` otherwise.

**Tech Stack:** Rust, reedline (target-gated, non-android), `std::io::IsTerminal`.

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-06-26-windows-ash-s3-line-editor-design.md`.
- **shellcore purity:** `src/shellcore/*` references only `std` + the `LineReader` trait — never reedline/crossterm. reedline lives in `src/line_editor.rs` (`cfg(not(target_os = "android"))`). Verified by android cdylib check.
- **reedline dependency** goes under `[target.'cfg(not(target_os = "android"))'.dependencies]` (next to crossterm/ratatui/portable-pty), NOT plain `[dependencies]`.
- **Non-TTY uses StdinLineReader** so pipes/scripts and the S2 gate e2e keep working.
- **Build env (WSL only):** `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **Exit-code detection:** NEVER `cmd | tail && echo OK` (pipe masks exit — caused an S1 CI round-trip). Use `if cmd >/tmp/log 2>&1; then echo PASS; else echo FAIL; tail /tmp/log; fi` or `set -o pipefail`.
- **fmt:** run actual `cargo fmt --all` (not just `--check`) before committing.
- **Verification gate (each compiling task):** `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"` green.

---

## File Structure

- `src/shellcore/repl.rs` (modify) — add `ReadOutcome`, `LineReader`, `read_outcome_from`, `StdinLineReader`; change `run` to take `reader`; loop consumes `ReadOutcome`.
- `src/bin/ash.rs` (modify) — Task 1: pass `StdinLineReader`; Task 3: select `ReedlineReader` on TTY.
- `Cargo.toml` (modify) — add reedline (target-gated).
- `src/lib.rs` (modify) — register `line_editor` (cfg not android).
- `src/line_editor.rs` (create) — `ReedlineReader`, `AshPrompt`, `map_signal`.

---

## Task 1: `LineReader` abstraction + StdinLineReader (`src/shellcore/repl.rs`, `src/bin/ash.rs`)

**Files:**
- Modify: `src/shellcore/repl.rs` (types + trait + `read_outcome_from` + `StdinLineReader` + `run` signature/loop; extend tests).
- Modify: `src/bin/ash.rs` (pass `StdinLineReader`).

**Interfaces:**
- Produces:
  - `pub enum ReadOutcome { Line(String), Eof, Interrupted }` (derives `Debug`).
  - `pub trait LineReader { fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome>; }`
  - `pub(crate) fn read_outcome_from(reader: &mut impl std::io::BufRead) -> std::io::Result<ReadOutcome>`
  - `pub struct StdinLineReader;` implementing `LineReader`.
  - `pub fn run(settings: ReplSettings, runner: Box<dyn ExternalRunner>, reader: Box<dyn LineReader>) -> Result<()>`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` in `src/shellcore/repl.rs`:

```rust
    #[test]
    fn read_outcome_eof_on_empty_input() {
        let mut c = std::io::Cursor::new(&b""[..]);
        assert!(matches!(read_outcome_from(&mut c).unwrap(), ReadOutcome::Eof));
    }

    #[test]
    fn read_outcome_line_trims_newline() {
        let mut c = std::io::Cursor::new(&b"echo hi\n"[..]);
        match read_outcome_from(&mut c).unwrap() {
            ReadOutcome::Line(l) => assert_eq!(l, "echo hi"),
            other => panic!("expected Line, got {other:?}"),
        }
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib shellcore::repl 2>&1 | tail -20'`
Expected: FAIL — `cannot find function read_outcome_from` / `ReadOutcome`.

- [ ] **Step 3: Add types, trait, reader, and change `run`**

In `src/shellcore/repl.rs`, add after the `ReplSettings`/`apply_settings` block:

```rust
/// 한 줄 읽기 결과.
#[derive(Debug)]
pub enum ReadOutcome {
    Line(String),
    Eof,
    Interrupted,
}

/// 프롬프트를 표시하고 한 줄을 읽는다. 구현은 호스트가 주입한다.
pub trait LineReader {
    fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome>;
}

/// 주입된 reader에서 한 줄을 읽어 결과로 분류한다(프롬프트 I/O 없는 순수 코어).
pub(crate) fn read_outcome_from(reader: &mut impl std::io::BufRead) -> std::io::Result<ReadOutcome> {
    let mut line = String::new();
    let n = reader.read_line(&mut line)?;
    if n == 0 {
        Ok(ReadOutcome::Eof)
    } else {
        Ok(ReadOutcome::Line(line.trim_end().to_string()))
    }
}

/// 기본 라인 reader(편집 없음). 임베드/비-TTY/테스트용. std만 사용.
pub struct StdinLineReader;
impl LineReader for StdinLineReader {
    fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome> {
        print!("{prompt}");
        io::stdout().flush().ok();
        let stdin = io::stdin();
        let mut lock = stdin.lock();
        read_outcome_from(&mut lock)
    }
}
```

Replace the `run` function with:

```rust
/// REPL을 실행한다. 라인 reader가 EOF/Interrupt/Line을 돌려준다.
pub fn run(
    settings: ReplSettings,
    runner: Box<dyn ExternalRunner>,
    mut reader: Box<dyn LineReader>,
) -> Result<()> {
    let mut engine = Engine::with_external_runner(runner);
    apply_settings(&mut engine, &settings);
    let home = crate::shellcore::util::home_dir();
    loop {
        let prompt = make_prompt(&engine.cwd, home.as_ref());
        match reader.read_line(&prompt)? {
            ReadOutcome::Eof => {
                println!();
                break;
            }
            ReadOutcome::Interrupted => continue,
            ReadOutcome::Line(line) => {
                if line.is_empty() {
                    continue;
                }
                match eval_line(&line, &mut engine) {
                    Ok(Value::Nothing) => {}
                    Ok(v) => println!("{}", format_value(&v)),
                    Err(e) => eprintln!("error: {e}"),
                }
                if let Some(code) = engine.exit_code {
                    std::process::exit(code);
                }
            }
        }
    }
    Ok(())
}
```

(The `use std::io::{self, Write};` at the top of the file already covers `io::stdout`/`io::stdin`/`Write`.)

- [ ] **Step 4: Update `ash.rs` to pass a reader**

In `src/bin/ash.rs`, change the `repl::run(settings, runner)` call to:

```rust
    let reader: Box<dyn ai_terminal::shellcore::repl::LineReader> =
        Box::new(ai_terminal::shellcore::repl::StdinLineReader);
    if let Err(e) = ai_terminal::shellcore::repl::run(settings, runner, reader) {
        eprintln!("ash: {e}");
        std::process::exit(1);
    }
```

(Keep the existing `settings`/`runner` construction above it.)

- [ ] **Step 5: Run tests + build**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib shellcore::repl 2>&1 | tail -8; if cargo build --bins >/tmp/b.log 2>&1; then echo BINS_OK; else echo BINS_FAIL; tail -15 /tmp/b.log; fi'`
Expected: repl tests PASS; `BINS_OK`.

- [ ] **Step 6: fmt + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
git add src/shellcore/repl.rs src/bin/ash.rs
git commit -m "refactor(shellcore): inject LineReader into the REPL loop"
```
(append `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`)

---

## Task 2: reedline `ReedlineReader` (`Cargo.toml`, `src/lib.rs`, `src/line_editor.rs`)

**Files:**
- Modify: `Cargo.toml` (add reedline, target-gated).
- Modify: `src/lib.rs` (register `line_editor`).
- Create: `src/line_editor.rs` (`ReedlineReader`, `AshPrompt`, `map_signal` + tests).

**Interfaces:**
- Consumes: `crate::shellcore::repl::{LineReader, ReadOutcome}` (Task 1).
- Produces: `pub struct ReedlineReader` with `pub fn new() -> anyhow::Result<Self>` implementing `LineReader`; `pub(crate) fn map_signal(sig: reedline::Signal) -> ReadOutcome`.

- [ ] **Step 1: Add the reedline dependency (target-gated)**

Run (this resolves a crossterm-compatible version and places it under the right target table):
`MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo add reedline --target "cfg(not(target_os = \"android\"))" 2>&1 | tail -8'`
Then confirm `Cargo.toml` shows `reedline = "..."` inside `[target.'cfg(not(target_os = "android"))'.dependencies]` (move it there manually if `cargo add` placed it elsewhere).

- [ ] **Step 2: Register the module**

In `src/lib.rs`, add (alphabetically near `lock`/`mask`; keep the cfg):

```rust
#[cfg(not(target_os = "android"))]
pub mod line_editor;
```

- [ ] **Step 3: Write the failing tests**

Create `src/line_editor.rs` with the test module first:

```rust
//! reedline 기반 라인 에디터(데스크톱 호스트 계층). shellcore는 이 모듈을 모른다(경계 유지).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::repl::ReadOutcome;
    use reedline::Signal;

    #[test]
    fn map_signal_success_to_line() {
        match map_signal(Signal::Success("x".to_string())) {
            ReadOutcome::Line(l) => assert_eq!(l, "x"),
            o => panic!("expected Line, got {o:?}"),
        }
    }

    #[test]
    fn map_signal_ctrld_is_eof_and_ctrlc_is_interrupt() {
        assert!(matches!(map_signal(Signal::CtrlD), ReadOutcome::Eof));
        assert!(matches!(map_signal(Signal::CtrlC), ReadOutcome::Interrupted));
    }

    #[test]
    fn ash_prompt_left_returns_injected_text() {
        let p = AshPrompt::new("~/x〉 ");
        assert_eq!(p.render_prompt_left(), "~/x〉 ");
    }
}
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib line_editor 2>&1 | tail -20'`
Expected: FAIL — `cannot find function map_signal` / `AshPrompt`.

- [ ] **Step 5: Write the implementation**

Insert above the `#[cfg(test)]` block in `src/line_editor.rs`:

```rust
use std::borrow::Cow;

use reedline::{Prompt, PromptEditMode, PromptHistorySearch, Reedline, Signal};

use crate::shellcore::repl::{LineReader, ReadOutcome};

/// reedline Signal → ReadOutcome. CtrlC=취소(Interrupted), CtrlD=EOF.
pub(crate) fn map_signal(sig: Signal) -> ReadOutcome {
    match sig {
        Signal::Success(line) => ReadOutcome::Line(line),
        Signal::CtrlD => ReadOutcome::Eof,
        Signal::CtrlC => ReadOutcome::Interrupted,
        _ => ReadOutcome::Interrupted,
    }
}

/// repl이 만든 프롬프트 문자열을 그대로 렌더하는 reedline Prompt.
struct AshPrompt {
    text: String,
}
impl AshPrompt {
    fn new(text: &str) -> Self {
        Self { text: text.to_string() }
    }
}
impl Prompt for AshPrompt {
    fn render_prompt_left(&self) -> Cow<str> {
        Cow::Owned(self.text.clone())
    }
    fn render_prompt_right(&self) -> Cow<str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> Cow<str> {
        Cow::Borrowed("")
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<str> {
        Cow::Borrowed("::: ")
    }
    fn render_prompt_history_search_indicator(&self, _hs: PromptHistorySearch) -> Cow<str> {
        Cow::Borrowed("")
    }
}

/// reedline 기반 라인 에디터(편집·in-session history·Ctrl-C/D).
pub struct ReedlineReader {
    editor: Reedline,
}
impl ReedlineReader {
    /// 실패 시 호출측이 StdinLineReader로 폴백할 수 있게 Result 반환.
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            editor: Reedline::create(),
        })
    }
}
impl LineReader for ReedlineReader {
    fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome> {
        let p = AshPrompt::new(prompt);
        self.editor
            .read_line(&p)
            .map(map_signal)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }
}
```

> Note: reedline's `Prompt` trait must be implemented exactly as the resolved version declares it. If the compiler reports a missing or extra method, adjust `AshPrompt` to match — return `Cow::Borrowed("")` for any extra required method. If `read_line` already returns `std::io::Result<Signal>`, the `.map_err(...)` is a harmless identity-ish conversion; keep it so the code compiles regardless of reedline's error type.

- [ ] **Step 6: Run tests + android boundary**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib line_editor 2>&1 | tail -8; rustup target add aarch64-linux-android >/dev/null 2>&1; if cargo check --lib --target aarch64-linux-android >/tmp/a.log 2>&1; then echo ANDROID_OK; else echo ANDROID_FAIL; tail -15 /tmp/a.log; fi'`
Expected: line_editor tests PASS; `ANDROID_OK` (reedline did not leak into the android build).

- [ ] **Step 7: fmt + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
git add Cargo.toml Cargo.lock src/lib.rs src/line_editor.rs
git commit -m "feat(line-editor): reedline ReedlineReader with AshPrompt"
```
(append the Co-Authored-By line)

---

## Task 3: Select reader by TTY in `ash` + verify (`src/bin/ash.rs`)

**Files:**
- Modify: `src/bin/ash.rs` (TTY-based reader selection, fail-soft).

**Interfaces:**
- Consumes: `ReedlineReader::new()` (Task 2), `StdinLineReader` (Task 1), `std::io::IsTerminal`.

- [ ] **Step 1: Update reader selection**

In `src/bin/ash.rs`, replace the Task-1 reader construction (the `let reader = Box::new(StdinLineReader)` block) with:

```rust
    use std::io::IsTerminal;
    let reader: Box<dyn ai_terminal::shellcore::repl::LineReader> = if std::io::stdin().is_terminal()
    {
        match ai_terminal::line_editor::ReedlineReader::new() {
            Ok(r) => Box::new(r),
            Err(e) => {
                eprintln!("ash: 라인에디터 초기화 실패({e}) — 기본 입력 사용");
                Box::new(ai_terminal::shellcore::repl::StdinLineReader)
            }
        }
    } else {
        Box::new(ai_terminal::shellcore::repl::StdinLineReader)
    };
    if let Err(e) = ai_terminal::shellcore::repl::run(settings, runner, reader) {
        eprintln!("ash: {e}");
        std::process::exit(1);
    }
```

- [ ] **Step 2: Build both binaries**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo build --bins >/tmp/b.log 2>&1; then echo BINS_OK; else echo BINS_FAIL; tail -20 /tmp/b.log; fi'`
Expected: `BINS_OK`.

- [ ] **Step 3: e2e — non-TTY path unchanged (StdinLineReader)**

Write `/mnt/d/workspace/terminal-project/terminal/.git/sdd/s3_e2e.sh` (a file, to avoid inline-quote breakage) with:

```sh
source ~/.cargo/env
cd /mnt/d/workspace/terminal-project/terminal
export CARGO_TARGET_DIR=$HOME/targets/ai-terminal
ASH="$CARGO_TARGET_DIR/debug/ash"
cargo build --bin ash >/tmp/b.log 2>&1 && echo BINS_OK || { echo BINS_FAIL; tail -10 /tmp/b.log; }
echo "-- safe echo (piped, non-TTY) --"
printf 'echo hi\nexit\n' | "$ASH" >/tmp/s.out 2>/tmp/s.err; cat /tmp/s.out
echo "-- blocked rm -rf / (piped) --"
printf 'rm -rf /\nexit\n' | "$ASH" >/tmp/blk.out 2>/tmp/blk.err
if grep -qi '차단' /tmp/blk.out /tmp/blk.err; then echo BLOCKED_OK; else echo BLOCKED_MISSING; cat /tmp/blk.err; fi
```
Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/d/workspace/terminal-project/terminal/.git/sdd/s3_e2e.sh`
Expected: `BINS_OK`; safe `echo hi` prints `hi` (StdinLineReader, since piped stdin is non-TTY); `BLOCKED_OK` (S2 gate still blocks `rm -rf /`). The REPL does not crash.

- [ ] **Step 4: Android boundary**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo check --lib --target aarch64-linux-android >/tmp/a.log 2>&1; then echo ANDROID_OK; else echo ANDROID_FAIL; tail -15 /tmp/a.log; fi'`
Expected: `ANDROID_OK`.

- [ ] **Step 5: Full verification gate**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; if cargo fmt --all -- --check >/dev/null 2>&1 && cargo clippy --all-targets --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1 && cargo test --features "storage tls remote" >/tmp/t.log 2>&1; then echo GATE_OK; else echo GATE_FAIL; tail -20 /tmp/c.log /tmp/t.log; fi'`
Expected: `GATE_OK`.

- [ ] **Step 6: Commit**

```bash
git add src/bin/ash.rs
git commit -m "feat(ash): use reedline editor on a TTY, stdin otherwise"
```
(append the Co-Authored-By line)

> **Manual verification (cannot be scripted in CI):** in a real terminal run `ash` and confirm: left/right + backspace editing, ↑/↓ recalls in-session history, Ctrl-C cancels the current line and re-prompts (does not exit), Ctrl-D on an empty line exits. Check both a native Windows console and a ConPTY host (Windows Terminal / WSL).

---

## Self-Review

**Spec coverage:**
- §3 `ReadOutcome`/`LineReader` → Task 1. §4 `StdinLineReader`/`read_outcome_from` → Task 1. §5 `ReedlineReader`/`AshPrompt`/signal map → Task 2. §6 `run` signature + ash TTY selection → Tasks 1 & 3. §7 reedline target-gated → Task 2 Step 1. §8 fail-soft → Task 3 Step 1 (ReedlineReader::new fallback). §9 tests → Tasks 1–3 (unit: read_outcome_from, map_signal, AshPrompt; e2e: Task 3 Step 3; manual note). §2 boundary → Task 2 Step 6 + Task 3 Step 4. §10 acceptance → all tasks + Task 3 Step 5. All covered.

**Placeholder scan:** No TBD/TODO. The reedline-version pin is resolved by `cargo add` (Task 2 Step 1), not left vague. The Prompt-trait note tells the implementer to match the resolved trait surface — concrete instruction, not a placeholder.

**Type consistency:** `ReadOutcome`/`LineReader`/`read_outcome_from`/`StdinLineReader` defined Task 1, consumed Tasks 2–3. `run(settings, runner, reader)` defined Task 1, called Task 1 ash and updated Task 3. `map_signal`/`ReedlineReader::new` defined Task 2, used Task 3. Names consistent.

**Note for implementer:** reedline's `Prompt` trait and `Signal`/`read_line` surfaces are version-specific (resolved by `cargo add`). The provided `AshPrompt`/`map_signal` match recent reedline; if the compiler flags a mismatch, adjust to the resolved version (extra Prompt methods → return `Cow::Borrowed("")`; non-`io::Error` read_line error → the `.map_err` wrapper already handles it). Do not change the `LineReader` trait or `ReadOutcome` to accommodate reedline — keep reedline specifics inside `line_editor.rs`.
