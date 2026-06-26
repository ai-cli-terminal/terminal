# ash 실 AI Provider 결선 설계 (S5 후속)

> **작성일**: 2026-06-27
> **유형**: 단일 슬라이스 구현 spec (S5 AI 통합의 후속 — mock→실 provider).
> **상위/선행**: `2026-06-27-windows-ash-s5-ai-integration-design.md` §11. S1(config), S5(AiRouter/GatewayAiRouter).
> **참고**: `src/ollama.rs`, `src/openai.rs`, `src/gateway.rs`, `src/provider.rs`, `src/http.rs`, `src/responder.rs`, `config.toml.example [ai]`, `ai ask` 결선(`src/main.rs` Command::Ask).

## 1. 목표

S5의 `GatewayAiRouter`가 **mock(echo) 대신 config `[ai]` 설정에 따라 실제 ollama/openai 백엔드**로 응답하게 한다.

- `[ai]` config 모델링(S1에서 이연): `provider`/`model`/`ollama_url`/`openai_url`.
- 기본 provider = **ollama**(localhost:11434). 미설정 사용자도 로컬 LLM이면 OOB 동작.
- 백엔드 도달불가/미설정/HTTPS-미지원은 **fail-soft**(`AiOutcome::Unavailable` → 메시지, 셸 비중단).

## 2. 경계 제약

- 변경은 데스크톱 모듈 `src/config.rs`(`[ai]` 모델) + `src/ai_router.rs`(gateway 구성). `shellcore`·android cdylib 빌드 불변.
- `from_environment()` 시그니처 불변 → `src/bin/ash.rs` 무변경.
- 비밀(API 키)은 **`OPENAI_API_KEY` 환경변수**로만 읽는다(config에 비밀 금지, `docs/RULES.md`).

## 3. `[ai]` Config 모델 (src/config.rs)

```rust
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct Ai {
    pub provider: String,      // "ollama" | "openai" | "mock"
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
// Config 에 필드 추가:
pub struct Config { pub general: General, pub ai: Ai }
```
- `#[serde(default)]`로 누락 필드 기본값, 미지 키 무시(S1과 동일). `Config`는 `Default` 파생(general+ai).
- 어휘는 `ai ask --backend` 이름(`ollama`/`openai`)과 일치. `config.toml.example`의 추상값(`local|remote|local_or_remote`)은 본 슬라이스에서 구체값으로 대체(example 주석도 후속에서 동기화 — 비목표).

## 4. `GatewayAiRouter` gateway 구성 (src/ai_router.rs)

```rust
impl GatewayAiRouter {
    /// config의 [ai]에 따라 실 gateway(또는 mock)로 구성한다.
    pub fn from_ai_config(ai: &crate::config::Ai) -> anyhow::Result<Self>;
    /// config 전체 로드 후 from_ai_config로 위임.
    pub fn from_environment() -> anyhow::Result<Self>;  // = from_ai_config(&config::load().config.ai)
}
```

`from_ai_config` (=`ai ask` 결선 재사용):
```rust
let cap = crate::provider::Provider::mock().models[0].clone();
let gw = match ai.provider.as_str() {
    "ollama" => {
        let b = crate::ollama::OllamaBackend::new(crate::http::TcpTransport, &ai.ollama_url, &ai.model);
        crate::gateway::Gateway::new(Box::new(b), cap)
    }
    "openai" => {
        let api_key = std::env::var("OPENAI_API_KEY").ok();
        let b = crate::openai::OpenAiBackend::new(crate::http::TcpTransport, &ai.openai_url, &ai.model, api_key);
        crate::gateway::Gateway::new(Box::new(b), cap)
    }
    _ => crate::gateway::Gateway::mock(),   // "mock"/미지
};
let responder = crate::responder::GatewayResponder::new(gw, crate::aitask::Timeouts::defaults().request)?;
let profile = ...활성 profile (기존)...;
Ok(Self { responder, profile })
```
- `try_handle`은 S5 그대로(분류→respond→AiOutcome 매핑). 구성만 바뀐다.

## 5. 에러 처리 (fail-soft 전면)

- ollama/openai 도달불가(미실행·네트워크) → `GatewayResponder`가 `AiOutcome::Unavailable`로 흡수 → 라우터가 "ash: AI 사용 불가: …" 출력, **REPL 지속**.
- **openai(HTTPS)는 `tls` feature 필요**: default(C-free) 빌드는 `http.rs`의 `#[cfg(not(feature="tls"))]` 경로라 https 연결 실패 → Unavailable. ollama(http)·mock은 default 빌드 동작.
- `from_environment`/`from_ai_config` 구성 자체 실패(드묾, 런타임 빌드 오류 등) → ash가 기존대로 `NoAiRouter` 폴백.

## 6. 동작 변화

기본 provider가 ollama이므로, **ollama 미실행 환경(CI/WSL/테스트)에서 NL 질의는 echo가 아니라 "AI 사용 불가"** 가 출력된다. 입력이 **AI로 라우팅**되는 사실(셸이 아님)은 유지된다. 따라서 e2e/검증은 "AI로 라우팅됨(셸 명령으로 실행되지 않음)"을 기준으로 한다(echo 내용 의존 제거).

## 7. 테스트

단위(`src/config.rs`):
- `Ai::default()` 값(provider=ollama, ollama_url 기본 등).
- `[ai] provider = "mock"` 파싱 → 반영. 누락 필드 기본값.

단위(`src/ai_router.rs`):
- `GatewayAiRouter::from_ai_config(&Ai{ provider:"mock", .. })` → `try_handle("how do I X?")`=true 이고 (mock echo로) 출력됨. `try_handle("ls -al")`=false.
- `from_ai_config(&Ai::default())`(ollama) → `Ok`(구성 성공; 네트워크 연결은 하지 않음).
- 기존 S5 router 테스트는 `from_ai_config(mock)`로 갱신해 결정적·무네트워크 유지.

e2e(WSL, 비-TTY): mock provider config로 `printf 'how do I X?\nexit\n' | ash` → AI 라우팅(echo). 기본(ollama, 미실행)에서는 NL이 "AI 사용 불가"로 처리되고 셸로 안 감(라우팅 검증). `echo hi`→셸, `rm -rf /`→차단 불변. android 경계 check.

## 8. 수용 기준

1. `[ai]` config(provider/model/ollama_url/openai_url)가 타입화 로드된다(기본 provider=ollama).
2. `GatewayAiRouter`가 config provider에 따라 ollama/openai/mock gateway로 응답한다.
3. 백엔드 도달불가·HTTPS-미지원은 fail-soft(Unavailable, 셸 비중단).
4. API 키는 `OPENAI_API_KEY` 환경변수로만.
5. `shellcore`/android cdylib 빌드 불변. `from_environment` 시그니처 불변(ash 무변경).
6. `cargo fmt --all -- --check`(실제 `cargo fmt --all` 후) · `cargo clippy --all-targets --features "storage tls remote" -D warnings` · `cargo test --features "storage tls remote"` green. (openai HTTPS 실동작은 `tls` feature 빌드에서만 — 본 슬라이스는 구성·fail-soft까지.)

## 9. 비목표

- `provider = "local_or_remote"` 자동 폴백, `[ai]` 타임아웃/예산/PII/send_context/trigger_alias 필드, 실 LLM 응답 e2e(ollama 실행 필요), 캐시 배지·usage/audit 기록, `config.toml.example` 추상값 동기화.
