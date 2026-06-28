# Windows ash Manual Verification Plan

> **Purpose:** turn the handoff's top priority ("interactive/Windows manual verification") into a repeatable evidence checklist. This plan is verification-only: do not change product code while running it. Any bug found becomes a new fix slice with its own spec/plan.

**Goal:** prove the v0.3.0 Windows-native `ash.exe` experience works in a real TTY where CI cannot exercise it: reedline editing, persisted history, natural-language AI routing, real Ollama/OpenAI fail-soft behavior, safety gate/audit, ConPTY smoke, and Git Bash/MSYS bridge execution.

**Current trigger:** `docs/HANDOFF.md` §5 priority 1. PM-1's final checkbox in `docs/TASK.md` can be marked complete only after this plan's acceptance criteria are captured.

## Partial Evidence Captured on 2026-06-27

This is not enough to mark PM-1 complete, because it was not run in Windows native TTY.

- Environment limitation: no `powershell.exe`, `cmd.exe`, `wsl.exe`, or Windows `ash.exe` was available in the current execution environment.
- TTY limitation: the provided PTY did not answer reedline cursor-position query `ESC[6n`; `ash` exited with `The cursor position could not be read within a normal duration`.
- Passed outside Windows native TTY: isolated Linux `ash` path with `XDG_CONFIG_HOME`/`XDG_DATA_HOME`, config fail-soft, mock AI routing, Ollama-not-running fail-soft, OpenAI no-key fail-soft, Critical gate block, High-risk non-interactive decline, and storage-backed `usage_events`/`commands`/`audit_events`.
- Passed repository gate: `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"`, `cargo test`, `cargo check --lib --target aarch64-linux-android`.

Still required in the next session: run Tasks 1-7 below in a real Windows Terminal/PowerShell and Git Bash/MSYS environment, then rerun or confirm Task 8 as needed.

## Global Constraints

