//! C ABI bridge for mobile `shellcore` bindings.
//!
//! This layer is intentionally JSON-in/JSON-out. Android keeps using JNI, while
//! future iOS Swift/Objective-C bindings can call these functions from the
//! `cdylib` without depending on JNI or a generated binding tool.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::mobile;

#[no_mangle]
pub extern "C" fn ai_terminal_mobile_initial_state_json() -> *mut c_char {
    string_to_c_ptr(mobile::initial_state_json())
}

/// Evaluates one mobile shell line from C-compatible UTF-8 strings.
///
/// # Safety
///
/// `input` and `state_json` must be non-null pointers to valid NUL-terminated
/// UTF-8 strings. The returned pointer is owned by Rust and must be released
/// with [`ai_terminal_mobile_free_string`].
#[no_mangle]
pub unsafe extern "C" fn ai_terminal_mobile_eval_line_json(
    input: *const c_char,
    state_json: *const c_char,
) -> *mut c_char {
    let input = match c_arg_to_string(input, "input") {
        Ok(value) => value,
        Err(error_json) => return string_to_c_ptr(error_json),
    };
    let state_json = match c_arg_to_string(state_json, "state_json") {
        Ok(value) => value,
        Err(error_json) => return string_to_c_ptr(error_json),
    };
    string_to_c_ptr(mobile::eval_line_json(&input, &state_json))
}

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
    string_to_c_ptr(mobile::eval_line_ai_json(
        &input,
        &state_json,
        &ai_config_json,
    ))
}

/// Frees a string returned by this module.
///
/// # Safety
///
/// `value` must be either null or a pointer previously returned by this module.
/// Passing any other pointer, or freeing the same pointer more than once, is
/// undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn ai_terminal_mobile_free_string(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(value));
    }
}

unsafe fn c_arg_to_string(value: *const c_char, name: &str) -> Result<String, String> {
    if value.is_null() {
        return Err(mobile::error_result_json(format!(
            "{name} pointer was null"
        )));
    }
    unsafe { CStr::from_ptr(value) }
        .to_str()
        .map(str::to_owned)
        .map_err(|err| mobile::error_result_json(format!("{name} was not valid UTF-8: {err}")))
}

fn string_to_c_ptr(value: String) -> *mut c_char {
    json_c_string(value).into_raw()
}

fn json_c_string(value: String) -> CString {
    CString::new(value).unwrap_or_else(|_| {
        CString::new(mobile::error_result_json(
            "mobile bridge produced an interior NUL byte",
        ))
        .expect("structured fallback JSON must not contain interior NUL bytes")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mobile::MobileEvalResult;
    use std::ptr;

    unsafe fn take_owned_json(ptr: *mut c_char) -> String {
        assert!(!ptr.is_null());
        let raw = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap().to_string();
        unsafe {
            ai_terminal_mobile_free_string(ptr);
        }
        raw
    }

    #[test]
    fn c_abi_returns_initial_state_json() {
        let raw = unsafe { take_owned_json(ai_terminal_mobile_initial_state_json()) };
        let state: crate::mobile::MobileSessionState = serde_json::from_str(&raw).unwrap();

        assert_eq!(state.workspace_root, ".");
        assert_eq!(state.cwd, ".");
        assert_eq!(state.vars, serde_json::json!({}));
    }

    #[test]
    fn c_abi_evaluates_mobile_shell_json() {
        let input = CString::new("[{size: 50} {size: 200}] | where size > 100").unwrap();
        let state = CString::new(crate::mobile::initial_state_json()).unwrap();

        let raw = unsafe {
            take_owned_json(ai_terminal_mobile_eval_line_json(
                input.as_ptr(),
                state.as_ptr(),
            ))
        };
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();

        assert!(result.ok, "{result:?}");
        assert_eq!(result.output_json, serde_json::json!([{ "size": 200 }]));
    }

    #[test]
    fn c_abi_reports_null_input_as_json_error() {
        let state = CString::new(crate::mobile::initial_state_json()).unwrap();

        let raw = unsafe {
            take_owned_json(ai_terminal_mobile_eval_line_json(
                ptr::null(),
                state.as_ptr(),
            ))
        };
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();

        assert!(!result.ok);
        assert_eq!(result.error.as_deref(), Some("input pointer was null"));
    }

    #[test]
    fn c_abi_reports_invalid_utf8_as_json_error() {
        let input = [0xff_u8, 0x00];
        let state = CString::new(crate::mobile::initial_state_json()).unwrap();

        let raw = unsafe {
            take_owned_json(ai_terminal_mobile_eval_line_json(
                input.as_ptr().cast::<c_char>(),
                state.as_ptr(),
            ))
        };
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();

        assert!(!result.ok);
        assert!(
            result
                .error
                .as_deref()
                .is_some_and(|err| err.contains("input was not valid UTF-8")),
            "{result:?}"
        );
    }

    #[test]
    fn c_abi_free_accepts_null() {
        unsafe {
            ai_terminal_mobile_free_string(ptr::null_mut());
        }
    }

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
}
