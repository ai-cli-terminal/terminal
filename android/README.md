# Android 스파이크

이 디렉터리는 PM-3 최소 Compose UI + Rust `MobileShell` JNI 연결 스파이크다.

범위:

- Compose 화면 1개: 상태, transcript, command input, run button.
- `TerminalViewModel`이 UI state를 소유한다.
- `ShellWorker`가 shell 평가를 단일 background thread에서 실행한다.
- `NativeShellBridge`가 JNI로 Rust `MobileShell`을 호출한다.
- `android/build-rust-jni.ps1`이 `libai_terminal.so`를 `app/src/main/jniLibs`에 빌드/복사한다.

Rust, rustup target, Android NDK가 준비되어 있으면 먼저 native library를 빌드한다.

```powershell
rustup target add aarch64-linux-android
android/build-rust-jni.ps1 -Profile debug -Targets aarch64-linux-android
```

그 다음 저장소 루트에서 Android APK를 조립한다.

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
gradle -p android :app:assembleDebug
```

`libai_terminal.so`가 없으면 APK는 컴파일되지만, 첫 명령 제출 시 transcript에
`native shell library not loaded` 오류가 표시된다.

현재 bridge 동작:

- 구조화 smoke 입력 동작: `[{size: 50} {size: 200}] | where size > 100`
- `let name = value`는 Rust `MobileShell` session vars를 갱신한다.
- 알 수 없는 외부 명령은 `external execution disabled`를 반환한다.

다음 slice:

1. worker non-blocking behavior를 JVM 또는 instrumentation test로 고정한다.
2. app-private workspace state와 cwd 표시를 실제 bridge state와 연결한다.
3. `build-rust-jni.ps1`/CI를 전체 release ABI 빌드로 확장한다.
