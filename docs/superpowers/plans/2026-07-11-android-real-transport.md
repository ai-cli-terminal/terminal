# 실 transport 슬라이스 (Android AI 2차) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Android AI 보조에 실 openai HTTP transport(OkHttp JNI)·지속 핸들·api_key redaction·openai capability를 더해 실기기에서 실 응답을 받게 한다.

**Architecture:** 앞 3개(redaction/capability/persistent 핸들)는 순수 Rust+FFI+Kotlin으로 CI 완결. HTTP는 Android 시스템 TLS를 쓰기 위해 Kotlin OkHttp에서 실행하고 Rust `JniHttpTransport`가 JNI로 동기 역호출한다. `MobileAi`는 핸들로 세션 내내 재사용되어 gateway 캐시를 유지한다.

**Tech Stack:** Rust(jni 0.21, tokio current-thread, serde), Kotlin(OkHttp 5.3.0, org.json), Android JNI/C ABI cdylib.

**정본 spec:** `docs/superpowers/specs/2026-07-11-android-real-transport-design.md`

## Global Constraints

각 Task의 요구사항에는 아래가 암묵적으로 포함된다(spec에서 verbatim):

- **브랜치**: `feat/android-real-transport`(develop 분기, 이미 생성 · spec 커밋 `921b2be`). main/develop 직접 커밋 금지.
- **OkHttp 의존**: `com.squareup.okhttp3:okhttp:5.3.0` (최신 안정판, Context7 확인).
- **openai capability**: `max_context_tokens: 128_000` (실효는 truncation 한도뿐).
- **디버그 config 파일**: `/sdcard/Download/ai-terminal-ai-config.json` (DEBUG 빌드 전용).
- **§3-11 불변**: AI는 제안 텍스트만 — 어떤 경로도 명령을 실행하지 않는다. shellcore pure 유지.
- **api_key**: 앱이 저장하지 않음(호출 시 전달) + Debug redaction으로 로그 유출 차단.
- **하위호환**: 기존 stateless `eval_line_ai_json`/`nativeEvalLineAi`/`ai_terminal_mobile_eval_line_ai_json`는 유지(iOS 등 소비자). Kotlin만 핸들 경로로 전환.
- **경계**: OkHttp JNI 역호출은 `mobile_http.rs`(android 게이트)에만 — non-android/데스크톱 불변.
- **Rust 검증 명령**(WSL, `<repo>`=`/mnt/d/workspace/terminal-project/terminal`, 워크트리면 그 경로):
  ```
  wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cargo cmd> && echo PASS || echo FAIL'
  ```
  - 판정은 **반드시 `&& echo PASS || echo FAIL`**(하니스에서 `echo $?`는 항상 0 · 판정 명령에 파이프 금지).
  - fmt는 `--check`가 아니라 실제 `cargo fmt --all` 후 확인.
- **aarch64 check**(mobile_http/JNI 컴파일 게이트): `cargo check --lib --target aarch64-linux-android`.
- **Android 검증 명령**(Windows, gradle wrapper): `cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest`.
- **git**: `git add -A` 금지 — 변경 파일만 명시적 add(`.omc/` 등 오커밋 방지).

---

## File Structure

| 파일 | 책임 | Task |
|---|---|---|
| `src/mobile_ai.rs` (modify) | `MobileAiConfig` Debug redaction, `build_gateway` openai cap 분기, `from_config` transport 결선 | 1, 2, 9 |
| `src/provider.rs` (modify) | `Provider::openai()` capability | 2 |
| `src/mobile.rs` (modify) | `create_mobile_ai`, `eval_line_ai_handle_json` 핸들 헬퍼 | 3 |
| `src/mobile_ffi.rs` (modify) | C ABI create/eval_handle/destroy (iOS 대칭) | 4 |
| `src/mobile_jni.rs` (modify) | JNI create/eval_handle/destroy | 5 |
| `src/mobile_http.rs` (create) | `JniHttpTransport` + `JNI_OnLoad` (android 게이트) | 9 |
| `src/lib.rs` (modify) | `mobile_http` 모듈 선언 | 9 |
| `android/.../ShellBridge.kt` (modify) | 핸들 인터페이스 + `NativeShellBridge` external | 6 |
| `android/.../ShellWorker.kt` (modify) | 핸들 생명주기(executor 직렬화) | 7 |
| `android/.../NativeHttp.kt` (create) | OkHttp 동기 postJson | 8 |
| `android/app/build.gradle.kts` (modify) | OkHttp 5.3.0 의존 | 8 |
| `android/.../TerminalViewModel.kt` (modify) | DEBUG config 주입 | 10 |
| `docs/android-real-transport-device-verification.md` (create) | 실기기 검증 절차 | 11 |

---

## Task 1: api_key Debug redaction

**Files:**
- Modify: `src/mobile_ai.rs:25` (derive), 뒤에 `impl Debug` 추가
- Test: `src/mobile_ai.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: 없음
- Produces: `MobileAiConfig`가 수동 `Debug`(api_key를 `<redacted>`로 표기). 시그니처·필드 불변.

- [ ] **Step 1: 실패 테스트 작성** — `src/mobile_ai.rs`의 `mod tests`에 추가:

```rust
    #[test]
    fn api_key_is_redacted_in_debug() {
        let cfg = MobileAiConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-super-secret-value".to_string()),
            ..Default::default()
        };
        let rendered = format!("{cfg:?}");
        assert!(!rendered.contains("sk-super-secret-value"), "api_key leaked: {rendered}");
        assert!(rendered.contains("<redacted>"), "{rendered}");
    }

    #[test]
    fn debug_shows_none_api_key_as_none() {
        let cfg = MobileAiConfig::default();
        assert!(format!("{cfg:?}").contains("api_key: None"), "{cfg:?}");
    }
```

- [ ] **Step 2: 실패 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib mobile_ai::tests::api_key_is_redacted_in_debug && echo PASS || echo FAIL'
```
예상: 컴파일은 되나 `FAIL`(파생 Debug가 키를 노출) — 실제로는 `derive(Debug)`가 남아 있으면 PASS일 수 있으니 Step 3 먼저 없이 확인. 파생 상태에선 `<redacted>` 미포함으로 FAIL.

- [ ] **Step 3: 구현** — `src/mobile_ai.rs:25`의 derive에서 `Debug` 제거 후 수동 impl 추가:

