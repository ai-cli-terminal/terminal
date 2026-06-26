# S2 (core) — ash Safety Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route `ash` external command execution through the `pipeline::execute` safety gate (risk → policy → preview → confirm → undo backup → argv exec) while keeping execution argv-direct and `shellcore` decoupled from desktop gate modules.

**Architecture:** A new desktop module `src/gated_runner.rs` implements `shellcore::external::ExternalRunner` and is injected into the REPL by `ash`. It reconstructs a command string for analysis only, reuses `pipeline::execute` with an argv `Executor` seam, and maps `ExecOutcome` to REPL messages. A `spawn_inherit` helper is extracted in `shellcore::external` so both `DesktopRunner` and the gate's executor share argv spawning.

**Tech Stack:** Rust. Reuses `pipeline`, `policy`, `risk`, `preview`, `undo`, `config` (desktop modules). `std::io::IsTerminal` (rust ≥ 1.70).

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-06-26-windows-ash-s2-safety-gate-design.md`.
- **shellcore purity:** `src/shellcore/*` MUST NOT reference desktop modules (`pipeline`/`policy`/`gated_runner`/`config`). `spawn_inherit` added to `shellcore::external` uses only `std::process`/`winexec` (allowed — shellcore's own code). Verified by android cdylib check.
- **Execution stays argv-direct** (no shell). Only analysis reconstructs a command string.
- **DesktopRunner external behavior preserved** after the `spawn_inherit` refactor.
- **Build env (WSL only):** run cargo via
  `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **Exit-code detection:** NEVER use `cmd | tail && echo OK` (the pipe masks `cmd`'s exit — this caused a false-positive fmt pass + CI round-trip in S1). Use `if cmd >/tmp/log 2>&1; then echo PASS; else echo FAIL; tail /tmp/log; fi` or `set -o pipefail`.
- **fmt:** run actual `cargo fmt --all` (not just `--check`) before committing; the implementers write one-line code that CI's rustfmt may reflow.
- **Verification gate (each compiling task):** `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"` green.

---

## File Structure

- `src/shellcore/external.rs` (modify) — add `pub fn spawn_inherit(...) -> Result<Option<i32>>`; refactor `DesktopRunner::run` to use it (behavior preserved); remove the two `run_desktop_command` cfg variants.
- `src/lib.rs` (modify) — register `#[cfg(not(target_os = "android"))] pub mod gated_runner;`.
- `src/gated_runner.rs` (create) — pure helpers (`command_string`, `decide_confirm`, `outcome_message`) + `GatedRunner`/`ArgvExecutor`/`StdinConfirmer`/`NullSink` + `run`/`from_environment`.
- `src/shellcore/repl.rs` (modify) — `run(settings, runner: Box<dyn ExternalRunner>)`.
- `src/bin/ash.rs` (modify) — build `GatedRunner` and inject.

---

## Task 1: Extract `spawn_inherit` in `shellcore::external`

**Files:**
- Modify: `src/shellcore/external.rs` (add `spawn_inherit`, rewrite `DesktopRunner::run`, delete both `run_desktop_command` fns; extend `#[cfg(test)]`).

**Interfaces:**
- Produces: `pub fn spawn_inherit(name: &str, args: &[String], cwd: &Path) -> anyhow::Result<Option<i32>>` — spawns argv with inherited stdio, returns the process exit code (`None` = terminated by signal / no code). Does not print. `NotFound` → `bail!("command not found: {name}")`.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod` (create one if absent) in `src/shellcore/external.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(windows))]
    #[test]
    fn spawn_inherit_returns_exit_code() {
        let cwd = std::env::temp_dir();
        let code = spawn_inherit("/bin/sh", &["-c".to_string(), "exit 7".to_string()], &cwd).unwrap();
        assert_eq!(code, Some(7));
    }

    #[cfg(not(windows))]
    #[test]
    fn spawn_inherit_missing_command_errs() {
        let cwd = std::env::temp_dir();
        assert!(spawn_inherit("definitely_not_a_real_cmd_zzz", &[], &cwd).is_err());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib shellcore::external 2>&1 | tail -20'`
Expected: FAIL — `cannot find function spawn_inherit`.

- [ ] **Step 3: Write the implementation**

In `src/shellcore/external.rs`, add `spawn_inherit` (two cfg variants) and replace `DesktopRunner::run` body. Delete the existing `#[cfg(not(windows))] fn run_desktop_command` and `#[cfg(windows)] fn run_desktop_command`.

```rust
/// argv를 cwd/현재 env로 stdio 상속 spawn하고 exit code를 반환한다(None=시그널 종료).
/// 출력하지 않는다(호출측이 결과 처리). NotFound는 "command not found"로 bail.
#[cfg(not(windows))]
pub fn spawn_inherit(name: &str, args: &[String], cwd: &Path) -> Result<Option<i32>> {
    use std::process::Command;
    match Command::new(name).args(args).current_dir(cwd).status() {
        Ok(st) => Ok(st.code()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("command not found: {name}")
        }
        Err(e) => bail!("failed to run {name}: {e}"),
    }
}

/// Windows: winexec로 .exe/.cmd/.ps1 해석 후 argv 직접 spawn(stdio 상속).
#[cfg(windows)]
pub fn spawn_inherit(name: &str, args: &[String], cwd: &Path) -> Result<Option<i32>> {
    use std::process::Command;
    let path_dirs = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .unwrap_or_default();
    let pathext_raw = std::env::var("PATHEXT").ok();
    let pathext = winexec::split_pathext(pathext_raw.as_deref());
    let invocation = winexec::resolve_windows_invocation(name, cwd, &path_dirs, &pathext)
        .ok_or_else(|| anyhow::anyhow!("command not found: {name}"))?;
    let plan = winexec::spawn_plan(invocation, args);
    let mut cmd = Command::new(plan.program);
    cmd.args(plan.args);
    match cmd.current_dir(cwd).status() {
        Ok(st) => Ok(st.code()),
        Err(e) => bail!("failed to run {name}: {e}"),
    }
}
```

Replace `DesktopRunner`'s `run` method body with (single version, no cfg split):

```rust
    fn run(&self, command: ExternalCommand<'_>) -> Result<Value> {
        let args: Vec<String> = command.args.iter().map(|v| v.coerce_string()).collect();
        match spawn_inherit(command.name, &args, command.cwd)? {
            Some(0) => {}
            Some(code) => eprintln!("[{}: exit {code}]", command.name),
            None => eprintln!("[{}: exit signal]", command.name),
        }
        Ok(Value::Nothing)
    }
```

Ensure imports at the top of the file still compile: the `#[cfg(windows)] use crate::shellcore::winexec::{...}` line may now be unused at module scope — move `use` of `winexec` inside the `#[cfg(windows)] spawn_inherit` (it already references `winexec::...` by path, so delete the top-level `#[cfg(windows)] use ... winexec ...` import if it triggers an unused-import warning under `-D warnings`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib shellcore::external 2>&1 | tail -12'`
Expected: PASS — `spawn_inherit_returns_exit_code`, `spawn_inherit_missing_command_errs` green.

- [ ] **Step 5: fmt + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
git add src/shellcore/external.rs
git commit -m "refactor(shellcore): extract spawn_inherit for shared argv exec"
```
(append `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>` to the commit message)

---

## Task 2: Gate pure helpers (`src/gated_runner.rs` + `src/lib.rs`)

**Files:**
- Create: `src/gated_runner.rs` (pure helpers + tests).
- Modify: `src/lib.rs` (register module).

**Interfaces:**
- Consumes: `crate::pipeline::ExecOutcome`, `crate::shellcore::value::Value`.
- Produces:
  - `pub(crate) fn command_string(name: &str, args: &[Value]) -> String`
  - `pub(crate) fn decide_confirm(answer: &str) -> bool`
  - `pub(crate) fn outcome_message(outcome: &ExecOutcome, name: &str) -> (Option<String>, Value)`

- [ ] **Step 1: Register the module**

In `src/lib.rs`, add after the `gate` module line (keep the `cfg` gate):

```rust
#[cfg(not(target_os = "android"))]
pub mod gated_runner;
```

- [ ] **Step 2: Write the failing tests**

Create `src/gated_runner.rs` with only the test module first (helpers added in Step 4):

```rust
//! ash 외부 실행 안전 게이트 (데스크톱 호스트 계층).
//! `shellcore::external::ExternalRunner`를 구현해 risk/policy/preview/undo/pipeline 게이트를
//! ash 외부 명령 앞단에 결선한다. shellcore는 이 모듈을 모른다(경계 유지).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::ExecOutcome;
    use crate::risk::RiskLevel;
    use crate::shellcore::value::Value;

    #[test]
    fn command_string_joins_argv() {
        let args = vec![Value::String("-rf".into()), Value::String("/x".into())];
        assert_eq!(command_string("rm", &args), "rm -rf /x");
        assert_eq!(command_string("ls", &[]), "ls");
    }

    #[test]
    fn decide_confirm_only_yes() {
        assert!(decide_confirm("y"));
        assert!(decide_confirm("Y"));
        assert!(decide_confirm("yes"));
        assert!(decide_confirm("YES"));
        assert!(!decide_confirm(""));
        assert!(!decide_confirm("n"));
        assert!(!decide_confirm("no"));
        assert!(!decide_confirm("maybe"));
    }

    #[test]
    fn outcome_message_maps_variants() {
        let (m, _) = outcome_message(
            &ExecOutcome::Blocked { level: RiskLevel::Critical, factors: vec!["x".into()] },
            "rm",
        );
        assert!(m.unwrap().contains("차단"));
        assert_eq!(outcome_message(&ExecOutcome::Declined, "rm").0.as_deref(), Some("ash: 취소됨"));
        assert!(outcome_message(&ExecOutcome::BackupRefused("big".into()), "rm").0.unwrap().contains("백업 거부"));
        assert!(outcome_message(&ExecOutcome::Ran { exit_code: 0, undo_id: None }, "ls").0.is_none());
        assert_eq!(
            outcome_message(&ExecOutcome::Ran { exit_code: 3, undo_id: None }, "ls").0.as_deref(),
            Some("[ls: exit 3]")
        );
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib gated_runner 2>&1 | tail -20'`
Expected: FAIL — `cannot find function command_string`.

- [ ] **Step 4: Write the helpers**

Insert above the `#[cfg(test)]` block in `src/gated_runner.rs`:

```rust
use crate::pipeline::ExecOutcome;
use crate::shellcore::value::Value;

/// argv를 분석용 명령 문자열로 재구성한다. 실행엔 쓰지 않는다(argv 직접 spawn).
/// 한계: 공백/특수문자가 든 인자는 분석 토크나이즈가 부정확할 수 있다(안전 분석은 보수적).
pub(crate) fn command_string(name: &str, args: &[Value]) -> String {
    let mut s = String::from(name);
    for a in args {
        s.push(' ');
        s.push_str(&a.coerce_string());
    }
    s
}

/// 확인 응답 판정: y/yes(대소문자 무시)만 승인.
pub(crate) fn decide_confirm(answer: &str) -> bool {
    matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// ExecOutcome → (stderr 메시지, REPL 반환 Value). 비-Ran은 미실행을 뜻한다.
pub(crate) fn outcome_message(outcome: &ExecOutcome, name: &str) -> (Option<String>, Value) {
    match outcome {
        ExecOutcome::Blocked { level, factors } => (
            Some(format!("ash: 정책상 차단됨 [{level:?}] — {}", factors.join(", "))),
            Value::Nothing,
        ),
        ExecOutcome::Declined => (Some("ash: 취소됨".to_string()), Value::Nothing),
        ExecOutcome::BackupRefused(reason) => (
            Some(format!("ash: 백업 거부({reason}) — 실행 중단")),
            Value::Nothing,
        ),
        ExecOutcome::Ran { exit_code, .. } => {
            if *exit_code == 0 {
                (None, Value::Nothing)
            } else {
                (Some(format!("[{name}: exit {exit_code}]")), Value::Nothing)
            }
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib gated_runner 2>&1 | tail -10'`
Expected: PASS — the three helper tests green.

- [ ] **Step 6: fmt + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
git add src/lib.rs src/gated_runner.rs
git commit -m "feat(gated-runner): pure command-string/confirm/outcome helpers"
```
(append the Co-Authored-By line)

---

## Task 3: `GatedRunner` wiring (`src/gated_runner.rs`)

**Files:**
- Modify: `src/gated_runner.rs` (add `GatedRunner`, `ArgvExecutor`, `StdinConfirmer`, `NullSink`, `from_environment`, `run`).

**Interfaces:**
- Consumes: Task 2 helpers; `shellcore::external::{spawn_inherit, ExternalRunner, ExternalCommand, ExecutionCapabilities}`; `pipeline::{execute, ExecConfig, Executor, Confirmer, OutputSink, ConfirmRequest}`; `policy::PolicyProfile`; `undo::{self, UndoLimits}`; `config`.
- Produces: `pub struct GatedRunner` with `pub fn from_environment() -> Self`, implementing `shellcore::external::ExternalRunner`.

> No new unit tests here: the only new logic is thin wiring over `pipeline::execute` (already tested) and the pure helpers (Task 2). Behavior is covered by Task 4's e2e. Verify via compile + clippy.

- [ ] **Step 1: Add imports + types + impls**

Add to the top of `src/gated_runner.rs` (below the existing `use` lines from Task 2):

```rust
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::pipeline::{self, ConfirmRequest, Confirmer, ExecConfig, Executor, OutputSink};
use crate::policy::PolicyProfile;
use crate::shellcore::external::{
    spawn_inherit, ExecutionCapabilities, ExternalCommand, ExternalRunner,
};
use crate::undo::{self, UndoLimits};
use crate::{config, risk};
```

> If `risk` is unused after writing the file (it is only needed if you reference `RiskLevel` outside tests), drop it from this `use` to satisfy `-D warnings`. The wiring below does not need `risk`.

Then add the types and impls:

```rust
/// 출력 싱크 무시(stdio는 상속 spawn으로 직접 터미널에 감).
struct NullSink;
impl OutputSink for NullSink {
    fn write(&mut self, _chunk: &str) {}
}

/// 원본 argv를 직접 spawn하는 Executor. 파이프라인이 넘기는 문자열/sink는 무시한다.
struct ArgvExecutor<'a> {
    name: &'a str,
    args: Vec<String>,
    cwd: &'a Path,
}
impl Executor for ArgvExecutor<'_> {
    fn run(&self, _command: &str, _sink: &mut dyn OutputSink) -> Result<i32> {
        Ok(spawn_inherit(self.name, &self.args, self.cwd)?.unwrap_or(-1))
    }
}

/// stdin 기반 확인. 비-TTY는 fail-closed(거부).
struct StdinConfirmer {
    is_tty: bool,
}
impl Confirmer for StdinConfirmer {
    fn confirm(&mut self, req: &ConfirmRequest) -> bool {
        if !self.is_tty {
            eprintln!("ash: 비대화형 입력 — 확인 불가로 거부: {}", req.command);
            return false;
        }
        eprintln!("⚠ 확인 필요: {}", req.command);
        eprintln!("  위험도: {:?}  요인: {}", req.level, req.factors.join(", "));
        if !req.backup_files.is_empty() {
            eprintln!("  백업 대상: {}", req.backup_files.join(", "));
        }
        eprint!("  실행할까요? [y/N] ");
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        if std::io::stdin().read_line(&mut answer).is_err() {
            return false;
        }
        decide_confirm(&answer)
    }
}

/// ash 외부 실행을 안전 게이트로 감싸는 runner.
pub struct GatedRunner {
    profile: PolicyProfile,
    undo_dir: PathBuf,
    limits: UndoLimits,
    is_tty: bool,
}

impl GatedRunner {
    /// config의 활성 profile + 기본 undo dir/limits로 구성한다. 실패는 fail-soft.
    pub fn from_environment() -> Self {
        let name = config::get_active_profile();
        let profile = PolicyProfile::by_name(&name).unwrap_or_else(PolicyProfile::balanced);
        let undo_dir = undo::default_undo_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("ai-terminal-undo"));
        Self {
            profile,
            undo_dir,
            limits: UndoLimits::defaults(),
            is_tty: std::io::stdin().is_terminal(),
        }
    }
}

impl ExternalRunner for GatedRunner {
    fn capabilities(&self) -> ExecutionCapabilities {
        ExecutionCapabilities::desktop_process()
    }

    fn run(&self, command: ExternalCommand<'_>) -> Result<crate::shellcore::value::Value> {
        let cmd = command_string(command.name, command.args);
        let cfg = ExecConfig {
            profile: &self.profile,
            undo_dir: &self.undo_dir,
            limits: self.limits,
        };
        let args: Vec<String> = command.args.iter().map(|v| v.coerce_string()).collect();
        let executor = ArgvExecutor {
            name: command.name,
            args,
            cwd: command.cwd,
        };
        let mut confirmer = StdinConfirmer { is_tty: self.is_tty };
        let mut sink = NullSink;
        let outcome = pipeline::execute(&cmd, &cfg, &executor, &mut confirmer, &mut sink)?;
        let (msg, value) = outcome_message(&outcome, command.name);
        if let Some(m) = msg {
            eprintln!("{m}");
        }
        Ok(value)
    }
}
```

- [ ] **Step 2: Compile + clippy**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo clippy --lib --features "storage tls remote" -- -D warnings >/tmp/clip.log 2>&1; then echo CLIPPY_CLEAN; else echo CLIPPY_FAIL; tail -20 /tmp/clip.log; fi'`
Expected: `CLIPPY_CLEAN`. (Fix any unused-import warnings by trimming the `use` list — e.g. drop `risk` if unused.)

- [ ] **Step 3: Run gated_runner tests**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib gated_runner 2>&1 | tail -8'`
Expected: PASS — Task 2 helper tests still green.

- [ ] **Step 4: fmt + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
git add src/gated_runner.rs
git commit -m "feat(gated-runner): GatedRunner over pipeline::execute (argv exec)"
```
(append the Co-Authored-By line)

---

## Task 4: Inject into `ash` + verify (`src/shellcore/repl.rs`, `src/bin/ash.rs`)

**Files:**
- Modify: `src/shellcore/repl.rs` (`run` signature).
- Modify: `src/bin/ash.rs` (build + inject `GatedRunner`).

**Interfaces:**
- Consumes: `GatedRunner::from_environment()` (Task 3); `shellcore::external::ExternalRunner`.
- Produces: `pub fn run(settings: ReplSettings, runner: Box<dyn external::ExternalRunner>) -> Result<()>`.

- [ ] **Step 1: Change `repl::run` to accept a runner**

In `src/shellcore/repl.rs`: add `use crate::shellcore::engine::external;`? No — reference via full path. Change the import line and the function. Replace:

```rust
use crate::shellcore::engine::{eval_line, Engine};
```
with:
```rust
use crate::shellcore::engine::{eval_line, Engine};
use crate::shellcore::external::ExternalRunner;
```

Change the `run` signature and engine construction:

```rust
pub fn run(settings: ReplSettings, runner: Box<dyn ExternalRunner>) -> Result<()> {
    let mut engine = Engine::with_external_runner(runner);
    apply_settings(&mut engine, &settings);
    // ... rest of the existing loop unchanged ...
```

(`Engine::with_external_runner` already exists. The rest of `run` is unchanged.)

- [ ] **Step 2: Inject from `ash.rs`**

Replace `src/bin/ash.rs` `main` body's final call:

```rust
    let runner: Box<dyn ai_terminal::shellcore::external::ExternalRunner> =
        Box::new(ai_terminal::gated_runner::GatedRunner::from_environment());
    if let Err(e) = ai_terminal::shellcore::repl::run(settings, runner) {
        eprintln!("ash: {e}");
        std::process::exit(1);
    }
```

(Keep the existing config load + `settings` construction above it.)

- [ ] **Step 3: Build both binaries**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo build --bins >/tmp/b.log 2>&1; then echo BINS_OK; else echo BINS_FAIL; tail -20 /tmp/b.log; fi'`
Expected: `BINS_OK`.

- [ ] **Step 4: Android boundary check**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; rustup target add aarch64-linux-android >/dev/null 2>&1; if cargo check --lib --target aarch64-linux-android >/tmp/a.log 2>&1; then echo ANDROID_OK; else echo ANDROID_FAIL; tail -20 /tmp/a.log; fi'`
Expected: `ANDROID_OK` (shellcore did not start referencing desktop gate modules).

- [ ] **Step 5: e2e — gate behaviors**

Run (pipes commands into `ash`; stdin is non-TTY here, so High/Confirm commands fail-closed, and `rm -rf /` is Blocked regardless):

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; B=$(cargo build --bin ash --message-format=json 2>/dev/null | python3 -c "import sys,json;\n[print(o[\"executable\"]) for l in sys.stdin if (o:=json.loads(l)).get(\"reason\")==\"compiler-artifact\" and o.get(\"executable\")]" | tail -1); echo \"=== safe ===\"; printf "echo hi\nexit\n" | \"$B\"; echo \"=== blocked ===\"; printf "rm -rf /\nexit\n" | \"$B\" 2>&1 | grep -i 차단 && echo BLOCKED_OK || echo BLOCKED_MISSING'
```
Expected: safe command prints `hi`; the `rm -rf /` run prints a `차단` (blocked) line → `BLOCKED_OK`. The shell does not crash (REPL continues to `exit`).

> If resolving the ash binary path via cargo-json is awkward in the runner, instead run `cargo run -q --bin ash` with the same piped stdin.

- [ ] **Step 6: Full verification gate**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; if cargo fmt --all -- --check >/dev/null 2>&1 && cargo clippy --all-targets --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1 && cargo test --features "storage tls remote" >/tmp/t.log 2>&1; then echo GATE_OK; else echo GATE_FAIL; tail -20 /tmp/c.log /tmp/t.log; fi'`
Expected: `GATE_OK`.

- [ ] **Step 7: Commit**

```bash
git add src/shellcore/repl.rs src/bin/ash.rs
git commit -m "feat(ash): inject GatedRunner so external commands pass the safety gate"
```
(append the Co-Authored-By line)

---

## Self-Review

**Spec coverage:**
- §3 `spawn_inherit` extraction → Task 1. §4 `GatedRunner`/`from_environment` → Task 3. §5.1 `command_string` / §5.3 `StdinConfirmer`+`decide_confirm` / §5.4 `outcome_message` → Tasks 2–3. §5.2 `ArgvExecutor` → Task 3. §6 `repl::run`+ash injection → Task 4. §7 fail-soft → Task 3 (`from_environment` fallbacks) + outcome mapping. §8 tests → Tasks 1–4 (unit: spawn_inherit, command_string, decide_confirm, outcome_message; e2e: Task 4 Step 5). §9 acceptance → Tasks 1–4 + Task 4 Step 6 gate. §2 boundary → Task 4 Step 4. All covered.
- Spec §5.5 outcome_message lives in Task 2 (defined) and is consumed in Task 3 — consistent.

**Placeholder scan:** No TBD/TODO; every code step has complete code and exact commands.

**Type consistency:** `spawn_inherit(name, args, cwd) -> Result<Option<i32>>` defined in Task 1, consumed in Task 3 (`ArgvExecutor`, `.unwrap_or(-1)`) and Task 1 (`DesktopRunner`). `command_string`/`decide_confirm`/`outcome_message` defined Task 2, used Task 3. `GatedRunner::from_environment` defined Task 3, used Task 4. `run(settings, runner)` defined Task 4 repl, called Task 4 ash. Names consistent.

**Note for implementer:** `RiskLevel` is only referenced in the Task 2 test (`crate::risk::RiskLevel`), not in non-test code — do not add `risk` to the non-test `use` list unless the compiler asks for it. Trim any unused import to satisfy `-D warnings`.
