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
- 외부 명령 전략은 PM-3E에서 비교했다. MVP는 `shellcore-only`를 유지한다.
- PM-3F Termux-compatible opt-in bridge design은 T0 `RUN_COMMAND` completion probe와 T1 helper-backed stream/cancel protocol로 나눴다.
- `Probe Termux`는 Termux 설치/permission을 확인하고 T0 `RUN_COMMAND` smoke 결과를 PendingIntent service로 받는다.
- T1 helper protocol substrate는 `argv` request JSON과 helper `events.ndjson` line을 `ShellStreamEvent`로 변환하는 순수 Kotlin 계약을 고정한다.
- T1 helper event polling은 `events.ndjson` offset/partial-line tracking과 terminal event stop을 고정하고, `ShellRunHandle.cancel()`은 shared job dir의 `cancel` file을 쓴다.

Termux T0 smoke on a real device:

1. Install Termux from a supported source.
2. In Termux, set `allow-external-apps=true` in `~/.termux/termux.properties`, then restart Termux.
3. Install the debug APK and grant the RunCommand permission:

```powershell
$adb="$env:LOCALAPPDATA\Android\Sdk\platform-tools\adb.exe"
& $adb install -r android/app/build/outputs/apk/debug/app-debug.apk
& $adb shell pm grant dev.aiterminal.android com.termux.permission.RUN_COMMAND
```

4. Launch AI Terminal and tap `Probe Termux`.

Expected transcript:

```text
> termux t0 smoke
termux echo: ok ASH_TERMUX_OK
termux pwd: ok /data/data/com.termux/files/home
termux stderr: ok ERR
termux non-zero: ok exit 7
```

Verified device capture:

```text
SM_F956N / R3CX60P3R5K
external / opt-in
termux: Termux T0 smoke ready
```

다음 slice:

1. T1 helper bootstrap UX와 shared staging workspace export/import boundary를 정한다.
2. Long-running stdout, cancel, large output smoke를 helper polling adapter에 연결한다.
3. Import한 파일을 여는 read-only builtin 또는 preview UX를 정한다.