```rust
// 변경 전: #[derive(Debug, Clone, PartialEq, Deserialize)]
#[derive(Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct MobileAiConfig {
    // ... 필드 불변 ...
}

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

- [ ] **Step 4: 통과 확인 + fmt**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; cargo test --lib mobile_ai::tests:: && echo PASS || echo FAIL'
```
예상: `PASS` (신규 2개 + 기존 mobile_ai 테스트 전부).

- [ ] **Step 5: 커밋**

```
git -C <repo> add src/mobile_ai.rs
git -C <repo> commit -m "feat(mobile): redact api_key in MobileAiConfig Debug"
```

---

## Task 2: openai 실 capability

**Files:**
- Modify: `src/provider.rs` (`impl Provider`에 `openai()` 추가)
- Modify: `src/mobile_ai.rs:73-87` (`build_gateway` openai 분기 cap 교체)
- Test: `src/provider.rs`, `src/mobile_ai.rs`

**Interfaces:**
- Consumes: `ModelCapability`, `Provider` (provider.rs 기존)
- Produces: `Provider::openai() -> Provider` (models[0].max_context_tokens = 128_000)

- [ ] **Step 1: 실패 테스트 작성** — `src/provider.rs`의 `mod tests`에 추가:

```rust
    #[test]
    fn openai_capability_has_larger_context_than_mock() {
        let openai = &Provider::openai().models[0];
        let mock = &Provider::mock().models[0];
        assert_eq!(openai.max_context_tokens, 128_000);
        assert!(openai.max_context_tokens > mock.max_context_tokens);
        assert_eq!(Provider::openai().name, "openai");
    }
```

- [ ] **Step 2: 실패 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib provider::tests::openai_capability_has_larger_context_than_mock && echo PASS || echo FAIL'
```
예상: `FAIL` — `no function or associated item named 'openai'`.

- [ ] **Step 3: 구현** — `src/provider.rs`의 `impl Provider`(mock 옆)에 추가:

```rust
    /// OpenAI 호환 capability(§31.9). gateway 실효는 max_context_tokens(truncation)뿐.
    pub fn openai() -> Provider {
        Provider {
            name: "openai".into(),
            display_name: "OpenAI".into(),
            models: vec![ModelCapability {
                name: "openai-default".into(),
                max_context_tokens: 128_000,
                max_output_tokens: 4_096,
                supports_streaming: true,
                supports_json_mode: true,
                supports_tool_use: false,
                supports_token_counting: true,
                supports_usage_reporting: true,
                supports_context_caching: false,
            }],
        }
    }
```

`src/mobile_ai.rs`의 `build_gateway`(현재 line 73-87)에서 cap을 provider별로:

```rust
pub fn build_gateway<T: HttpTransport + 'static>(cfg: &MobileAiConfig, transport: T) -> Gateway {
    match cfg.provider.as_str() {
        "openai" => Gateway::new(
            Box::new(OpenAiBackend::new(
                transport,
                &cfg.openai_url,
                &cfg.model,
                cfg.api_key.clone(),
            )),
            Provider::openai().models[0].clone(),
        ),
        _ => Gateway::mock(),
    }
}
```
(상단 `use crate::provider::Provider;`는 이미 존재 — 확인.)

- [ ] **Step 4: 통과 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; cargo test --lib provider::tests:: mobile_ai::tests:: && echo PASS || echo FAIL'
```
예상: `PASS` (신규 + 기존 `openai_provider_with_mock_transport_answers` 유지).

- [ ] **Step 5: 커밋**

```
git -C <repo> add src/provider.rs src/mobile_ai.rs
git -C <repo> commit -m "feat(mobile): use real openai capability in build_gateway"
```

---

## Task 3: persistent 핸들 — Rust 헬퍼

**Files:**
- Modify: `src/mobile.rs` (핸들 헬퍼 2개 추가)
- Test: `src/mobile.rs`

**Interfaces:**
- Consumes: `MobileAi::from_config`, `MobileAiConfig`, `MobileShell`, `serialize_result`, `eval_line_with_ai` (mobile.rs/mobile_ai.rs 기존)
- Produces:
  - `create_mobile_ai(ai_config_json: &str) -> Option<Box<MobileAi>>`
  - `eval_line_ai_handle_json(ai: &MobileAi, input: &str, state_json: &str) -> String`

- [ ] **Step 1: 실패 테스트 작성** — `src/mobile.rs`의 `mod tests`에 추가:

```rust
    #[test]
    fn create_mobile_ai_rejects_invalid_json() {
        assert!(create_mobile_ai("{not-json").is_none());
    }

    #[test]
    fn handle_eval_reuses_same_instance() {
        let ai = create_mobile_ai("{}").expect("mock ai");
        let state = initial_state_json();
        // 같은 핸들로 2회 평가 — 재구성 없이 정상 응답(핸들 재사용 안전성).
        for _ in 0..2 {
            let raw = eval_line_ai_handle_json(&ai, "큰 파일 찾아줘", &state);
            let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();
            assert!(result.ok, "{result:?}");
            assert_eq!(result.ai.as_ref().map(|a| a.kind.as_str()), Some("answered"), "{result:?}");
        }
    }
```

- [ ] **Step 2: 실패 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib mobile::tests::handle_eval_reuses_same_instance && echo PASS || echo FAIL'
```
예상: `FAIL` — `cannot find function create_mobile_ai`.

- [ ] **Step 3: 구현** — `src/mobile.rs`에 `use crate::mobile_ai::{MobileAi, MobileAiConfig, MobileAiOutcome};`가 이미 있음(line 14). 핸들 헬퍼를 `eval_line_ai_json`(line 62) 근처에 추가:

```rust
/// config JSON으로 MobileAi를 만든다. 파싱·구성 실패는 None(호출측 폴백, §3-3).
pub fn create_mobile_ai(ai_config_json: &str) -> Option<Box<MobileAi>> {
    serde_json::from_str::<MobileAiConfig>(ai_config_json)
        .ok()
        .and_then(|cfg| MobileAi::from_config(&cfg).ok())
        .map(Box::new)
}

