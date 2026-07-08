# 2026-07-06 iOS Mobile Common JSON Bridge

## Context

PR #65 locked the iOS constrained command boundary in `MobileShell`: pure
`shellcore` commands remain allowed, common Linux/userland commands fail closed,
and the capability flags report no spawn, PTY, userland, or direct networking.

The next PM-4 implementation target is a TestFlight self-contained
`shellcore` REPL spike. This Windows host cannot create or verify an iOS
TestFlight build because there is no iOS/Xcode project or macOS toolchain here.
The locally actionable slice is to make the Rust mobile binding contract
platform-neutral before Swift/UniFFI/C-ABI work begins.

## Goal

Move the JSON state/eval bridge out of Android JNI-only code into the common
`mobile` module so Android and future iOS bindings share the same contract:

- initial session state serializes as JSON;
- `input + state_json -> MobileEvalResult JSON`;
- invalid state JSON falls back to a fresh pure mobile shell;
- serialization errors return a structured failure result;
- Android JNI remains a thin transport wrapper around the common bridge.

## Non-Goals

- Do not create an iOS/Xcode project on this Windows host.
- Do not introduce UniFFI, cbindgen, Swift, or TestFlight build steps yet.
- Do not widen the iOS product promise beyond self-contained `shellcore`.
- Do not change Android app UI behavior.

## Implementation Plan

1. Add common helpers to `src/mobile.rs`:
   - `initial_state_json()`
   - `eval_line_json(input, state_json)`
   - `error_result_json(message)`
2. Move JSON bridge tests from Android JNI semantics into common mobile tests.
3. Update `src/mobile_jni.rs` so JNI only reads Java strings and delegates to
   the common helpers.
4. Gate `mobile_jni` to Android targets in `src/lib.rs` so future iOS builds do
   not pull a JNI module into the public crate surface.
5. Update PM-4 docs to mark the Rust-side common bridge slice complete and keep
   the actual TestFlight SwiftUI shell open.

## Verification

- `git diff --check`
- Targeted Rust tests through CI, because local Windows PATH has no `cargo` and
  WSL has no installed distro on this host.
- Android JNI packaging CI, to prove the Android target still sees
  `mobile_jni`.

## Next Slice

After this bridge lands, the first macOS/Xcode-hosted iOS slice should create
or select an iOS app scaffold and bind a minimal SwiftUI REPL screen to the
common mobile JSON bridge.
