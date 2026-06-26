# S1 — `ash` Config 로딩 설계

> **작성일**: 2026-06-26
> **유형**: 단일 슬라이스 구현 spec (Windows ash 완성 로드맵 S1).
> **상위**: `2026-06-26-windows-ash-completion-scoping-design.md` (#3 Config).
> **정본 스키마**: `../../../document/planning/10_환경_설정_템플릿.md §13`, 발췌는 `config.toml.example`.

## 1. 목표

`ash`(및 `ai`)가 사용자 config(`~/.config/ai-terminal/config.toml`)를 **타입화된 구조로 로드**하고, 이후 슬라이스가 읽을 단일 소스를 만든다. S1은 **최소 `[general]` 섹션만** 모델링하고, 손상/부재에 **fail-soft**로 동작한다.

YAGNI: `[ai]`·`[security]`·`[profiles]`는 각자 소비 슬라이스(S2·S5)에서 `Config`를 확장한다.

## 2. 경계 제약 (핵심)

`shellcore`는 android/pure 타깃에도 컴파일된다(`lib.rs`에서 `pub mod shellcore;` 무게이트). 따라서:

- **config 로딩은 데스크톱 호스트 계층에만 둔다**: `src/config.rs`(데스크톱 모듈, `cfg(not(target_os="android"))`)와 `src/bin/ash.rs`(데스크톱 bin).
- `shellcore::repl`은 `crate::config`를 **참조하지 않는다**. 대신 shellcore-로컬 plain 설정 struct를 주입받는다.
- 근거: 데스크톱 전용 의존/모듈이 android cdylib에 새지 않도록 한다(termios 회귀와 동형의 규율).

## 3. 데이터 모델

`src/config.rs`에 추가:

```rust
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct General {
    pub default_shell: Option<String>,
    pub history_limit: usize,
}

impl Default for General {
    fn default() -> Self {
        Self { default_shell: None, history_limit: 10_000 }
    }
}
// Config: derive(Default) (general: General::default())
```

- `#[serde(default)]`로 누락 필드는 기본값. 미지의 키는 **무시**(향후 슬라이스 호환 + 사용자 오타에 관대). `deny_unknown_fields` 사용 안 함.
- `history_limit = 10_000`은 `config.toml.example`과 일치.

## 4. 로딩 API

```rust
/// config.toml 경로(기본 위치). config_dir()/config.toml.
pub fn config_path() -> Result<PathBuf>;

/// 결과: 로드된 Config + 어디서 왔는지(진단용).
pub enum ConfigSource { Default, File(PathBuf) }
pub struct LoadedConfig { pub config: Config, pub source: ConfigSource, pub warning: Option<String> }

/// 기본 위치에서 fail-soft 로드. 부재→Default, 파싱오류→Default+warning.
pub fn load() -> LoadedConfig;

/// 경로 주입 가능한 순수 코어(테스트용).
pub fn load_from(path: &Path) -> LoadedConfig;
```

`load_from` 동작:
- 파일 없음 → `LoadedConfig{ Default, source: Default, warning: None }`.
- 읽기/`toml::from_str` 실패 → `Default` + `warning: Some("config.toml 파싱 실패(<err>) — 기본값 사용")`. **에러를 전파하지 않는다**(세션 비중단).
- 성공 → `config`, `source: File(path)`, `warning: None`.

## 5. shellcore 주입

`shellcore::repl`에 plain 설정 struct를 추가(데스크톱 config와 분리):

```rust
// src/shellcore/repl.rs
#[derive(Debug, Clone, Default)]
pub struct ReplSettings {
    pub history_limit: usize,   // 0 = 무제한/미사용 (S4에서 소비)
    pub default_shell: Option<String>,
}
pub fn run(settings: ReplSettings) -> Result<()>;   // 기존 run()을 시그니처 확장
```

- `repl::run`은 `ReplSettings`를 받아 `Engine`/REPL 상태에 보관한다. S1에서는 **보관만**(history_limit은 S4가, default_shell은 후속이 소비). shellcore는 `crate::config`를 모른다.
- `Default for ReplSettings`로 임베드/테스트는 빈 설정 사용 가능.

`src/bin/ash.rs`:
```rust
fn main() {
    let loaded = ai_terminal::config::load();
    if let Some(w) = &loaded.warning { eprintln!("ash: {w}"); }
    let settings = ai_terminal::shellcore::repl::ReplSettings {
        history_limit: loaded.config.general.history_limit,
        default_shell: loaded.config.general.default_shell.clone(),
    };
    if let Err(e) = ai_terminal::shellcore::repl::run(settings) { eprintln!("ash: {e}"); std::process::exit(1); }
}
```

## 6. 가시적 소비 — `ai doctor`

`ai doctor`(이미 존재)에 config 진단 줄 추가:
```
config: <path> (source=file|default)
  general.history_limit = 10000
  general.default_shell = <unset|/bin/bash>
```
- `source=default`면 경로는 "기본값(파일 없음)"으로 표기.
- 파싱 경고가 있으면 함께 표시.

## 7. 에러 처리 원칙

- **fail-soft 전면**: config 문제는 절대 `ash`/`ai`를 종료시키지 않는다. 기본값 + stderr 경고 1줄.
- 비밀값은 config.toml에 두지 않는 정책(문서/주석). 진단 출력 시 값은 그대로 표시하되, 향후 민감 필드 추가 시 `mask` 적용(S1 `[general]`엔 민감 필드 없음).

## 8. 테스트

단위(`src/config.rs`):
- `load_from` 부재 → Default, source=Default, warning=None.
- 유효 toml → 값 반영, source=File.
- 부분 toml(`history_limit`만) → 나머지 기본값.
- 손상 toml → Default + warning Some, **패닉/에러 전파 없음**.
- 미지 키 포함 toml → 무시하고 성공.

단위(`src/shellcore/repl.rs`):
- `ReplSettings` 주입이 보관되는지(가능하면 `run` 분해 또는 설정 보유 헬퍼로 검증). 순수성: `repl`이 `crate::config` 미참조(컴파일 경계).

e2e(WSL):
- config.toml 작성 후 `ai doctor`가 source=file + 값 표시.
- 손상 config로 `ai doctor`/`ash`가 종료 없이 경고+기본값.
- android cdylib 회귀 없음: `cargo check --lib --target aarch64-linux-android` green(shellcore가 config 미참조 확인).

## 9. 수용 기준

1. `~/.config/ai-terminal/config.toml`의 `[general]`이 타입화 로드된다.
2. 부재/손상에 fail-soft(기본값 + 경고, 비중단).
3. `ai doctor`가 config 경로·source·`[general]` 값을 표시한다.
4. `ash`가 `ReplSettings`를 주입받아 보관한다(shellcore는 `crate::config` 미참조 — android 빌드 불변).
5. `cargo fmt --all -- --check` · `cargo clippy --all-targets --features "storage tls remote" -D warnings` · `cargo test --features "storage tls remote"` green.

## 10. 비목표

`[ai]`/`[security]`/`[profiles]` 모델링·active_profile 통합 변경·history 동작(S4)·config 재작성/마이그레이션·환경변수 override(후속 필요 시).
