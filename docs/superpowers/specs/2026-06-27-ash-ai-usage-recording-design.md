# ash AI Usage Recording Design

## 1. Problem

The handoff identifies AI usage recording as the next implementation priority after manual Windows verification. The codebase already has `usage_events` storage and `Store::record_usage`, but the current AI paths are inconsistent:

- `ai ask` records usage on success, but the stored provider/model are hard-coded as `mock`/`mock-model` even when config selects Ollama or OpenAI.
- `ai dispatch` records mock usage for its current mock responder path.
- `ash` AI routing (`GatewayAiRouter`) does not record usage at all and does not inject the existing budget snapshot into its gateway.

This creates an accounting gap: shell-command audit is recorded from `ash`, but AI usage from the `ash` natural-language path is invisible.

## 2. Goals

- Record successful AI usage from `ai ask` and `ash` AI routing when built with `storage`.
- Store the actual configured provider/model, not a hard-coded mock pair.
- Use one shared helper for token/cost/cached-token accounting so CLI and `ash` stay consistent.
- Preserve fail-soft behavior: storage open/write errors must never break the shell or AI response path.
- Preserve android boundary: no storage/gateway desktop dependency should enter `shellcore` or Android cdylib paths.
- Keep local/cache semantics explicit:
  - Ollama/local provider cost is `0.0`.
  - Exact/Semantic cache hits cost `0.0`.
  - Remote backend call with non-local provider uses `usage::estimate_cost` until provider-reported pricing exists.

## 3. Non-Goals

- Provider-reported token/cost parsing.
- Monthly rolling budget windows.
- Changing `usage_events` schema.
- Recording failed, blocked, cancelled, or unavailable AI attempts as usage events. Those may become audit/telemetry later, but not usage.
- Auto-executing AI suggestions.

## 4. Proposed Architecture

Add a small desktop-safe accounting helper, `src/ai_usage.rs`, registered behind `#[cfg(feature = "storage")]` if possible or with storage-only functions behind that cfg.

Core pure interface:

```rust
pub struct AiUsageSummary {
    pub provider: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    pub cost_usd: f64,
    pub estimated: bool,
}

pub fn summarize(
    ai: &crate::config::Ai,
    source: crate::cache::CacheSource,
    input_tokens: usize,
    output_tokens: usize,
) -> AiUsageSummary
```

Storage side:

```rust
#[cfg(feature = "storage")]
pub fn record(summary: &AiUsageSummary, session_id: Option<&str>) -> anyhow::Result<()>
```

Cost rules:

- `provider == "ollama"`: `cost_usd = 0.0`, `estimated = false`.
- `source != CacheSource::Backend`: `cost_usd = 0.0`, `estimated = false`, `cached_tokens = input_tokens`.
- `provider == "openai"` and `source == Backend`: use `usage::estimate_cost`, `estimated = true`.
- `provider == "mock"` or unknown provider: `cost_usd = 0.0`, `estimated = false` unless a future provider registry says otherwise.

`model` is `ai.model`. If empty, store `"default"` to avoid blank accounting rows.

## 5. Integration Points

### `ai ask`

Replace the duplicated cost calculation and hard-coded `record_usage("mock", "mock-model", ...)` with `ai_usage::summarize(&config.ai, source, input_tokens, output_tokens)` and `ai_usage::record`.

The displayed cost badge should come from the summary, so output and storage cannot drift.

### `ash` AI Router

Extend `GatewayAiRouter` to keep a copy of `config::Ai`.

```rust
pub struct GatewayAiRouter {
    responder: GatewayResponder,
    profile: PolicyProfile,
    ai: config::Ai,
}
```

When `AiOutcome::Answered` returns from `respond`, call the same helper and ignore storage errors.

Also mirror `ai ask` budget behavior in `GatewayAiRouter::from_ai_config`: when `storage` is enabled and `Store::open_default()` succeeds, call `gw.with_budget(store.total_cost(None).unwrap_or(0.0), usage::BudgetConfig::defaults())` before creating `GatewayResponder`.

### `ai dispatch`

The current `run_dispatch` path uses `GatewayResponder::mock()`. It may keep mock provider/model semantics, but it should use the helper with an explicit mock `config::Ai` so the accounting rule is shared.

## 6. Tests

Unit tests:

- `summarize_openai_backend_estimates_cost`
- `summarize_ollama_backend_costs_zero`
- `summarize_cache_hit_costs_zero_and_counts_cached_tokens`
- `summarize_empty_model_uses_default`

Storage tests under `storage`:

- `record_writes_provider_model_tokens_and_cost`

Integration-level tests:

- `GatewayAiRouter` with mock provider records usage on `try_handle("how ...?")` when storage feature is enabled. Use a temp `AI_TERMINAL_*` data/config override only if the repo already has one; otherwise keep this at helper/store unit level and avoid global home mutation.
- `ai ask` display and record use configured provider/model. If CLI env isolation is too invasive, cover the behavior through a factored function and leave CLI e2e manual.

Boundary tests:

- `cargo check --lib --target aarch64-linux-android` must remain green.
- `shellcore` must not import `ai_usage`, `store`, `gateway`, `dispatch`, or `config`.

## 7. Acceptance Criteria

- `ai ask` successful responses record provider/model from config.
- `ash` AI successful responses record usage when `storage` is enabled.
- Storage failures are swallowed and do not affect AI output or shell liveness.
- Cache/local paths record zero cost; OpenAI backend calls use estimated cost.
- Budget gate applies to `ash` AI routing the same way it applies to `ai ask`.
- Full gates pass: fmt, clippy with `"storage tls remote"`, tests with `"storage tls remote"`, default tests, Android lib check.
