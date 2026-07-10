# Android AI 보조 이식 구현 계획 (openai backend + mock transport)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Android 터미널에 AI 보조를 이식한다 — 자연어 입력을 dispatch로 분류해 AI가 명령/설명을 **제안**(자동 실행 금지)하고, 셸 명령은 기존 shellcore 평가를 그대로 유지한다.

**Architecture:** AI 스택(intent·dispatch·gateway·openai·http 트레이트 + 의존 클로저 risk·policy·cache·mask·provider·tokenwin·usage·aitask)의 `cfg(not(target_os = "android"))`를 android 포함으로 넓히고(실 I/O `TcpTransport`·`post_json_tls`만 android 제외), 신설 `src/mobile_ai.rs`가 gateway를 tokio current-thread 런타임으로 구동한다. `mobile.rs`는 AI config가 주어졌을 때만 dispatch 분류로 Ai 경로를 제안으로 돌리고, JNI/C ABI/Kotlin은 3-인자 eval 표면(`nativeEvalLineAi`)을 추가한다. 실 HTTP transport(OkHttp JNI)·실기기 검증은 후속 슬라이스.

**Tech Stack:** Rust(tokio current-thread runtime, serde, 기존 AI 스택 재사용 — 신규 외부 crate 0), Kotlin/Compose(org.json), JNI.

