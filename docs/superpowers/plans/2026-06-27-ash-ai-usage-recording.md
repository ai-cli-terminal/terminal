# ash AI Usage Recording Implementation Plan

> **For agentic workers:** implement only after `2026-06-27-windows-ash-manual-verification.md` is either complete or explicitly deferred by the controller. This is priority 2 from `docs/HANDOFF.md`.

**Goal:** make `ai ask`, `ai dispatch`, and `ash` natural-language AI paths record consistent usage events with correct provider/model, token counts, cache cost semantics, and fail-soft storage behavior.

**Spec:** `docs/superpowers/specs/2026-06-27-ash-ai-usage-recording-design.md`.

## Global Constraints

- **No shellcore dependency leak:** do not import `store`, `gateway`, `dispatch`, `config`, or `ai_usage` from `src/shellcore/*`.
- **Storage is optional:** default build remains C-free. All storage writes are behind `#[cfg(feature = "storage")]` and ignored on failure.
- **No schema migration:** reuse `usage_events`.
- **Only successful `Answered` responses are usage.** `Blocked`, `Unavailable`, timeout, cancellation, and backend errors do not create usage rows in this slice.
- **Budget parity:** `ash` AI path should inject the same `total_cost(None)` budget snapshot as `ai ask` when storage is available.
- **Build env (WSL only):**
  `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`

## File Structure

- `src/ai_usage.rs` (create) — shared summary + optional storage recorder.
- `src/lib.rs` (modify) — register `ai_usage`.
- `src/main.rs` (modify) — replace duplicated/hard-coded usage writes in `ai ask` and `run_dispatch`.
- `src/ai_router.rs` (modify) — store config `Ai`, inject budget, record usage on `Answered`.

## Task 1: Add Shared Usage Summary Helper

**Files:**
- Create: `src/ai_usage.rs`
- Modify: `src/lib.rs`

- [x] Write tests first:

```rust
#[test]
fn summarize_openai_backend_estimates_cost() { ... }

#[test]
fn summarize_ollama_backend_costs_zero() { ... }

#[test]
fn summarize_cache_hit_costs_zero_and_counts_cached_tokens() { ... }

#[test]
fn summarize_empty_model_uses_default() { ... }
```

- [x] Implement:

```rust
pub struct AiUsageSummary { ... }
pub fn summarize(ai: &config::Ai, source: CacheSource, input_tokens: usize, output_tokens: usize) -> AiUsageSummary
```

- [x] Add storage recorder:

```rust
#[cfg(feature = "storage")]
pub fn record(summary: &AiUsageSummary, session_id: Option<&str>) -> anyhow::Result<()> {
    let store = crate::store::Store::open_default()?;
    store.record_usage(
        &summary.provider,
        &summary.model,
        summary.input_tokens,
        summary.output_tokens,
        summary.cached_tokens,
        summary.cost_usd,
        session_id,
    )?;
    Ok(())
}
```

- [x] Register `pub mod ai_usage;` in `src/lib.rs`.

- [ ] Verify:

```bash
cargo test --lib ai_usage
cargo test --lib --features storage ai_usage
```

## Task 2: Fix `ai ask` Usage Provider/Model

**Files:**
- Modify: `src/main.rs`

- [x] Replace the existing local cost calculation in the `Command::Ask` success arm with `ai_usage::summarize(&cfg.config.ai, source, input_tokens, output_tokens)` or the local variable that already holds the loaded config.
- [x] Print the cost badge from `AiUsageSummary.cost_usd` and `estimated`.
- [x] Replace the hard-coded `record_usage("mock", "mock-model", ...)` with `ai_usage::record(&summary, None)`.
- [x] Keep storage errors ignored.

Verification:

```bash
cargo test --features "storage tls remote"
cargo test
```

Manual check after build:

```powershell
ash-or-ai-binary ask "hello" # with provider=mock/openai/ollama configs as available
ai usage
```

Expected: usage rows reflect configured provider/model. Ollama/cache rows cost zero.

## Task 3: Use Helper in `ai dispatch`

**Files:**
- Modify: `src/main.rs`

- [x] In `run_dispatch`, replace the direct `record_usage("mock", "mock-model", ...)` call with a mock `config::Ai` plus `ai_usage::summarize`.
- [x] Keep output text unchanged except cost/cached badge if the controller chooses to expose it. Minimal acceptable output remains existing token summary.

Verification:

```bash
cargo test --lib dispatch
cargo test --features "storage tls remote"
```

## Task 4: Record Usage from `ash` AI Router

**Files:**
- Modify: `src/ai_router.rs`

- [x] Add `ai: config::Ai` to `GatewayAiRouter`.
- [x] In `from_ai_config`, clone and store the config.
- [x] When constructing the gateway, mirror `ai ask` budget injection:

```rust
#[cfg(feature = "storage")]
let gw = match crate::store::Store::open_default() {
    Ok(store) => {
        let spent = store.total_cost(None).unwrap_or(0.0);
        gw.with_budget(spent, crate::usage::BudgetConfig::defaults())
    }
    Err(_) => gw,
};
```

- [x] In `try_handle`, on `Ok(AiOutcome::Answered { input_tokens, output_tokens, source, .. })`, summarize and record usage. Then print the trailing newline as before.
- [x] Do not record on `Blocked`, `Unavailable`, or `Err`.
- [x] Add a test that the router still handles AI/shell classification after the struct change. If storage isolation is easy in this repo, add a storage-feature test for usage row creation; otherwise rely on `ai_usage` storage unit coverage and manual verification.

Verification:

```bash
cargo test --lib ai_router
cargo test --features "storage tls remote"
```

## Task 5: Android Boundary and Full Gate

- [ ] Run android boundary:

```bash
cargo check --lib --target aarch64-linux-android
```

- [ ] Run full verification:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --all-targets --features "storage tls remote" -- -D warnings
cargo test --features "storage tls remote"
cargo test
```

## Task 6: Documentation Updates

Only after implementation and verification pass:

- [ ] Update `docs/TASK.md` to mark AI usage recording follow-up complete or add a checked PM follow-up line.
- [ ] Add a `docs/HISTORY.md` top entry with provider/model accounting, ash AI path coverage, budget parity, and test commands.
- [ ] Refresh `docs/HANDOFF.md` priority list so AI usage is no longer open.
- [ ] If user-facing behavior changes in `ai usage` output, update `README.md`.

## Implementation Note

Implementation was applied in this session, but verification is blocked in the current sandbox: Windows has no `cargo`/`rustfmt`, `ash.exe` is not present, and `wsl.exe` reports no installed Linux distribution. Leave verification and completion documentation unchecked until a Rust-capable environment runs the gates above.

## Acceptance Criteria

- `ai ask` no longer records all usage as `mock/mock-model`.
- `ash` AI route records usage for successful answers in storage builds.
- Storage failures never alter user-visible AI success/failure behavior.
- OpenAI backend calls use estimated cost; Ollama/mock/cache cost zero.
- Budget gate is active in `ash` AI path when storage is available.
- Full Rust and Android boundary gates are green.