/// 이미 만든 MobileAi 핸들로 한 줄을 평가한다(gateway 캐시 유지 — per-call 재생성 없음).
pub fn eval_line_ai_handle_json(ai: &MobileAi, input: &str, state_json: &str) -> String {
    let state = serde_json::from_str::<MobileSessionState>(state_json)
        .unwrap_or_else(|_| MobileShell::new().state());
    let mut shell = MobileShell::from_state(state);
    serialize_result(&shell.eval_line_with_ai(input, ai))
}
```

- [ ] **Step 4: 통과 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; cargo test --lib mobile::tests:: && echo PASS || echo FAIL'
```
예상: `PASS`.

- [ ] **Step 5: 커밋**

```
git -C <repo> add src/mobile.rs
git -C <repo> commit -m "feat(mobile): add persistent MobileAi handle helpers"
```

---

## Task 4: persistent 핸들 — C ABI (iOS 대칭)

**Files:**
- Modify: `src/mobile_ffi.rs` (create/eval_handle/destroy 추가)
- Test: `src/mobile_ffi.rs`

**Interfaces:**
- Consumes: `mobile::create_mobile_ai`, `mobile::eval_line_ai_handle_json`, `MobileAi`
- Produces:
  - `ai_terminal_mobile_create_ai(config: *const c_char) -> *mut MobileAi`
  - `ai_terminal_mobile_eval_line_ai_handle(ai: *mut MobileAi, input, state) -> *mut c_char`
  - `ai_terminal_mobile_destroy_ai(ai: *mut MobileAi)`

- [ ] **Step 1: 실패 테스트 작성** — `src/mobile_ffi.rs`의 `mod tests`에 추가:

```rust
    #[test]
    fn c_abi_handle_create_eval_destroy() {
        let cfg = CString::new("{}").unwrap();
        let ai = unsafe { ai_terminal_mobile_create_ai(cfg.as_ptr()) };
        assert!(!ai.is_null());

        let input = CString::new("큰 파일 찾아줘").unwrap();
        let state = CString::new(crate::mobile::initial_state_json()).unwrap();
        let raw = unsafe {
            take_owned_json(ai_terminal_mobile_eval_line_ai_handle(
                ai, input.as_ptr(), state.as_ptr(),
            ))
        };
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();
        assert_eq!(result.ai.as_ref().map(|a| a.kind.as_str()), Some("answered"), "{result:?}");

        unsafe { ai_terminal_mobile_destroy_ai(ai) };
    }

    #[test]
    fn c_abi_create_ai_null_config_returns_null() {
        let ai = unsafe { ai_terminal_mobile_create_ai(std::ptr::null()) };
        assert!(ai.is_null());
    }

    #[test]
    fn c_abi_eval_handle_null_handle_is_error_result() {
        let input = CString::new("x").unwrap();
        let state = CString::new(crate::mobile::initial_state_json()).unwrap();
        let raw = unsafe {
            take_owned_json(ai_terminal_mobile_eval_line_ai_handle(
                std::ptr::null_mut(), input.as_ptr(), state.as_ptr(),
            ))
        };
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();
        assert!(!result.ok);
    }
```

- [ ] **Step 2: 실패 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib mobile_ffi::tests::c_abi_handle_create_eval_destroy && echo PASS || echo FAIL'
```
예상: `FAIL` — 함수 미정의.

- [ ] **Step 3: 구현** — `src/mobile_ffi.rs`에 추가(`use crate::mobile;` 기존, `use crate::mobile_ai::MobileAi;` 필요):

```rust
use crate::mobile_ai::MobileAi;

/// config JSON으로 지속 MobileAi 핸들을 만든다. 실패 시 null.
///
/// # Safety
/// `config`는 유효한 NUL 종단 UTF-8 포인터여야 한다. 반환 핸들은
/// [`ai_terminal_mobile_destroy_ai`]로만 해제한다.
#[no_mangle]
pub unsafe extern "C" fn ai_terminal_mobile_create_ai(config: *const c_char) -> *mut MobileAi {
    let config = match c_arg_to_string(config, "config") {
        Ok(value) => value,
        Err(_) => return std::ptr::null_mut(),
    };
    match mobile::create_mobile_ai(&config) {
        Some(ai) => Box::into_raw(ai),
        None => std::ptr::null_mut(),
    }
}

/// 지속 핸들로 한 줄을 평가한다. null 핸들은 구조화 오류 결과를 돌려준다.
///
/// # Safety
/// `ai`는 [`ai_terminal_mobile_create_ai`]가 준 유효 핸들(또는 null)이어야 하고,
/// `input`/`state_json`은 유효한 NUL 종단 UTF-8 포인터여야 한다. 반환 포인터는
/// [`ai_terminal_mobile_free_string`]로 해제한다.
#[no_mangle]
pub unsafe extern "C" fn ai_terminal_mobile_eval_line_ai_handle(
    ai: *mut MobileAi,
    input: *const c_char,
    state_json: *const c_char,
) -> *mut c_char {
    if ai.is_null() {
        return string_to_c_ptr(mobile::error_result_json("ai handle was null"));
    }
    let input = match c_arg_to_string(input, "input") {
        Ok(value) => value,
        Err(error_json) => return string_to_c_ptr(error_json),
    };
    let state_json = match c_arg_to_string(state_json, "state_json") {
        Ok(value) => value,
        Err(error_json) => return string_to_c_ptr(error_json),
    };
    let ai_ref: &MobileAi = unsafe { &*ai };
    string_to_c_ptr(mobile::eval_line_ai_handle_json(ai_ref, &input, &state_json))
}

/// 지속 핸들을 해제한다.
///
/// # Safety
/// `ai`는 null이거나 [`ai_terminal_mobile_create_ai`]가 준 핸들이어야 하며,
/// 같은 핸들을 두 번 해제하면 UB다.
#[no_mangle]
pub unsafe extern "C" fn ai_terminal_mobile_destroy_ai(ai: *mut MobileAi) {
    if ai.is_null() {
        return;
    }
    unsafe { drop(Box::from_raw(ai)) };
}
```

- [ ] **Step 4: 통과 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; cargo test --lib mobile_ffi::tests:: && echo PASS || echo FAIL'
```
예상: `PASS` (신규 3개 + 기존 C ABI 테스트 유지).

- [ ] **Step 5: 커밋**

```
git -C <repo> add src/mobile_ffi.rs
git -C <repo> commit -m "feat(mobile): add persistent AI handle to C ABI"
```

---

## Task 5: persistent 핸들 — JNI

