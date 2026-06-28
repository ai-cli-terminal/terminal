# Verification Resume Runbook — 2026-06-27

> Use this when a Rust-capable Windows or WSL environment is available. It closes the currently blocked gates from `2026-06-27-priority-execution.md` and determines whether AI usage can be marked complete.

## Scope

This runbook verifies the current pending change set:

- AI usage accounting implementation:
  - `src/ai_usage.rs`
  - `src/lib.rs`
  - `src/main.rs`
  - `src/ai_router.rs`
- Documentation/plans:
  - `docs/HANDOFF.md`
  - `docs/TASK.md`
  - `docs/superpowers/plans/2026-06-27-*`
  - `docs/superpowers/specs/2026-06-27-ash-ai-usage-recording-design.md`

It does not complete Windows/TTY manual verification; that remains in `2026-06-27-windows-ash-manual-verification.md`.

## Preconditions

- Rust toolchain with `cargo`, `rustfmt`, and `clippy`.
- For preferred repo verification: WSL Ubuntu with Rust installed.
- For Windows smoke: Windows Rust/MSVC environment or release artifact.
- Android Rust target installed for boundary check:

```bash
rustup target add aarch64-linux-android
```

## Step 1: Confirm Worktree

```powershell
git -c safe.directory=D:/workspace/terminal-project/terminal -C D:\workspace\terminal-project\terminal status --short --branch
git -c safe.directory=D:/workspace/terminal-project/terminal -C D:\workspace\terminal-project\terminal diff --check
```

Expected:

- No `diff --check` output.
- Known unrelated `android/.omc/` may remain untracked.

## Step 2: Rust Gates in WSL

Preferred command wrapper:

```powershell
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all -- --check'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets --features "storage tls remote" -- -D warnings'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo check --lib --target aarch64-linux-android'
```

If WSL is unavailable but Windows Rust is installed, run the same cargo commands directly in PowerShell from repo root, except the Android check may require Android linker/target setup.

## Step 3: Focused AI Usage Checks

Run these before the full suite if narrowing failures:

```bash
cargo test --lib ai_usage
cargo test --lib --features storage ai_usage
cargo test --lib ai_router
cargo test --lib dispatch
```

Expected:

- `ai_usage` unit tests pass.
- Storage-feature test writes one in-memory usage row and reads total cost.
- Existing `ai_router` routing tests still pass.

## Step 4: Manual CLI Smoke for Usage Accounting

After building a binary with `storage` enabled:

```powershell
cargo build --bins --features storage
.\target\debug\ai.exe ask "hello from usage smoke" --backend mock
.\target\debug\ai.exe usage
```

Expected:

- `ai ask` prints token/cost summary.
- `ai usage` shows a new usage event.
- The row should not be hard-coded to an unrelated provider/model for non-mock backends.

Optional provider-specific smoke:

```powershell
.\target\debug\ai.exe ask "hello" --backend ollama --model <installed-model>
$env:OPENAI_API_KEY="..."
.\target\debug\ai.exe ask "hello" --backend openai --model <model>
```

Expected:

- Ollama cost is zero.
- OpenAI backend calls show estimated cost.
- Failures are friendly and do not prevent later shell/CLI use.

## Step 5: Existing Windows Smoke

If Windows Rust is available:

```powershell
pwsh scripts/smoke.ps1
```

This covers Windows native build, ConPTY smoke, `ai.exe` basics, `ash.exe` core, `.cmd`, `.ps1`, and exit-code propagation. It does not replace real TTY manual verification.

## Step 6: Completion Updates After Green

Only after all required gates pass:

- [ ] In `docs/superpowers/plans/2026-06-27-ash-ai-usage-recording.md`, mark Task 5 verification items complete.
- [ ] In `docs/superpowers/plans/2026-06-27-priority-execution.md`, mark blocked AI usage Rust gates complete.
- [ ] In `docs/TASK.md`, change AI usage follow-up from `[~]` to `[x]`.
- [ ] Add a top entry to `docs/HISTORY.md` with exact commands and results.
- [ ] Refresh `docs/HANDOFF.md` so AI usage is no longer "검증 대기".

## If A Gate Fails

- Do not mark AI usage complete.
- Record the failing command, exit code, and first actionable compiler/test error.
- Fix code in the smallest scoped patch.
- Re-run the focused failing test first, then the full gate set.

## Current Blocker Snapshot

As of this sandbox:

- `cargo` is not installed on Windows PATH.
- `rustfmt` is not installed on Windows PATH.
- `wsl.exe` has no usable installed distro.
- `target/debug/ash.exe` is absent.
- `git diff --check` passes.
