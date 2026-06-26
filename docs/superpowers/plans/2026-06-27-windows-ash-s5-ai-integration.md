# S5 — ash AI Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route ash REPL input through `intent::classify`: natural-language queries go to the AI gateway (mock), everything else stays the shell `eval_line` path, with AI failures absorbed as fail-soft so the shell never blocks.

**Architecture:** A `shellcore::repl::AiRouter` trait (`NoAiRouter` default, std-only) is injected into `repl::run`; on each submitted line the REPL asks the router to `try_handle` it — if handled (AI), it continues, else it falls through to `eval_line`. A desktop `GatewayAiRouter` (`src/ai_router.rs`) classifies via `dispatch::dispatch` and answers via the existing `GatewayResponder` (mock echo gateway), mapping `AiOutcome::{Answered,Blocked,Unavailable}` to printed output.

**Tech Stack:** Rust. Reuses `intent`, `dispatch`, `responder::GatewayResponder`, `policy`, `config` (desktop modules).

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-06-27-windows-ash-s5-ai-integration-design.md`.
- **shellcore purity:** `src/shellcore/repl.rs` references only `std` + the `AiRouter` trait — never intent/dispatch/gateway. The router impl lives in `src/ai_router.rs` (`cfg(not(target_os = "android"))`). Verified by android cdylib check.
- **S5 uses the mock (echo) gateway** (`GatewayResponder::mock()`), like `ai dispatch`. Real ollama/openai providers are a follow-on.
- **fail-soft:** AI timeout/cancel/backend failure → `AiOutcome::Unavailable` → message + REPL continues; never aborts the shell.
- **Build env (WSL only):** `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **Exit-code detection:** NEVER `cmd | tail && echo OK` (pipe masks exit). Use `if cmd >/tmp/log 2>&1; then echo PASS; else echo FAIL; tail /tmp/log; fi` or `set -o pipefail`.
- **fmt + clippy:** run actual `cargo fmt --all`, then **verify** `cargo clippy --all-targets --features "storage tls remote" -- -D warnings` exits 0 (don't trust — re-run).
- **Verification gate (each compiling task):** `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"` green.

---

## File Structure

- `src/shellcore/repl.rs` (modify) — add `AiRouter` trait + `NoAiRouter`; add `router` param to `run`; call `try_handle` before `eval_line`.
- `src/bin/ash.rs` (modify) — Task 1: pass `NoAiRouter`; Task 3: pass `GatewayAiRouter`.
- `src/lib.rs` (modify) — register `ai_router` (cfg not android).
- `src/ai_router.rs` (create) — `GatewayAiRouter`, `StdoutSink`.

---

## Task 1: `AiRouter` trait + REPL injection (`src/shellcore/repl.rs`, `src/bin/ash.rs`)

**Files:**
- Modify: `src/shellcore/repl.rs` (trait + `NoAiRouter` + `run` signature + loop; test).
- Modify: `src/bin/ash.rs` (pass `NoAiRouter`).

**Interfaces:**
- Produces:
  - `pub trait AiRouter { fn try_handle(&mut self, input: &str) -> bool; }`
  - `pub struct NoAiRouter;` implementing `AiRouter` (always false).
  - `pub fn run(settings: ReplSettings, runner: Box<dyn ExternalRunner>, reader: Box<dyn LineReader>, router: Box<dyn AiRouter>) -> Result<()>`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` in `src/shellcore/repl.rs`:

```rust
    #[test]
    fn no_ai_router_never_handles() {
        let mut r = NoAiRouter;
        assert!(!r.try_handle("ls -al"));
        assert!(!r.try_handle("how do I undo a commit?"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib shellcore::repl 2>&1 | tail -20'`
Expected: FAIL — `cannot find type NoAiRouter`.

- [ ] **Step 3: Add the trait, default, and router param**

In `src/shellcore/repl.rs`, add after the `LineReader`/`StdinLineReader` block:

```rust
/// 입력이 AI 질의면 처리(응답 출력)하고 true, 셸이면 false를 반환한다.
pub trait AiRouter {
    fn try_handle(&mut self, input: &str) -> bool;
}

/// 기본 라우터: 항상 false(모든 입력을 셸로). 임베드/비-AI/테스트용. std만 사용.
pub struct NoAiRouter;
impl AiRouter for NoAiRouter {
    fn try_handle(&mut self, _input: &str) -> bool {
        false
    }
}
```

Change the `run` signature to add `router` and call it in the `Line` arm:

```rust
pub fn run(
    settings: ReplSettings,
    runner: Box<dyn ExternalRunner>,
    mut reader: Box<dyn LineReader>,
    mut router: Box<dyn AiRouter>,
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
                if router.try_handle(&line) {
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

- [ ] **Step 4: Pass `NoAiRouter` from `ash.rs`**

In `src/bin/ash.rs`, change the final call to add a router (default for now):

```rust
    let router: Box<dyn ai_terminal::shellcore::repl::AiRouter> =
        Box::new(ai_terminal::shellcore::repl::NoAiRouter);
    if let Err(e) = ai_terminal::shellcore::repl::run(settings, runner, reader, router) {
        eprintln!("ash: {e}");
        std::process::exit(1);
    }
```

- [ ] **Step 5: Run tests + build**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib shellcore::repl 2>&1 | tail -6; if cargo build --bins >/tmp/b.log 2>&1; then echo BINS_OK; else echo BINS_FAIL; tail -15 /tmp/b.log; fi'`
Expected: repl tests PASS; `BINS_OK`.

- [ ] **Step 6: fmt + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
git add src/shellcore/repl.rs src/bin/ash.rs
git commit -m "refactor(shellcore): inject AiRouter into the REPL loop"
```
(append `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`)

---

## Task 2: `GatewayAiRouter` (`src/lib.rs`, `src/ai_router.rs`)

**Files:**
- Modify: `src/lib.rs` (register module).
- Create: `src/ai_router.rs` (`GatewayAiRouter`, `StdoutSink` + test).

**Interfaces:**
- Consumes: `shellcore::repl::AiRouter` (Task 1), `dispatch::{dispatch, Route, AiResponder, AiOutcome}`, `responder::GatewayResponder`, `policy::PolicyProfile`, `config`, `pipeline::OutputSink`.
- Produces: `pub struct GatewayAiRouter` with `pub fn from_environment() -> anyhow::Result<Self>`, implementing `AiRouter`.

- [ ] **Step 1: Register the module**

In `src/lib.rs`, add (near `aitask`, keep the cfg):

```rust
#[cfg(not(target_os = "android"))]
pub mod ai_router;
```

- [ ] **Step 2: Write the failing test**

Create `src/ai_router.rs` with the test module first:

```rust
//! ash 입력 AI 라우팅(데스크톱 호스트 계층). shellcore는 이 모듈을 모른다(경계 유지).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::repl::AiRouter;

    #[test]
    fn routes_ai_queries_and_leaves_shell() {
        let mut router = GatewayAiRouter::from_environment().unwrap();
        assert!(router.try_handle("how do I undo a commit?")); // AiQuery → handled
        assert!(router.try_handle("ai explain last-error")); // AiInline → handled
        assert!(!router.try_handle("ls -al")); // Shell → not handled
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib ai_router 2>&1 | tail -20'`
Expected: FAIL — `cannot find type GatewayAiRouter`.

- [ ] **Step 4: Write the implementation**

Insert above the `#[cfg(test)]` block in `src/ai_router.rs`:

```rust
use std::io::Write;

use crate::config;
use crate::dispatch::{self, AiOutcome, AiResponder, Route};
use crate::pipeline::OutputSink;
use crate::policy::PolicyProfile;
use crate::responder::GatewayResponder;
use crate::shellcore::repl::AiRouter;

/// AI 응답을 stdout으로 흘려보내는 sink.
struct StdoutSink;
impl OutputSink for StdoutSink {
    fn write(&mut self, c: &str) {
        print!("{c}");
        let _ = std::io::stdout().flush();
    }
}

/// 자연어 입력을 게이트웨이로 라우팅하는 AiRouter. S5는 mock(echo) 게이트웨이.
pub struct GatewayAiRouter {
    responder: GatewayResponder,
    profile: PolicyProfile,
}

impl GatewayAiRouter {
    /// mock(echo) 게이트웨이 + config 활성 profile로 구성한다.
    pub fn from_environment() -> anyhow::Result<Self> {
        let responder = GatewayResponder::mock()?;
        let profile = PolicyProfile::by_name(&config::get_active_profile())
            .unwrap_or_else(PolicyProfile::balanced);
        Ok(Self { responder, profile })
    }
}

impl AiRouter for GatewayAiRouter {
    fn try_handle(&mut self, input: &str) -> bool {
        let prompt = match dispatch::dispatch(input, &self.profile) {
            Route::Ai { prompt } => prompt,
            _ => return false,
        };
        let mut sink = StdoutSink;
        match self.responder.respond(&prompt, &mut sink) {
            Ok(AiOutcome::Answered { .. }) => println!(),
            Ok(AiOutcome::Blocked(r)) => eprintln!("ash: AI 차단됨: {r}"),
            Ok(AiOutcome::Unavailable(r)) => eprintln!("ash: AI 사용 불가: {r}"),
            Err(e) => eprintln!("ash: AI 오류: {e}"),
        }
        true
    }
}
```

- [ ] **Step 5: Run test + android boundary**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib ai_router 2>&1 | tail -6; rustup target add aarch64-linux-android >/dev/null 2>&1; if cargo check --lib --target aarch64-linux-android >/tmp/a.log 2>&1; then echo ANDROID_OK; else echo ANDROID_FAIL; tail -15 /tmp/a.log; fi'`
Expected: `routes_ai_queries_and_leaves_shell` PASS; `ANDROID_OK`.

- [ ] **Step 6: fmt + clippy + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; if cargo clippy --lib --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1; then echo CLIPPY_CLEAN; else echo CLIPPY_FAIL; tail -20 /tmp/c.log; fi'
git add src/lib.rs src/ai_router.rs
git commit -m "feat(ai-router): GatewayAiRouter routes NL to the mock gateway"
```
(append the Co-Authored-By line; only commit if CLIPPY_CLEAN)

---

## Task 3: Wire `GatewayAiRouter` into `ash` + verify (`src/bin/ash.rs`)

**Files:**
- Modify: `src/bin/ash.rs` (replace `NoAiRouter` with `GatewayAiRouter`, fail-soft).

**Interfaces:**
- Consumes: `GatewayAiRouter::from_environment` (Task 2), `NoAiRouter` (Task 1).

- [ ] **Step 1: Use `GatewayAiRouter` (fail-soft to `NoAiRouter`)**

In `src/bin/ash.rs`, replace the Task-1 `router` construction with:

```rust
    let router: Box<dyn ai_terminal::shellcore::repl::AiRouter> =
        match ai_terminal::ai_router::GatewayAiRouter::from_environment() {
            Ok(r) => Box::new(r),
            Err(_) => Box::new(ai_terminal::shellcore::repl::NoAiRouter),
        };
```
(keep the `repl::run(settings, runner, reader, router)` call below unchanged.)

- [ ] **Step 2: Build both binaries**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo build --bins >/tmp/b.log 2>&1; then echo BINS_OK; else echo BINS_FAIL; tail -20 /tmp/b.log; fi'`
Expected: `BINS_OK`.

- [ ] **Step 3: Android boundary**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo check --lib --target aarch64-linux-android >/tmp/a.log 2>&1; then echo ANDROID_OK; else echo ANDROID_FAIL; tail -15 /tmp/a.log; fi'`
Expected: `ANDROID_OK`.

- [ ] **Step 4: e2e — NL routes to AI, shell still works, gate intact**

Write `/mnt/d/workspace/terminal-project/terminal/.git/sdd/s5_e2e.sh`:

```sh
source ~/.cargo/env
cd /mnt/d/workspace/terminal-project/terminal
export CARGO_TARGET_DIR=$HOME/targets/ai-terminal
ASH="$CARGO_TARGET_DIR/debug/ash"
cargo build --bin ash >/tmp/b.log 2>&1 && echo BINS_OK || { echo BINS_FAIL; tail -10 /tmp/b.log; }
echo "-- NL query (AI, mock echo) --"
printf 'how do I list files?\nexit\n' | "$ASH" >/tmp/ai.out 2>/dev/null
if grep -qi 'how do I list files' /tmp/ai.out; then echo AI_OK; else echo AI_MISSING; cat /tmp/ai.out; fi
echo "-- shell echo hi --"
printf 'echo hi\nexit\n' | "$ASH" >/tmp/s.out 2>/dev/null
if grep -q '^.*hi' /tmp/s.out; then echo SHELL_OK; else echo SHELL_MISSING; cat /tmp/s.out; fi
echo "-- blocked rm -rf / --"
printf 'rm -rf /\nexit\n' | "$ASH" >/tmp/blk.out 2>/tmp/blk.err
if grep -qi '차단' /tmp/blk.out /tmp/blk.err; then echo BLOCKED_OK; else echo BLOCKED_MISSING; cat /tmp/blk.err; fi
```
Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/d/workspace/terminal-project/terminal/.git/sdd/s5_e2e.sh`
Expected: `BINS_OK`; `AI_OK` (the mock echo returns the prompt text, so the NL query routed to AI); `SHELL_OK` (`echo hi` ran the shell path); `BLOCKED_OK` (S2 gate still blocks `rm -rf /`).

- [ ] **Step 5: Full verification gate**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; if cargo fmt --all -- --check >/dev/null 2>&1 && cargo clippy --all-targets --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1 && cargo test --features "storage tls remote" >/tmp/t.log 2>&1; then echo GATE_OK; else echo GATE_FAIL; tail -20 /tmp/c.log /tmp/t.log; fi'`
Expected: `GATE_OK`.

- [ ] **Step 6: Commit**

```bash
git add src/bin/ash.rs
git commit -m "feat(ash): route natural-language input to the AI gateway"
```
(append the Co-Authored-By line)

> **Manual verification (cannot be scripted in a TTY):** in a real terminal, run `ash`, type `how do I undo a commit?` and confirm an AI (echo) answer prints instead of `command not found`; type `ls` and confirm it runs as a shell command; type `ai hello` and confirm AI handles it.

---

## Self-Review

**Spec coverage:**
- §3 `AiRouter`/`NoAiRouter` → Task 1. §4 `run` signature + loop → Task 1. §5 `GatewayAiRouter`/`from_environment`/`try_handle`/`StdoutSink` → Task 2. §6 ash wiring (fail-soft to NoAiRouter) → Task 3. §7 "AI 제안 명령은 게이트 통과" — satisfied by leaving shell commands on the existing gated `eval_line` path (no auto-execute); no task needed. §8 fail-soft → Task 2 (AiOutcome mapping) + Task 3 (from_environment fallback). §9 tests → Tasks 1–2 (NoAiRouter, GatewayAiRouter routing) + Task 3 e2e + manual note. §2 boundary → Task 2/3 android check. §10 acceptance → all + Task 3 Step 5. All covered.

**Placeholder scan:** No TBD/TODO; every code step has complete code and exact commands.

**Type consistency:** `AiRouter`/`NoAiRouter` (Task 1) consumed by Task 2 (`impl AiRouter`) and Task 3 (fallback). `run(settings, runner, reader, router)` (Task 1) called in Task 1 ash and unchanged in Task 3. `GatewayAiRouter::from_environment` (Task 2) used in Task 3. `dispatch::{dispatch, Route, AiResponder, AiOutcome}`, `GatewayResponder::mock`, `PolicyProfile::by_name` match the existing (read-confirmed) signatures.

**Note for implementer:** the mock gateway (`GatewayResponder::mock()`) echoes the prompt — that is expected; S5 is about routing, not a real LLM. Do not wire ollama/openai here (follow-on). Keep all AI specifics inside `ai_router.rs`; `shellcore::repl` must only know `AiRouter`.
