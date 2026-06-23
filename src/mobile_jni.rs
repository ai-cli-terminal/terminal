//! JNI bridge for the Android Compose spike.
//!
//! The Kotlin side passes session state as JSON and receives a serialized
//! `MobileEvalResult`. The native boundary stays intentionally narrow while
//! PM-3 is still deciding workspace and process/userland strategy.

use jni::objects::{JObject, JString};
use jni::sys::jstring;
use jni::JNIEnv;

use crate::mobile::{MobileEvalResult, MobileSessionState, MobileShell};

fn eval_line_json(input: &str, state_json: &str) -> String {
    let state = serde_json::from_str::<MobileSessionState>(state_json)
        .unwrap_or_else(|_| MobileShell::new().state());
    let mut shell = MobileShell::from_state(state);
    let result = shell.eval_line(input);
    serialize_result(&result)
}

fn serialize_result(result: &MobileEvalResult) -> String {
    serde_json::to_string(result).unwrap_or_else(|err| {
        let fallback = MobileEvalResult {
            ok: false,
            output_json: serde_json::Value::Null,
            output_text: String::new(),
            error: Some(format!("failed to serialize mobile result: {err}")),
            state: MobileShell::new().state(),
        };
        serde_json::to_string(&fallback).unwrap_or_else(|_| {
            r#"{"ok":false,"output_json":null,"output_text":"","error":"failed to serialize mobile result","state":{"cwd":".","vars":{},"exit_code":null}}"#.to_string()
        })
    })
}

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
        (Ok(input), Ok(state_json)) => eval_line_json(&input, &state_json),
        (Err(err), _) | (_, Err(err)) => serialize_result(&MobileEvalResult {
            ok: false,
            output_json: serde_json::Value::Null,
            output_text: String::new(),
            error: Some(format!("failed to read JNI string: {err}")),
            state: MobileShell::new().state(),
        }),
    };

    match env.new_string(response) {
        Ok(value) => value.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jni_json_bridge_evaluates_mobile_shell() {
        let state = serde_json::to_string(&MobileShell::new().state()).unwrap();
        let raw = eval_line_json("[{size: 50} {size: 200}] | where size > 100", &state);
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();

        assert!(result.ok, "{result:?}");
        assert_eq!(result.output_json, serde_json::json!([{ "size": 200 }]));
    }

    #[test]
    fn jni_json_bridge_preserves_session_state() {
        let state = serde_json::to_string(&MobileShell::new().state()).unwrap();
        let first: MobileEvalResult =
            serde_json::from_str(&eval_line_json("let limit = 100", &state)).unwrap();
        let next_state = serde_json::to_string(&first.state).unwrap();
        let raw = eval_line_json("[{size: 200}] | where size > $limit | length", &next_state);
        let second: MobileEvalResult = serde_json::from_str(&raw).unwrap();

        assert_eq!(second.output_json, serde_json::json!(1));
    }
}
