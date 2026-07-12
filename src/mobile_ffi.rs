//! C ABI bridge for mobile `shellcore` bindings.
//!
//! This layer is intentionally JSON-in/JSON-out. Android keeps using JNI, while
//! future iOS Swift/Objective-C bindings can call these functions from the
//! `cdylib` without depending on JNI or a generated binding tool.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::mobile;
use crate::mobile_ai::MobileAi;

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
    string_to_c_ptr(mobile::eval_line_ai_handle_json(
        ai_ref,
        &input,
        &state_json,
    ))
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

    #[test]
    fn c_abi_handle_create_eval_destroy() {
        let cfg = CString::new("{}").unwrap();
        let ai = unsafe { ai_terminal_mobile_create_ai(cfg.as_ptr()) };
        assert!(!ai.is_null());

        let input = CString::new("큰 파일 찾아줘").unwrap();
        let state = CString::new(crate::mobile::initial_state_json()).unwrap();
        let raw = unsafe {
            take_owned_json(ai_terminal_mobile_eval_line_ai_handle(
                ai,
                input.as_ptr(),
                state.as_ptr(),
            ))
        };
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            result.ai.as_ref().map(|a| a.kind.as_str()),
            Some("answered"),
            "{result:?}"
        );

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
                std::ptr::null_mut(),
                input.as_ptr(),
                state.as_ptr(),
            ))
        };
        let result: MobileEvalResult = serde_json::from_str(&raw).unwrap();
        assert!(!result.ok);
    }
}
