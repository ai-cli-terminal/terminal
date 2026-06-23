# HANDOFF — ai-cli-terminal (2026-06-23)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`,
`docs/HISTORY.md`, `docs/superpowers/` 아래 spec/plan 문서다. 이 파일은
재개 가이드와 다음 작업 우선순위만 압축한다.

## 1. 현재 상태

작업 repo는 `D:\workspace\terminal-project\terminal`이고 브랜치는 `main`이다.
현재 `main`은 `origin/main`과 동기화되어 있다.

제품 방향은 플랫폼별 독립 로컬 터미널 `ash`다. 모바일도 PWA 승인 화면이
아니라 온디바이스 로컬 터미널을 장기 목표로 둔다. Android는 PM-3 로컬
터미널 스파이크가 진행 중이며, 현재는 Kotlin/Compose UI + worker thread +
Rust `MobileShell` JNI bridge까지 완료된 상태다.

## 2. 최근 완료 산출

| 커밋 | 내용 |
|---|---|
| `4739139` | `src/mobile.rs`에 Rust `MobileShell` pure core boundary 추가 |
| `0e419b9` | `android/` Kotlin/Compose skeleton과 `ShellWorker` background thread 추가 |
| `68a3ccd` | `FakeShellBridge` 제거, `NativeShellBridge` -> Rust JNI `MobileShell` 연결 |
| `57e5ab4` | JNI bridge rustfmt 정리 |

주요 파일:

| 파일 | 내용 |
|---|---|
| `src/mobile.rs` | Android/iOS가 감쌀 pure `shellcore` session boundary |
| `src/mobile_jni.rs` | Android JNI export `NativeShellBridge.nativeEvalLine(input, stateJson)` |
| `android/app/src/main/java/dev/aiterminal/android/ShellBridge.kt` | Kotlin `NativeShellBridge`, JSON state encode/decode, native load error handling |
| `android/build-rust-jni.ps1` | NDK linker로 `libai_terminal.so`를 빌드하고 `jniLibs/<abi>`로 복사 |
| `android/README.md` | Android native library build + APK assemble 절차 |
| `docs/superpowers/specs/2026-06-23-android-local-terminal-spike.md` | PM-3 Android local terminal spike spec |
| `docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md` | PM workflow, PM-3D/PM-3E 후속 작업 |

## 3. 검증 상태

- 로컬 `gradle -p android :app:assembleDebug` 통과.
- 로컬 `git diff --check` 통과.
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

로컬 Windows 세션에는 `cargo`/`rustup`이 없어서 `android/build-rust-jni.ps1`로
실제 Android `.so` 생성은 실행하지 못했다. 스크립트 PowerShell parse는 통과했다.

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
- Android MVP는 계속 `shellcore-only`다. Termux-compatible 또는 bundled
  minimal userland는 workspace/files 경계가 검증된 뒤 비교한다.

## 5. 다음 세션 첫 작업

정본 workflow:
`docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md`.

1. **PM-3D — Workspace와 파일**
   - app-private workspace root를 정의한다.
   - Android document API import/export 경계를 정한다.
   - `MobileShell` state의 cwd를 실제 workspace 표시와 연결한다.
   - 좁은 모바일 화면용 cwd/workspace/status 표현을 결정한다.

2. **Android Rust `.so` 전체 ABI/CI 패키징**
   - `arm64-v8a`, `armeabi-v7a`, `x86`, `x86_64` 대상 빌드를 자동화한다.
   - `android/build-rust-jni.ps1`를 CI에서도 실행 가능한 형태로 다듬는다.
   - `jniLibs` 패키징 후 native smoke를 추가한다.

3. **Worker behavior test**
   - JVM 또는 instrumentation test로 `ShellWorker`가 UI thread 밖에서 평가하고
     main thread로 결과를 post하는 계약을 고정한다.

4. **PM-3E — 외부 명령 전략 비교**
   - `shellcore-only`, Termux-compatible, bundled minimal userland를 비교한다.
   - 이 작업은 workspace/files 경계가 먼저 정리된 뒤 진행한다.

## 6. 재개 명령

```powershell
git -C D:\workspace\terminal-project\terminal status --short --branch
git -C D:\workspace\terminal-project\terminal log --oneline -5
rg -n "PM-3|JNI|workspace|ABI|패키징" D:\workspace\terminal-project\terminal\docs D:\workspace\terminal-project\terminal\android
```

Android APK assemble:

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
gradle -p android :app:assembleDebug
```

Android Rust JNI library build, Rust toolchain이 있는 환경에서:

```powershell
rustup target add aarch64-linux-android
android/build-rust-jni.ps1 -Profile debug -Targets aarch64-linux-android
```

## 7. 주의

- 현재 working tree에는 추적 변경이 없어야 한다. 무시된 local output은
  `.omc/`, `Cargo.lock`, `android/.gradle/`, `android/app/build/`,
  `android/build/` 정도가 남을 수 있다.
- `git add -A` 대신 의도한 파일만 stage한다.
- Windows shell의 PSReadLine profile warning은 exit code가 0이면 무시해도 된다.
- 다음 세션에서 `docs/HANDOFF.md`와 gstack checkpoint
  `pm3-android-jni-handoff`를 함께 보면 가장 빠르게 재개할 수 있다.
