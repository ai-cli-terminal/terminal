# S6 — ash MSYS Bridge Runner Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When `AI_TERMINAL_WINDOWS_PROFILE=msys` is active (with `MSYSTEM` present), run ash external commands through the MSYS POSIX host (`sh -lc "<cmd>"`) instead of native argv spawn, keeping the S2 gate unchanged.

**Architecture:** Add pure `msys::bridge_invocation` + `msys::active_profile` (std-only, android-safe) to `shellcore::msys`, then branch `gated_runner`'s `ArgvExecutor::run` on the active profile — MSYS reuses `spawn_inherit("sh", ["-lc", cmd], cwd)` (the `command` arg already carries the reconstructed command string); native keeps argv spawn.

**Tech Stack:** Rust, `std::process` (via existing `spawn_inherit`), `std::env`. Reuses `shellcore::msys::select_profile`.

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-06-27-windows-ash-s6-msys-bridge-design.md`.
- **shellcore purity:** `shellcore::msys` additions use only `std`. The execution branch lives in `src/gated_runner.rs` (desktop). `shellcore::external`/`DesktopRunner`/android cdylib unchanged. Verified by android cdylib check.
- **No native/MSYS mixing:** the profile is exclusive — MSYS profile → sh host only; native → winexec/argv only.
- **Execution is Windows+MSYS-only** and cannot be tested in WSL/CI-Linux (where `active_profile` resolves to native); only the pure functions are unit-tested. Manual Windows verification covers the sh path.
- **Build env (WSL only):** `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **Exit-code detection:** NEVER `cmd | tail && echo OK` (pipe masks exit). Use `if cmd >/tmp/log 2>&1; then echo PASS; else echo FAIL; tail /tmp/log; fi` or `set -o pipefail`.
- **fmt + clippy:** run actual `cargo fmt --all`, then verify `cargo clippy --all-targets --features "storage tls remote" -- -D warnings` exits 0 (re-run; don't trust).
- **Verification gate (each compiling task):** `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"` green.

---

## File Structure

- `src/shellcore/msys.rs` (modify) — add `bridge_invocation` + `active_profile`.
- `src/gated_runner.rs` (modify) — branch `ArgvExecutor::run` on `msys::active_profile()`.

---

## Task 1: pure MSYS bridge helpers (`src/shellcore/msys.rs`)

**Files:**
- Modify: `src/shellcore/msys.rs` (add two functions + a test).

**Interfaces:**
- Consumes: existing `select_profile`, `WindowsShellProfile`, `ProfileSelection`, `PROFILE_ENV`.
- Produces:
  - `pub fn bridge_invocation(command: &str) -> (String, Vec<String>)`
  - `pub fn active_profile() -> WindowsShellProfile`

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` in `src/shellcore/msys.rs`:

```rust
    #[test]
    fn bridge_invocation_wraps_in_posix_host() {
        let (prog, args) = bridge_invocation("ls -al /c/Users");
        assert_eq!(prog, "sh");
        assert_eq!(args, vec!["-lc".to_string(), "ls -al /c/Users".to_string()]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib shellcore::msys 2>&1 | tail -20'`
Expected: FAIL — `cannot find function bridge_invocation`.

- [ ] **Step 3: Write the implementation**

In `src/shellcore/msys.rs`, add (after `is_msys_environment`, before the test module):

```rust
/// MSYS POSIX host 호출을 구성한다. `sh`가 PATH에서 POSIX tool을 찾고 POSIX path를
/// 해석하므로 ash는 path 변환/tool 스캔을 하지 않는다. (host는 `sh` 고정)
pub fn bridge_invocation(command: &str) -> (String, Vec<String>) {
    (
        "sh".to_string(),
        vec!["-lc".to_string(), command.to_string()],
    )
}

/// 현재 활성 Windows 셸 profile. env(`AI_TERMINAL_WINDOWS_PROFILE`/`MSYSTEM`/
/// `MSYSTEM_PREFIX`)를 읽어 `select_profile`로 판정한다. Selected가 아니면 native로
/// 안전 폴백(비-Windows/MSYS밖/미지 profile 포함).
pub fn active_profile() -> WindowsShellProfile {
    let profile_env = std::env::var(PROFILE_ENV).ok();
    let msystem = std::env::var("MSYSTEM").ok();
    let prefix = std::env::var("MSYSTEM_PREFIX").ok();
    match select_profile(profile_env.as_deref(), msystem.as_deref(), prefix.as_deref()) {
        ProfileSelection::Selected(profile) => profile,
        _ => WindowsShellProfile::NativeWindows,
    }
}
```

> Note: `active_profile` reads process env, so it is not unit-tested directly (env-dependent, flaky); its logic is covered by the existing `select_profile` tests. The bridge branch that calls it is verified by build + manual Windows run.

- [ ] **Step 4: Run test to verify it passes**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib shellcore::msys 2>&1 | tail -8'`
Expected: PASS — `bridge_invocation_wraps_in_posix_host` plus the existing `select_profile` tests green.

- [ ] **Step 5: fmt + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
git add src/shellcore/msys.rs
git commit -m "feat(msys): bridge_invocation + active_profile helpers"
```
(append `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`)

---

## Task 2: route gated execution through MSYS when active (`src/gated_runner.rs`)

**Files:**
- Modify: `src/gated_runner.rs` (`ArgvExecutor::run` branch + import).

**Interfaces:**
- Consumes: `msys::{active_profile, bridge_invocation}`, `msys::WindowsShellProfile` (Task 1); existing `spawn_inherit`.

> No new unit test: the MSYS branch only runs on Windows+MSYS (on WSL/CI `active_profile` returns native, so the existing native e2e exercises the unchanged path). The branch is verified by compile + clippy + the native e2e + manual Windows verification.

- [ ] **Step 1: Add the import**

In `src/gated_runner.rs`, add to the `use` block (near the existing `use crate::shellcore::external::...`):

```rust
use crate::shellcore::msys::{self, WindowsShellProfile};
```

- [ ] **Step 2: Branch `ArgvExecutor::run` on the active profile**

Replace the `impl Executor for ArgvExecutor<'_>` body in `src/gated_runner.rs`:

```rust
impl Executor for ArgvExecutor<'_> {
    fn run(&self, command: &str, _sink: &mut dyn OutputSink) -> Result<i32> {
        // MSYS profile이면 POSIX host(sh -lc)로, 아니면 native argv 직접 실행.
        // `command`는 pipeline이 넘긴 재구성된 명령 문자열이다.
        let code = match msys::active_profile() {
            WindowsShellProfile::MsysBridge => {
                let (prog, args) = msys::bridge_invocation(command);
                spawn_inherit(&prog, &args, self.cwd)?
            }
            WindowsShellProfile::NativeWindows => {
                spawn_inherit(self.name, &self.args, self.cwd)?
            }
        };
        Ok(code.unwrap_or(-1))
    }
}
```

- [ ] **Step 3: Build + clippy**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo build --bins >/tmp/b.log 2>&1; then echo BINS_OK; else echo BINS_FAIL; tail -20 /tmp/b.log; fi; if cargo clippy --lib --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1; then echo CLIPPY_CLEAN; else echo CLIPPY_FAIL; tail -20 /tmp/c.log; fi'`
Expected: `BINS_OK`, `CLIPPY_CLEAN`.

- [ ] **Step 4: Android boundary**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; if cargo check --lib --target aarch64-linux-android >/tmp/a.log 2>&1; then echo ANDROID_OK; else echo ANDROID_FAIL; tail -15 /tmp/a.log; fi'`
Expected: `ANDROID_OK`.

- [ ] **Step 5: e2e — native path unchanged (no MSYS on WSL)**

Write `/mnt/d/workspace/terminal-project/terminal/.git/sdd/s6_e2e.sh`:

```sh
source ~/.cargo/env
cd /mnt/d/workspace/terminal-project/terminal
export CARGO_TARGET_DIR=$HOME/targets/ai-terminal
ASH="$CARGO_TARGET_DIR/debug/ash"
cargo build --bin ash >/tmp/b.log 2>&1 && echo BINS_OK || { echo BINS_FAIL; tail -10 /tmp/b.log; }
printf 'echo hi\nexit\n' | "$ASH" >/tmp/s.out 2>/dev/null
if grep -q 'hi' /tmp/s.out; then echo SHELL_OK; else echo SHELL_MISSING; cat /tmp/s.out; fi
printf 'rm -rf /\nexit\n' | "$ASH" >/tmp/blk.out 2>/tmp/blk.err
if grep -qi '차단' /tmp/blk.out /tmp/blk.err; then echo BLOCKED_OK; else echo BLOCKED_MISSING; cat /tmp/blk.err; fi
```
Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/d/workspace/terminal-project/terminal/.git/sdd/s6_e2e.sh`
Expected: `BINS_OK`; `SHELL_OK` (`echo hi` runs the native path — no MSYSTEM on WSL so `active_profile` is native); `BLOCKED_OK` (S2 gate intact).

- [ ] **Step 6: Full verification gate**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; if cargo fmt --all -- --check >/dev/null 2>&1 && cargo clippy --all-targets --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1 && cargo test --features "storage tls remote" >/tmp/t.log 2>&1; then echo GATE_OK; else echo GATE_FAIL; tail -20 /tmp/c.log /tmp/t.log; fi'`
Expected: `GATE_OK`.

- [ ] **Step 7: Commit**

```bash
git add src/gated_runner.rs
git commit -m "feat(ash): run external commands via MSYS sh when the msys profile is active"
```
(append the Co-Authored-By line)

> **Manual verification (Windows + Git Bash/MSYS only):** install Git for Windows, open Git Bash, run `AI_TERMINAL_WINDOWS_PROFILE=msys ash.exe`, then `uname`, `grep foo somefile`, `ls -al /c/Users` — confirm POSIX tools run via `sh` and exit codes propagate. Without `AI_TERMINAL_WINDOWS_PROFILE` (native), confirm commands do NOT go through `sh`. Confirm `rm -rf /` is still blocked by the S2 gate.

---

## Self-Review

**Spec coverage:**
- §3 `bridge_invocation` + `active_profile` → Task 1. §4 `ArgvExecutor` profile branch → Task 2. §5 error handling (sh NotFound bail; native fallback) → covered by reusing `spawn_inherit` + `active_profile` native fallback. §6 tests → Task 1 (`bridge_invocation`) + Task 2 (build/native-e2e/manual). §2 boundary → Task 2 Step 4 android check. §7 acceptance → all + Task 2 Step 6. All covered. (Spec §4 described an `ArgvExecutor.command` field; the plan uses the existing `command` parameter instead — simpler, same behavior.)

**Placeholder scan:** No TBD/TODO; complete code + exact commands.

**Type consistency:** `bridge_invocation(&str) -> (String, Vec<String>)` and `active_profile() -> WindowsShellProfile` (Task 1) consumed by Task 2's branch. `WindowsShellProfile::{MsysBridge, NativeWindows}` match the existing enum. `spawn_inherit(name, args, cwd) -> Result<Option<i32>>` (from S2) reused with `.unwrap_or(-1)` as in the current code.

**Note for implementer:** do not add a struct field to `ArgvExecutor`; the `command: &str` parameter to `run` already carries the reconstructed command string that the MSYS branch needs. Keep the native arm byte-for-byte equivalent to today's behavior.
