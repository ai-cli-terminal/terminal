# ash Real AI Provider Implementation Plan (S5 follow-on)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Model a minimal `[ai]` config and have `GatewayAiRouter` build a real ollama/openai backend from it (mock fallback), so ash answers natural-language queries with a real LLM instead of the echo mock.

**Architecture:** `src/config.rs` gains an `Ai` sub-struct (`provider`/`model`/`ollama_url`/`openai_url`, default provider=ollama) on `Config`. `GatewayAiRouter::from_ai_config(&Ai)` selects an ollama/openai/mock backend exactly like `ai ask`, and `from_environment` delegates to it; unreachable backends fall back to `AiOutcome::Unavailable` (fail-soft). Only `config.rs` + `ai_router.rs` change; `from_environment` keeps its signature so `ash` is untouched.

**Tech Stack:** Rust. Reuses `ollama`, `openai`, `gateway`, `provider`, `http::TcpTransport`, `responder::GatewayResponder`, `aitask::Timeouts`.

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-06-27-windows-ash-real-ai-provider-design.md`.
- **shellcore purity:** changes are in `src/config.rs` + `src/ai_router.rs` (both desktop). `shellcore`/android unchanged. Verified by android cdylib check.
- **Secrets:** the OpenAI key is read only from the `OPENAI_API_KEY` env var — never config.
- **openai over HTTPS needs the `tls` feature**; in the default build it fails at runtime → Unavailable (acceptable). Do NOT cfg-gate the provider selection — build all three arms; https failure is handled at runtime.
- **Build env (WSL only):** `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **Exit-code detection:** NEVER `cmd | tail && echo OK` (pipe masks exit). Use `if cmd >/tmp/log 2>&1; then echo PASS; else echo FAIL; tail /tmp/log; fi` or `set -o pipefail`.
- **fmt + clippy:** run actual `cargo fmt --all`, then verify `cargo clippy --all-targets --features "storage tls remote" -- -D warnings` exits 0 (re-run; don't trust).
- **Verification gate (each compiling task):** `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"` green.

---

## File Structure

- `src/config.rs` (modify) — add `Ai` struct + `ai` field on `Config`.
- `src/ai_router.rs` (modify) — `from_ai_config(&Ai)`; `from_environment` delegates; update tests.

---

## Task 1: `[ai]` config model (`src/config.rs`)

**Files:**
- Modify: `src/config.rs` (add `Ai` struct, add `ai` to `Config`, extend tests).

**Interfaces:**
- Produces: `pub struct Ai { pub provider: String, pub model: String, pub ollama_url: String, pub openai_url: String }` (default provider="ollama"); `Config { pub general: General, pub ai: Ai }`.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` in `src/config.rs`:

```rust
    #[test]
    fn ai_defaults_to_ollama() {
        let c = Config::default();
        assert_eq!(c.ai.provider, "ollama");
        assert_eq!(c.ai.ollama_url, "http://localhost:11434");
        assert_eq!(c.ai.model, "default");
    }

    #[test]
    fn ai_section_parses_and_fills_defaults() {
        let p = tmp_file("cfg_ai.toml");
        std::fs::write(&p, "[ai]\nprovider = \"mock\"\nmodel = \"m\"\n").unwrap();
        let loaded = load_from(&p);
        assert_eq!(loaded.config.ai.provider, "mock");
        assert_eq!(loaded.config.ai.model, "m");
        assert_eq!(loaded.config.ai.ollama_url, "http://localhost:11434"); // default kept
        let _ = std::fs::remove_file(&p);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib config:: 2>&1 | tail -20'`
Expected: FAIL — `no field ai on Config` / `cannot find type Ai`.

- [ ] **Step 3: Add the `Ai` struct and field**

In `src/config.rs`, add the `Ai` struct next to `General`:

```rust
/// `[ai]` 섹션. provider 어휘는 `ai ask --backend`와 일치한다.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct Ai {
    pub provider: String,
    pub model: String,
    pub ollama_url: String,
    pub openai_url: String,
}

impl Default for Ai {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "default".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            openai_url: "https://api.openai.com".to_string(),
        }
    }
}
```

Add the `ai` field to `Config`:

```rust
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub ai: Ai,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib config:: 2>&1 | tail -8'`
Expected: PASS — the two new `ai` tests plus existing config tests green.

- [ ] **Step 5: fmt + commit**

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
git add src/config.rs
git commit -m "feat(config): model [ai] provider/model/urls (default ollama)"
```
(append `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`)

---

## Task 2: Real gateway in `GatewayAiRouter` (`src/ai_router.rs`)

**Files:**
- Modify: `src/ai_router.rs` (`from_ai_config`, `from_environment`, imports, tests).

**Interfaces:**
- Consumes: `config::Ai` (Task 1); `provider::Provider`, `ollama::OllamaBackend`, `openai::OpenAiBackend`, `gateway::Gateway`, `http::TcpTransport`, `responder::GatewayResponder`, `aitask::Timeouts`.
- Produces: `pub fn GatewayAiRouter::from_ai_config(ai: &crate::config::Ai) -> anyhow::Result<Self>`; `from_environment` delegates to it.

- [ ] **Step 1: Update the tests (deterministic mock; ollama constructs)**

In `src/ai_router.rs`, replace the existing `routes_ai_queries_and_leaves_shell` test and add an ollama-construction test:

```rust
    fn mock_router() -> GatewayAiRouter {
        GatewayAiRouter::from_ai_config(&crate::config::Ai {
            provider: "mock".to_string(),
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn routes_ai_queries_and_leaves_shell() {
        let mut router = mock_router();
        assert!(router.try_handle("how do I undo a commit?")); // AiQuery → handled
        assert!(router.try_handle("ai explain last-error")); // AiInline → handled
        assert!(!router.try_handle("ls -al")); // Shell → not handled
    }

    #[test]
    fn ollama_config_constructs() {
        assert!(GatewayAiRouter::from_ai_config(&crate::config::Ai::default()).is_ok());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib ai_router 2>&1 | tail -20'`
Expected: FAIL — `no function from_ai_config`.

- [ ] **Step 3: Add `from_ai_config` + delegate `from_environment`**

In `src/ai_router.rs`, extend the imports (merge with existing `use crate::...` lines — keep `dispatch`, `pipeline::OutputSink`, `policy::PolicyProfile`, `responder::GatewayResponder`, `shellcore::repl::AiRouter`, `config`):

```rust
use crate::aitask::Timeouts;
use crate::gateway::Gateway;
use crate::http::TcpTransport;
use crate::ollama::OllamaBackend;
use crate::openai::OpenAiBackend;
use crate::provider::Provider;
```

Replace the `impl GatewayAiRouter { pub fn from_environment ... }` block with:

```rust
impl GatewayAiRouter {
    /// config의 [ai]에 따라 실 gateway(또는 mock)로 구성한다.
    pub fn from_ai_config(ai: &crate::config::Ai) -> anyhow::Result<Self> {
        let cap = Provider::mock().models[0].clone();
        let gw = match ai.provider.as_str() {
            "ollama" => {
                let b = OllamaBackend::new(TcpTransport, &ai.ollama_url, &ai.model);
                Gateway::new(Box::new(b), cap)
            }
            "openai" => {
                let api_key = std::env::var("OPENAI_API_KEY").ok();
                let b = OpenAiBackend::new(TcpTransport, &ai.openai_url, &ai.model, api_key);
                Gateway::new(Box::new(b), cap)
            }
            _ => Gateway::mock(),
        };
        let responder = GatewayResponder::new(gw, Timeouts::defaults().request)?;
        let profile = PolicyProfile::by_name(&config::get_active_profile())
            .unwrap_or_else(PolicyProfile::balanced);
        Ok(Self { responder, profile })
    }

    /// config 전체를 로드해 [ai]로 구성한다.
    pub fn from_environment() -> anyhow::Result<Self> {
        Self::from_ai_config(&config::load().config.ai)
    }
}
```

> Note: this mirrors `ai ask`'s backend wiring (`src/main.rs` `Command::Ask`). `OllamaBackend::new(transport, base_url, model)` and `OpenAiBackend::new(transport, base_url, model, Option<String>)` are the confirmed signatures. If `Gateway`/`GatewayResponder::new` need a `mut` binding or a different arg order, follow the compiler.

- [ ] **Step 4: Run tests + android boundary**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; set -o pipefail; cargo test --lib ai_router 2>&1 | tail -8; if cargo check --lib --target aarch64-linux-android >/tmp/a.log 2>&1; then echo ANDROID_OK; else echo ANDROID_FAIL; tail -15 /tmp/a.log; fi'`
Expected: `routes_ai_queries_and_leaves_shell` + `ollama_config_constructs` PASS; `ANDROID_OK`.

- [ ] **Step 5: e2e — mock-config NL echoes, shell + gate intact**

Write `/mnt/d/workspace/terminal-project/terminal/.git/sdd/aiprov_e2e.sh`:

```sh
source ~/.cargo/env
cd /mnt/d/workspace/terminal-project/terminal
export CARGO_TARGET_DIR=$HOME/targets/ai-terminal
ASH="$CARGO_TARGET_DIR/debug/ash"
cargo build --bin ash >/tmp/b.log 2>&1 && echo BINS_OK || { echo BINS_FAIL; tail -10 /tmp/b.log; }
D=$(mktemp -d); mkdir -p "$D/ai-terminal"; printf '[ai]\nprovider = "mock"\n' > "$D/ai-terminal/config.toml"
echo "-- NL query with mock provider (echo) --"
printf 'how do I list files?\nexit\n' | XDG_CONFIG_HOME="$D" "$ASH" >/tmp/ai.out 2>/dev/null
if grep -qi 'how do I list files' /tmp/ai.out; then echo AI_OK; else echo AI_MISSING; cat /tmp/ai.out; fi
echo "-- shell echo hi --"
printf 'echo hi\nexit\n' | XDG_CONFIG_HOME="$D" "$ASH" >/tmp/s.out 2>/dev/null
if grep -q 'hi' /tmp/s.out; then echo SHELL_OK; else echo SHELL_MISSING; cat /tmp/s.out; fi
echo "-- blocked rm -rf / --"
printf 'rm -rf /\nexit\n' | XDG_CONFIG_HOME="$D" "$ASH" >/tmp/blk.out 2>/tmp/blk.err
if grep -qi '차단' /tmp/blk.out /tmp/blk.err; then echo BLOCKED_OK; else echo BLOCKED_MISSING; cat /tmp/blk.err; fi
rm -rf "$D"
```
Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/d/workspace/terminal-project/terminal/.git/sdd/aiprov_e2e.sh`
Expected: `BINS_OK`; `AI_OK` (mock provider echoes the NL query); `SHELL_OK`; `BLOCKED_OK`.

- [ ] **Step 6: Full verification gate**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; if cargo fmt --all -- --check >/dev/null 2>&1 && cargo clippy --all-targets --features "storage tls remote" -- -D warnings >/tmp/c.log 2>&1 && cargo test --features "storage tls remote" >/tmp/t.log 2>&1; then echo GATE_OK; else echo GATE_FAIL; tail -20 /tmp/c.log /tmp/t.log; fi'`
Expected: `GATE_OK`.

- [ ] **Step 7: Commit**

```bash
git add src/ai_router.rs
git commit -m "feat(ai-router): build ollama/openai gateway from [ai] config"
```
(append the Co-Authored-By line)

> **Manual verification (needs a real LLM):** with ollama running locally and `[ai] provider = "ollama"`, `model = "<a pulled model>"`, run `ash` and ask `how do I undo a commit?` to confirm a real answer; without ollama running, confirm it prints `ash: AI 사용 불가: ...` (fail-soft) and the shell keeps working.

---

## Self-Review

**Spec coverage:**
- §3 `[ai]` config model → Task 1. §4 `from_ai_config`/`from_environment` → Task 2. §5 fail-soft (Unavailable, https-without-tls) → Task 2 (build all arms; runtime fallback) + existing GatewayResponder behavior. §6 behavior change (ollama default) → reflected in Task 2 e2e using a mock config for determinism + manual note for ollama. §7 tests → Tasks 1–2. §2 boundary → Task 2 Step 4. §8 acceptance → all + Task 2 Step 6. All covered.

**Placeholder scan:** No TBD/TODO; complete code + exact commands throughout.

**Type consistency:** `Ai { provider, model, ollama_url, openai_url }` (Task 1) consumed by `from_ai_config` (Task 2). `Config { general, ai }` (Task 1). `OllamaBackend::new`/`OpenAiBackend::new`/`Gateway::new`/`GatewayResponder::new`/`Provider::mock().models[0]`/`Timeouts::defaults().request` match the read-confirmed signatures and `ai ask`'s wiring.

**Note for implementer:** keep the provider selection un-gated (build ollama/openai/mock arms unconditionally); openai-over-HTTPS simply fails at runtime without the `tls` feature and is absorbed as `Unavailable`. Do not add new config fields beyond the four; timeouts/budget are a follow-on.
