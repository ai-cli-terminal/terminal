# Verification-Pending Package Plan — 2026-06-27

> Blocked work is excluded here. This packages the completed AI usage implementation so it can be committed or opened as a PR with an explicit verification-pending status.

## Why This Is Next

The following work is blocked in the current sandbox:

- Rust gates: no `cargo`/`rustfmt`.
- WSL gates: no usable installed distro.
- Windows/TTY ash verification: no `ash.exe` and no interactive TTY evidence.
- Android PM-3: gated behind AI usage verification and Windows completion.

The next non-blocked action is to prepare a clean commit/PR package that accurately states what changed and what still must be verified elsewhere.

## Included Changes

Code:

- `src/ai_usage.rs`
- `src/lib.rs`
- `src/main.rs`
- `src/ai_router.rs`

Docs/plans:

- `docs/HANDOFF.md`
- `docs/TASK.md`
- `docs/superpowers/specs/2026-06-27-ash-ai-usage-recording-design.md`
- `docs/superpowers/plans/2026-06-27-ash-ai-usage-recording.md`
- `docs/superpowers/plans/2026-06-27-windows-ash-manual-verification.md`
- `docs/superpowers/plans/2026-06-27-priority-execution.md`
- `docs/superpowers/plans/2026-06-27-ai-usage-source-review.md`
- `docs/superpowers/plans/2026-06-27-verification-resume-runbook.md`
- `docs/superpowers/plans/2026-06-27-verification-pending-package.md`

Excluded:

- `android/.omc/` existing untracked path.

## Pre-Package Checks

- [x] `git diff --check` passes.
- [x] Direct `record_usage` calls are centralized in `src/ai_usage.rs` except existing store tests.
- [x] `shellcore` has no new dependency leak.
- [x] Source review completed: `docs/superpowers/plans/2026-06-27-ai-usage-source-review.md`.
- [x] Resume runbook prepared: `docs/superpowers/plans/2026-06-27-verification-resume-runbook.md`.
- [ ] Rust fmt/clippy/test gates remain pending by environment.

## Suggested Commit

```text
feat(ai): record AI usage from ask, dispatch, and ash
```

Suggested body:

```text
Adds a shared ai_usage helper for provider/model/token/cache/cost accounting,
then wires it into ai ask, ai dispatch, and ash AI routing.

The storage writes remain feature-gated and fail-soft, cache/local paths record
zero cost, and OpenAI backend calls use estimated cost until provider-reported
pricing exists. ash also receives the same storage-backed budget snapshot as
ai ask.

Verification pending in this sandbox: cargo/rustfmt are unavailable and WSL has
no usable distro. Manual source review and git diff --check passed.
```

## Suggested PR Body

```markdown
## Summary

- Add `ai_usage` helper for normalized AI usage accounting.
- Record successful AI usage from `ai ask`, `ai dispatch`, and ash AI routing.
- Use configured provider/model instead of hard-coded `mock/mock-model` in `ai ask`.
- Keep storage writes feature-gated and fail-soft.
- Inject storage-backed budget snapshot into ash AI routing.
- Add handoff/runbook docs for verification-pending follow-up.

## Verification

- [x] `git diff --check`
- [x] Manual source review: `docs/superpowers/plans/2026-06-27-ai-usage-source-review.md`
- [ ] `cargo fmt --all -- --check` — pending: no Rust toolchain in sandbox
- [ ] `cargo clippy --all-targets --features "storage tls remote" -- -D warnings` — pending
- [ ] `cargo test --features "storage tls remote"` — pending
- [ ] `cargo test` — pending
- [ ] `cargo check --lib --target aarch64-linux-android` — pending

Runbook for pending verification:
`docs/superpowers/plans/2026-06-27-verification-resume-runbook.md`

## Risk

- Accounting-only change; no schema migration.
- Failed storage writes are intentionally ignored so AI responses remain fail-soft.
- AI usage is recorded only for successful `Answered` outcomes.
- Android/shellcore boundary is preserved.
```

## Next Action After Package

1. Commit/open PR with verification-pending language, or run the verification resume runbook in a Rust-capable environment.
2. If verification passes, mark AI usage complete in `TASK/HISTORY/HANDOFF`.
3. Then resume Windows/TTY ash manual verification.
