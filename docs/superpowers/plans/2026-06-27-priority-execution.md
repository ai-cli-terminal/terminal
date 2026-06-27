# Priority Execution Plan — 2026-06-27

> This is the live execution order for the current handoff. It does not replace the detailed feature plans; it links them and records what is done, blocked, and next.

## Priority Order

1. **Close AI usage recording verification**
   Detailed plan: `docs/superpowers/plans/2026-06-27-ash-ai-usage-recording.md`
2. **Run Windows/TTY ash manual verification**
   Detailed plan: `docs/superpowers/plans/2026-06-27-windows-ash-manual-verification.md`
3. **After Windows verification, resume Android PM-3**
   First candidates: shared staging UX decision and imported file reader expansion in `docs/TASK.md` PM-3.

## P0 — Current Environment Check

- [x] Confirmed current sandbox cannot run Rust validation:
  - Windows `cargo` not found.
  - Windows `rustfmt` not found.
  - `target/debug/ash.exe` not present.
  - `wsl.exe` has no usable installed distro in this sandbox; `wsl --list --online` also fails due network restrictions.
- [x] Confirmed non-build validation available:
  - `git diff --check` passes.
  - Source and docs can be inspected.
- [x] Existing unrelated untracked path preserved:
  - `android/.omc/`

## P1 — AI Usage Recording

Detailed spec/plan:

- `docs/superpowers/specs/2026-06-27-ash-ai-usage-recording-design.md`
- `docs/superpowers/plans/2026-06-27-ash-ai-usage-recording.md`

Implementation status:

- [x] Added shared usage helper: `src/ai_usage.rs`.
- [x] Registered helper in `src/lib.rs`.
- [x] Replaced `ai ask` hard-coded `mock/mock-model` usage writes with config-derived provider/model.
- [x] Reused the helper in `ai dispatch`.
- [x] Added ash AI router usage recording for successful answers when `storage` is enabled.
- [x] Added ash AI budget snapshot injection when `storage` is enabled.
- [x] Kept storage failures fail-soft by ignoring recorder errors at call sites.
- [x] Kept `shellcore` clean of `store`/`config`/`ai_usage` dependencies.

Local static checks:

- [x] `git diff --check`
- [x] Manual scan for remaining direct/hard-coded usage writes in touched paths.
- [x] Manual source review completed: `docs/superpowers/plans/2026-06-27-ai-usage-source-review.md`.
- [x] Verification resume runbook prepared: `docs/superpowers/plans/2026-06-27-verification-resume-runbook.md`.
- [x] Verification-pending package prepared: `docs/superpowers/plans/2026-06-27-verification-pending-package.md`.
- [ ] `cargo fmt --all -- --check` — blocked: no Rust toolchain.
- [ ] `cargo clippy --all-targets --features "storage tls remote" -- -D warnings` — blocked: no Rust toolchain.
- [ ] `cargo test --features "storage tls remote"` — blocked: no Rust toolchain.
- [ ] `cargo test` — blocked: no Rust toolchain.
- [ ] `cargo check --lib --target aarch64-linux-android` — blocked: no Rust toolchain/WSL.

Before marking complete:

- [ ] Run all blocked Rust gates in a Rust-capable environment.
- [ ] If green, update `docs/TASK.md` AI usage item from `[~]` to `[x]`.
- [ ] Add a top entry to `docs/HISTORY.md` with exact command results.
- [ ] Refresh `docs/HANDOFF.md` so AI usage is no longer "검증 대기".

## P2 — Windows/TTY ash Manual Verification

Detailed plan:

- `docs/superpowers/plans/2026-06-27-windows-ash-manual-verification.md`

Current status:

- [ ] Not started in a real Windows TTY.
- [ ] Needs Windows-native `ash.exe` or release asset.
- [ ] Needs real terminal checks for reedline editing, history, AI fail-soft, safety gate, ConPTY, and Git Bash/MSYS bridge.

Before marking complete:

- [ ] Complete all acceptance criteria in the manual verification plan.
- [ ] Update `docs/TASK.md` PM-1 "Windows 완료 검증" to `[x]`.
- [ ] Add `docs/HISTORY.md` entry with evidence matrix.
- [ ] Refresh `docs/HANDOFF.md` priority list.

## P3 — Android PM-3 Resume Gate

Do not start new Android feature work until:

- [ ] AI usage implementation is verified or explicitly deferred.
- [ ] Windows/TTY ash verification is complete, or the controller explicitly overrides the gate.

First Android work after the gate:

- [ ] Shared staging UX decision: path input 유지 vs SAF-backed directory picker.
- [ ] Imported file UX expansion: read-only builtin or structured table reader.
- [ ] Distribution path decision: APK/F-Droid first, Play Store only after policy review.

## Next Action

Run `docs/superpowers/plans/2026-06-27-verification-resume-runbook.md` in a Rust-capable environment. If committing before that, use `docs/superpowers/plans/2026-06-27-verification-pending-package.md` and keep the commit/PR explicitly marked "verification pending"; do not start Android PM-3 unless the controller explicitly overrides the gate.
