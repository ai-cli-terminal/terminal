# HANDOFF — ai-cli-terminal (2026-06-25)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`,
`docs/HISTORY.md`, `docs/superpowers/` 아래 spec/plan 문서다. 이 파일은
재개 가이드와 다음 작업 우선순위만 압축한다.

## 1. 현재 상태

작업 repo는 `D:\workspace\terminal-project\terminal`이고 브랜치는 `main`이다.
현재 `main`은 `origin/main`과 동기화되어 있다.

제품 방향은 플랫폼별 독립 로컬 터미널 `ash`다. 모바일도 PWA 승인 화면이
아니라 온디바이스 로컬 터미널을 장기 목표로 둔다. Android는 PM-3 로컬
터미널 스파이크가 진행 중이며, 현재는 Kotlin/Compose UI + stream/cancel-tested worker thread +
Rust `MobileShell` JNI bridge + instrumentation smoke + app-private workspace boundary +
document import/export + 전체 ABI JNI 패키징 CI 경로 + PM-3E 외부 명령 전략 비교 +
PM-3F Termux-compatible opt-in bridge design + T0 probe substrate까지
완료된 상태다.

## 2. 최근 완료 산출

| 커밋 | 내용 |
|---|---|
| `4739139` | `src/mobile.rs`에 Rust `MobileShell` pure core boundary 추가 |
| `0e419b9` | `android/` Kotlin/Compose skeleton과 `ShellWorker` background thread 추가 |
| `68a3ccd` | `FakeShellBridge` 제거, `NativeShellBridge` -> Rust JNI `MobileShell` 연결 |
| `57e5ab4` | JNI bridge rustfmt 정리 |
| 이번 커밋 | PM-3F Termux T0 real-device smoke + T1 helper protocol/polling/cancel substrate |

주요 파일:

| 파일 | 내용 |
|---|---|
| `src/mobile.rs` | Android/iOS가 감쌀 pure `shellcore` session boundary |
| `src/mobile_jni.rs` | Android JNI export `NativeShellBridge.nativeEvalLine(input, stateJson)` |
| `android/app/src/main/java/dev/aiterminal/android/ShellBridge.kt` | Kotlin `NativeShellBridge`, JSON state encode/decode, native load error handling |
| `android/build-rust-jni.ps1` | NDK linker로 `libai_terminal.so`를 빌드하고 `jniLibs/<abi>`로 복사 |
| `.github/workflows/ci.yml` | Android JNI packaging job: 4 ABI `.so` build + Gradle verify + APK assemble |
| `android/app/build.gradle.kts` | `verifyNativeLibraries` task로 4 ABI `libai_terminal.so` 존재 검증 |
| `android/app/src/test/java/dev/aiterminal/android/ShellWorkerTest.kt` | Worker thread 평가 + result poster callback JVM 계약 테스트 |
| `android/app/src/main/java/dev/aiterminal/android/ShellStream.kt` | `Started`/`Stdout`/`Stderr`/`Finished`/`Cancelled` stream event 계약 |
| `android/app/src/androidTest/java/dev/aiterminal/android/NativeShellBridgeInstrumentedTest.kt` | 실제 APK에서 `NativeShellBridge` -> Rust `MobileShell` 호출 smoke |
| `android/app/src/main/java/dev/aiterminal/android/WorkspaceDocuments.kt` | SAF import/export를 app-private workspace 복사 경계로 연결 |
| `docs/superpowers/specs/2026-06-24-android-stream-cancel-contract.md` | Android worker stream/cancel 계약 |
| `docs/superpowers/specs/2026-06-24-android-external-command-strategy.md` | PM-3E Android 외부 명령 전략 비교와 다음 spike 결정 |
| `docs/superpowers/specs/2026-06-25-termux-compatible-opt-in-bridge-design.md` | PM-3F T0 `RUN_COMMAND` probe와 T1 helper-backed stream/cancel bridge design |
| `android/app/src/main/java/dev/aiterminal/android/TermuxBridge.kt` | Termux availability, T0 echo probe intent, PendingIntent result service, result decoding |
| `android/app/src/test/java/dev/aiterminal/android/TermuxBridgeTest.kt` | T0 availability/result decoding JVM contract tests |
| `android/README.md` | Android native library build + APK assemble 절차 |
| `docs/superpowers/specs/2026-06-23-android-local-terminal-spike.md` | PM-3 Android local terminal spike spec |
| `docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md` | PM workflow, PM-3D~PM-3E 완료와 다음 Termux bridge 작업 |

## 3. 검증 상태

- 로컬 `gradle -p android :app:assembleDebug` 통과.
- 로컬 `gradle -p android :app:testDebugUnitTest` 통과.
- 로컬 `gradle -p android :app:assembleDebugAndroidTest` 통과.
- 로컬 `git diff --check` 통과.
- 로컬 Windows PATH에 `cargo`가 없어 Rust unit test와 Android Rust `.so` 실제 빌드는
  이 세션에서 실행하지 못했다. `android/build-rust-jni.ps1` PowerShell parse는 통과했다.