**Files:**
- Modify: `src/mobile_jni.rs` (create/eval_handle/destroy 추가)
- 검증: aarch64 `cargo check` (JNI는 실기기 계약 — 순수 단위테스트 없음)

**Interfaces:**
- Consumes: `mobile::create_mobile_ai`, `mobile::eval_line_ai_handle_json`, `MobileAi`, `jni` (기존 import)
- Produces (Kotlin `NativeShellBridge`가 선언):
  - `nativeCreateAi(configJson: JString) -> jlong` (실패 0)
  - `nativeEvalLineAiHandle(handle: jlong, input, stateJson) -> jstring`
  - `nativeDestroyAi(handle: jlong)`

- [ ] **Step 1: 구현** — `src/mobile_jni.rs`에 추가(상단 `use jni::sys::{jlong, jstring};`로 확장, `use crate::mobile_ai::MobileAi;` 추가):

```rust
use jni::sys::{jlong, jstring};
use crate::mobile_ai::MobileAi;

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_aiterminal_android_NativeShellBridge_nativeCreateAi(
    mut env: JNIEnv,
    _this: JObject,
    ai_config_json: JString,
) -> jlong {
    let config = match env.get_string(&ai_config_json).map(String::from) {
        Ok(value) => value,
        Err(_) => return 0,
    };
    match crate::mobile::create_mobile_ai(&config) {
        Some(ai) => Box::into_raw(ai) as jlong,
        None => 0,
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_aiterminal_android_NativeShellBridge_nativeEvalLineAiHandle(
    mut env: JNIEnv,
    _this: JObject,
    handle: jlong,
    input: JString,
    state_json: JString,
) -> jstring {
    let response = if handle == 0 {
        crate::mobile::error_result_json("ai handle was null")
    } else {
        match (
            env.get_string(&input).map(String::from),
            env.get_string(&state_json).map(String::from),
        ) {
            (Ok(input), Ok(state_json)) => {
                let ai: &MobileAi = unsafe { &*(handle as *const MobileAi) };
                crate::mobile::eval_line_ai_handle_json(ai, &input, &state_json)
            }
            (Err(err), _) | (_, Err(err)) => {
                crate::mobile::error_result_json(format!("failed to read JNI string: {err}"))
            }
        }
    };
    match env.new_string(response) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_aiterminal_android_NativeShellBridge_nativeDestroyAi(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) {
    if handle != 0 {
        unsafe { drop(Box::from_raw(handle as *mut MobileAi)) };
    }
}
```

- [ ] **Step 2: aarch64 컴파일 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; rustup target add aarch64-linux-android; cargo check --lib --target aarch64-linux-android && echo PASS || echo FAIL'
```
예상: `PASS` (JNI 3함수가 android 타깃에 컴파일).

- [ ] **Step 3: 데스크톱 회귀 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --lib --all-targets -- -D warnings && echo PASS || echo FAIL'
```
예상: `PASS` (mobile_jni는 android 게이트라 데스크톱 clippy엔 미포함 — 무피처 orphan 없음).

- [ ] **Step 4: 커밋**

```
git -C <repo> add src/mobile_jni.rs
git -C <repo> commit -m "feat(mobile): add persistent AI handle to JNI bridge"
```

---

## Task 6: Kotlin ShellBridge 핸들 인터페이스

**Files:**
- Modify: `android/app/src/main/java/dev/aiterminal/android/ShellBridge.kt`
- Test: `android/app/src/test/java/dev/aiterminal/android/ShellBridgeCodecTest.kt`

**Interfaces:**
- Consumes: `encodeAiConfig`, `encodeState`, `decodeResult` (ShellBridge.kt 기존)
- Produces (ShellWorker가 소비):
  - `ShellBridge.createAi(config: ShellAiConfig): Long` (default 0L)
  - `ShellBridge.evalLineAiHandle(handle: Long, input: String, state: ShellState): ShellEvalResult` (default → evalLine)
  - `ShellBridge.destroyAi(handle: Long)` (default no-op)

- [ ] **Step 1: 실패 테스트 작성** — `ShellBridgeCodecTest.kt`에 추가(fake bridge로 default 위임 검증):

```kotlin
    @Test
    fun defaultBridgeHandleMethodsAreNoOpFallback() {
        val bridge = object : ShellBridge {
            override fun evalLine(input: String, state: ShellState): ShellEvalResult =
                ShellEvalResult(ok = true, outputText = "ran:$input", outputJson = "null", error = null, state = state)
        }
        val state = ShellState()
        // default createAi=0, destroyAi no-op(throw 없음), evalLineAiHandle→evalLine 위임
        assertEquals(0L, bridge.createAi(ShellAiConfig()))
        bridge.destroyAi(0L)
        val result = bridge.evalLineAiHandle(0L, "x", state)
        assertEquals("ran:x", result.outputText)
    }
```

- [ ] **Step 2: 실패 확인**

```
cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest --tests "dev.aiterminal.android.ShellBridgeCodecTest"
```
예상: 컴파일 실패(`createAi`/`evalLineAiHandle`/`destroyAi` 미정의).

- [ ] **Step 3: 구현** — `ShellBridge.kt`의 `interface ShellBridge`(line 35-40)에 default 메서드 추가:

```kotlin
interface ShellBridge {
    fun evalLine(input: String, state: ShellState): ShellEvalResult

    fun evalLineAi(input: String, state: ShellState, aiConfig: ShellAiConfig): ShellEvalResult =
        evalLine(input, state)

    /** 지속 AI 핸들 생성. 미지원 구현은 0(호출측 폴백). */
    fun createAi(config: ShellAiConfig): Long = 0L

    /** 지속 핸들로 평가. 미지원 구현은 기존 evalLine으로 폴백. */
    fun evalLineAiHandle(handle: Long, input: String, state: ShellState): ShellEvalResult =
        evalLine(input, state)

    /** 지속 핸들 해제. 미지원 구현은 no-op. */
    fun destroyAi(handle: Long) {}
}
```

`NativeShellBridge`에 external + 구현 추가(기존 `evalLineAi` 패턴 재사용):

