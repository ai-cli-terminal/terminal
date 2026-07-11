# 실 transport 슬라이스 (Android AI 2차) — 설계

- **날짜**: 2026-07-11
- **상태**: 설계 승인됨 (구현 대기)
- **베이스**: `develop` (`438e0c5`, Android AI 보조 1차 랜딩 완료)
- **선행 spec**: `docs/superpowers/specs/2026-07-10-android-ai-assist-design.md` (1차 — mock echo + `UnavailableTransport`)
- **브랜치(예정)**: `feat/android-real-transport` (develop 분기, develop 경유 2단계 PR)

## 1. 배경

1차 Android AI 보조 슬라이스(#99/#100)는 AI 스택(dispatch → gateway → openai backend)을
모바일 경계에서 동기 구동하는 골격을 완성했다. 그러나 다음 4개가 백로그로 남았다(1차 최종 리뷰):

1. **api_key redaction** — `MobileAiConfig`가 `derive(Debug)`라 `api_key`가 로그에 평문 노출 가능.
2. **openai 실 capability** — `build_gateway`가 openai 분기에서도 `Provider::mock()` capability 사용.
3. **persistent MobileAi** — JNI/C ABI가 stateless라 매 줄 평가마다 gateway+tokio runtime 재생성
   → gateway 캐시(exact+semantic) 무효화, 진행 중 요청 취소 불가.
4. **OkHttp JNI transport** — openai provider가 `UnavailableTransport`(정직한 실패)에 결선되어
   실 HTTP 전송이 없음. 실기기(SM-F956N) openai 검증 미수행.

이 슬라이스는 위 4개를 **한 번에** 처리한다(사용자 결정). 앞 3개는 CI/단위테스트로 완결되고,
OkHttp transport는 코드까지 완성하되 **실 openai 검증만 실기기에서 사용자 후속**이다(이 환경 불가).

## 2. 목표 / 비목표

### 목표
- openai provider가 실 HTTP로 응답을 받는다(실기기, 시스템 TLS 경유).
- 세션 동안 `MobileAi`(및 gateway 캐시)가 지속되어 재사용된다.
- api_key가 Debug/로그에 노출되지 않는다.
- openai capability가 정직하게 반영된다(truncation 한도 등).
- 데스크톱/non-android 동작·경계 불변, CI 전 잡 green 유지.

### 비목표
- 실 openai 응답의 실기기 검증(사용자 후속 — 절차만 문서화).
- streaming 응답, tool use(1차와 동일 MVP 제외).
- openai 이외 provider(ollama 등)의 모바일 결선.
- 프로덕션 AI 설정 UI(디버그 전용 주입만).
- shellcore에 실행 능력 추가 — **§3-11: 제안 텍스트만, 자동 실행 금지** 불변.

## 3. 확정된 설계 결정 (브레인스토밍)

| # | 결정 | 근거 |
|---|---|---|
| D1 | 4개 항목 모두 이 슬라이스 | persistent 핸들이 transport의 토대라 함께 가는 것이 자연스러움 |
| D2 | 동기 JNI + OkHttp 타임아웃 (비동기 브리지 아님) | current-thread 런타임에 자연·요청당 1개라 경쟁 없음·복잡도 최소 |
| D3 | 핸들 기반 JNI (전역 캐시 아님) | 생명주기 명시적·gateway 캐시 유지·iOS C ABI 대칭 확장 |
| D4 | 디버그 전용 config 주입 (UI 아님) | 프로덕션 UX(mock) 불변·최소 침습·검증에 충분 |

### D2의 취소 트레이드오프 (명시적 수용)
`aitask::run_cancellable`(§16.2)은 `tokio::select!`로 future를 drop해 취소한다. blocking JNI
호출은 current-thread 런타임의 유일 스레드를 점유하므로 **`Notify` 기반 취소·`tokio::time::sleep`
타임아웃이 그 구간에는 걸리지 않는다.** 상한은 **OkHttp `callTimeout`(전체 호출)** 이 담당한다.
화면 이탈 등 사용자 취소는 결과를 버리는 것(fail-soft)으로 흡수한다. 이는 수용된 트레이드오프다.

## 4. 아키텍처 개요

```
Kotlin UI (TerminalViewModel)
  └ DEBUG: /sdcard/Download/ai-terminal-ai-config.json → ShellAiConfig(openai)   [T7]
      └ ShellWorker.aiConfig setter → executor에 reconfigure post                 [T4]
          └ NativeShellBridge.createAi(json) → jlong 핸들 (보관·재사용)           [T3-JNI]
              └ nativeEvalLineAiHandle(handle, input, state)                       [T3-JNI]
                  └ Rust MobileAi (persistent: gateway 캐시 유지)                  [T3-Rust]
                      └ openai backend → JniHttpTransport.post_json               [T5-Rust]
                          └ JNI 역호출 → Kotlin NativeHttp.postJson (OkHttp 동기)  [T5-Kotlin]
                              └ Android 시스템 TLS → api.openai.com
```

의존 순서(커밋 순서): **T1(redaction) → T2(capability) → T3(핸들 Rust+FFI) → T4(Kotlin 핸들 전환)
→ T5(OkHttp transport) → T6(from_config 결선) → T7(디버그 주입) → T8(검증 문서)**.

T1·T2는 독립(순수 Rust). T3~T6이 persistent+transport 본체. T7은 검증 수단. T8은 문서.

---

## 5. Task별 계약

### T1 — api_key Debug redaction (순수 Rust)

**파일**: `src/mobile_ai.rs`

- `MobileAiConfig`의 `#[derive(...)]`에서 `Debug` 제거(`Clone, PartialEq, Deserialize`는 유지).
- 수동 `impl std::fmt::Debug for MobileAiConfig` 추가. `api_key`는 값이 아니라 존재 여부만:
  ```rust
  impl std::fmt::Debug for MobileAiConfig {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          f.debug_struct("MobileAiConfig")
              .field("provider", &self.provider)
              .field("model", &self.model)
              .field("openai_url", &self.openai_url)
              .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
              .finish()
      }
  }
  ```
- **주의**: `MobileAiConfig`를 `Debug`로 파생 출력하던 다른 곳이 없는지 확인(현재 없음).

**테스트**(mobile_ai.rs):
- `api_key_is_redacted_in_debug`: `provider="openai", api_key=Some("sk-secret")` → `format!("{cfg:?}")`가
  `sk-secret`를 포함하지 않고 `<redacted>`를 포함.
- `debug_shows_none_api_key_distinctly`: `api_key=None` → `None` 표기(존재 여부 구분 유지).

**수용 기준**: `format!("{cfg:?}")`에 실제 키 문자열이 절대 나타나지 않는다.

---

### T2 — openai 실 capability (순수 Rust)

**파일**: `src/provider.rs`, `src/mobile_ai.rs`

- `provider.rs`에 `Provider::openai()` 추가. gateway에서 **실효는 `max_context_tokens`(truncation 한도,
  gateway.rs:120)뿐**이지만 나머지 필드도 정직하게 채운다:
  ```rust
  impl Provider {
      pub fn openai() -> Provider {
          Provider {
              name: "openai".into(),
              display_name: "OpenAI".into(),
              models: vec![ModelCapability {
                  name: "openai-default".into(),
                  max_context_tokens: 128_000,   // 보수적 현대 기본값
                  max_output_tokens: 4_096,
                  supports_streaming: true,       // 실제 코드는 non-streaming(§비목표)
                  supports_json_mode: true,
                  supports_tool_use: false,       // MVP 제외
                  supports_token_counting: true,  // openai usage 반환
                  supports_usage_reporting: true,
                  supports_context_caching: false,
              }],
          }
      }
  }
  ```
- `mobile_ai.rs::build_gateway`의 openai 분기: `let cap = Provider::mock().models[0].clone();`을
  provider별로 분기 — openai면 `Provider::openai().models[0].clone()`, 그 외(mock)면 기존대로.
  (mock 분기 `Gateway::mock()`은 불변.)

**테스트**(provider.rs + mobile_ai.rs):
- `openai_provider_has_larger_context_than_mock`: `Provider::openai().models[0].max_context_tokens`
  > `Provider::mock().models[0].max_context_tokens`.
- 기존 `openai_provider_with_mock_transport_answers`(mobile_ai.rs:169)가 여전히 통과(capability 교체가
  응답 경로를 깨지 않음).

**수용 기준**: openai gateway가 128k capability를 사용하고, mock 경로·기존 테스트는 불변.

---

### T3 — persistent 핸들: Rust FFI + JNI + C ABI

**파일**: `src/mobile.rs`(헬퍼), `src/mobile_jni.rs`(JNI), `src/mobile_ffi.rs`(C ABI)

#### T3a — mobile.rs 핸들 헬퍼
```rust
/// config JSON으로 MobileAi를 만든다. 파싱·구성 실패는 None(호출측 폴백).
pub fn create_mobile_ai(ai_config_json: &str) -> Option<Box<MobileAi>> {
    serde_json::from_str::<MobileAiConfig>(ai_config_json)
        .ok()
        .and_then(|cfg| MobileAi::from_config(&cfg).ok())
        .map(Box::new)
}

/// 이미 만든 MobileAi 핸들로 한 줄을 평가한다(gateway 캐시 유지).
pub fn eval_line_ai_handle_json(ai: &MobileAi, input: &str, state_json: &str) -> String {
    let state = serde_json::from_str::<MobileSessionState>(state_json)
        .unwrap_or_else(|_| MobileShell::new().state());
    let mut shell = MobileShell::from_state(state);
    serialize_result(&shell.eval_line_with_ai(input, ai))
}
```
- 기존 stateless `eval_line_ai_json`(mobile.rs:62)은 **유지**(하위호환).

#### T3b — mobile_jni.rs (Android JNI) — 3함수 신설
```rust
// nativeCreateAi(configJson) -> jlong (실패 0)
//   create_mobile_ai → Box::into_raw(box) as jlong; None이면 0.
// nativeEvalLineAiHandle(handle: jlong, input, stateJson) -> jstring
//   handle==0이면 mobile::error_result_json(...); else &*(handle as *const MobileAi)로 eval.
// nativeDestroyAi(handle: jlong)
//   handle!=0이면 drop(Box::from_raw(handle as *mut MobileAi)).
```
- 안전성: 핸들은 `Box<MobileAi>`의 raw pointer. `create`가 leak, `destroy`가 반환. 이중 destroy·잘못된
  포인터는 Kotlin 계약으로 방지(0 sentinel + close-once).

#### T3c — mobile_ffi.rs (iOS C ABI) — 대칭 3함수
```rust
// ai_terminal_mobile_create_ai(config: *const c_char) -> *mut MobileAi (실패 null)
// ai_terminal_mobile_eval_line_ai_handle(ai: *mut MobileAi, input, state) -> *mut c_char
// ai_terminal_mobile_destroy_ai(ai: *mut MobileAi)
```
- `# Safety` 문서 주석 필수(기존 mobile_ffi 스타일). null 핸들 방어.
- 기존 stateless `ai_terminal_mobile_eval_line_ai_json`은 유지(하위호환).

**테스트**:
- mobile.rs: `handle_eval_reuses_same_instance` — `create_mobile_ai("{}")`(mock) 핸들에 같은 자연어를
  2회 `eval_line_ai_handle_json` → 둘 다 `kind="answered"`, 패닉·누수 없음(핸들 1개 재사용이 정상 동작).
  (gateway 캐시 로직 자체는 gateway.rs 테스트가 이미 커버 — 여기선 핸들 재사용 안전성만 계약.)
- mobile.rs: `create_mobile_ai_rejects_invalid_json` — `"{not-json"` → `None`.
- mobile_ffi.rs: `c_abi_handle_create_eval_destroy` — create→eval(answered)→destroy 왕복, null config는
  null 핸들, null 핸들 eval은 error result.
- mobile_jni.rs: JNI는 실기기/에뮬레이터 계약이라 순수 단위테스트 어려움 → 로직은 mobile.rs 헬퍼에서
  검증하고 JNI는 얇은 위임만 유지.

**수용 기준**: 핸들 왕복이 메모리 안전하고, 같은 핸들 재사용 시 매번 재구성하지 않는다.

---

### T4 — Kotlin 핸들 전환 (ShellBridge + ShellWorker)

**파일**: `android/app/src/main/java/dev/aiterminal/android/ShellBridge.kt`, `ShellWorker.kt`

#### T4a — ShellBridge 인터페이스 확장 (default로 하위호환)
```kotlin
interface ShellBridge {
    fun evalLine(input: String, state: ShellState): ShellEvalResult
    fun evalLineAi(input: String, state: ShellState, aiConfig: ShellAiConfig): ShellEvalResult =
        evalLine(input, state)

    // 핸들 기반(persistent). default 구현은 미지원(0/no-op)이라 fake bridge 하위호환.
    fun createAi(config: ShellAiConfig): Long = 0L
    fun evalLineAiHandle(handle: Long, input: String, state: ShellState): ShellEvalResult =
        evalLine(input, state)
    fun destroyAi(handle: Long) {}
}
```
- `NativeShellBridge`: `external fun nativeCreateAi(configJson: String): Long`,
  `nativeEvalLineAiHandle(handle: Long, input: String, stateJson: String): String`,
  `nativeDestroyAi(handle: Long)` 추가 + 인터페이스 구현(기존 `encodeAiConfig`/`decodeResult` 재사용,
  `UnsatisfiedLinkError`/`RuntimeException` 방어는 기존 패턴 동일). `createAi` 예외 시 0 반환.

#### T4b — ShellWorker 핸들 생명주기
- `aiConfig: ShellAiConfig?` setter를 **executor(single-thread)에 reconfigure 작업 post**하도록 변경:
  - config 설정/변경: executor에서 기존 핸들 `destroyAi` → `createAi(config)` → 핸들 보관.
  - config=null: executor에서 기존 핸들 destroy, 핸들 0.
  - `close()`: executor에 핸들 destroy 후 shutdown.
- eval 경로(`submitStreaming`): executor 스레드에서 현재 핸들이 있으면 `evalLineAiHandle(handle, ...)`,
  없으면 기존 `evalLine`. **핸들 생성·사용·파괴가 모두 executor 스레드라 race 없음.**
- 핸들은 `private var aiHandle: Long = 0L` 한 필드에 보관하고 **create/destroy/eval(읽기·쓰기) 전부
  executor 스레드에서만** 수행한다(setter는 값만 저장하고 reconfigure를 executor에 post — 필드를 직접
  건드리지 않음). 다른 스레드는 핸들에 접근하지 않으므로 별도 동기화 불필요.

**테스트**(ShellWorkerTest 계열, fake bridge):
- `worker_creates_handle_when_ai_config_set`: aiConfig 설정 시 fake bridge의 createAi 1회 호출·핸들 보관.
- `worker_recreates_handle_on_config_change`: config 교체 시 old destroy + new create.
- `worker_destroys_handle_on_close_and_null`: close·null 설정 시 destroy 호출.
- `worker_uses_handle_path_for_eval`: 핸들 있으면 evalLineAiHandle 경로 사용.
- 기존 `TerminalViewModelAiTest`가 여전히 통과(폴백·mock 경로 불변).

**수용 기준**: 세션 동안 핸들 1개가 재사용되고, config 변경·종료 시 정확히 정리된다.

---

### T5 — OkHttp JNI transport

**파일**: `android/app/src/main/java/dev/aiterminal/android/NativeHttp.kt`(신규),
`android/app/build.gradle.kts`, `src/mobile_http.rs`(신규, android 게이트), `src/lib.rs`

#### T5a — Kotlin NativeHttp (OkHttp 동기)
```kotlin
package dev.aiterminal.android
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.MediaType.Companion.toMediaType
import java.util.concurrent.TimeUnit

object NativeHttp {
    private val JSON = "application/json; charset=utf-8".toMediaType()
    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .callTimeout(75, TimeUnit.SECONDS)   // 전체 호출 상한(D2 취소 대체)
        .build()

    /** Rust JniHttpTransport가 JNI로 호출. 실패는 예외(Rust가 Err로 변환). */
    @JvmStatic
    fun postJson(url: String, body: String, bearer: String?): String {
        val builder = Request.Builder().url(url).post(body.toRequestBody(JSON))
        if (bearer != null) builder.header("Authorization", "Bearer $bearer")
        client.newCall(builder.build()).execute().use { resp ->
            val text = resp.body?.string() ?: ""
            if (!resp.isSuccessful) throw java.io.IOException("HTTP ${resp.code}: $text")
            return text
        }
    }
}
```
- `build.gradle.kts` dependencies에 `implementation("com.squareup.okhttp3:okhttp:5.3.0")` 추가.
  (OkHttp 5.3.0 = 최신 안정판, Context7 확인. minSdk/compileSdk 35와 호환.)

#### T5b — Rust JniHttpTransport (android 전용)
```rust
// src/mobile_http.rs  (#![cfg(target_os = "android")])
static JAVA_VM: OnceLock<jni::JavaVM> = OnceLock::new();

#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: jni::JavaVM, _reserved: *mut c_void) -> jni::sys::jint {
    let _ = JAVA_VM.set(vm);
    jni::sys::JNI_VERSION_1_6
}

pub struct JniHttpTransport;  // ZST — 상태는 전역 VM

impl crate::http::HttpTransport for JniHttpTransport {
    async fn post_json(&self, url: &str, body: &str, bearer: Option<&str>) -> anyhow::Result<String> {
        // 1) JAVA_VM.get() → attach_current_thread (이미 attach된 JNI 호출 스레드라 cheap)
        // 2) NativeHttp 클래스 find, postJson(String,String,String?) static 호출
        //    - bearer는 Option → JObject(null 또는 new_string)
        // 3) env.exception_check() 시 exception_describe/clear 후 Err
        // 4) 반환 JString → String
    }
}
```
- 예외 처리: JNI 호출 후 반드시 `exception_check`. 발생 시 `exception_clear` + 메시지로 `Err`
  → gateway가 `RequestError::Failed` → outcome "unavailable"(fail-soft, 셸 안 막음).
- `lib.rs`에 `#[cfg(target_os = "android")] pub mod mobile_http;` 추가.

#### T5c — 클래스/메서드 캐싱
- 1차 구현은 매 호출 `find_class`/`get_static_method_id`로 단순하게. 성능 최적화(전역 캐시)는 후속.

**테스트**:
- NativeHttp: 순수 인코딩/요청 형태 단위테스트는 OkHttp 실호출 없이 어려움 → 계약(헤더·본문 형태)은
  코드 리뷰로, 실 호출은 실기기. 최소한 `postJson` 시그니처·bearer 분기 컴파일 검증.
- Rust: `mobile_http`는 android 전용이라 데스크톱 test에서 제외. aarch64 `cargo check`로 컴파일 보장.
- non-android: `from_config`가 openai면 여전히 `UnavailableTransport`(T6) → 기존
  `openai_without_real_transport_is_unavailable` 테스트 유지.

**수용 기준**: android aarch64 빌드에 JNI_OnLoad·JniHttpTransport가 포함되고 컴파일된다.
실 응답은 실기기 검증(T8 절차).

---

### T6 — from_config transport 결선 (Rust)

**파일**: `src/mobile_ai.rs`

- `MobileAi::from_config`를 타깃별 분기:
  ```rust
  pub fn from_config(cfg: &MobileAiConfig) -> anyhow::Result<MobileAi> {
      #[cfg(target_os = "android")]
      { Self::with_gateway(build_gateway(cfg, crate::mobile_http::JniHttpTransport)) }
      #[cfg(not(target_os = "android"))]
      { Self::with_gateway(build_gateway(cfg, UnavailableTransport)) }
  }
  ```
- mock provider는 `build_gateway`에서 transport를 무시(`Gateway::mock()`)하므로 android/non-android 동일.
  openai provider만 실제 transport(android=Jni, non-android=Unavailable) 사용.
- `UnavailableTransport`는 non-android 경로에서 계속 필요하므로 유지.

**테스트**: non-android 기존 4개 mobile_ai 테스트 전부 유지(특히 openai=unavailable, mock=answered).

**수용 기준**: 데스크톱 동작 불변, android만 openai 실 transport 결선.

---

### T7 — 디버그 전용 config 주입 (Kotlin, DEBUG only)

**파일**: `android/app/src/main/java/dev/aiterminal/android/TerminalViewModel.kt`
(+ 필요 시 작은 로더 헬퍼)

- AI 토글 지점(`TerminalViewModel.kt:53`)에서 분기:
  ```kotlin
  worker.aiConfig = if (aiEnabled) resolveAiConfig() else null
  // resolveAiConfig(): BuildConfig.DEBUG && /sdcard/Download/ai-terminal-ai-config.json 존재 시
  //   그 파일을 ShellAiConfig(openai)로 파싱, 아니면 ShellAiConfig()(mock).
  ```
- config 파일 형식(문서화): `{"provider":"openai","model":"gpt-4o-mini","openai_url":"https://api.openai.com","api_key":"sk-..."}`.
- 릴리스 빌드(`!BuildConfig.DEBUG`)는 무조건 mock — 프로덕션 UX 불변. api_key는 앱이 저장하지 않고
  호출 시에만 전달(파일→config→JNI→Rust, 로그 redaction은 T1).
- 파일 읽기 실패·파싱 실패는 mock 폴백(fail-soft).
- 상태 문자열(`:57` "mock provider…")도 실제 provider 반영하도록 소폭 조정.

**테스트**: `resolveAiConfig`의 파싱/폴백 로직 단위테스트(파일 접근은 추상화해 주입). DEBUG 게이트는
`BuildConfig.DEBUG`라 단위테스트는 파서 계약 위주.

**수용 기준**: DEBUG에서 파일 있으면 openai, 없거나 릴리스면 mock. 파싱 실패는 mock 폴백.

---

### T8 — 실기기 검증 문서

**파일**: `docs/` 신규(예: `docs/android-real-transport-device-verification.md`)

절차 문서화:
1. DEBUG APK 빌드/설치(`build-rust-jni.sh` + gradle assembleDebug).
2. `/sdcard/Download/ai-terminal-ai-config.json`에 실 openai config(사용자 키) 배치.
3. 앱에서 AI 토글 ON → 자연어 입력(예: "큰 파일 찾아줘").
4. 확인 항목: openai 실 응답(mock echo 아님)·응답 지연·오류 시 fail-soft(셸 계속 동작)·
   로그에 api_key 미노출(T1)·핸들 재사용(세션 중 재구성 없음).
5. 네트워크 차단/키 오류 시 "unavailable" outcome으로 흡수(§3-3).

---

## 6. 검증 전략

| 계층 | 이 세션 (자동) | 실기기 (사용자 후속) |
|---|---|---|
| Rust 단위 | T1 redaction, T2 capability, T3 핸들 왕복, T6 non-android unavailable | — |
| Rust 빌드 | WSL fmt·clippy(무피처 + `storage tls remote`)·test·**aarch64 check**(T5 컴파일) | — |
| Kotlin 단위 | T4 핸들 생명주기(fake bridge), T7 config 파서 (gradle testDebugUnitTest) | — |
| 통합 CI | GitHub CI 4잡(android JNI 4 ABI 패키징 포함) | — |
| E2E | — | T8 절차: openai 실 응답·fail-soft·redaction·핸들 재사용 |

**WSL 검증 조합**(1차 교훈 재적용): CI 미러 = 무피처 clippy/test + `storage tls remote` + aarch64 check.
CI가 무피처 `clippy --all-targets`도 돌리므로 feature-gated 모듈의 orphan import 주의.

## 7. 경계 규율 & 리스크

### 경계
- **shellcore pure 불변** — AI는 mobile 계층에만. `eval_line_with_ai`는 dispatch가 Ai로 라우팅한
  자연어만 제안하고 셸 상태 미변경(mobile.rs:173 계약 유지).
- **§3-11 자동 실행 금지** — 제안 텍스트만. transport가 실 응답을 받아도 실행 경로 없음.
- **OkHttp JNI 역호출은 `mobile_http.rs`(android 게이트)에만** — non-android/데스크톱 불변.
- **api_key 미저장** — 호출 시 전달, Debug redaction(T1)으로 로그 유출 차단.

### 리스크
| 리스크 | 완화 |
|---|---|
| 핸들 누수/이중 free | 0/null sentinel + Kotlin close-once + executor 직렬화. Rust 왕복 테스트. |
| blocking JNI가 취소 무력화(D2) | OkHttp `callTimeout` 상한 + fail-soft. 수용된 트레이드오프. |
| JNI_OnLoad/attach 실패 | `exception_check` + Err→unavailable(셸 안 막음). 실기기 검증 필수. |
| OkHttp 5.x 의존 추가로 APK/빌드 영향 | 5.3.0 안정판·Android 표준. gradle 4 ABI 패키징 CI로 확인. |
| aarch64 크로스컴파일 깨짐 | Task마다 `cargo check --target aarch64-linux-android` 선행(1차 게이트 교훈). |

## 8. 커밋 / PR 계획

- 브랜치: `feat/android-real-transport`(develop 분기). 격리 워크트리 권장(1차 관례).
- 커밋: T1 → T2 → T3 → T4 → T5 → T6 → T7 → T8 순(논리적 분리).
- PR: `feat/android-real-transport` → develop 1건. CI 4잡 green 확인 후 사용자 승인 머지.
- develop→main 릴리스는 별도(사용자 결정).

## 9. 미해결 / 후속
- OkHttp 클래스/메서드 ID 전역 캐싱(성능 최적화) — 후속.
- 프로덕션 AI 설정 UI(provider/key 입력) — 별도 슬라이스.
- streaming 응답 — 별도.
- 실 openai 검증 결과 반영(성공 시 이 spec에 실측 추가) — 사용자 후속.