- **No code changes during verification.** If a behavior fails, record the exact repro and stop that sub-check; create a follow-up fix plan instead of patching inline.
- **Use Windows native binaries for product checks.** WSL is allowed only for repository verification gates and log inspection.
- **Do not count builtins as external gate/audit evidence.** `echo`, `cd`, `where`, and other `shellcore::builtins` do not pass through `GatedRunner`; use external commands for gate/audit checks.
- **Capture evidence.** Keep a short local note with command, terminal used, expected/actual result, and pass/fail. Do not commit secrets, local API keys, or full home paths.
- **WSL cargo commands:**
  `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **Exit-code checks:** avoid `$?` through nested shell invocations; use `cmd && echo OK || echo FAIL` or direct PowerShell `$LASTEXITCODE` in the same shell.

## Task 1: Prepare Binary and Clean Test State

- [ ] Confirm branch and tag context:

```powershell
git -c safe.directory=D:/workspace/terminal-project/terminal -C D:\workspace\terminal-project\terminal branch --show-current
git -c safe.directory=D:/workspace/terminal-project/terminal -C D:\workspace\terminal-project\terminal describe --tags --always
```

- [ ] Build or locate the Windows-native `ash.exe`.

Preferred if a local Windows toolchain exists:

```powershell
cargo build --bin ash
.\target\debug\ash.exe --version
```

Fallback: use the v0.3.0 GitHub Release asset and record the exact asset name + SHA256 verification result.

- [ ] Create a temporary test config/data environment. Prefer a disposable user profile/config dir over the normal daily config. If the app does not support a config-dir override, record the actual config path and back up any existing test-impacting file before starting.

## Task 2: S3 Line Editor TTY Behavior

Run in Windows Terminal / PowerShell with a real TTY:

- [ ] Start `ash.exe`.
- [ ] Type a command, use left/right arrows and backspace in the middle, then submit. Expected: submitted line matches edited text.
- [ ] Press `Ctrl-C` on an empty prompt. Expected: prompt returns, session remains alive.
- [ ] Press `Ctrl-D` on an empty prompt. Expected: clean EOF/exit.
- [ ] Submit a non-ASCII line that is classified as AI or rejected safely. Expected: no rendering corruption or panic.
- [ ] Restart `ash.exe` and confirm prompt still renders normally.

Evidence to capture: terminal app/version, the edited command, visible result, and whether `Ctrl-C`/`Ctrl-D` matched expectations.

## Task 3: S4 History Persistence and Filtering

- [ ] In a real TTY, run two harmless external commands and one builtin.
- [ ] Use Up/Down arrows before exit. Expected: recent commands can be recalled and edited.
- [ ] Exit and restart `ash.exe`; use Up arrow. Expected: persisted history is loaded.
- [ ] Enter sensitive-looking commands such as `export OPENAI_API_KEY=...`, `password=...`, or a bearer-token shape. Expected: they are not persisted to `ash_history`.
- [ ] Corrupt the disposable `ash_history` file with invalid content and restart. Expected: ash starts with a warning or silent fallback, not a crash.

Evidence to capture: history file location, a redacted excerpt proving harmless commands persisted and sensitive commands did not.

## Task 4: S5 AI Routing and Real Provider Fail-Soft

Use config `[ai]` provider/model values explicitly for each case.

- [ ] `provider="mock"`: ask a natural-language question in `ash`. Expected: AI path handles it, shell commands such as `dir` or `where` still go to shell path.
- [ ] `provider="ollama"` with Ollama not running. Expected: a friendly "AI unavailable" style message and the shell remains usable.
- [ ] `provider="ollama"` with Ollama running and model available. Expected: response is printed, prompt returns, subsequent shell command works.
- [ ] `provider="openai"` without `OPENAI_API_KEY`. Expected: fail-soft unavailable/authorization error, shell remains usable.
- [ ] If using OpenAI with a key, verify HTTPS works only in a `tls`-enabled binary and no secret is printed or persisted.

Evidence to capture: provider/model, one redacted transcript per case, and whether shell continued after AI failure.

## Task 5: Safety Gate, Preview, Undo, and Audit

- [ ] Run a harmless external command. Expected: it executes through gated runner and, when storage is enabled, records `source="ash"` in command/audit storage.
- [ ] Run a High-risk external command that requires confirmation. Expected: interactive confirmation appears in TTY; decline path blocks execution and records non-Ran audit.
- [ ] Run a Critical command such as a known `risk::assess` critical fixture. Expected: blocked without execution.
- [ ] Run a file-modifying command in a disposable directory. Expected: preview/undo behavior matches existing S2/S7 contracts; no write outside the test directory.
- [ ] Confirm builtins are not misused as evidence: builtin `echo`/`cd` can work, but they should not be counted as gated external execution.

Evidence to capture: command text, risk/decision display if shown, storage query result with secrets redacted.

## Task 6: ConPTY Smoke

- [ ] Run the existing Windows ConPTY smoke path if present in tests/scripts, or manually run an interactive program under `ash.exe` that requires terminal round-trip.
- [ ] Confirm output, input, and exit code propagate.
- [ ] Confirm `Ctrl-C` does not wedge the parent shell.

Evidence to capture: command, visible marker round-trip, exit status.

## Task 7: Git Bash/MSYS Bridge

Run from Git Bash or MSYS2 shell.

- [ ] Native default: start `ash.exe` without `AI_TERMINAL_WINDOWS_PROFILE=msys`; run a POSIX-only command such as `uname`. Expected: it should not silently borrow MSYS behavior unless it is otherwise on native PATH by normal Windows rules.
- [ ] MSYS opt-in: run `AI_TERMINAL_WINDOWS_PROFILE=msys ash.exe`.
- [ ] In MSYS profile, run:

```sh
uname
ls -al /c/Users
printf 'foo\nbar\n' | grep foo
```

Expected: commands execute via `sh -lc`, POSIX paths/tools work, exit codes propagate.

- [ ] In MSYS profile, run a dangerous command fixture. Expected: S2 gate still blocks before `sh` execution.

Evidence to capture: shell (`Git Bash` or `MSYS2`), `MSYSTEM`, opt-in env, command output, exit-code behavior.

## Task 8: Repository Verification Gate

After manual checks, run the standard gates from WSL:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --features "storage tls remote" -- -D warnings
cargo test --features "storage tls remote"
cargo test
cargo check --lib --target aarch64-linux-android
```

Android app gate, only if this verification touched Android-facing docs or build files:

```powershell
cd terminal/android
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
.\gradlew :app:testDebugUnitTest
```

## Completion Updates

When all tasks pass:

- [ ] Update `docs/TASK.md` PM-1 "Windows 완료 검증" to `[x]`.
- [ ] Add a top entry to `docs/HISTORY.md` summarizing the manual verification matrix and exact date.
- [ ] Refresh `docs/HANDOFF.md` so priority 1 is no longer listed as open.
- [ ] If any product-facing support claim changed, update `README.md`.

## Acceptance Criteria

- Windows Terminal/PowerShell TTY checks pass for line editor, history, AI fail-soft, and safety gate.
- At least one real Ollama attempt is captured; if unavailable, the unavailable state is explicitly captured and fail-soft is proven.
- Git Bash/MSYS opt-in bridge is manually proven with POSIX path/tool commands.
- Critical command is blocked before execution in both native and MSYS profiles.
- Storage-enabled usage/audit checks are queried with redacted evidence.
- WSL Rust gates and Android `cargo check --lib --target aarch64-linux-android` are green.