```kotlin
    override fun createAi(config: ShellAiConfig): Long {
        return try {
            loadNativeLibrary()
            nativeCreateAi(encodeAiConfig(config))
        } catch (error: UnsatisfiedLinkError) {
            0L
        } catch (error: RuntimeException) {
            0L
        }
    }

    override fun evalLineAiHandle(handle: Long, input: String, state: ShellState): ShellEvalResult {
        return try {
            loadNativeLibrary()
            decodeResult(nativeEvalLineAiHandle(handle, input, encodeState(state)), state)
        } catch (error: UnsatisfiedLinkError) {
            err("native shell library not loaded: ${error.message}", state)
        } catch (error: RuntimeException) {
            err("native shell bridge failed: ${error.message}", state)
        }
    }

    override fun destroyAi(handle: Long) {
        try {
            loadNativeLibrary()
            if (handle != 0L) nativeDestroyAi(handle)
        } catch (error: UnsatisfiedLinkError) {
            // 라이브러리 미로드면 해제할 것도 없음
        } catch (error: RuntimeException) {
            // 해제 실패는 무시(best-effort)
        }
    }

    private external fun nativeCreateAi(aiConfigJson: String): Long
    private external fun nativeEvalLineAiHandle(handle: Long, input: String, stateJson: String): String
    private external fun nativeDestroyAi(handle: Long)
```
(기존 `private external fun nativeEvalLine`/`nativeEvalLineAi`와 나란히.)

- [ ] **Step 4: 통과 확인**

```
cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest --tests "dev.aiterminal.android.ShellBridgeCodecTest"
```
예상: BUILD SUCCESSFUL, 신규 테스트 통과.

- [ ] **Step 5: 커밋**

```
git -C <repo> add android/app/src/main/java/dev/aiterminal/android/ShellBridge.kt android/app/src/test/java/dev/aiterminal/android/ShellBridgeCodecTest.kt
git -C <repo> commit -m "feat(android): add persistent AI handle to ShellBridge"
```

---

## Task 7: Kotlin ShellWorker 핸들 생명주기

**Files:**
- Modify: `android/app/src/main/java/dev/aiterminal/android/ShellWorker.kt`
- Test: `android/app/src/test/java/dev/aiterminal/android/ShellWorkerTest.kt`

**Interfaces:**
- Consumes: `ShellBridge.createAi/evalLineAiHandle/destroyAi` (Task 6)
- Produces: `ShellWorker.aiConfig` setter가 executor에서 핸들 생성/교체/파괴를 직렬화

- [ ] **Step 1: 실패 테스트 작성** — `ShellWorkerTest.kt`에 추가. 호출을 기록하는 fake bridge 사용(단일스레드 executor로 결정적 검증):

```kotlin
    private class RecordingBridge : ShellBridge {
        val created = java.util.concurrent.CopyOnWriteArrayList<ShellAiConfig>()
        val destroyed = java.util.concurrent.CopyOnWriteArrayList<Long>()
        val handleEvals = java.util.concurrent.CopyOnWriteArrayList<Long>()
        var nextHandle = 100L
        override fun evalLine(input: String, state: ShellState) =
            ShellEvalResult(ok = true, outputText = "plain:$input", outputJson = "null", error = null, state = state)
        override fun createAi(config: ShellAiConfig): Long { created.add(config); return nextHandle++ }
        override fun evalLineAiHandle(handle: Long, input: String, state: ShellState): ShellEvalResult {
            handleEvals.add(handle)
            return ShellEvalResult(ok = true, outputText = "ai:$input", outputJson = "null", error = null, state = state)
        }
        override fun destroyAi(handle: Long) { destroyed.add(handle) }
    }

    // 동기 executor: 제출 즉시 실행(테스트 결정성).
    private fun directExecutor(): java.util.concurrent.ExecutorService =
        java.util.concurrent.Executors.newSingleThreadExecutor()

    @Test
    fun settingAiConfigCreatesHandleAndEvalUsesIt() {
        val bridge = RecordingBridge()
        val worker = ShellWorker(bridge, executor = directExecutor(), resultPoster = { it() })
        worker.aiConfig = ShellAiConfig(provider = "openai")
        Thread.sleep(100) // executor 반영 대기
        assertEquals(1, bridge.created.size)

        val latch = java.util.concurrent.CountDownLatch(1)
        worker.submit("hello", ShellState()) { latch.countDown() }
        latch.await(2, java.util.concurrent.TimeUnit.SECONDS)
        assertEquals(1, bridge.handleEvals.size)
        assertEquals(100L, bridge.handleEvals[0])
    }

    @Test
    fun changingAiConfigDestroysOldHandleAndCreatesNew() {
        val bridge = RecordingBridge()
        val worker = ShellWorker(bridge, executor = directExecutor(), resultPoster = { it() })
        worker.aiConfig = ShellAiConfig(provider = "openai")
        worker.aiConfig = ShellAiConfig(provider = "mock")
        Thread.sleep(150)
        assertEquals(2, bridge.created.size)
        assertEquals(1, bridge.destroyed.size)
        assertEquals(100L, bridge.destroyed[0])
    }

    @Test
    fun nullAiConfigDestroysHandle() {
        val bridge = RecordingBridge()
        val worker = ShellWorker(bridge, executor = directExecutor(), resultPoster = { it() })
        worker.aiConfig = ShellAiConfig(provider = "openai")
        worker.aiConfig = null
        Thread.sleep(150)
        assertEquals(1, bridge.destroyed.size)
    }
```

- [ ] **Step 2: 실패 확인**

```
cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest --tests "dev.aiterminal.android.ShellWorkerTest"
```
예상: 컴파일/실행 실패(현재 `aiConfig`는 단순 `@Volatile var`, 핸들 없음).

- [ ] **Step 3: 구현** — `ShellWorker.kt`에서 `aiConfig`(line 26 `@Volatile var aiConfig: ShellAiConfig? = null`)를 custom setter + 핸들 필드로 교체:

```kotlin
    // aiHandle은 executor 스레드에서만 접근(직렬화 → race 없음).
    // currentAiConfig는 @Volatile로 동기 저장 — 호출 스레드가 setter 직후 즉시
    // readback해야 한다(TerminalViewModel.toggleAi가 worker.aiConfig를 동기 설정 후
    // 곧바로 읽는 계약). 핸들 create/destroy만 executor로 직렬화한다.
    private var aiHandle: Long = 0L

    @Volatile
    private var currentAiConfig: ShellAiConfig? = null

    /** 설정되면 pure eval이 AI 보조 경로로 간다. null이면 기존 그대로. */
    var aiConfig: ShellAiConfig?
        get() = currentAiConfig
        set(value) {
            currentAiConfig = value // 동기 저장(즉시 readback)
            executor.execute { reconfigureAiHandle(value) } // 핸들 조작만 직렬화
        }

    // executor 스레드 전용. 기존 핸들 파괴 후 새 config로 재생성.
    private fun reconfigureAiHandle(next: ShellAiConfig?) {
        if (aiHandle != 0L) {
            bridge.destroyAi(aiHandle)
            aiHandle = 0L
        }
        if (next != null) {
            aiHandle = bridge.createAi(next)
        }
    }
```

