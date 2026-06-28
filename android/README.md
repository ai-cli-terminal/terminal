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

WSL/Linux에서 Linux-host NDK를 사용할 때는:

```bash
export ANDROID_NDK_HOME="$HOME/.local/opt/android-ndk-r28c-linux/android-ndk-r28c"
bash android/build-rust-jni.sh --profile release
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
- `Import`는 Android document picker에서 선택한 파일을 app-private workspace로 복사하고, UTF-8 text preview를 transcript에 남긴다.
- `Export`는 transcript를 사용자가 선택한 document URI로 쓴다.
- Worker stream 계약은 `Started` / `Stdout` / `Stderr` / `Finished` / `Cancelled` event를 사용한다.
- 외부 명령 전략은 PM-3E에서 비교했다. MVP는 `shellcore-only`를 유지한다.
- PM-3F Termux-compatible opt-in bridge design은 T0 `RUN_COMMAND` completion probe와 T1 helper-backed stream/cancel protocol로 나눴다.
- `Probe Termux`는 Termux 설치/permission을 확인하고 T0 `RUN_COMMAND` smoke 결과를 PendingIntent service로 받는다.
- T1 helper protocol substrate는 `argv` request JSON과 helper `events.ndjson` line을 `ShellStreamEvent`로 변환하는 순수 Kotlin 계약을 고정한다.
- T1 helper event polling은 `events.ndjson` offset/partial-line tracking과 terminal event stop을 고정하고, `ShellRunHandle.cancel()`은 shared job dir의 `cancel` file을 쓴다.
- `Install Helper`는 Termux `RUN_COMMAND`로 `~/.ash-termux-bridge/helper.sh`를 설치하고 `self-test`를 실행한다. `python3`가 있으면 Python supervisor를 쓰고, 없으면 app-written argv fallback files와 shell log polling fallback을 쓴다.
- T1 helper-backed adapter는 helper self-test만으로는 켜지지 않는다. 사용자가 입력한 shared staging path의 app write smoke와 helper event-file smoke가 통과한 뒤에만 `external execution disabled`인 단일 argv command를 `~/.ash-termux-bridge/helper.sh run <job-dir>`로 재시도한다.
- T1 adapter가 shared staging으로 연결된 뒤의 `Cancel`은 active run handle을 통해 helper job dir의 `cancel` marker를 쓴다. 실제 child process interrupt는 Termux-side helper 구현이 담당한다.

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
core / private
termux: Termux T0 smoke ready; install helper for streamed external commands
```

Termux T1 helper bootstrap:

1. In Termux, allow storage access (`termux-setup-storage`) or grant Termux storage permission from Android settings.
2. In AI Terminal, tap `Install Helper`.
3. After `termux helper: ok`, enter a shared staging path that both the app and Termux can access, or tap `Pick` and choose a primary shared-storage directory such as `Download/ash-termux-bridge`.
4. The app keeps external commands disabled until the shared staging smoke passes.

The shared staging picker is intentionally a path helper, not a SAF-backed execution backend. Termux helper jobs still use a filesystem directory, so the app maps Android's primary external-storage tree URI to a Termux-visible `/sdcard/...` path and leaves the manual path input available for unsupported trees or device-specific layouts.

Manual T1 helper real-device smoke requires an explicit shared staging directory. Do not use the app external-files directory for this smoke.

```powershell
gradle -p android :app:connectedDebugAndroidTest -Pandroid.testInstrumentationRunnerArguments.termuxRealDeviceSmoke=true -Pandroid.testInstrumentationRunnerArguments.termuxBridgeStagingDir=/path/visible/to/app/and/termux
```

Verified device capture:

```text
SM_F956N / R3CX60P3R5K
staging: /sdcard/Download/ash-termux-bridge
Termux storage permission: granted
Termux helper real-device smoke: OK (2 tests)
```

Imported document UX:

- `Import` copies the selected document into the app-private workspace and shows a bounded UTF-8 preview.
- `Open Last` reopens the most recent imported workspace file read-only with a larger bounded preview.
- Binary or non-UTF-8 content is not rendered in transcript, and the reopen path is canonicalized back under the workspace root.

다음 slice:

1. Release signing/metadata를 준비하고 실제 release APK/F-Droid packaging을 검증한다.

Distribution route:

