# S1 — ash Config Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `ash`/`ai` load `~/.config/ai-terminal/config.toml` into a typed minimal `[general]` config (fail-soft), surfaced by `ai doctor`, with `shellcore` decoupled from the desktop config module.

**Architecture:** Config loading lives in the desktop host layer (`src/config.rs` + `src/bin/ash.rs`). `shellcore::repl::run` takes a plain `ReplSettings` struct and maps it onto `Engine` fields; `shellcore` never references `crate::config`, so the android cdylib build stays clean. `ai doctor` prints a config diagnostic via a pure formatter.

**Tech Stack:** Rust, `serde` (derive), `toml = "0.8"` (both already in `Cargo.toml`).

## Global Constraints

- **Spec:** `docs/superpowers/specs/2026-06-26-windows-ash-s1-config-loading-design.md`.
- **shellcore purity:** `src/shellcore/*` MUST NOT reference `crate::config` (desktop-only module gated `cfg(not(target_os = "android"))`). Verified by an android cdylib check.
- **fail-soft:** config absence/corruption never aborts `ash`/`ai`; default values + one stderr warning line.
- **Build env (WSL only):** Rust toolchain is in WSL. Run cargo via:
  `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
  Exit codes are only reliable through `cmd && echo OK || echo FAIL` control flow (this harness flattens `echo $?` to 0).
- **Verification gate (every task that compiles code):** `cargo fmt --all -- --check`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`, `cargo test --features "storage tls remote"` must be green.
- **Korean comments** match surrounding code style; run `cargo fmt --all` after editing chained/Korean-string code.

---

## File Structure

- `src/config.rs` (modify) — add `Config`/`General`/`ConfigSource`/`LoadedConfig`, `config_path()`, `load_from()`, `load()`. (existing `active_profile` API untouched)
- `src/main.rs` (modify) — add pure `format_config_diagnostics()`; print it inside `run_doctor` (`src/main.rs:1535`).
- `src/shellcore/engine.rs` (modify) — add `history_limit`/`default_shell` fields to `Engine`, init in `with_external_runner` (`src/shellcore/engine.rs:36`).
- `src/shellcore/repl.rs` (modify) — add `ReplSettings`, `apply_settings()`, change `run()` to `run(settings: ReplSettings)`.
- `src/bin/ash.rs` (modify) — load config → build `ReplSettings` → `run(settings)`.

---

## Task 1: Config loader (`src/config.rs`)