`submitStreaming`(line 43-51)의 eval 분기를 핸들 기반으로:

```kotlin
        executor.execute {
            val result = runCatching {
                val handle = aiHandle
                val config = currentAiConfig
                when {
                    // 지속 핸들 우선(이 슬라이스 목표: 실기기 openai 지속 핸들 경로)
                    handle != 0L -> bridge.evalLineAiHandle(handle, input, state)
                    // handle 미지원(createAi가 0 반환)인데 config는 설정됨 →
                    // legacy per-call evalLineAi 폴백(하위호환·fail-soft; Task 6 createAi
                    // doc "미지원 구현은 0(호출측 폴백)" 계약).
                    config != null -> bridge.evalLineAi(input, state, config)
                    else -> bridge.evalLine(input, state)
                }
            }
                .getOrElse { error -> /* 기존 error ShellEvalResult 그대로 */ }
            // ... 이하 기존 로직 불변 ...
```

`close()`(line 90-93)에 핸들 파괴 추가:

```kotlin
    fun close() {
        executor.execute {
            if (aiHandle != 0L) {
                bridge.destroyAi(aiHandle)
                aiHandle = 0L
            }
        }
        executor.shutdown()
        try { executor.awaitTermination(1, java.util.concurrent.TimeUnit.SECONDS) } catch (_: InterruptedException) {}
        (externalAdapterRef.getAndSet(null) as? AutoCloseable)?.close()
    }
```
(주의: 기존 `executor.shutdownNow()`를 `shutdown()`+await로 바꿔 핸들 파괴 작업이 실행되게 함. `getOrElse` 블록의 error result는 기존 코드 유지.)

- [ ] **Step 4: 통과 확인**

```
cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest
```
예상: BUILD SUCCESSFUL, 신규 3개 + 기존 ShellWorkerTest·TerminalViewModelAiTest 통과.

- [ ] **Step 5: 커밋**

```
git -C <repo> add android/app/src/main/java/dev/aiterminal/android/ShellWorker.kt android/app/src/test/java/dev/aiterminal/android/ShellWorkerTest.kt
git -C <repo> commit -m "feat(android): persistent AI handle lifecycle in ShellWorker"
```

---

## Task 8: OkHttp Kotlin NativeHttp + 의존

**Files:**
- Create: `android/app/src/main/java/dev/aiterminal/android/NativeHttp.kt`
- Modify: `android/app/build.gradle.kts` (dependencies)

**Interfaces:**
- Produces (Rust `JniHttpTransport`가 JNI로 호출): `NativeHttp.postJson(url: String, body: String, bearer: String?): String` (`@JvmStatic`, 실패 시 throw)

> **선행(별도 chore 커밋)**: OkHttp 5.3.0이 Kotlin stdlib 2.2.21을 strict로 끌어오므로 `android/build.gradle.kts`의 Kotlin plugin(`kotlin.android`+`plugin.compose`)을 2.0.21→2.2.21로 범프해야 한다(2.0.21 컴파일러는 metadata 2.2.0을 못 읽음). AGP 8.7.3·Compose BOM 2024.10.01 호환·기존 79 테스트 green 실측 완료(BUILD SUCCESSFUL). 이 범프는 Task 8 feat 커밋과 분리한다.

- [ ] **Step 1: OkHttp 의존 추가** — `android/app/build.gradle.kts`의 `dependencies {`(line 86) 블록, `testImplementation("org.json...")` 근처에 추가:

```kotlin
    implementation("com.squareup.okhttp3:okhttp:5.3.0")
```

- [ ] **Step 2: NativeHttp 작성** — `NativeHttp.kt` 신규:

```kotlin
package dev.aiterminal.android

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.util.concurrent.TimeUnit

/**
 * Rust JniHttpTransport가 JNI로 호출하는 동기 HTTP 포스트.
 * Android 시스템 TLS를 사용한다(Rust rustls 크로스컴파일 회피).
 * 실패는 예외로 던지고 Rust가 Err로 흡수한다(fail-soft, §3-3).
 */
object NativeHttp {
    private val JSON = "application/json; charset=utf-8".toMediaType()

    private val client = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(60, TimeUnit.SECONDS)
        .callTimeout(75, TimeUnit.SECONDS) // 전체 호출 상한(동기 취소 대체)
        .build()

    @JvmStatic
    fun postJson(url: String, body: String, bearer: String?): String {
        val builder = Request.Builder().url(url).post(body.toRequestBody(JSON))
        if (bearer != null) builder.header("Authorization", "Bearer $bearer")
        client.newCall(builder.build()).execute().use { response ->
            val text = response.body?.string() ?: ""
            if (!response.isSuccessful) {
                throw java.io.IOException("HTTP ${response.code}: $text")
            }
            return text
        }
    }
}
```

- [ ] **Step 3: 컴파일 확인**(OkHttp 해석 + Kotlin 컴파일)

```
cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:compileDebugKotlin
```
예상: BUILD SUCCESSFUL (OkHttp 5.3.0 해석, NativeHttp 컴파일).

- [ ] **Step 4: 커밋**

```
git -C <repo> add android/app/src/main/java/dev/aiterminal/android/NativeHttp.kt android/app/build.gradle.kts
git -C <repo> commit -m "feat(android): add OkHttp NativeHttp sync client"
```

---

## Task 9: OkHttp Rust JniHttpTransport + from_config 결선

**Files:**
- Create: `src/mobile_http.rs` (android 게이트)
- Modify: `src/lib.rs:139` (모듈 선언)
- Modify: `src/mobile_ai.rs` (`from_config` android 분기)
- 검증: aarch64 `cargo check` + 데스크톱 회귀