- CI에 새 `android JNI packaging` job을 추가했다. 원격 Actions에서 검증 필요:
  SDK/NDK 설치 -> 4 ABI Rust build -> `:app:verifyNativeLibraries` ->
  `:app:testDebugUnitTest` -> emulator `:app:connectedDebugAndroidTest` -> `:app:assembleDebug`.
- GitHub Actions `28021366018` 통과:
  - fmt
  - clippy
  - tests
  - storage/tls builds
  - cargo audit
  - Windows release build
  - Windows ConPTY smoke
  - `ash.exe` smoke
  - self-contained check

## 4. 중요한 결정

- Android binding은 이번 slice에서 UniFFI가 아니라 direct JNI로 선택했다.
  generator/runtime/toolchain 표면을 작게 유지하기 위해서다.
- Rust library는 `rlib`와 `cdylib`를 함께 산출한다.
- Android target에서는 `src/lib.rs`의 `target_os = "android"` cfg로 모바일
  cdylib 범위를 `shellcore`, `mobile`, `mobile_jni` 중심으로 좁힌다.
- Kotlin/Rust bridge는 JSON-in/JSON-out이다. `ShellState`는
  `MobileSessionState` JSON으로 넘어가고, Rust `MobileEvalResult` JSON은
  Kotlin `ShellEvalResult`로 복원된다.
- `libai_terminal.so`가 없으면 앱은 첫 명령 제출 시 transcript에
  `native shell library not loaded` 오류를 표시한다. ViewModel 생성 시점에는
  crash하지 않는다.
- Android MVP는 계속 `shellcore-only`다. PM-3E 비교 결과 다음 후보는
  Termux-compatible opt-in bridge이며, bundled minimal userland는 보류한다.
- PM-3F bridge design은 Termux integration을 T0 `RUN_COMMAND` completion probe와
  T1 `ash-termux-helper` stream/cancel protocol로 나눴다. T0만으로는 실제
  incremental stream/cancel ready로 표시하지 않는다.
- T0 substrate는 실제 Termux 설치 기기(`SM_F956N / R3CX60P3R5K`)에서
  `allow-external-apps`, stdout/stderr, non-zero exit smoke까지 통과했다.
- T1 helper protocol substrate는 `argv` request JSON과 helper `events.ndjson`
  line을 `ShellStreamEvent`로 변환하는 순수 Kotlin 계약까지 고정했다.
- T1 helper event file polling은 offset/partial-line tracking, terminal event stop,
  truncate reset을 고정했고, `ShellRunHandle.cancel()`은 shared job dir의 `cancel`
  file을 쓰는 handle로 고정했다.
- Native package smoke는 CI에서 `jniLibs` 산출물 존재, APK assemble, emulator
  `NativeShellBridge` 호출로 고정한다.
- Android document picker는 direct mount가 아니라 copy-in/copy-out이다.
- shellcore-only cancel은 cooperative UI cancel이다. future PTY/userland adapter에서
  실제 interrupt/timeout을 구현해야 한다.

## 5. 다음 세션 첫 작업

정본 workflow:
`docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md`.

1. **Termux T1 helper-backed stream/cancel**
   - helper bootstrap UX와 shared staging workspace 경계를 정한다.
   - long-running stdout, cancel, large output smoke를 polling adapter에 연결한다.

2. **Imported file UX**
   - Import한 파일을 어떻게 inspect할지 정한다: read-only builtin, preview pane,
     또는 structured table reader.

## 6. 재개 명령

```powershell
git -C D:\workspace\terminal-project\terminal status --short --branch
git -C D:\workspace\terminal-project\terminal log --oneline -5
rg -n "PM-3|JNI|workspace|ABI|패키징|Termux|external command" D:\workspace\terminal-project\terminal\docs D:\workspace\terminal-project\terminal\android
```

Android APK assemble:

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
gradle -p android :app:assembleDebug
```

Android Rust JNI library build, Rust toolchain이 있는 환경에서:

```powershell
android/build-rust-jni.ps1 -Profile debug
gradle -p android :app:verifyNativeLibraries
gradle -p android :app:connectedDebugAndroidTest
```

## 7. 주의

- 현재 working tree에는 추적 변경이 없어야 한다. 무시된 local output은
  `.omc/`, `Cargo.lock`, `android/.gradle/`, `android/app/build/`,
  `android/build/` 정도가 남을 수 있다.
- `git add -A` 대신 의도한 파일만 stage한다.
- Windows shell의 PSReadLine profile warning은 exit code가 0이면 무시해도 된다.
- 다음 세션에서 `docs/HANDOFF.md`와 gstack checkpoint
  `pm3-android-jni-handoff`를 함께 보면 가장 빠르게 재개할 수 있다.