**Files:**
- Modify: `src/config.rs` (append types + functions; extend `#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: existing `config_dir() -> Result<PathBuf>` (`src/config.rs:11`).
- Produces:
  - `pub struct Config { pub general: General }`
  - `pub struct General { pub default_shell: Option<String>, pub history_limit: usize }`
  - `pub enum ConfigSource { Default, File(PathBuf) }`
  - `pub struct LoadedConfig { pub config: Config, pub source: ConfigSource, pub warning: Option<String> }`
  - `pub fn config_path() -> Result<PathBuf>`
  - `pub fn load_from(path: &Path) -> LoadedConfig`
  - `pub fn load() -> LoadedConfig`

- [ ] **Step 1: Write the failing tests**

Append to the `#[cfg(test)] mod tests` block in `src/config.rs`:

```rust
    #[test]
    fn load_missing_file_yields_defaults() {
        let p = tmp_file("cfg_missing.toml");
        let _ = std::fs::remove_file(&p);
        let loaded = load_from(&p);
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.source, ConfigSource::Default);
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.config.general.history_limit, 10_000);
    }

    #[test]
    fn load_valid_file_reads_values() {
        let p = tmp_file("cfg_valid.toml");
        std::fs::write(&p, "[general]\ndefault_shell = \"/bin/bash\"\nhistory_limit = 42\n").unwrap();
        let loaded = load_from(&p);
        assert_eq!(loaded.config.general.history_limit, 42);
        assert_eq!(loaded.config.general.default_shell.as_deref(), Some("/bin/bash"));
        assert_eq!(loaded.source, ConfigSource::File(p.clone()));
        assert!(loaded.warning.is_none());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn load_partial_file_fills_missing_with_defaults() {
        let p = tmp_file("cfg_partial.toml");
        std::fs::write(&p, "[general]\nhistory_limit = 5\n").unwrap();
        let loaded = load_from(&p);
        assert_eq!(loaded.config.general.history_limit, 5);
        assert_eq!(loaded.config.general.default_shell, None);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn load_corrupt_file_is_failsoft() {
        let p = tmp_file("cfg_corrupt.toml");
        std::fs::write(&p, "this is = = not valid toml [[[").unwrap();
        let loaded = load_from(&p);
        assert_eq!(loaded.config, Config::default());
        assert_eq!(loaded.source, ConfigSource::Default);
        assert!(loaded.warning.is_some());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn load_unknown_keys_are_ignored() {
        let p = tmp_file("cfg_unknown.toml");
        std::fs::write(&p, "[general]\nhistory_limit = 7\nfuture_field = true\n[unknown_section]\nx = 1\n").unwrap();
        let loaded = load_from(&p);
        assert_eq!(loaded.config.general.history_limit, 7);
        assert!(loaded.warning.is_none());
        let _ = std::fs::remove_file(&p);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib config:: 2>&1 | tail -20'`
Expected: FAIL — `cannot find type Config`/`function load_from not found`.

- [ ] **Step 3: Write the implementation**

Append to `src/config.rs` (after the existing functions, before `#[cfg(test)]`):

```rust
/// config.toml 경로(기본 위치): config_dir()/config.toml.
pub fn config_path() -> Result<PathBuf> {
    Ok(config_dir()?.join("config.toml"))
}

/// `[general]` 섹션(최소 스키마). 누락 필드는 기본값.
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct General {
    pub default_shell: Option<String>,
    pub history_limit: usize,
}

impl Default for General {
    fn default() -> Self {
        Self {
            default_shell: None,
            history_limit: 10_000,
        }
    }
}

/// 사용자 config.toml 타입 모델(최소). 후속 슬라이스가 섹션을 확장한다.
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
}

/// 로드된 config의 출처(진단용).
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSource {
    Default,
    File(PathBuf),
}

/// fail-soft 로드 결과.
#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub config: Config,
    pub source: ConfigSource,
    pub warning: Option<String>,
}

/// 주어진 경로에서 fail-soft 로드한다. 부재→기본값, 파싱오류→기본값+경고.
/// 에러를 전파하지 않는다(세션 비중단).
pub fn load_from(path: &Path) -> LoadedConfig {
    match std::fs::read_to_string(path) {
        Err(_) => LoadedConfig {
            config: Config::default(),
            source: ConfigSource::Default,
            warning: None,
        },
        Ok(text) => match toml::from_str::<Config>(&text) {
            Ok(config) => LoadedConfig {
                config,
                source: ConfigSource::File(path.to_path_buf()),
                warning: None,
            },
            Err(e) => LoadedConfig {
                config: Config::default(),
                source: ConfigSource::Default,
                warning: Some(format!("config.toml 파싱 실패({e}) — 기본값 사용")),
            },
        },
    }
}

/// 기본 위치에서 로드한다. 경로 해석 실패도 fail-soft.
pub fn load() -> LoadedConfig {
    match config_path() {
        Ok(p) => load_from(&p),
        Err(_) => LoadedConfig {
            config: Config::default(),
            source: ConfigSource::Default,
            warning: None,
        },
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib config:: 2>&1 | tail -20'`
Expected: PASS — all `config::` tests (including the 5 new) green.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat(config): add fail-soft config.toml loader (minimal [general])"
```

---

## Task 2: `ai doctor` config diagnostics (`src/main.rs`)

**Files:**
- Modify: `src/main.rs` — add `format_config_diagnostics()`; call it in `run_doctor` (`src/main.rs:1535`); add a unit test in `src/main.rs` tests.

**Interfaces:**
- Consumes: `config::{load, LoadedConfig, ConfigSource, Config, General}` from Task 1.
- Produces: `fn format_config_diagnostics(loaded: &config::LoadedConfig) -> String` (pure).

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `src/main.rs`:

```rust
    #[test]
    fn config_diagnostics_show_file_source_and_values() {
        let loaded = ai_terminal::config::LoadedConfig {
            config: ai_terminal::config::Config {
                general: ai_terminal::config::General {
                    default_shell: Some("/bin/bash".to_string()),
                    history_limit: 123,
                },
            },
            source: ai_terminal::config::ConfigSource::File(std::path::PathBuf::from("/cfg/config.toml")),
            warning: None,
        };
        let out = format_config_diagnostics(&loaded);
        assert!(out.contains("file: /cfg/config.toml"), "{out}");
        assert!(out.contains("general.history_limit = 123"), "{out}");
        assert!(out.contains("general.default_shell = /bin/bash"), "{out}");
    }

    #[test]
    fn config_diagnostics_show_default_and_warning() {
        let loaded = ai_terminal::config::LoadedConfig {
            config: ai_terminal::config::Config::default(),
            source: ai_terminal::config::ConfigSource::Default,
            warning: Some("boom".to_string()),
        };
        let out = format_config_diagnostics(&loaded);
        assert!(out.contains("default (no file)"), "{out}");
        assert!(out.contains("general.default_shell = <unset>"), "{out}");
        assert!(out.contains("warning: boom"), "{out}");
    }
```

> Note: if `src/main.rs` refers to the crate as `crate::config` rather than `ai_terminal::config`, adjust the test paths to match the file's existing import style. Check the top of `src/main.rs` for `use ai_terminal::...` vs `use crate::...`.

- [ ] **Step 2: Run test to verify it fails**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --bin ai config_diagnostics 2>&1 | tail -20'`
Expected: FAIL — `cannot find function format_config_diagnostics`.

- [ ] **Step 3: Write the implementation**

Add this free function to `src/main.rs` (near `run_doctor`). Use the crate path style that matches the file (`config::` if `use ai_terminal::config;` exists, else `ai_terminal::config::`):

```rust
/// `ai doctor`용 config 진단 텍스트(순수 포매터).
fn format_config_diagnostics(loaded: &ai_terminal::config::LoadedConfig) -> String {
    use std::fmt::Write as _;
    let source = match &loaded.source {
        ai_terminal::config::ConfigSource::File(p) => format!("file: {}", p.display()),
        ai_terminal::config::ConfigSource::Default => "default (no file)".to_string(),
    };
    let shell = loaded
        .config
        .general
        .default_shell
        .as_deref()
        .unwrap_or("<unset>");
    let mut out = String::new();
    let _ = writeln!(out, "config: {source}");
    let _ = writeln!(out, "  general.history_limit = {}", loaded.config.general.history_limit);
    let _ = write!(out, "  general.default_shell = {shell}");
    if let Some(w) = &loaded.warning {
        let _ = write!(out, "\n  warning: {w}");
    }
    out
}
```

Then, inside `run_doctor` (`src/main.rs:1535`), add a line where the other diagnostics are printed:

```rust
    println!("{}", format_config_diagnostics(&ai_terminal::config::load()));
```

- [ ] **Step 4: Run test to verify it passes**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --bin ai config_diagnostics 2>&1 | tail -20'`
Expected: PASS — both diagnostics tests green.

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat(doctor): show config path, source, and [general] values"
```

---

## Task 3: shellcore `ReplSettings` + `Engine` fields (`src/shellcore/engine.rs`, `src/shellcore/repl.rs`)

**Files:**
- Modify: `src/shellcore/engine.rs` — add two fields + init.
- Modify: `src/shellcore/repl.rs` — add `ReplSettings`, `apply_settings`, change `run` signature; extend tests.

**Interfaces:**
- Consumes: `Engine` (`src/shellcore/engine.rs`).
- Produces:
  - `Engine.history_limit: usize`, `Engine.default_shell: Option<String>` (public fields).
  - `pub struct ReplSettings { pub history_limit: usize, pub default_shell: Option<String> }` (derives `Debug, Clone, Default`).
  - `pub(crate) fn apply_settings(engine: &mut Engine, settings: &ReplSettings)`.
  - `pub fn run(settings: ReplSettings) -> anyhow::Result<()>` (signature change).

- [ ] **Step 1: Write the failing tests**

Add to `#[cfg(test)] mod tests` in `src/shellcore/repl.rs`:

```rust
    #[test]
    fn repl_settings_default_is_neutral() {
        let s = ReplSettings::default();
        assert_eq!(s.history_limit, 0);
        assert_eq!(s.default_shell, None);
    }

    #[test]
    fn apply_settings_maps_onto_engine() {
        let mut engine = crate::shellcore::engine::Engine::new();
        let settings = ReplSettings {
            history_limit: 99,
            default_shell: Some("/bin/zsh".to_string()),
        };
        apply_settings(&mut engine, &settings);
        assert_eq!(engine.history_limit, 99);
        assert_eq!(engine.default_shell.as_deref(), Some("/bin/zsh"));
    }
```

Add to `#[cfg(test)] mod tests` in `src/shellcore/engine.rs`:

```rust
    #[test]
    fn engine_settings_fields_default_neutral() {
        let e = Engine::new();
        assert_eq!(e.history_limit, 0);
        assert_eq!(e.default_shell, None);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib shellcore::repl shellcore::engine 2>&1 | tail -25'`
Expected: FAIL — `no field history_limit on Engine`, `cannot find type ReplSettings`.

- [ ] **Step 3a: Add fields to `Engine`**

In `src/shellcore/engine.rs`, add to the `Engine` struct (after `workspace_root`):

```rust
    pub history_limit: usize,
    pub default_shell: Option<String>,
```

And in `with_external_runner` (`src/shellcore/engine.rs:36`), add to the constructed `Self { ... }`:

```rust
            history_limit: 0,
            default_shell: None,
```

(`new()` and `pure()` delegate to `with_external_runner`, so no other constructor changes are needed.)

- [ ] **Step 3b: Add `ReplSettings` + `apply_settings`, change `run` signature**

In `src/shellcore/repl.rs`, add near the top (after imports):

```rust
/// REPL 호스트 설정(데스크톱 config에서 주입). shellcore는 `crate::config`를 모른다.
#[derive(Debug, Clone, Default)]
pub struct ReplSettings {
    pub history_limit: usize,
    pub default_shell: Option<String>,
}

/// 주입된 설정을 엔진 상태에 매핑한다(S4 history 등이 소비).
pub(crate) fn apply_settings(engine: &mut Engine, settings: &ReplSettings) {
    engine.history_limit = settings.history_limit;
    engine.default_shell = settings.default_shell.clone();
}
```

Change the `run` signature and apply settings right after the engine is built:

```rust
pub fn run(settings: ReplSettings) -> Result<()> {
    let mut engine = Engine::new();
    apply_settings(&mut engine, &settings);
    // ... existing loop unchanged ...
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib shellcore::repl shellcore::engine 2>&1 | tail -25'`
Expected: PASS — new repl/engine tests green. (`src/bin/ash.rs` will not compile yet — that's Task 4. Use `--lib` here so this task's tests run against the library.)

- [ ] **Step 5: Commit**

```bash
git add src/shellcore/engine.rs src/shellcore/repl.rs
git commit -m "feat(shellcore): inject ReplSettings into Engine (history_limit, default_shell)"
```

---

## Task 4: Wire `ash` to config + full verification (`src/bin/ash.rs`)

**Files:**
- Modify: `src/bin/ash.rs` — load config, build `ReplSettings`, call `run(settings)`.

**Interfaces:**
- Consumes: `ai_terminal::config::load()` (Task 1), `ai_terminal::shellcore::repl::{ReplSettings, run}` (Task 3).
- Produces: a `ash` binary that loads config and is fail-soft.

- [ ] **Step 1: Update `src/bin/ash.rs`**

Replace the body of `main` in `src/bin/ash.rs`:

```rust
//! `ash` — AI SHell(가칭). 독립 구조화 셸 REPL 진입점.

fn main() {
    let loaded = ai_terminal::config::load();
    if let Some(warning) = &loaded.warning {
        eprintln!("ash: {warning}");
    }
    let settings = ai_terminal::shellcore::repl::ReplSettings {
        history_limit: loaded.config.general.history_limit,
        default_shell: loaded.config.general.default_shell.clone(),
    };
    if let Err(e) = ai_terminal::shellcore::repl::run(settings) {
        eprintln!("ash: {e}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 2: Build both binaries**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build --bins 2>&1 | tail -5' && echo BUILD_OK || echo BUILD_FAIL`
Expected: `BUILD_OK` — both `ai` and `ash` compile.

- [ ] **Step 3: Verify android cdylib boundary (shellcore must not pull config)**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; rustup target add aarch64-linux-android >/dev/null 2>&1; cargo check --lib --target aarch64-linux-android 2>&1 | tail -8' && echo ANDROID_OK || echo ANDROID_FAIL`
Expected: `ANDROID_OK` — lib still compiles for android (shellcore did not start referencing `crate::config`).

- [ ] **Step 4: e2e — doctor shows file source; corrupt config is fail-soft**

Run:
```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; D=$(mktemp -d); mkdir -p "$D/ai-terminal"; printf "[general]\nhistory_limit = 314\n" > "$D/ai-terminal/config.toml"; XDG_CONFIG_HOME="$D" cargo run -q --bin ai -- doctor 2>&1 | grep -E "config:|history_limit"; printf "broken [[[\n" > "$D/ai-terminal/config.toml"; XDG_CONFIG_HOME="$D" cargo run -q --bin ai -- doctor 2>&1 | grep -E "warning|history_limit = 10000"; rm -rf "$D"'
```
Expected: first run prints `config: file: .../config.toml` and `general.history_limit = 314`; second run prints a `warning:` line and falls back to `general.history_limit = 10000`. Process exits 0 both times (fail-soft).

- [ ] **Step 5: Full verification gate**

Run: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all -- --check 2>&1 | tail -3 && cargo clippy --all-targets --features "storage tls remote" -- -D warnings 2>&1 | tail -5 && cargo test --features "storage tls remote" 2>&1 | tail -8' && echo GATE_OK || echo GATE_FAIL`
Expected: `GATE_OK` — fmt clean, clippy no warnings, all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/bin/ash.rs
git commit -m "feat(ash): load config.toml and inject ReplSettings at startup"
```

---

## Self-Review

**Spec coverage:**
- §3 data model → Task 1. §4 loading API → Task 1. §5 shellcore injection → Task 3 + Task 4. §6 doctor → Task 2. §7 fail-soft → Task 1 (corrupt test) + Task 4 e2e. §8 tests → Tasks 1–4. §9 acceptance → covered by Tasks 1–4 + Task 4 Step 5 gate. §2 boundary → Task 4 Step 3 android check. All covered.
- §8 mentions "repl이 crate::config 미참조" — enforced by Task 4 Step 3 (android check fails if shellcore pulls a desktop-gated module).

**Placeholder scan:** No TBD/TODO; all steps carry concrete code and exact commands.

**Type consistency:** `Config`/`General`/`ConfigSource`/`LoadedConfig`/`load_from`/`load` defined in Task 1 and consumed verbatim in Tasks 2 & 4. `ReplSettings { history_limit, default_shell }` and `apply_settings` defined in Task 3 and consumed in Task 4. `Engine.history_limit`/`Engine.default_shell` defined in Task 3a and used in 3b. Names consistent across tasks.

**Note for implementer:** Task 2's test/import path (`ai_terminal::config::` vs `crate::config::`) depends on how `src/main.rs` already references the lib crate — check the file's existing imports and match them; the binary crate root may use either form.