**Interfaces:**
- Consumes: `crate::http::HttpTransport`, `jni::JavaVM`, `NativeHttp.postJson` (Kotlin, Task 8)
- Produces: `crate::mobile_http::JniHttpTransport` (android 전용, `HttpTransport` 구현)

- [ ] **Step 1: mobile_http.rs 작성** — 파일 전체가 android 게이트:

```rust
//! Android 실 HTTP transport (JNI 역호출 → Kotlin OkHttp).
//!
//! `HttpTransport::post_json`을 Kotlin `NativeHttp.postJson` 동기 호출로 구현한다.
//! Android 시스템 TLS를 쓰므로 Rust rustls 크로스컴파일이 불필요하다. 동기 blocking이라
//! current-thread 런타임의 취소/타임아웃은 이 구간에 안 걸리고 OkHttp callTimeout이 상한이다
//! (spec §3 D2). 실패는 Err → gateway가 unavailable로 흡수한다(§3-3, 셸 안 막음).
#![cfg(target_os = "android")]

use std::os::raw::c_void;
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use jni::objects::{JObject, JValue};
use jni::sys::{jint, JNI_VERSION_1_6};
use jni::JavaVM;

use crate::http::HttpTransport;

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

/// 네이티브 라이브러리 로드 시 JavaVM을 캐시한다(transport가 스레드 attach에 사용).
#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut c_void) -> jint {
    let _ = JAVA_VM.set(vm);
    JNI_VERSION_1_6
}

/// JNI 역호출 기반 HTTP transport(상태 없음 — VM은 전역).
pub struct JniHttpTransport;

impl HttpTransport for JniHttpTransport {
    async fn post_json(&self, url: &str, body: &str, bearer: Option<&str>) -> Result<String> {
        let vm = JAVA_VM
            .get()
            .ok_or_else(|| anyhow!("JavaVM 미초기화 (JNI_OnLoad 누락)"))?;
        // 이미 JNI 호출 스택의 스레드라 attach는 기존 env를 돌려준다(cheap).
        let mut env = vm
            .attach_current_thread()
            .map_err(|e| anyhow!("attach_current_thread: {e}"))?;

        let j_url = env.new_string(url).map_err(|e| anyhow!("url: {e}"))?;
        let j_body = env.new_string(body).map_err(|e| anyhow!("body: {e}"))?;
        let j_bearer = match bearer {
            Some(b) => env.new_string(b).map_err(|e| anyhow!("bearer: {e}"))?.into(),
            None => JObject::null(),
        };

        let result = env.call_static_method(
            "dev/aiterminal/android/NativeHttp",
            "postJson",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
            &[
                JValue::Object(&j_url.into()),
                JValue::Object(&j_body.into()),
                JValue::Object(&j_bearer),
            ],
        );

        // 예외는 반드시 확인·클리어(안 하면 다음 JNI 호출이 오염된다).
        if env.exception_check().unwrap_or(false) {
            let _ = env.exception_clear();
            return Err(anyhow!("NativeHttp.postJson threw (network/HTTP error)"));
        }

        let obj = result.map_err(|e| anyhow!("call postJson: {e}"))?.l()
            .map_err(|e| anyhow!("postJson return: {e}"))?;
        let text: String = env
            .get_string(&obj.into())
            .map_err(|e| anyhow!("read response: {e}"))?
            .into();
        Ok(text)
    }
}
```

- [ ] **Step 2: lib.rs 모듈 선언** — `src/lib.rs:139`(`pub mod mobile_jni;` 다음)에 추가:

```rust
#[cfg(target_os = "android")]
pub mod mobile_http;
```

- [ ] **Step 3: from_config 결선** — `src/mobile_ai.rs`의 `MobileAi::from_config`(현재 line 98-100)를 타깃 분기로:

```rust
    /// config로 구성한다. android는 openai에 실 JNI transport, 그 외는 UnavailableTransport.
    pub fn from_config(cfg: &MobileAiConfig) -> anyhow::Result<MobileAi> {
        #[cfg(target_os = "android")]
        {
            Self::with_gateway(build_gateway(cfg, crate::mobile_http::JniHttpTransport))
        }
        #[cfg(not(target_os = "android"))]
        {
            Self::with_gateway(build_gateway(cfg, UnavailableTransport))
        }
    }
```
(`UnavailableTransport`는 non-android 경로에서 계속 필요 — 유지.)

- [ ] **Step 4: aarch64 컴파일 확인**

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo check --lib --target aarch64-linux-android && echo PASS || echo FAIL'
```
예상: `PASS` (mobile_http·JNI_OnLoad·JniHttpTransport가 android에 컴파일).

- [ ] **Step 5: 데스크톱 회귀 확인**(non-android 불변)

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all; cargo test --lib mobile_ai::tests:: && cargo clippy --lib --all-targets -- -D warnings && echo PASS || echo FAIL'
```
예상: `PASS` (특히 `openai_without_real_transport_is_unavailable` — 데스크톱은 여전히 unavailable).

- [ ] **Step 6: 커밋**

```
git -C <repo> add src/mobile_http.rs src/lib.rs src/mobile_ai.rs
git -C <repo> commit -m "feat(mobile): wire JniHttpTransport for android openai"
```

---

## Task 10: 디버그 config 주입

**Files:**
- Modify: `android/app/src/main/java/dev/aiterminal/android/TerminalViewModel.kt:53`
- Test: `android/app/src/test/java/dev/aiterminal/android/TerminalViewModelAiTest.kt`

**Interfaces:**
- Consumes: `ShellAiConfig`, `BuildConfig.DEBUG`
- Produces: `resolveAiConfig(reader): ShellAiConfig` — DEBUG+파일 있으면 openai, 아니면 mock

- [ ] **Step 1: 실패 테스트 작성** — 파일 접근을 주입 가능한 순수 파서로 분리해 테스트. `TerminalViewModelAiTest.kt`에 추가:

```kotlin
    @Test
    fun parseAiConfigReadsOpenaiFromJson() {
        val json = """{"provider":"openai","model":"gpt-4o-mini","openai_url":"https://api.openai.com","api_key":"sk-x"}"""
        val cfg = parseAiConfigJson(json)
        assertEquals("openai", cfg.provider)
        assertEquals("gpt-4o-mini", cfg.model)
        assertEquals("sk-x", cfg.apiKey)
    }

    @Test
    fun parseAiConfigFallsBackToMockOnBadJson() {
        assertEquals("mock", parseAiConfigJson("{not-json").provider)
        assertEquals(ShellAiConfig(), parseAiConfigJson("{not-json"))
    }
```

