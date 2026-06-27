# AI Usage Source Review Plan — 2026-06-27

> Follow-up to `2026-06-27-priority-execution.md`. Rust gates are blocked in the current sandbox, so this document captures the next useful action: a manual source review of the AI usage implementation before a Rust-capable environment runs fmt/clippy/tests.

## Review Scope

Files in scope:

- `src/ai_usage.rs`
- `src/lib.rs`
- `src/main.rs`
- `src/ai_router.rs`
- `docs/TASK.md`
- `docs/HANDOFF.md`
- `docs/superpowers/plans/2026-06-27-ash-ai-usage-recording.md`
- `docs/superpowers/plans/2026-06-27-priority-execution.md`

Out of scope:

- New Android feature work.
- Windows/TTY manual verification.
- Schema migration.
- Provider-reported token/cost parsing.

## Checklist

- [x] No direct `Store::record_usage` calls remain outside the `ai_usage` helper and existing store tests.
- [x] `ai ask` records provider/model from the selected CLI/config values, not hard-coded `mock/mock-model`.
- [x] `ai dispatch` uses the same summary helper for mock responder accounting.
- [x] ash AI router records usage only for successful `Answered` outcomes.
- [x] `Blocked`, `Unavailable`, timeout/cancel/error paths do not create usage rows.
- [x] Storage writes remain feature-gated and fail-soft.
- [x] ash AI budget snapshot is injected when `storage` is enabled.
- [x] Default/non-storage build should not have obvious unused-field or unused-variable warnings in touched code.
- [x] `shellcore` has no new dependencies on `store`, `config`, `ai_usage`, `gateway`, or `dispatch`.
- [x] Cost semantics match the spec: OpenAI backend estimated, Ollama/mock/cache zero-cost, cache hit cached-token accounting.
- [x] Documentation accurately says implementation is done but Rust/TTY verification remains blocked.

## Review Commands

```powershell
rg -n "record_usage\(" src
rg -n "ai_usage::record|ai_usage::summarize|Store::open_default\(\)|BudgetConfig::defaults\(\)" src\main.rs src\ai_router.rs src\ai_usage.rs src\shellcore
rg -n "crate::(store|config|ai_usage|gateway|dispatch)|ai_terminal::(store|config|ai_usage|gateway|dispatch)" src\shellcore
git -c safe.directory=D:/workspace/terminal-project/terminal -C . diff --check
```

## Results

- [x] Review complete.
- [x] Findings fixed or documented.
- [x] Remaining blocked gates are listed in `2026-06-27-priority-execution.md`.

### Evidence

- `rg -n "record_usage\(" src` returns only:
  - `src/ai_usage.rs` helper call
  - `src/store.rs` API definition
  - existing `src/store.rs` tests
- `rg` over `src/shellcore` for `store|config|ai_usage|gateway|dispatch` returns only an existing comment in `repl.rs`; no dependency leak.
- `git diff --check` passes.
- `cargo`/`rustfmt`/WSL checks remain blocked by environment, not source-review findings.

## Completion Rule

This review can pass without Rust execution, but it cannot mark AI usage complete. Completion still requires a Rust-capable environment to run fmt, clippy, tests, and Android boundary check.
