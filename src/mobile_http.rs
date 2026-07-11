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
            Some(b) => env
                .new_string(b)
                .map_err(|e| anyhow!("bearer: {e}"))?
                .into(),
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

        let obj = result
            .map_err(|e| anyhow!("call postJson: {e}"))?
            .l()
            .map_err(|e| anyhow!("postJson return: {e}"))?;
        let text: String = env
            .get_string(&obj.into())
            .map_err(|e| anyhow!("read response: {e}"))?
            .into();
        Ok(text)
    }
}