- [ ] **Step 2: 실패 확인**

```
cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest --tests "dev.aiterminal.android.TerminalViewModelAiTest"
```
예상: 컴파일 실패(`parseAiConfigJson` 미정의).

- [ ] **Step 3: 구현** — `TerminalViewModel.kt`에 순수 파서 + DEBUG 로더 추가. 파서(top-level 또는 companion, org.json 사용):

```kotlin
internal fun parseAiConfigJson(raw: String): ShellAiConfig {
    return try {
        val obj = org.json.JSONObject(raw)
        ShellAiConfig(
            provider = obj.optString("provider", "mock"),
            model = obj.optString("model", "default"),
            openaiUrl = obj.optString("openai_url", "https://api.openai.com"),
            apiKey = if (obj.isNull("api_key")) null else obj.optString("api_key").ifEmpty { null },
        )
    } catch (_: org.json.JSONException) {
        ShellAiConfig() // mock 폴백(fail-soft)
    }
}
```

DEBUG 로더 + 토글 결선. `TerminalViewModel.kt:53`(`worker.aiConfig = if (aiEnabled) ShellAiConfig() else null`)를:

```kotlin
        worker.aiConfig = if (aiEnabled) resolveAiConfig() else null
```
로 바꾸고 헬퍼 추가(파일 접근은 DEBUG에서만):

```kotlin
    // DEBUG 빌드에서만 /sdcard/Download/ai-terminal-ai-config.json을 openai config로 읽는다.
    // 릴리스·파일 없음·파싱 실패는 mock(프로덕션 UX 불변). api_key는 저장하지 않고 호출 시 전달.
    private fun resolveAiConfig(): ShellAiConfig {
        if (!BuildConfig.DEBUG) return ShellAiConfig()
        val file = java.io.File("/sdcard/Download/ai-terminal-ai-config.json")
        return if (file.canRead()) parseAiConfigJson(file.readText()) else ShellAiConfig()
    }
```
상태 문자열(`:57`)도 실제 provider 반영: `"AI assist enabled (${worker.aiConfig?.provider ?: "mock"} provider; suggestions only, nothing auto-runs)"`.

- [ ] **Step 4: 통과 확인**

```
cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest
```
예상: BUILD SUCCESSFUL, 신규 파서 테스트 + 기존 전부 통과.

- [ ] **Step 5: 커밋**

```
git -C <repo> add android/app/src/main/java/dev/aiterminal/android/TerminalViewModel.kt android/app/src/test/java/dev/aiterminal/android/TerminalViewModelAiTest.kt
git -C <repo> commit -m "feat(android): debug-only openai config injection"
```

---

## Task 11: 실기기 검증 문서

**Files:**
- Create: `docs/android-real-transport-device-verification.md`

- [ ] **Step 1: 문서 작성** — 실기기(SM-F956N) openai 검증 절차:

```markdown
# 실 transport 실기기 검증 (SM-F956N, openai)

이 슬라이스의 OkHttp JNI transport는 실기기에서만 실 openai 응답을 검증할 수 있다(CI 불가).

## 준비
1. DEBUG APK 빌드: `cd terminal/android && ./build-rust-jni.sh --profile release && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:assembleDebug`
2. 설치: `adb install -r app/build/outputs/apk/debug/app-debug.apk`
3. openai config 배치(실 키):
   `adb push ai-terminal-ai-config.json /sdcard/Download/ai-terminal-ai-config.json`
   내용: `{"provider":"openai","model":"gpt-4o-mini","openai_url":"https://api.openai.com","api_key":"sk-..."}`

## 확인 항목
- [ ] AI 토글 ON → 자연어 입력("큰 파일 찾아줘") → **openai 실 응답**(mock echo 아님).
- [ ] 응답이 제안으로만 표시되고 자동 실행되지 않음(§3-11).
- [ ] 네트워크 차단/키 오류 시 "unavailable"로 흡수, 셸은 계속 동작(§3-3).
- [ ] `adb logcat`에 api_key 평문 미노출(Debug redaction 확인).
- [ ] 세션 중 여러 줄 입력 시 핸들 재사용(재구성 로그 없음).

## 정리
- 검증 후 `adb shell rm /sdcard/Download/ai-terminal-ai-config.json`로 키 파일 제거.
```

- [ ] **Step 2: 커밋**

```
git -C <repo> add docs/android-real-transport-device-verification.md
git -C <repo> commit -m "docs(android): real-transport device verification procedure"
```

---

## 최종 통합 검증 (모든 Task 완료 후)

- [ ] **Rust 전체**(WSL, CI 미러 조합):

```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd <repo>; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo clippy --all-targets --features "storage tls remote" -- -D warnings && cargo test && cargo test --features "storage tls remote" && cargo check --lib --target aarch64-linux-android && echo ALL_PASS || echo SOME_FAIL'
```
예상: `ALL_PASS`.

- [ ] **Android 전체**(Windows gradle):

```
cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest
```
예상: BUILD SUCCESSFUL, 0 fail.

- [ ] **PR 생성**: `feat/android-real-transport` → develop. CI 4잡(fmt·clippy·test·android JNI 4 ABI 패키징) green 확인 후 사용자 승인 머지.

---

## Self-Review (작성자 체크 — 완료)

**Spec coverage:** spec §5 T1~T8 전부 매핑 — T1→Task1, T2→Task2, T3→Task3/4/5, T4→Task6/7, T5→Task8/9, T6→Task9(from_config), T7→Task10, T8→Task11. ✓

**Placeholder scan:** 모든 step에 실제 코드·명령·예상 출력 포함. "적절한 오류 처리" 류 없음. ✓

**Type consistency:** `create_mobile_ai`(Task3)→`ai_terminal_mobile_create_ai`(Task4)/`nativeCreateAi`(Task5) 시그니처 일치. `eval_line_ai_handle_json`(Task3)을 C ABI·JNI가 동일 참조. `JniHttpTransport`(Task9)를 `from_config`가 참조. Kotlin `createAi/evalLineAiHandle/destroyAi`(Task6)를 ShellWorker(Task7)가 소비. `parseAiConfigJson`(Task10) 일관. ✓