**정본 spec:** `docs/superpowers/specs/2026-07-10-android-ai-assist-design.md` (PR #99)

## Global Constraints

- 브랜치: `feat/android-ai-assist` (base `origin/develop` = `b0fe6ba`), 워크트리 `D:\workspace\terminal-project\terminal-ai-wt` (WSL: `/mnt/d/workspace/terminal-project/terminal-ai-wt`). PR base는 **develop** (main 직접 금지 — 글로벌 브랜치 규칙).
- **§3-11 자동 실행 금지**: AI 결과는 제안 텍스트로만 반환·표시. 이 계획의 어떤 코드도 AI 출력을 셸 평가/실행으로 넘기지 않는다.
- **계층 경계**: `dispatch`/`gateway`/`openai` 참조는 `src/mobile.rs`·`src/mobile_ai.rs`에만 추가. `src/shellcore/*`는 단 한 줄도 수정 금지.
- **의존 추가 금지**: Cargo.toml 수정 금지(신규 crate 금지 — async 구동은 기존 tokio current-thread 재사용, `futures` 추가하지 않음). Kotlin도 신규 의존 금지.
- **android 실 I/O 제외**: `http.rs`의 `TcpTransport`·`build_request`·`extract_body`·`host_header`·`post_json_tls`는 `cfg(not(target_os = "android"))` 유지. `HttpTransport` 트레이트·`Scheme`·`parse_url`만 android 포함.
- 모든 Rust 커밋 전 `cargo fmt --all`(체크가 아니라 **실제 포맷**) 실행. clippy는 `-D warnings`.
- `git add -A` 금지 — 명시적 파일 경로로만 add. 커밋 메시지는 conventional(`feat:`/`refactor:`/`docs:`) + 한국어 본문.
- WSL 검증 규약([[terminal-build-env]]): 명령 판정은 파이프 금지, `cmd && echo PASS || echo FAIL` 또는 로그파일+exit 판정. `wsl.exe -- bash -lc '<한 줄>'` 형태(멀티라인 금지). cargo 앞에 `source ~/.cargo/env; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal` 필수.
- 각 Rust 태스크의 공통 검증 3종(아래 "공통 검증 명령" 참조): ① 무피처 `cargo test --all-targets` ② 무피처 `cargo clippy --all-targets -- -D warnings` ③ `cargo check --lib --target aarch64-linux-android`.
- 새 Rust 코드의 doc comment·에러 메시지는 기존 파일과 같이 한국어. Kotlin은 기존 스타일대로 주석 최소.

### 공통 검증 명령 (Rust 태스크 끝마다)

WSL에서 (한 줄씩):

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && echo FMT_DONE || echo FMT_FAIL'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets -- -D warnings >/tmp/clippy.log 2>&1 && echo CLIPPY_PASS || { echo CLIPPY_FAIL; tail -30 /tmp/clippy.log; }'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --all-targets >/tmp/test.log 2>&1 && echo TEST_PASS || { echo TEST_FAIL; tail -30 /tmp/test.log; }'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; rustup target add aarch64-linux-android >/dev/null 2>&1; cargo check --lib --target aarch64-linux-android >/tmp/android.log 2>&1 && echo ANDROID_PASS || { echo ANDROID_FAIL; tail -40 /tmp/android.log; }'
```

Kotlin 태스크 검증(git-bash, Windows):

```bash
cd /d/workspace/terminal-project/terminal-ai-wt/android && ANDROID_HOME=$HOME/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest
```

---

## 파일 구조 (전체 조감)

| 파일 | 역할 | 태스크 |
|---|---|---|
| `src/lib.rs` | 13개 모듈 cfg 확장 + `mobile_ai` 선언 | 1, 2 |
| `src/http.rs` | 실 I/O만 android 제외로 내부 분리 | 1 |
| `src/dispatch.rs` | pipeline 의존부만 android 제외로 내부 분리 | 1 |
| `src/mobile_ai.rs` (신설) | AI config·gateway 구동기(런타임 보유)·transport 주입 지점 | 2 |
| `src/mobile.rs` | `MobileEvalResult.ai` 필드 + AI 라우팅 eval + JSON 브리지 | 3 |
| `src/mobile_ffi.rs` | 3-인자 C ABI `ai_terminal_mobile_eval_line_ai_json` | 4 |
| `src/mobile_jni.rs` | JNI `nativeEvalLineAi` | 4 |
| `android/.../ShellBridge.kt` | `ShellAiConfig`·`AiSuggestion`·`evalLineAi`·codec | 5 |
| `android/.../ShellWorker.kt` | `aiConfig` 프로퍼티로 AI eval 경로 선택 | 5 |
| `android/.../ShellBridgeCodecTest.kt` (신설) | codec JVM 테스트 | 5 |
| `android/.../TerminalViewModel.kt` | `EntryKind.AiSuggestion`·`aiEnabled`/`toggleAi`·제안 렌더 | 6 |
| `android/.../MainActivity.kt` | AI 토글 버튼·제안 색/prefix·탭→입력 채움 | 6 |
| `android/.../TerminalViewModelAiTest.kt` (신설) | ViewModel AI 흐름 JVM 테스트 | 6 |
| `docs/TASK.md`, `docs/HANDOFF.md` | 완료 기록 | 7 |

설계 근거 두 가지(spec 대비 실측 확정):
- **spec §4-1의 "gateway `tokio::sync::Notify` 걸림돌"은 실측상 컴파일 장애물이 아닐 가능성이 높다.** tokio는 Cargo.toml **무조건 의존**이라 이미 android 타깃에서 컴파일되고 있다(2026-06-26 termios 사건의 교훈: 무조건 의존은 전부 컴파일됨 — 역으로 tokio가 문제였다면 현 android 빌드가 이미 깨졌어야 함). 따라서 Notify std 대체 없이 cfg만 넓히고 **Task 1의 `--target aarch64-linux-android` check로 확정**한다. check가 실패하면 그 에러가 곧 실측 — 서브에이전트는 즉흥 수정하지 말고 에러 전문을 보고하고 정지한다.
- **async 구동은 `futures::executor::block_on`(spec 검토안) 대신 tokio current-thread 런타임**(`responder.rs`의 기존 패턴)을 쓴다. 의존 0 추가로 spec §8-2(의존 최소)의 상위 충족. 단 데스크톱 `GatewayResponder`는 `tokio::signal::ctrl_c`·`context::gather`(둘 다 모바일 부적합/미컴파일) 때문에 재사용 불가 — mobile 전용 구동기를 `mobile_ai.rs`에 만든다. shellcore `AiRouter` 트레이트는 repl 전용 주입점이라 mobile(비-repl 진입)에서는 재사용하지 않고 `mobile.rs` 자체 분기를 쓴다(spec §4-2의 검토 항목에 대한 확정).

---

### Task 1: AI 스택 android-compat 확장 (컴파일 게이트)

**Files:**
- Modify: `src/lib.rs` (13개 `#[cfg(not(target_os = "android"))]` 제거 + 헤더 주석 갱신)
- Modify: `src/http.rs` (실 I/O만 android 제외)
- Modify: `src/dispatch.rs` (pipeline 의존부만 android 제외)

**Interfaces:**
- Consumes: 없음 (기존 코드만 재배치)
- Produces: android 타깃에서 컴파일되는 `crate::{intent, dispatch(Route·dispatch만), risk, policy, cache, mask, provider, tokenwin, usage, aitask, gateway, openai, http(HttpTransport·Scheme·parse_url만)}` — Task 2·3이 android에서 이 모듈들을 사용한다.

동작 변경 0 태스크(순수 cfg 재배치)이므로 신규 테스트 없음 — 기존 데스크톱 테스트 전체 + android check가 회귀 스위트다.

- [ ] **Step 1: lib.rs cfg 확장**

`src/lib.rs`에서 아래 **13개 모듈**의 `#[cfg(not(target_os = "android"))]` 속성 줄을 제거한다(모듈 선언 자체는 유지):

보안 코어 클러스터: `mask`, `policy`, `risk` (3개)
AI/게이트웨이 클러스터: `aitask`, `cache`, `dispatch`, `gateway`, `http`, `intent`, `openai`, `provider`, `tokenwin` (9개)
저장/사용량 클러스터: `usage` (1개)

**유지(제거 금지)**: `ai_router`, `ollama`, `pipeline`, `planner`, `responder`, `verify`, `verify_agent`, `cmdparse`, `context`, `explain`, `index`, 그 외 모든 데스크톱 모듈의 cfg는 그대로.

파일 헤더 doc(8행 부근)의 "android는 `shellcore`/`mobile*`만 컴파일한다"를 다음으로 교체:

```rust
//! (`cfg(not(target_os = "android"))`)이며 android는 `shellcore`/`mobile*`과 AI 보조
//! 스택(intent·dispatch 분류·gateway·openai + 순수 의존: risk·policy·cache·mask·
//! provider·tokenwin·usage·aitask, 실 I/O transport 제외)을 컴파일한다.
```

- [ ] **Step 2: http.rs — 실 I/O만 android 제외**

`src/http.rs`에서 android에 포함되는 것: `HttpTransport` 트레이트, `Scheme`, `parse_url`. android에서 제외할 것에 아래처럼 cfg를 붙인다:

```rust
#[cfg(not(target_os = "android"))]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(not(target_os = "android"))]
use tokio::net::TcpStream;
```

```rust
#[cfg(not(target_os = "android"))]
pub struct TcpTransport;

#[cfg(not(target_os = "android"))]
impl HttpTransport for TcpTransport {
```

```rust
#[cfg(not(target_os = "android"))]
fn build_request(host_header: &str, path: &str, body: &str, bearer: Option<&str>) -> String {
```

```rust
#[cfg(not(target_os = "android"))]
fn extract_body(resp: &str) -> Result<String> {
```

```rust
#[cfg(not(target_os = "android"))]
fn host_header(scheme: Scheme, host: &str, port: u16) -> String {
```

`post_json_tls` 두 variant는 기존 feature cfg에 android 조건을 합성한다:

```rust
#[cfg(all(feature = "tls", not(target_os = "android")))]
async fn post_json_tls(host: &str, port: u16, req: &str) -> Result<String> {
```

```rust
#[cfg(all(not(feature = "tls"), not(target_os = "android")))]
async fn post_json_tls(_host: &str, _port: u16, _req: &str) -> Result<String> {
```

모듈 doc(파일 상단)에 한 줄 추가: `//! android 타깃은 트레이트·URL 파서만 포함한다(실 I/O transport는 데스크톱 전용 — 모바일 실 전송은 후속 OkHttp JNI).`

`mod tests`는 수정하지 않는다(`cfg(test)`는 android `check --lib`/`build --lib`에서 컴파일되지 않음).

- [ ] **Step 3: dispatch.rs — pipeline 의존부만 android 제외**

`src/dispatch.rs`에서 android에 포함되는 것: `Route`, `dispatch()`, `AiOutcome`(pipeline 무관 — `cache::CacheSource`만 참조). 아래에 cfg를 붙인다:

```rust
#[cfg(not(target_os = "android"))]
use crate::pipeline::{self, ExecConfig, ExecOutcome, OutputSink};
```

```rust
#[cfg(not(target_os = "android"))]
pub trait AiResponder {
```

```rust
#[cfg(not(target_os = "android"))]
#[derive(Debug, PartialEq)]
pub enum Handled {
```

```rust
#[cfg(not(target_os = "android"))]
pub struct Handlers<'a> {
```

```rust
#[cfg(not(target_os = "android"))]
pub fn run(
```

(주의: `Handled`의 기존 `#[derive(Debug, PartialEq)]`는 유지하고 그 위에 cfg만 추가한다. `mod tests`는 수정하지 않는다.)

- [ ] **Step 4: android check — 이 계획의 핵심 게이트**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; rustup target add aarch64-linux-android >/dev/null 2>&1; cargo check --lib --target aarch64-linux-android >/tmp/android.log 2>&1 && echo ANDROID_PASS || { echo ANDROID_FAIL; tail -60 /tmp/android.log; }'
```

Expected: `ANDROID_PASS`.
**FAIL이면(예: tokio 관련 unresolved/미지원)**: 수정을 시도하지 말고 `/tmp/android.log`의 에러 전문을 보고서에 담아 **NEEDS_CONTEXT로 정지**한다 — cfg 확장 범위 재설계는 컨트롤러 결정 사항.

- [ ] **Step 5: 데스크톱 회귀 확인**

공통 검증 명령의 fmt → clippy → test를 순서대로 실행.
Expected: `FMT_DONE`, `CLIPPY_PASS`, `TEST_PASS` (기존 테스트 수 그대로, 0 fail — cfg 재배치는 데스크톱 컴파일 산출물 불변).

- [ ] **Step 6: Commit**

```bash
git -C /d/workspace/terminal-project/terminal-ai-wt add src/lib.rs src/http.rs src/dispatch.rs
git -C /d/workspace/terminal-project/terminal-ai-wt commit -m "refactor(android): AI 스택을 android 타깃에 포함 (실 I/O transport 제외)

intent·dispatch(분류)·gateway·openai·http(트레이트)와 순수 의존
(risk·policy·cache·mask·provider·tokenwin·usage·aitask)의 cfg(not android)를
제거한다. tokio는 이미 무조건 의존이라 android에서 컴파일된다(aarch64 check로
확정). http 실 I/O(TcpTransport·post_json_tls)와 dispatch의 pipeline 실행
경로(run/Handlers/AiResponder/Handled)는 android 제외를 유지한다."
```

---

### Task 2: `mobile_ai.rs` — 모바일 AI 구동기

**Files:**
- Create: `src/mobile_ai.rs`
- Modify: `src/lib.rs` (모바일 클러스터에 `pub mod mobile_ai;` 1줄)

**Interfaces:**
- Consumes: Task 1이 android에 포함시킨 `crate::aitask::{RequestError, Timeouts}`, `crate::gateway::{Gateway, GatewayOutcome}`, `crate::http::HttpTransport`, `crate::openai::OpenAiBackend`, `crate::provider::Provider`.
- Produces (Task 3·후속 transport 슬라이스가 사용):
  - `pub struct MobileAiConfig { pub provider: String, pub model: String, pub openai_url: String, pub api_key: Option<String> }` — serde Deserialize, `#[serde(default)]`, 기본 provider `"mock"`.
  - `pub struct MobileAiOutcome { pub kind: String, pub text: String }` — serde Serialize/Deserialize. kind ∈ `"answered" | "blocked" | "unavailable"`.
  - `pub fn build_gateway<T: HttpTransport + 'static>(cfg: &MobileAiConfig, transport: T) -> Gateway`
  - `pub struct MobileAi` + `MobileAi::from_config(cfg: &MobileAiConfig) -> anyhow::Result<MobileAi>` + `MobileAi::with_gateway(gateway: Gateway) -> anyhow::Result<MobileAi>` + `MobileAi::suggest(&self, prompt: &str, cwd: &str) -> MobileAiOutcome`

- [ ] **Step 1: 실패하는 테스트와 함께 파일 뼈대 작성**

`src/mobile_ai.rs` 전체를 다음 내용으로 생성:

```rust
//! 모바일 AI 보조 구동기 (spec `2026-07-10-android-ai-assist-design.md`).
//!
//! AI 스택(dispatch 분류 → gateway → openai backend)을 모바일 경계에서 동기
//! 구동한다. async는 데스크톱 `responder.rs`와 같은 tokio current-thread
//! 런타임을 재사용한다(신규 의존 없음). 실 HTTP transport는 후속 슬라이스
//! (OkHttp JNI)이며, 이번 슬라이스의 openai provider는 [`UnavailableTransport`]
//! 로 결선되어 정직하게 Unavailable을 돌려준다(가짜 성공 금지). mock provider
//! 는 echo 게이트웨이로 동작해 라우팅·마스킹 경로를 실기기 없이 검증한다.
//!
//! **§3-11**: 이 모듈은 제안 텍스트만 만든다 — 어떤 경로도 명령을 실행하지 않는다.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::Notify;

use crate::aitask::Timeouts;
use crate::gateway::{Gateway, GatewayOutcome};
use crate::http::HttpTransport;
use crate::openai::OpenAiBackend;
use crate::provider::Provider;

/// 모바일 AI 설정. JNI/C ABI 경계로 JSON 전달된다(필드 누락은 기본값).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct MobileAiConfig {
    /// "mock"(echo) 또는 "openai". 그 외 값은 mock으로 취급.
    pub provider: String,
    pub model: String,
    pub openai_url: String,
    /// 비밀은 호스트 앱이 보관하고 호출 시에만 전달한다(저장하지 않음).
    pub api_key: Option<String>,
}

impl Default for MobileAiConfig {
    fn default() -> Self {
        Self {
            provider: "mock".to_string(),
            model: "default".to_string(),
            openai_url: "https://api.openai.com".to_string(),
            api_key: None,
        }
    }
}

/// AI 제안 결과(JSON 직렬화 계약). kind: "answered" | "blocked" | "unavailable".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MobileAiOutcome {
    pub kind: String,
    pub text: String,
}

/// 실 transport 미결선 자리(후속 슬라이스: OkHttp JNI). openai 선택 시
/// 조용한 가짜 성공 대신 명확한 실패를 돌려준다(fail-soft, §3-3).
pub struct UnavailableTransport;

impl HttpTransport for UnavailableTransport {
    async fn post_json(
        &self,
        _url: &str,
        _body: &str,
        _bearer: Option<&str>,
    ) -> anyhow::Result<String> {
        Err(anyhow::anyhow!(
            "android HTTP transport 미결선 (후속 슬라이스: OkHttp JNI)"
        ))
    }
}

/// config와 주입 transport로 게이트웨이를 구성한다.
/// 후속 슬라이스는 실 transport만 바꿔 꽂으면 된다(교체 지점).
pub fn build_gateway<T: HttpTransport + 'static>(cfg: &MobileAiConfig, transport: T) -> Gateway {
    let cap = Provider::mock().models[0].clone();
    match cfg.provider.as_str() {
        "openai" => Gateway::new(
            Box::new(OpenAiBackend::new(
                transport,
                &cfg.openai_url,
                &cfg.model,
                cfg.api_key.clone(),
            )),
            cap,
        ),
        _ => Gateway::mock(),
    }
}

/// 모바일 AI 구동기 — gateway + current-thread 런타임 보유.
pub struct MobileAi {
    gateway: Gateway,
    runtime: tokio::runtime::Runtime,
    timeout: Duration,
}

impl MobileAi {
    /// config로 구성한다(이번 슬라이스의 openai는 [`UnavailableTransport`] 결선).
    pub fn from_config(cfg: &MobileAiConfig) -> anyhow::Result<MobileAi> {
        Self::with_gateway(build_gateway(cfg, UnavailableTransport))
    }

    /// 주어진 게이트웨이로 구성한다(테스트·후속 transport 주입 지점).
    pub fn with_gateway(gateway: Gateway) -> anyhow::Result<MobileAi> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()?;
        Ok(MobileAi {
            gateway,
            runtime,
            timeout: Timeouts::defaults().request,
        })
    }

    /// 자연어 prompt에 대한 제안을 만든다(§3-11: 실행하지 않는다).
    /// 실패·타임아웃은 "unavailable"로 흡수해 셸 사용을 막지 않는다(§3-3).
    pub fn suggest(&self, prompt: &str, cwd: &str) -> MobileAiOutcome {
        let ctx = format!("cwd={cwd}");
        let timeout = self.timeout;
        let gw = &self.gateway;
        let result = self.runtime.block_on(async {
            let cancel = Arc::new(Notify::new());
            gw.ask_cancellable(prompt, &ctx, timeout, cancel).await
        });
        match result {
            Ok(GatewayOutcome::Answered { text, .. }) => MobileAiOutcome {
                kind: "answered".to_string(),
                text,
            },
            Ok(GatewayOutcome::Blocked(reason)) => MobileAiOutcome {
                kind: "blocked".to_string(),
                text: reason,
            },
            Err(err) => MobileAiOutcome {
                kind: "unavailable".to_string(),
                text: err.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_json_parses_with_defaults() {
        let cfg: MobileAiConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(cfg.provider, "mock");
        assert_eq!(cfg.model, "default");
        assert_eq!(cfg.openai_url, "https://api.openai.com");
        assert_eq!(cfg.api_key, None);

        let cfg: MobileAiConfig =
            serde_json::from_str(r#"{"provider":"openai","api_key":"sk-1"}"#).unwrap();
        assert_eq!(cfg.provider, "openai");
        assert_eq!(cfg.api_key.as_deref(), Some("sk-1"));
    }

    #[test]
    fn mock_provider_suggests_via_echo() {
        let ai = MobileAi::from_config(&MobileAiConfig::default()).unwrap();
        let out = ai.suggest("큰 파일 찾아줘", "/app");
        assert_eq!(out.kind, "answered");
        assert!(out.text.contains("큰 파일 찾아줘"), "{out:?}");
        assert!(out.text.contains("cwd=/app"), "{out:?}");
    }

    #[test]
    fn openai_provider_with_mock_transport_answers() {
        struct CannedTransport;
        impl HttpTransport for CannedTransport {
            async fn post_json(
                &self,
                url: &str,
                _body: &str,
                bearer: Option<&str>,
            ) -> anyhow::Result<String> {
                assert!(url.ends_with("/v1/chat/completions"), "{url}");
                assert_eq!(bearer, Some("sk-test"));
                Ok(r#"{"choices":[{"message":{"content":"ls -al"}}]}"#.to_string())
            }
        }
        let cfg = MobileAiConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-test".to_string()),
            ..Default::default()
        };
        let ai = MobileAi::with_gateway(build_gateway(&cfg, CannedTransport)).unwrap();
        let out = ai.suggest("list files", "/app");
        assert_eq!(out.kind, "answered");
        assert_eq!(out.text, "ls -al");
    }

    #[test]
    fn openai_without_real_transport_is_unavailable() {
        let cfg = MobileAiConfig {
            provider: "openai".to_string(),
            ..Default::default()
        };
        let ai = MobileAi::from_config(&cfg).unwrap();
        let out = ai.suggest("list files", "/app");
        assert_eq!(out.kind, "unavailable");
        assert!(out.text.contains("transport"), "{out:?}");
    }

    #[test]
    fn private_key_prompt_is_blocked_by_masking() {
        let ai = MobileAi::from_config(&MobileAiConfig::default()).unwrap();
        let out = ai.suggest("-----BEGIN OPENSSH PRIVATE KEY-----", "/app");
        assert_eq!(out.kind, "blocked");
    }

    #[test]
    fn unknown_provider_falls_back_to_mock() {
        let cfg = MobileAiConfig {
            provider: "ollama".to_string(),
            ..Default::default()
        };
        let ai = MobileAi::from_config(&cfg).unwrap();
        let out = ai.suggest("what is this?", "/app");
        assert_eq!(out.kind, "answered", "{out:?}");
    }
}
```

`src/lib.rs`의 모바일 클러스터를 다음으로 변경:

```rust
// === 모바일 (Android/iOS 공통 bridge + AI 보조 + Android JNI) ===
pub mod mobile;
pub mod mobile_ai;
pub mod mobile_ffi;
#[cfg(target_os = "android")]
pub mod mobile_jni;
```

- [ ] **Step 2: 테스트 실행 — 통과 확인**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib mobile_ai >/tmp/t.log 2>&1 && { echo TEST_PASS; grep "test result" /tmp/t.log; } || { echo TEST_FAIL; tail -40 /tmp/t.log; }'
```

Expected: `TEST_PASS`, `6 passed; 0 failed` (mobile_ai 필터 기준).

- [ ] **Step 3: 공통 검증 (fmt → clippy → test 전체 → android check)**

공통 검증 명령 4종 실행. Expected: 전부 PASS. (android check에 신설 모듈 포함 — `UnavailableTransport`·`MobileAi`가 aarch64에서 컴파일되는지 확인하는 지점.)

- [ ] **Step 4: Commit**

```bash
git -C /d/workspace/terminal-project/terminal-ai-wt add src/mobile_ai.rs src/lib.rs
git -C /d/workspace/terminal-project/terminal-ai-wt commit -m "feat(mobile): mobile_ai — 모바일 AI 게이트웨이 구동기

MobileAiConfig(JSON 계약)·MobileAiOutcome(answered/blocked/unavailable)·
build_gateway(transport 주입 지점)·MobileAi(tokio current-thread 구동).
mock=echo, openai=UnavailableTransport(실 transport는 후속 OkHttp JNI —
가짜 성공 대신 정직한 unavailable). §3-11: 제안만 만들고 실행하지 않는다."
```

---

### Task 3: `mobile.rs` AI 라우팅 결선 + JSON 계약 확장

**Files:**
- Modify: `src/mobile.rs`

**Interfaces:**
- Consumes: Task 2의 `crate::mobile_ai::{MobileAi, MobileAiConfig, MobileAiOutcome}`; Task 1의 `crate::dispatch::{self, Route}`, `crate::policy::PolicyProfile`.
- Produces (Task 4·5가 사용):
  - `MobileEvalResult`에 `pub ai: Option<MobileAiOutcome>` 필드(직렬화 시 None이면 필드 생략 — 기존 Kotlin/JSON 계약 하위호환).
  - `MobileShell::eval_line_with_ai(&mut self, input: &str, ai: &MobileAi) -> MobileEvalResult`
  - `pub fn eval_line_ai_json(input: &str, state_json: &str, ai_config_json: &str) -> String`

- [ ] **Step 1: 실패하는 테스트 작성**

`src/mobile.rs`의 `mod tests` 끝에 추가:

```rust
    #[test]
    fn mobile_ai_routes_natural_language_to_suggestion() {
        use crate::mobile_ai::{MobileAi, MobileAiConfig};
        let mut shell = MobileShell::new();
        let ai = MobileAi::from_config(&MobileAiConfig::default()).unwrap();
        let before = shell.state();

        let out = shell.eval_line_with_ai("이 로그 분석해줘", &ai);
        assert!(out.ok, "{out:?}");
        let suggestion = out.ai.expect("ai outcome");
        assert_eq!(suggestion.kind, "answered");
        assert!(suggestion.text.contains("분석"), "{suggestion:?}");
        assert_eq!(out.output_text, "");
        assert_eq!(out.state, before, "AI 경로는 셸 상태를 바꾸지 않는다");
    }

    #[test]
    fn mobile_ai_leaves_shell_commands_to_shellcore() {
        use crate::mobile_ai::{MobileAi, MobileAiConfig};
        let mut shell = MobileShell::new();
        let ai = MobileAi::from_config(&MobileAiConfig::default()).unwrap();

        let out = shell.eval_line_with_ai("[{size: 200}] | length", &ai);
        assert!(out.ok, "{out:?}");
        assert!(out.ai.is_none());
        assert_eq!(out.output_json, serde_json::json!(1));
    }

    #[test]
    fn mobile_ai_json_bridge_round_trips() {
        let raw = eval_line_ai_json("큰 파일 찾아줘", &initial_state_json(), "{}");
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();
        assert!(result.ok, "{result:?}");
        assert_eq!(
            result.ai.as_ref().map(|a| a.kind.as_str()),
            Some("answered"),
            "{result:?}"
        );
    }

    #[test]
    fn mobile_ai_json_bridge_invalid_config_falls_back_to_shell() {
        let raw = eval_line_ai_json("[1 2 3] | length", &initial_state_json(), "{not-json");
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();
        assert!(result.ok, "{result:?}");
        assert!(result.ai.is_none());
        assert_eq!(result.output_json, serde_json::json!(3));
    }

    #[test]
    fn plain_eval_json_omits_ai_field() {
        let raw = eval_line_json("print \"x\"", &initial_state_json());
        assert!(!raw.contains("\"ai\""), "{raw}");
    }
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib mobile:: >/tmp/t.log 2>&1 && echo UNEXPECTED_PASS || { echo FAIL_AS_EXPECTED; grep -E "error\[|cannot find" /tmp/t.log | head -5; }'
```

Expected: `FAIL_AS_EXPECTED` — `eval_line_with_ai`/`eval_line_ai_json`/`ai` 필드 미정의 컴파일 에러.

- [ ] **Step 3: 구현**

`src/mobile.rs` 상단 import에 추가:

```rust
use crate::dispatch::{self, Route};
use crate::mobile_ai::{MobileAi, MobileAiConfig, MobileAiOutcome};
use crate::policy::PolicyProfile;
```

`MobileEvalResult`에 필드 추가(기존 필드 뒤, `state` 앞):

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MobileEvalResult {
    pub ok: bool,
    pub output_json: serde_json::Value,
    pub output_text: String,
    pub error: Option<String>,
    /// AI 제안(§3-11: 표시용 — 실행 아님). None이면 JSON에서 필드 생략(하위호환).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<MobileAiOutcome>,
    pub state: MobileSessionState,
}
```

기존 `MobileEvalResult` 리터럴 4곳에 `ai: None,` 추가 — `eval_line`의 세 분기(`Ok(Ok)`/`Ok(Err)`/`Err`)와 `error_result_json`의 fallback. (`serialize_result`의 수동 fallback JSON 문자열은 필드가 옵셔널이므로 수정 불필요.)

`eval_line_json` 아래에 브리지 함수 추가:

```rust
/// AI 보조가 켜진 한 줄 평가(JSON-in/JSON-out).
///
/// `ai_config_json` 파싱·구성 실패는 AI 없는 기존 평가로 폴백한다
/// (fail-soft, §3-3: AI 장애가 셸을 막지 않는다).
pub fn eval_line_ai_json(input: &str, state_json: &str, ai_config_json: &str) -> String {
    let state = serde_json::from_str::<MobileSessionState>(state_json)
        .unwrap_or_else(|_| MobileShell::new().state());
    let mut shell = MobileShell::from_state(state);
    let result = match serde_json::from_str::<MobileAiConfig>(ai_config_json)
        .ok()
        .and_then(|cfg| MobileAi::from_config(&cfg).ok())
    {
        Some(ai) => shell.eval_line_with_ai(input, &ai),
        None => shell.eval_line(input),
    };
    serialize_result(&result)
}
```

`impl MobileShell`에 메서드 추가(`eval_line` 아래):

```rust
    /// AI 보조 평가: 자연어(Route::Ai)는 제안으로 돌리고, 셸 명령·빈 입력은
    /// 기존 [`eval_line`](Self::eval_line) 그대로 평가한다. 제안은 실행되지
    /// 않으며(§3-11) 셸 상태도 바꾸지 않는다.
    pub fn eval_line_with_ai(&mut self, input: &str, ai: &MobileAi) -> MobileEvalResult {
        match dispatch::dispatch(input, &PolicyProfile::balanced()) {
            Route::Ai { prompt } => {
                let cwd = self.engine.cwd.display().to_string();
                let outcome = ai.suggest(&prompt, &cwd);
                MobileEvalResult {
                    ok: true,
                    output_json: serde_json::Value::Null,
                    output_text: String::new(),
                    error: None,
                    ai: Some(outcome),
                    state: self.state(),
                }
            }
            _ => self.eval_line(input),
        }
    }
```

- [ ] **Step 4: 테스트 실행 — 통과 확인**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib mobile >/tmp/t.log 2>&1 && { echo TEST_PASS; grep "test result" /tmp/t.log; } || { echo TEST_FAIL; tail -40 /tmp/t.log; }'
```

Expected: `TEST_PASS`, 0 failed (mobile·mobile_ai·mobile_ffi 테스트 전부).

- [ ] **Step 5: 공통 검증 (fmt → clippy → test 전체 → android check)**

Expected: 전부 PASS.

- [ ] **Step 6: Commit**

```bash
git -C /d/workspace/terminal-project/terminal-ai-wt add src/mobile.rs
git -C /d/workspace/terminal-project/terminal-ai-wt commit -m "feat(mobile): eval_line AI 라우팅 — 자연어는 제안, 셸 평가는 불변

dispatch 분류로 Route::Ai만 MobileAi.suggest로 보내고 Shell/Empty는 기존
eval_line 그대로. MobileEvalResult에 옵셔널 ai 필드(None이면 JSON 생략 —
기존 Kotlin 디코더 하위호환). AI config 파싱 실패는 기존 평가로 폴백(§3-3).
AI 경로는 셸 상태를 바꾸지 않고 아무것도 실행하지 않는다(§3-11)."
```

---

### Task 4: C ABI + JNI 표면 (3-인자 AI eval)

**Files:**
- Modify: `src/mobile_ffi.rs`
- Modify: `src/mobile_jni.rs`

**Interfaces:**
- Consumes: Task 3의 `mobile::eval_line_ai_json(input, state_json, ai_config_json)`.
- Produces (Task 5 Kotlin이 사용):
  - C ABI: `ai_terminal_mobile_eval_line_ai_json(input, state_json, ai_config_json) -> *mut c_char` (해제는 기존 `ai_terminal_mobile_free_string`)
  - JNI: `Java_dev_aiterminal_android_NativeShellBridge_nativeEvalLineAi(env, this, input, state_json, ai_config_json) -> jstring`

- [ ] **Step 1: 실패하는 테스트 작성**

`src/mobile_ffi.rs`의 `mod tests` 끝에 추가:

```rust
    #[test]
    fn c_abi_evaluates_ai_line_json() {
        let input = CString::new("큰 파일 찾아줘").unwrap();
        let state = CString::new(crate::mobile::initial_state_json()).unwrap();
        let ai_cfg = CString::new("{}").unwrap();

        let raw = unsafe {
            take_owned_json(ai_terminal_mobile_eval_line_ai_json(
                input.as_ptr(),
                state.as_ptr(),
                ai_cfg.as_ptr(),
            ))
        };
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();

        assert!(result.ok, "{result:?}");
        assert_eq!(
            result.ai.as_ref().map(|a| a.kind.as_str()),
            Some("answered"),
            "{result:?}"
        );
    }

    #[test]
    fn c_abi_ai_reports_null_config_as_json_error() {
        let input = CString::new("x").unwrap();
        let state = CString::new(crate::mobile::initial_state_json()).unwrap();

        let raw = unsafe {
            take_owned_json(ai_terminal_mobile_eval_line_ai_json(
                input.as_ptr(),
                state.as_ptr(),
                ptr::null(),
            ))
        };
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();

        assert!(!result.ok);
        assert_eq!(
            result.error.as_deref(),
            Some("ai_config_json pointer was null")
        );
    }
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib mobile_ffi >/tmp/t.log 2>&1 && echo UNEXPECTED_PASS || { echo FAIL_AS_EXPECTED; grep -E "error\[|cannot find" /tmp/t.log | head -3; }'
```

Expected: `FAIL_AS_EXPECTED` — `ai_terminal_mobile_eval_line_ai_json` 미정의.

- [ ] **Step 3: 구현**

`src/mobile_ffi.rs`의 `ai_terminal_mobile_eval_line_json` 아래에 추가:

```rust
/// Evaluates one mobile shell line with AI assist enabled.
///
/// # Safety
///
/// `input`, `state_json`, and `ai_config_json` must be non-null pointers to
/// valid NUL-terminated UTF-8 strings. The returned pointer is owned by Rust
/// and must be released with [`ai_terminal_mobile_free_string`].
#[no_mangle]
pub unsafe extern "C" fn ai_terminal_mobile_eval_line_ai_json(
    input: *const c_char,
    state_json: *const c_char,
    ai_config_json: *const c_char,
) -> *mut c_char {
    let input = match c_arg_to_string(input, "input") {
        Ok(value) => value,
        Err(error_json) => return string_to_c_ptr(error_json),
    };
    let state_json = match c_arg_to_string(state_json, "state_json") {
        Ok(value) => value,
        Err(error_json) => return string_to_c_ptr(error_json),
    };
    let ai_config_json = match c_arg_to_string(ai_config_json, "ai_config_json") {
        Ok(value) => value,
        Err(error_json) => return string_to_c_ptr(error_json),
    };
    string_to_c_ptr(mobile::eval_line_ai_json(&input, &state_json, &ai_config_json))
}
```

`src/mobile_jni.rs`의 기존 함수 아래에 추가:

```rust
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_aiterminal_android_NativeShellBridge_nativeEvalLineAi(
    mut env: JNIEnv,
    _this: JObject,
    input: JString,
    state_json: JString,
    ai_config_json: JString,
) -> jstring {
    let response = match (
        env.get_string(&input).map(String::from),
        env.get_string(&state_json).map(String::from),
        env.get_string(&ai_config_json).map(String::from),
    ) {
        (Ok(input), Ok(state_json), Ok(ai_config_json)) => {
            mobile::eval_line_ai_json(&input, &state_json, &ai_config_json)
        }
        (Err(err), _, _) | (_, Err(err), _) | (_, _, Err(err)) => {
            mobile::error_result_json(format!("failed to read JNI string: {err}"))
        }
    };

    match env.new_string(response) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
```

- [ ] **Step 4: 테스트 실행 — 통과 확인**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib mobile_ffi >/tmp/t.log 2>&1 && { echo TEST_PASS; grep "test result" /tmp/t.log; } || { echo TEST_FAIL; tail -40 /tmp/t.log; }'
```

Expected: `TEST_PASS`, 0 failed.

- [ ] **Step 5: 공통 검증 (fmt → clippy → test 전체 → android check)**

Expected: 전부 PASS. (android check가 `mobile_jni::nativeEvalLineAi` 컴파일을 확인.)

- [ ] **Step 6: Commit**

```bash
git -C /d/workspace/terminal-project/terminal-ai-wt add src/mobile_ffi.rs src/mobile_jni.rs
git -C /d/workspace/terminal-project/terminal-ai-wt commit -m "feat(mobile): AI eval 3-인자 C ABI + JNI 표면

ai_terminal_mobile_eval_line_ai_json(iOS 대비 C ABI)과
NativeShellBridge.nativeEvalLineAi(Android JNI). 기존 2-인자 표면은 불변."
```

---

### Task 5: Kotlin bridge/worker AI 결선

**Files:**
- Modify: `android/app/src/main/java/dev/aiterminal/android/ShellBridge.kt`
- Modify: `android/app/src/main/java/dev/aiterminal/android/ShellWorker.kt`
- Create: `android/app/src/test/java/dev/aiterminal/android/ShellBridgeCodecTest.kt`

**Interfaces:**
- Consumes: Task 4의 JNI `nativeEvalLineAi`. Rust JSON 계약: 요청 `{"provider","model","openai_url","api_key"}`, 응답 `ai: {"kind","text"}`(옵셔널).
- Produces (Task 6이 사용):
  - `data class ShellAiConfig(val provider: String = "mock", val model: String = "default", val openaiUrl: String = "https://api.openai.com", val apiKey: String? = null)`
  - `data class AiSuggestion(val kind: String, val text: String)`
  - `ShellEvalResult`에 `val ai: AiSuggestion? = null` (마지막 파라미터, 기존 5-인자 위치 호출 호환)
  - `ShellBridge.evalLineAi(input, state, aiConfig)` — 인터페이스 default 구현은 `evalLine`으로 위임(기존 fake 구현 호환)
  - `ShellWorker.aiConfig: ShellAiConfig?` — 설정 시 eval이 AI 경로 사용

- [ ] **Step 1: 실패하는 테스트 작성**

`android/app/src/test/java/dev/aiterminal/android/ShellBridgeCodecTest.kt` 생성:

```kotlin
package dev.aiterminal.android

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ShellBridgeCodecTest {
    @Test
    fun decodeResultParsesAiSuggestion() {
        val raw = """
            {"ok":true,"output_json":null,"output_text":"","error":null,
             "ai":{"kind":"answered","text":"ls -al"},
             "state":{"workspace_root":"/app","cwd":"/app","vars":{},"exit_code":null}}
        """.trimIndent()

        val result = decodeResult(raw, ShellState())

        assertEquals(AiSuggestion(kind = "answered", text = "ls -al"), result.ai)
        assertEquals(true, result.ok)
    }

    @Test
    fun decodeResultWithoutAiFieldIsNull() {
        val raw = """
            {"ok":true,"output_json":null,"output_text":"x","error":null,
             "state":{"workspace_root":"/app","cwd":"/app","vars":{},"exit_code":null}}
        """.trimIndent()

        val result = decodeResult(raw, ShellState())

        assertNull(result.ai)
    }

    @Test
    fun encodeAiConfigProducesRustContract() {
        val json = JSONObject(
            encodeAiConfig(ShellAiConfig(provider = "openai", apiKey = "sk-1")),
        )

        assertEquals("openai", json.getString("provider"))
        assertEquals("default", json.getString("model"))
        assertEquals("https://api.openai.com", json.getString("openai_url"))
        assertEquals("sk-1", json.getString("api_key"))
    }

    @Test
    fun encodeAiConfigNullApiKeyIsJsonNull() {
        val json = JSONObject(encodeAiConfig(ShellAiConfig()))

        assertEquals("mock", json.getString("provider"))
        assertEquals(true, json.isNull("api_key"))
    }
}
```

`ShellWorkerTest.kt` 끝(클래스 내부)에 추가:

```kotlin
    @Test
    fun aiConfigRoutesSubmitToEvalLineAi() {
        val seenConfig = AtomicReference<ShellAiConfig>()
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult {
                error("aiConfig가 설정되면 evalLineAi를 써야 한다")
            }

            override fun evalLineAi(
                input: String,
                state: ShellState,
                aiConfig: ShellAiConfig,
            ): ShellEvalResult {
                seenConfig.set(aiConfig)
                return ShellEvalResult(
                    ok = true,
                    outputText = "",
                    outputJson = "null",
                    error = null,
                    state = state,
                    ai = AiSuggestion(kind = "answered", text = "du -sh *"),
                )
            }
        }
        val executor = Executors.newSingleThreadExecutor()
        val posted = ArrayBlockingQueue<() -> Unit>(8)
        val worker = ShellWorker(
            bridge = bridge,
            executor = executor,
            resultPoster = ResultPoster { block -> posted.put(block) },
        )
        worker.aiConfig = ShellAiConfig()
        val recorder = StreamEventRecorder()

        worker.submitStreaming("큰 파일 찾아줘", ShellState(), recorder)
        drainPostedUntilTerminal(posted, recorder)

        val events = recorder.snapshot()
        assertEquals(ShellAiConfig(), seenConfig.get())
        val finished = events.last() as ShellStreamEvent.Finished
        assertEquals(AiSuggestion("answered", "du -sh *"), finished.result.ai)

        worker.close()
    }

    @Test
    fun withoutAiConfigSubmitUsesPlainEvalLine() {
        val aiCalled = AtomicBoolean(false)
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult =
                ShellEvalResult(
                    ok = true,
                    outputText = "plain",
                    outputJson = "\"plain\"",
                    error = null,
                    state = state,
                )

            override fun evalLineAi(
                input: String,
                state: ShellState,
                aiConfig: ShellAiConfig,
            ): ShellEvalResult {
                aiCalled.set(true)
                return evalLine(input, state)
            }
        }
        val executor = Executors.newSingleThreadExecutor()
        val posted = ArrayBlockingQueue<() -> Unit>(8)
        val worker = ShellWorker(
            bridge = bridge,
            executor = executor,
            resultPoster = ResultPoster { block -> posted.put(block) },
        )
        val recorder = StreamEventRecorder()

        worker.submitStreaming("ls", ShellState(), recorder)
        drainPostedUntilTerminal(posted, recorder)

        assertFalse(aiCalled.get())
        assertEquals(ShellStreamEvent.Stdout("plain"), recorder.snapshot()[1])

        worker.close()
    }
```

- [ ] **Step 2: 테스트 실행 — 실패(컴파일 에러) 확인**

```bash
cd /d/workspace/terminal-project/terminal-ai-wt/android && ANDROID_HOME=$HOME/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest 2>&1 | tail -15
```

Expected: BUILD FAILED — `ShellAiConfig`/`AiSuggestion`/`evalLineAi`/`aiConfig`/`decodeResult`(unresolved) 컴파일 에러.

- [ ] **Step 3: 구현 — ShellBridge.kt**

`ShellBridge.kt`를 다음처럼 수정한다.

`ShellState` 아래에 추가:

```kotlin
data class ShellAiConfig(
    val provider: String = "mock",
    val model: String = "default",
    val openaiUrl: String = "https://api.openai.com",
    val apiKey: String? = null,
)

data class AiSuggestion(
    val kind: String,
    val text: String,
)
```

`ShellEvalResult`에 마지막 파라미터 추가:

```kotlin
data class ShellEvalResult(
    val ok: Boolean,
    val outputText: String,
    val outputJson: String,
    val error: String?,
    val state: ShellState,
    val ai: AiSuggestion? = null,
)
```

`ShellBridge` 인터페이스에 default 메서드 추가:

```kotlin
interface ShellBridge {
    fun evalLine(input: String, state: ShellState): ShellEvalResult

    fun evalLineAi(input: String, state: ShellState, aiConfig: ShellAiConfig): ShellEvalResult =
        evalLine(input, state)
}
```

`NativeShellBridge`에 override 추가(`evalLine` 아래):

```kotlin
    override fun evalLineAi(
        input: String,
        state: ShellState,
        aiConfig: ShellAiConfig,
    ): ShellEvalResult {
        return try {
            loadNativeLibrary()
            decodeResult(nativeEvalLineAi(input, encodeState(state), encodeAiConfig(aiConfig)), state)
        } catch (error: UnsatisfiedLinkError) {
            err("native shell library not loaded: ${error.message}", state)
        } catch (error: RuntimeException) {
            err("native shell bridge failed: ${error.message}", state)
        }
    }

    private external fun nativeEvalLineAi(input: String, stateJson: String, aiConfigJson: String): String
```

파일 하단 codec — `encodeState` 아래에 추가하고, `decodeResult`는 `private fun` → `internal fun`으로 승격 + `ai` 파싱 추가:

```kotlin
internal fun encodeAiConfig(config: ShellAiConfig): String {
    val encoded = JSONObject()
    encoded.put("provider", config.provider)
    encoded.put("model", config.model)
    encoded.put("openai_url", config.openaiUrl)
    encoded.put("api_key", config.apiKey ?: JSONObject.NULL)
    return encoded.toString()
}
```

```kotlin
internal fun decodeResult(raw: String, fallbackState: ShellState): ShellEvalResult {
    return try {
        val json = JSONObject(raw)
        val stateJson = json.optJSONObject("state")
        val nextState = if (stateJson == null) fallbackState else decodeState(stateJson, fallbackState)

        ShellEvalResult(
            ok = json.optBoolean("ok", false),
            outputText = json.optString("output_text", ""),
            outputJson = jsonValueToString(json.opt("output_json")),
            error = if (json.isNull("error")) null else json.optString("error"),
            state = nextState,
            ai = decodeAiSuggestion(json.optJSONObject("ai")),
        )
    } catch (error: JSONException) {
        err("native shell returned invalid JSON: ${error.message}", fallbackState)
    }
}

private fun decodeAiSuggestion(json: JSONObject?): AiSuggestion? {
    if (json == null) return null
    return AiSuggestion(
        kind = json.optString("kind", "unavailable"),
        text = json.optString("text", ""),
    )
}
```

- [ ] **Step 4: 구현 — ShellWorker.kt**

`externalCommandsEnabled` 아래에 프로퍼티 추가:

```kotlin
    /** 설정되면 pure eval이 AI 보조 경로(evalLineAi)로 간다. null이면 기존 그대로. */
    @Volatile
    var aiConfig: ShellAiConfig? = null
```

`submitStreaming`의 bridge 호출부를 교체:

```kotlin
            val result = runCatching {
                val config = aiConfig
                if (config != null) {
                    bridge.evalLineAi(input, state, config)
                } else {
                    bridge.evalLine(input, state)
                }
            }
                .getOrElse { error ->
                    ShellEvalResult(
                        ok = false,
                        outputText = "",
                        outputJson = "null",
                        error = error.message ?: error::class.java.simpleName,
                        state = state,
                    )
                }
```

- [ ] **Step 5: 테스트 실행 — 통과 확인**

```bash
cd /d/workspace/terminal-project/terminal-ai-wt/android && ANDROID_HOME=$HOME/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest 2>&1 | tail -5
```

Expected: `BUILD SUCCESSFUL` (기존 45+ 테스트 + 신규 6개 전부 통과).

- [ ] **Step 6: Commit**

```bash
git -C /d/workspace/terminal-project/terminal-ai-wt add android/app/src/main/java/dev/aiterminal/android/ShellBridge.kt android/app/src/main/java/dev/aiterminal/android/ShellWorker.kt android/app/src/test/java/dev/aiterminal/android/ShellBridgeCodecTest.kt android/app/src/test/java/dev/aiterminal/android/ShellWorkerTest.kt
git -C /d/workspace/terminal-project/terminal-ai-wt commit -m "feat(android): ShellBridge/Worker AI 결선

ShellAiConfig·AiSuggestion·evalLineAi(인터페이스 default는 evalLine 위임 —
기존 fake 호환)·nativeEvalLineAi external. ShellEvalResult에 옵셔널 ai.
ShellWorker.aiConfig 설정 시에만 AI eval 경로를 탄다(기본 null=기존 동작)."
```

---

### Task 6: Kotlin ViewModel/UI — 제안 표시 + 토글 (자동 실행 없음)

**Files:**
- Modify: `android/app/src/main/java/dev/aiterminal/android/TerminalViewModel.kt`
- Modify: `android/app/src/main/java/dev/aiterminal/android/MainActivity.kt`
- Create: `android/app/src/test/java/dev/aiterminal/android/TerminalViewModelAiTest.kt`

**Interfaces:**
- Consumes: Task 5의 `ShellAiConfig`, `AiSuggestion`, `ShellWorker.aiConfig`, `ShellEvalResult.ai`.
- Produces: `EntryKind.AiSuggestion`, `TerminalViewModel.aiEnabled: Boolean`, `TerminalViewModel.toggleAi()`. UI: AI 토글 버튼, 제안 엔트리(보라색, `ai> ` prefix), 제안 탭 → 입력창 채움(사용자가 Run을 눌러야 실행 — §3-11 확인 UX).

- [ ] **Step 1: 실패하는 테스트 작성**

`android/app/src/test/java/dev/aiterminal/android/TerminalViewModelAiTest.kt` 생성:

```kotlin
package dev.aiterminal.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.ArrayBlockingQueue
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

class TerminalViewModelAiTest {
    private class PostedQueueFixture {
        val posted = ArrayBlockingQueue<() -> Unit>(16)

        fun worker(bridge: ShellBridge): ShellWorker =
            ShellWorker(
                bridge = bridge,
                executor = Executors.newSingleThreadExecutor(),
                resultPoster = ResultPoster { block -> posted.put(block) },
            )

        fun drain(viewModel: TerminalViewModel) {
            while (viewModel.isBusy) {
                val block = posted.poll(2, TimeUnit.SECONDS) ?: break
                block.invoke()
            }
            assertFalse("viewmodel should settle after drain", viewModel.isBusy)
        }
    }

    private fun suggestionBridge(suggestion: AiSuggestion) = object : ShellBridge {
        override fun evalLine(input: String, state: ShellState): ShellEvalResult =
            error("AI가 켜지면 evalLineAi 경로를 써야 한다")

        override fun evalLineAi(
            input: String,
            state: ShellState,
            aiConfig: ShellAiConfig,
        ): ShellEvalResult =
            ShellEvalResult(
                ok = true,
                outputText = "",
                outputJson = "null",
                error = null,
                state = state,
                ai = suggestion,
            )
    }

    @Test
    fun toggleAiWiresWorkerConfigAndAnnounces() {
        val fixture = PostedQueueFixture()
        val worker = fixture.worker(suggestionBridge(AiSuggestion("answered", "x")))
        val viewModel = TerminalViewModel(worker, null, ShellState())

        assertFalse(viewModel.aiEnabled)
        assertNull(worker.aiConfig)

        viewModel.toggleAi()

        assertTrue(viewModel.aiEnabled)
        assertEquals(ShellAiConfig(), worker.aiConfig)
        assertEquals(EntryKind.Output, viewModel.transcript.last().kind)

        viewModel.toggleAi()

        assertFalse(viewModel.aiEnabled)
        assertNull(worker.aiConfig)

        worker.close()
    }

    @Test
    fun answeredSuggestionRendersAsAiSuggestionEntry() {
        val fixture = PostedQueueFixture()
        val worker = fixture.worker(suggestionBridge(AiSuggestion("answered", "du -sh *")))
        val viewModel = TerminalViewModel(worker, null, ShellState())

        viewModel.toggleAi()
        viewModel.updateInput("큰 파일 찾아줘")
        viewModel.submit()
        fixture.drain(viewModel)

        val entry = viewModel.transcript.last()
        assertEquals(EntryKind.AiSuggestion, entry.kind)
        assertEquals("du -sh *", entry.text)

        worker.close()
    }

    @Test
    fun blockedSuggestionRendersAsError() {
        val fixture = PostedQueueFixture()
        val worker = fixture.worker(suggestionBridge(AiSuggestion("blocked", "masking failed")))
        val viewModel = TerminalViewModel(worker, null, ShellState())

        viewModel.toggleAi()
        viewModel.updateInput("secret 보내줘")
        viewModel.submit()
        fixture.drain(viewModel)

        val entry = viewModel.transcript.last()
        assertEquals(EntryKind.Error, entry.kind)
        assertEquals("ai blocked: masking failed", entry.text)

        worker.close()
    }
}
```

- [ ] **Step 2: 테스트 실행 — 실패(컴파일 에러) 확인**

```bash
cd /d/workspace/terminal-project/terminal-ai-wt/android && ANDROID_HOME=$HOME/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest 2>&1 | tail -10
```

Expected: BUILD FAILED — `aiEnabled`/`toggleAi`/`EntryKind.AiSuggestion` unresolved.

- [ ] **Step 3: 구현 — TerminalViewModel.kt**

`EntryKind`에 variant 추가:

```kotlin
enum class EntryKind {
    Command,
    Output,
    Error,
    AiSuggestion,
}
```

`isBusy` 프로퍼티 아래에 상태·토글 추가:

```kotlin
    var aiEnabled by mutableStateOf(false)
        private set

    fun toggleAi() {
        aiEnabled = !aiEnabled
        worker.aiConfig = if (aiEnabled) ShellAiConfig() else null
        transcript += TranscriptEntry(
            EntryKind.Output,
            if (aiEnabled) {
                "AI assist enabled (mock provider; suggestions only, nothing auto-runs)"
            } else {
                "AI assist disabled"
            },
        )
    }
```

`submit()`의 `Finished` 분기를 다음으로 교체:

```kotlin
                is ShellStreamEvent.Finished -> {
                    val result = event.result
                    sessionState = result.state
                    val ai = result.ai
                    if (ai != null) {
                        if (ai.kind == "answered") {
                            transcript += TranscriptEntry(EntryKind.AiSuggestion, ai.text)
                        } else {
                            transcript += TranscriptEntry(EntryKind.Error, "ai ${ai.kind}: ${ai.text}")
                        }
                    }
                    if (!result.ok && !result.error.isNullOrBlank()) {
                        transcript += TranscriptEntry(EntryKind.Error, result.error)
                    }
                    activeRun = null
                    isBusy = false
                }
```

- [ ] **Step 4: 구현 — MainActivity.kt**

import 추가:

```kotlin
import androidx.compose.foundation.clickable
```

`TerminalScreen`의 `SessionStatus(...)` 호출에 인자 2개 추가(`onVerifyTermuxStaging` 아래):

```kotlin
                aiEnabled = viewModel.aiEnabled,
                onToggleAi = viewModel::toggleAi,
```

`Transcript(...)` 호출에 탭 콜백 추가:

```kotlin
            Transcript(
                entries = viewModel.transcript,
                onSuggestionTap = viewModel::updateInput,
                modifier = Modifier
                    .weight(1f)
                    .fillMaxWidth(),
            )
```

`SessionStatus` 시그니처에 파라미터 추가(`onVerifyTermuxStaging: () -> Unit,` 아래):

```kotlin
    aiEnabled: Boolean,
    onToggleAi: () -> Unit,
```

"Probe Termux" Row에 AI 토글 버튼 추가:

```kotlin
        Row(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Button(onClick = onProbeTermux, enabled = !busy) {
                Text("Probe Termux")
            }
            Button(onClick = onInstallTermuxHelper, enabled = !busy) {
                Text("Install Helper")
            }
            Button(onClick = onToggleAi, enabled = !busy) {
                Text(if (aiEnabled) "AI: on" else "AI: off")
            }
        }
```

`Transcript` composable을 다음으로 교체(제안 색/prefix/탭 — 탭하면 입력창만 채우고 실행은 사용자가 Run을 눌러야 한다):

```kotlin
@Composable
private fun Transcript(
    entries: List<TranscriptEntry>,
    onSuggestionTap: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val listState = rememberLazyListState()
    LaunchedEffect(entries.size) {
        if (entries.isNotEmpty()) {
            listState.animateScrollToItem(entries.lastIndex)
        }
    }

    LazyColumn(
        state = listState,
        modifier = modifier
            .background(Color(0xFF101418))
            .padding(12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        items(entries) { entry ->
            val color = when (entry.kind) {
                EntryKind.Command -> Color(0xFF9CCAFF)
                EntryKind.Output -> Color(0xFFE6EDF3)
                EntryKind.Error -> Color(0xFFFFB4AB)
                EntryKind.AiSuggestion -> Color(0xFFC5B3FF)
            }
            val prefix = when (entry.kind) {
                EntryKind.Command -> "> "
                EntryKind.Output -> ""
                EntryKind.Error -> "error: "
                EntryKind.AiSuggestion -> "ai> "
            }
            val entryModifier = if (entry.kind == EntryKind.AiSuggestion) {
                Modifier.clickable { onSuggestionTap(entry.text) }
            } else {
                Modifier
            }
            Text(
                text = prefix + entry.text,
                color = color,
                fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.bodyMedium,
                modifier = entryModifier,
            )
        }
    }
}
```

- [ ] **Step 5: 테스트 실행 — 통과 확인**

```bash
cd /d/workspace/terminal-project/terminal-ai-wt/android && ANDROID_HOME=$HOME/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest 2>&1 | tail -5
```

Expected: `BUILD SUCCESSFUL` (신규 3개 포함 전부 통과).

- [ ] **Step 6: Commit**

```bash
git -C /d/workspace/terminal-project/terminal-ai-wt add android/app/src/main/java/dev/aiterminal/android/TerminalViewModel.kt android/app/src/main/java/dev/aiterminal/android/MainActivity.kt android/app/src/test/java/dev/aiterminal/android/TerminalViewModelAiTest.kt
git -C /d/workspace/terminal-project/terminal-ai-wt commit -m "feat(android): AI 제안 transcript UI + 토글 (자동 실행 없음)

EntryKind.AiSuggestion(보라, 'ai> ')·aiEnabled/toggleAi(worker.aiConfig 결선)·
Finished에서 ai kind별 렌더(answered→제안, blocked/unavailable→error).
제안 탭은 입력창만 채운다 — 실행은 사용자가 Run을 눌러야 하며 그때 기존
shellcore 경계를 그대로 탄다(§3-11 확인 UX)."
```

---

### Task 7: 전 조합 최종 검증 + 문서 기록

**Files:**
- Modify: `docs/TASK.md` (PM-3 섹션 체크 항목 1개 추가)
- Modify: `docs/HANDOFF.md` (§0 재개점 + §1 Android 요약 갱신)

**Interfaces:**
- Consumes: Task 1~6 전부 (whole-branch 검증).
- Produces: CI와 동일 조합의 로컬 green 증거 + 재개 문서.

- [ ] **Step 1: CI 미러 전 조합 검증 (WSL)**

한 줄씩 실행(전부 PASS여야 함 — CI `check` 잡과 동일 순서):

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all -- --check >/tmp/v.log 2>&1 && echo PASS_FMT || { echo FAIL_FMT; cat /tmp/v.log; }'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets -- -D warnings >/tmp/v.log 2>&1 && echo PASS_CLIPPY || { echo FAIL_CLIPPY; tail -30 /tmp/v.log; }'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --all-targets >/tmp/v.log 2>&1 && { echo PASS_TEST; grep -c "test result: ok" /tmp/v.log; } || { echo FAIL_TEST; tail -30 /tmp/v.log; }'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build --features storage >/tmp/v.log 2>&1 && echo PASS_STORAGE || { echo FAIL_STORAGE; tail -20 /tmp/v.log; }'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets --features tls -- -D warnings >/tmp/v.log 2>&1 && cargo build --features "storage tls" >>/tmp/v.log 2>&1 && echo PASS_TLS || { echo FAIL_TLS; tail -30 /tmp/v.log; }'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets --features trust -- -D warnings >/tmp/v.log 2>&1 && cargo test --features trust >>/tmp/v.log 2>&1 && echo PASS_TRUST || { echo FAIL_TRUST; tail -30 /tmp/v.log; }'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build --bins --features "storage tls remote trust" >/tmp/v.log 2>&1 && echo PASS_RELEASE_FEATURES || { echo FAIL_RELEASE_FEATURES; tail -20 /tmp/v.log; }'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-ai-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo check --lib --target aarch64-linux-android >/tmp/v.log 2>&1 && echo PASS_ANDROID || { echo FAIL_ANDROID; tail -30 /tmp/v.log; }'
```

- [ ] **Step 2: gradle 전체 테스트 (git-bash)**

```bash
cd /d/workspace/terminal-project/terminal-ai-wt/android && ANDROID_HOME=$HOME/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest 2>&1 | tail -3
```

Expected: `BUILD SUCCESSFUL`.

- [ ] **Step 3: 문서 갱신**

`docs/TASK.md` PM-3 섹션의 `- [x] Android/iOS 공용 Rust mobile JSON eval/state bridge 계약 고정 (...)` 줄 바로 아래에 추가:

```markdown
- [x] Android AI 보조 이식(1차, mock transport): AI 스택(intent·dispatch 분류·gateway·openai + 순수 의존)을 android 타깃에 포함(실 I/O transport 제외), `mobile_ai` 구동기 + `eval_line_ai_json` 3-인자 JNI/C ABI, Kotlin `evalLineAi`/`aiConfig`/`EntryKind.AiSuggestion` UI(제안 표시·탭→입력 채움, 자동 실행 금지 §3-11). 실 HTTP transport(OkHttp JNI)·실기기 openai 검증은 후속. 정본: `docs/superpowers/specs/2026-07-10-android-ai-assist-design.md`, `docs/superpowers/plans/2026-07-11-android-ai-assist.md`
```

`docs/HANDOFF.md` `## 0. 재개점` 목록 맨 위에 추가:

```markdown
- **Android AI 보조 1차 랜딩(2026-07-11)**: 자연어→AI 제안(mock provider, 자동 실행 금지 §3-11),
  openai backend는 config·계약까지 결선(실 HTTP transport는 후속 OkHttp JNI 슬라이스).
  AI 스택이 android 타깃에 포함됨(실 I/O 제외). 다음 Android 후보 = 실 transport + 실기기
  (`SM-F956N`) openai 검증. 정본: `docs/superpowers/specs/2026-07-10-android-ai-assist-design.md`,
  `docs/superpowers/plans/2026-07-11-android-ai-assist.md`.
```

`docs/HANDOFF.md` `## 1. 플랫폼 현황`의 `**Android(PM-3)**` 불릿에서 `shellcore-only 로컬 터미널.`을 `shellcore 로컬 터미널 + AI 보조(자연어→제안, mock provider — 실 transport 후속).`으로 교체.

- [ ] **Step 4: Commit + 컨트롤러 최종 검증으로 인계**

```bash
git -C /d/workspace/terminal-project/terminal-ai-wt add docs/TASK.md docs/HANDOFF.md
git -C /d/workspace/terminal-project/terminal-ai-wt commit -m "docs: Android AI 보조 1차(mock transport) 완료 기록

TASK.md PM-3 체크 + HANDOFF 재개점/플랫폼 현황 갱신. 후속 = 실 HTTP
transport(OkHttp JNI) + 실기기 openai 검증."
```

이후 컨트롤러가 whole-branch 리뷰 → push → PR(base **develop**) 생성.

---

## Self-Review 기록 (spec 대비)

- §3 결정 1(AI는 mobile 계층): dispatch/gateway import는 `mobile.rs`·`mobile_ai.rs`에만 — Task 3 import 목록으로 강제, shellcore 무수정. ✓
- §3 결정 2(openai + config 선택): `MobileAiConfig.provider` + `build_gateway` openai 분기(Task 2). ✓
- §3 결정 3(MockTransport 이번/실 transport 후속): 테스트는 `CannedTransport` 주입 검증, 프로덕션 openai 경로는 `UnavailableTransport`(정직한 실패), mock provider가 기본 UX. 교체 지점 `build_gateway`/`with_gateway` 명시. ✓
- §3 결정 4·§8-4(자동 실행 금지): Rust는 제안 반환만, Kotlin 탭은 입력 채움만(Run은 사용자). ✓
- §4-1(gateway Notify·budget): 실측 근거로 무변경 + Task 1 check 확정, 실패 시 NEEDS_CONTEXT 정지. budget은 mobile에서 미부여(no-op). ✓
- §4-2(MobileEvalResult 확장·AiRouter 재사용 검토): `ai` 옵셔널 필드(하위호환), AiRouter 대신 자체 분기 — 근거 문서화. ✓
- §4-3(UI): EntryKind.AiSuggestion + 토글 + 구분 렌더. ✓
- §6(테스트): 태스크별 aarch64 check + 무피처 test/clippy, Task 7에서 CI 전 조합 + gradle. 실기기는 범위 밖(§7). ✓
- 타입 일관성: `MobileAiOutcome{kind,text}` ↔ Kotlin `AiSuggestion(kind,text)`, config 키 `openai_url`/`api_key` ↔ `encodeAiConfig`, `eval_line_ai_json` 3-인자 ↔ C ABI/JNI/`evalLineAi` — 교차 확인 완료.
