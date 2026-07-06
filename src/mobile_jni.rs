//! JNI bridge for the Android Compose spike.
//!
//! The Kotlin side passes session state as JSON and receives a serialized
//! `MobileEvalResult`. The native boundary stays intentionally narrow while
//! PM-3 is still deciding workspace and process/userland strategy.

use jni::objects::{JObject, JString};
use jni::sys::jstring;
use jni::JNIEnv;

use crate::mobile;

#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_dev_aiterminal_android_NativeShellBridge_nativeEvalLine(
    mut env: JNIEnv,
    _this: JObject,
    input: JString,
    state_json: JString,
) -> jstring {
    let response = match (
        env.get_string(&input).map(String::from),
        env.get_string(&state_json).map(String::from),
    ) {
        (Ok(input), Ok(state_json)) => mobile::eval_line_json(&input, &state_json),
        (Err(err), _) | (_, Err(err)) => {
            mobile::error_result_json(format!("failed to read JNI string: {err}"))
        }
    };

    match env.new_string(response) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