- Direct APK/GitHub Release first.
- F-Droid next after release metadata, signing guidance, and build reproducibility constraints are ready.
- Google Play is deferred for the Termux-enabled build until policy review is complete; a later Play candidate may need a core-only or reduced bridge flavor.
- Store metadata draft lives under `android/fastlane/metadata/android/en-US`.
- CI and release use the checked-in Gradle wrapper with Android API 35, build-tools 35.0.0, and NDK r28c (`28.2.13676358`).

Decision record: `docs/superpowers/specs/2026-06-28-android-distribution-route.md`.

Release packaging status:

- `:app:testDebugUnitTest` is green with `ANDROID_HOME=$env:LOCALAPPDATA\Android\Sdk`.
- `android/build-rust-jni.sh --profile release` has been verified from WSL with NDK r28c Linux prebuilt, staging all four ABI `libai_terminal.so` files.
- `:app:assembleRelease :app:verifyNativeLibraries` is green after JNI staging and currently produces `app-release-unsigned.apk`.
- Android `versionName` is read from the repository `VERSION` file and `versionCode` is computed as `major * 10000 + minor * 100 + patch`. For `0.3.1`, the release candidate uses `versionCode=301`.
- `:app:verifyFdroidReleaseInputs` verifies release metadata and the matching Fastlane/F-Droid changelog before packaging.
- `android/fdroid-version.properties` mirrors the Gradle-computed Android version for F-Droid regex-based update checks, and `android/fdroiddata/metadata/dev.aiterminal.android.yml` is the fdroiddata submission draft.
- `android/smoke-fdroid-metadata.ps1` runs the fdroidserver metadata dry-run in an isolated WSL virtualenv and verifies `fdroid lint`, `fdroid rewritemeta`, and no canonical formatting diff.
- `android/smoke-fdroid-release-activation.ps1 -Commit <full-40-char-release-commit>` dry-runs the final fdroiddata activation step by removing the temporary `disable`, replacing `TODO_NEXT_ANDROID_RELEASE_COMMIT`, running `fdroid rewritemeta`, and linting the activated metadata copy.
- Fastlane phone screenshots live under `android/fastlane/metadata/android/en-US/images/phoneScreenshots`. Current captures were taken from the `Medium_Phone` emulator after launching the debug APK:
  - `01-home.png` shows the shellcore-only app surface and default command input.
  - `02-run-result.png` shows the default command result (`# size` / `200`) after tapping `Run`.
- Latest verified unsigned release APK: `app-release-unsigned.apk`, 8,942,194 bytes, SHA256 `21b82a2b68f25d143346244d1f0a7129c68ca6d96dfa5921462fb44700ba2aeb`.
- Signed release output is opt-in through environment variables: `AI_TERMINAL_ANDROID_KEYSTORE`, `AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD`, `AI_TERMINAL_ANDROID_KEY_ALIAS`, `AI_TERMINAL_ANDROID_KEY_PASSWORD`.
- Local signing wiring can be validated without real secrets by running `android/smoke-release-signing.ps1`; it creates a throwaway keystore under `artifacts/android-signing-smoke`, builds `app-release.apk`, and verifies it with `apksigner`.
- Latest throwaway signed smoke APK: `app-release.apk`, 8,950,386 bytes, SHA256 `245844e4cc684c24868158be6edbb8443a7e9b310054f668cf2274dfa0da492f`; `apksigner verify` reports v2 signature verification and one signer.
- GitHub secret-shaped signing values can be validated locally with `android/smoke-github-signing-secrets.ps1`. Use `-UseThrowawayKeystore` to exercise the exact base64 decode path without real release secrets. The throwaway APK hash and certificate fingerprint are run-specific and are written to `artifacts/android-github-signing-preflight/android-github-signing-preflight-evidence.json`; `apksigner --print-certs` must report v2 signature verification and one signer.
- Tag-triggered GitHub Releases build and upload an Android universal APK asset. Signed output needs `AI_TERMINAL_ANDROID_KEYSTORE_BASE64`, `AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD`, `AI_TERMINAL_ANDROID_KEY_ALIAS`, and `AI_TERMINAL_ANDROID_KEY_PASSWORD` GitHub secrets; otherwise the workflow uploads `ai-terminal-android-universal-unsigned.apk`.
