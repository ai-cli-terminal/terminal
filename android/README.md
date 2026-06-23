# Android Spike

This directory is the PM-3 minimal Compose UI + worker-thread spike.

Scope:

- One Compose screen: status, transcript, command input, run button.
- `TerminalViewModel` owns UI state.
- `ShellWorker` runs shell evaluation on a single background thread.
- `ShellBridge` is the seam where JNI/UniFFI will wrap Rust `MobileShell`.
- `FakeShellBridge` is temporary and keeps this spike runnable before native binding work.

Run from the repository root when Android SDK and Gradle plugin dependencies are available:

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
gradle -p android :app:assembleDebug
```

Current bridge behavior:

- Structured smoke input works: `[{size: 50} {size: 200}] | where size > 100`
- `let name = value` updates fake session vars.
- Unknown commands return `external execution disabled`.

Next slice:

1. Replace `FakeShellBridge` with JNI or UniFFI binding to Rust `MobileShell`.
2. Add an Android instrumentation or JVM test for worker non-blocking behavior.
3. Add app-private workspace state and cwd display from the real bridge.
