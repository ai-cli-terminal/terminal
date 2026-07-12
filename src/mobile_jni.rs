//! JNI bridge for the Android Compose spike.
//!
//! The Kotlin side passes session state as JSON and receives a serialized
//! `MobileEvalResult`. The native boundary stays intentionally narrow while
//! PM-3 is still deciding workspace and process/userland strategy.

use jni::objects::{JObject, JString};
use jni::sys::{jlong, jstring};
use jni::JNIEnv;

use crate::mobile;
use crate::mobile_ai::MobileAi;

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
