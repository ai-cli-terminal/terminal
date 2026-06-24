# Android 스파이크

이 디렉터리는 PM-3 최소 Compose UI + Rust `MobileShell` JNI 연결 스파이크다.

범위:

- Compose 화면 1개: 상태, transcript, command input, run button.
- `TerminalViewModel`이 UI state를 소유한다.
- `ShellWorker`가 shell 평가를 단일 background thread에서 실행한다.
- `NativeShellBridge`가 JNI로 Rust `MobileShell`을 호출한다.
- `android/build-rust-jni.ps1`이 `libai_terminal.so`를 `app/src/main/jniLibs/<abi>`에 빌드/복사한다.

Rust, rustup, Android NDK가 준비되어 있으면 먼저 native library를 빌드한다. 기본값은
Android 4개 ABI 전체다.

```powershell
android/build-rust-jni.ps1 -Profile debug
```

그 다음 저장소 루트에서 Android APK를 조립한다.

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
gradle -p android :app:assembleDebug
```

Native packaging smoke:

```powershell
gradle -p android :app:verifyNativeLibraries
```

Worker JVM contract test:

```powershell
gradle -p android :app:testDebugUnitTest
```

Instrumentation smoke APK build:

```powershell
gradle -p android :app:assembleDebugAndroidTest
```

With `.so` files built and an emulator/device attached:

```powershell
gradle -p android :app:connectedDebugAndroidTest
```

`libai_terminal.so`가 없으면 APK는 컴파일되지만, 첫 명령 제출 시 transcript에
`native shell library not loaded` 오류가 표시된다.

현재 bridge 동작:

- 구조화 smoke 입력 동작: `[{size: 50} {size: 200}] | where size > 100`
- `let name = value`는 Rust `MobileShell` session vars를 갱신한다.
- 알 수 없는 외부 명령은 `external execution disabled`를 반환한다.
- 기본 workspace는 `Context.filesDir/ash-workspace`다.
- `cd`와 `ls`는 app-private workspace root 밖 경로를 거부한다.
- 상태바는 좁은 화면에 맞춰 workspace/cwd basename과 `core / private` capability만 표시한다.
- `Import`는 Android document picker에서 선택한 파일을 app-private workspace로 복사한다.
- `Export`는 transcript를 사용자가 선택한 document URI로 쓴다.
- Worker stream 계약은 `Started` / `Stdout` / `Stderr` / `Finished` / `Cancelled` event를 사용한다.
- 외부 명령 전략은 PM-3E에서 비교했다. MVP는 `shellcore-only`를 유지하고, 다음 후보는 Termux-compatible opt-in bridge다.

다음 slice:

1. Termux-compatible opt-in bridge design spike를 시작한다.
2. Import한 파일을 여는 read-only builtin 또는 preview UX를 정한다.
3. 실제 userland/PTY adapter에서 interrupt/timeout 구현을 붙인다.
