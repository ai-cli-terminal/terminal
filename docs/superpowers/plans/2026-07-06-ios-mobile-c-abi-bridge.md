# 2026-07-06 iOS Mobile C ABI Bridge

## Context

PR #66 moved mobile `input + state_json -> MobileEvalResult JSON` handling into
the common Rust `mobile` module. Android JNI now delegates to that common
bridge, and `mobile_jni` is Android-only.

The remaining PM-4 implementation target is still a macOS/Xcode-hosted
TestFlight SwiftUI REPL scaffold. This Windows host cannot create or verify the
iOS app, but it can prepare the Rust `cdylib` ABI surface that Swift or
Objective-C will call later.

## Goal

Expose the common mobile JSON bridge through a small C ABI:

- return initial mobile session state JSON;
- evaluate one line from `input` and `state_json`;
- return a structured JSON error for null or invalid UTF-8 C strings;
- document and test that returned strings are owned by Rust and must be freed
  by a matching free function.

## Proposed ABI

```c
char *ai_terminal_mobile_initial_state_json(void);
char *ai_terminal_mobile_eval_line_json(const char *input, const char *state_json);
void ai_terminal_mobile_free_string(char *value);
```

The ABI remains JSON-in/JSON-out so the first SwiftUI shell can bind to a
stable Rust surface before UniFFI, cbindgen headers, or a richer typed Swift API
are selected.

## Non-Goals

- Do not create an iOS/Xcode project on this host.
- Do not add UniFFI/cbindgen-generated headers yet.
- Do not add async/background command execution.
- Do not change Android JNI behavior.

## Implementation Plan

1. Add `src/mobile_ffi.rs` with the C ABI exports.
2. Keep all shell evaluation delegated to `mobile::eval_line_json`.
3. Add Rust tests for initial state, eval, null pointer errors, invalid UTF-8
   errors, and free safety for non-null pointers.
4. Export `mobile_ffi` from `src/lib.rs`.
5. Update PM-4 docs so the next macOS/Xcode slice can bind SwiftUI to the C ABI
   bridge.

## Verification

- `git diff --check`
- PR CI `fmt · clippy · test`
- PR CI `android JNI packaging`
- PR CI `windows build + self-contained check`

Local Windows still does not expose `cargo` on PATH, so Rust execution evidence
comes from CI.

## Next Slice

On a macOS/Xcode host, create or select an iOS app scaffold, add a tiny Swift
wrapper around this C ABI, and render a SwiftUI REPL screen that calls
`ai_terminal_mobile_eval_line_json`.
