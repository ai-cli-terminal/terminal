# HANDOFF — ai-cli-terminal (2026-07-24)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`, `docs/HISTORY.md`,
`CHANGELOG.md`, `docs/INSTALL.md`, `docs/releases/`, `docs/superpowers/` 아래 spec/plan 문서다.
이 파일은 **재개 가이드와 다음 작업 우선순위만** 압축한다. 세부 이력은 위 정본 문서와 git 로그에 있다.

## 0. 재개점 (2026-07-24)

- **★ windows-wsl-parity G1 PowerShell 페인 T2~T6 완료. `develop`=`df26c9a`**(PR #109 머지).
  - T2~T4 백엔드: `powershell_plan`/`powershell_command`, `probe_powershell`, runtime inventory,
    `terminal_open_runtime("powershell")` ConPTY 경로.
  - T5 프론트: `RuntimeId`/런타임 선택 UI/라벨·노트/launch summary/launch key 결선.
  - T6: `AI_TERMINAL_GUI_SMOKE_RUNTIME=powershell`로 production GUI가 PowerShell 페인을 자동 시작하고
    `Write-Output PWSH_SMOKE_OK`를 전송하는 repeatable smoke 경로 추가. 2026-07-24 이 host에서 PowerShell 7.6.3
    `pwsh.exe` child, GUI 내부 마커 출력, transcript와 PrintWindow 화면 evidence를 확인했다.
  - 검증: 프론트 6파일 25테스트, TypeScript/Vite build, desktop Rust 4테스트, Windows ConPTY smoke,
    PR CI 5잡 green(마지막 Android JNI 잡은 merge 뒤 계속 실행). 다음 Windows 기능 후보는
    GT4(gated GUI 입력창 + runtime pane `[RAW]` 라벨)다.
- **★ desktop 프론트 테스트 하네스 랜딩.**(PR #111 머지, base=develop, CI green).
  `desktop/src`(자동검증 전무였음)에 **vitest+happy-dom 하네스** + 런타임 순수 로직 특성화 테스트 6파일(23테스트)
  + CI `desktop-frontend` 잡(ubuntu-latest, **`npm install`**)으로 프론트 CI authoritative화. 정본
  `docs/superpowers/{specs,plans}/2026-07-19-desktop-frontend-test-harness*`. 이 host `cd desktop && npm test`(Node
  v24)로 재검증 가능. 함정 2개(정본 [[terminal-build-env]] 메모리 미러): ① 순환 모듈 TDZ(`layout.ts` 최상위
  `loadWorkspaceState()`) → `desktop/test/setup.ts`가 의존성순서 `await import()` **top-level 워밍** 후 테스트는
  정적 import. 테스트 대상 소스모듈 스텁 mock 금지. ② `npm ci`가 vitest/happy-dom의 optional wasm/napi 전이의존
  (`@emnapi/*`) lockfile mismatch로 실패 → CI는 `npm install`. PowerShell 런타임 추가 후 25테스트로 증가했다.
- **v0.5.0 릴리스 완료.** `develop→main` 2단계 PR(#102 버전범프 release→develop, #103 develop→main).
  `main`=`9c6e218`·`develop`=`5b6963e`(트리 동기, main이 merge commit 1 앞), 태그 **v0.5.0** 발행,
  공개 Release 자산 14개(Linux/Windows `ai`·`ash` + `.sha256`, Windows GUI zip + NSIS installer
  `AI.Terminal_0.5.0_x64-setup.exe`, `ai-terminal-android-universal-unsigned.apk`). release.yml 5잡 success.
  - 버전 0.4.0→0.5.0(minor, Android AI 신기능). 범프 `4acc7bb`: VERSION·Cargo(package version만)·
    desktop 5(package.json·package-lock[자기 패키지만]·src-tauri Cargo.toml·Cargo.lock[자기만]·tauri.conf.json)·
    CHANGELOG[0.5.0]·README·**F-Droid metadata 3곳**(versionCode 500).
  - 교훈(재확인): **버전 범프 시 Android F-Droid metadata 3곳 동반 갱신** 필수 —
    `android/fdroid-version.properties`, `android/fdroiddata/metadata/dev.aiterminal.android.yml`
    (Builds versionName/versionCode + CurrentVersion/CurrentVersionCode),
    `android/fastlane/.../changelogs/<versionCode>.txt` 신규. `verifyFdroidReleaseInputs`가 강제하며
    데스크톱 CI엔 안 걸리고 release.yml android APK 잡(태그 push)에서만 터진다
    (로컬 `cd android && ./gradlew :app:verifyFdroidReleaseInputs`로 선검증). third-party 의존·
    desktop lockfile 의존 버전은 불변, 자기 패키지 version만 범프. `commit: TODO_NEXT_ANDROID_RELEASE_COMMIT` 미기입.
- **Android AI 보조 = 실 transport까지 완료.** 1차 mock(#99·#100: 자연어→AI 제안, 자동 실행 금지 §3-11) +
  2차 실 transport(#101): OkHttp JNI transport(Rust `JniHttpTransport` + Kotlin `NativeHttp`, Android 시스템 TLS로
  rustls 크로스컴파일 회피), 지속 `MobileAi` 핸들(Rust 헬퍼→C-ABI→JNI→Kotlin `ShellWorker` 생명주기),
  api_key Debug redaction, openai 128k capability, DEBUG-only openai config 주입
  (`/sdcard/Download/ai-terminal-ai-config.json`). Kotlin 툴체인 2.0.21→2.2.21·Android `buildConfig` 활성화.
  §3-11 자동실행 0·하위호환(stateless `eval_line_ai_json`/`nativeEvalLineAi`/iOS C-ABI) 유지.
  정본: `docs/superpowers/{specs,plans}/2026-07-11-android-real-transport*`,
  `docs/android-real-transport-device-verification.md`.
  - **다음 Android = 실기기 `SM-F956N` openai 실 응답 검증**(이 host 밖, device-only). 절차:
    DEBUG APK 빌드 → adb install → config push → AI 토글 → 실 응답·§3-11·fail-soft·logcat api_key redaction·
    핸들 재사용 확인.
  - backlog Minor **랜딩 완료**(#108 `1b97e5d`, TDD): `ShellAiConfig.toString()` apiKey redaction(`<redacted>`
    마커, Rust `mobile_ai.rs` Debug 컨벤션 미러) `4bcb395` + `ShellWorker.close()` `if(executor.isShutdown) return`
    멱등 가드 `c7cc7d1`. gradle 검증(git-bash/PowerShell, JDK21 `C:\tools\jdk-21`, `ANDROID_HOME`=`~\AppData\Local\Android\Sdk`).
- **Wave2 워크스페이스 리팩토링·trust 스택**: v0.4.0 시점에 **전부 main 랜딩 완료**(Wave2 split #93·#94·#95,
  trust 채널 #69~80). 이제 stale — 신규 작업 아님.

## 1. 플랫폼 현황 (요약)

- **Linux/WSL·Windows**: `ai`(CLI) + `ash`(독립 구조화 셸, 안전 게이트·reedline·history·AI 라우팅·MSYS
  bridge) + Windows 독립 GUI `ai-terminal.exe`(portable zip·NSIS installer, 내부 ConPTY runtime). 정본
  `docs/superpowers/specs/2026-06-27-windows-gui-terminal-pivot-design.md`.
- **Android(PM-3)**: shellcore 로컬 터미널 + **AI 보조(실 transport)** — 자연어→제안(기본 mock, DEBUG 빌드+
  config 파일 시 실 openai OkHttp JNI). imported workspace document reader(content kind·byte/line metadata·
  safe summary), `Export Last`(SAF), selected-file helper(`List Files`/`Find Last`), Termux shared staging
  진단(app-write/helper-marker), UTF-8 preview 경계. 실기기 smoke(`SM-F956N`) green(로컬 기능). 실 transport
  openai 실 응답은 실기기 검증 후속. 무서명 universal APK 배포. Termux external은 explicit opt-in.
- **iOS/iPadOS(PM-4)**: 제한적 로컬 터미널 research 경계 고정
  (`docs/superpowers/plans/2026-07-06-ios-ipados-local-terminal-research-boundary.md`). 공통 Rust `mobile`
  JSON eval/state bridge + `cdylib` C ABI(`src/mobile_ffi.rs`, 실 transport 슬라이스서 persistent 핸들 C-ABI
  대칭 추가). 다음 후보 = TestFlight SwiftUI `shellcore` REPL scaffold(macOS/Xcode 환경 필요).
- **RA/PWA companion**: 원격 승인(Noise XX + Ed25519, C-free) + PWA companion(pair/approve/monitor) + managed
  relay(M2) evidence chain 완료. product default `live-loopback`(public bind off, endpoint auto-start
  disabled). 정본 `docs/superpowers/plans/2026-07-05-ra-pwa-relay-managed-runtime-operator-setup-production-closeout.md`.

## 2. 남은 외부 릴리스 blocker

2026-07-24 `-RunMsiBuild` 증거로 **Windows MSI는 ready**가 됐다. 현재
`npm run check:release-followup` 기준 blocked 항목은 **Android signing secrets**
(repo secret names 비어 있음), **F-Droid build/buildserver evidence** 두 개다.
외부 작업자 handoff packet은 `npm run export:release-followup-evidence-packet`. 절차는
`docs/releases/release-followup-runbook.md`. secret 값은 읽거나 저장하지 않는다.

## 3. 빌드·검증 환경 메모

- 코어(`terminal/`)는 Windows native Rust로 release build와 ConPTY 테스트가 가능하다. 이 host의 WSL 배포판은
  2026-07-24 점검에서 등록되지 않아 WSL 명령은 사용할 수 없었다.
- **desktop(`desktop/src-tauri`, Tauri) 빌드(2026-07-24 갱신)**: Windows native `cargo test`와 MSVC 링크,
  Tauri production `--no-bundle` 빌드가 성공했다. production protocol 빌드는 Tauri CLI를 사용해야 하며 plain
  `cargo build --release`는 `devUrl`을 유지한다. Tauri-managed WiX 3.14 도구로 MSI 실제 빌드까지 성공했고
  `msi.checks.{buildExitCodeZero,msiGenerated,msiSha256Recorded}`가 모두 true다.
- **프론트(`desktop/src`)**: Vitest 25테스트 + TypeScript/Vite build를 로컬과 CI에서 검증한다.
- feature gate: 기본 C-free. `storage`(rusqlite)·`tls`(ring/nasm) C 필요 → 게이트. 검증은 `storage tls remote`
  조합 + 무피처 build + `--target aarch64-linux-android` check.
- Android 실제 프로젝트는 `terminal/android`(repo 밖 `terminal-project/android` 스텁과 혼동 금지).
  gradle은 **git-bash/Windows**(JDK21 + `ANDROID_HOME=~/AppData/Local/Android/Sdk`)에서
  `cd .../terminal/android && ./gradlew :app:testDebugUnitTest`. 툴체인: Kotlin 2.2.21·AGP 8.7.3·
  Compose BOM 2024.10.01·buildConfig on·JVM17·compileSdk35·minSdk26.
- **git worktree 함정(2026-07-11)**: 격리 워크트리를 만들면 그 `.git` gitdir이 Windows 절대경로라
  **WSL git이 파싱 불가**(`git worktree list`에 `prunable`로 보임). 워크트리에서 git은 **native Windows git
  (PowerShell 또는 git-bash)**, push는 **메인 레포 `terminal`(WSL git 정상, 브랜치 ref 공유) 경유**.
  cargo/gradle은 워크트리 경로에서 정상 동작.
- `artifacts/`는 smoke evidence 작업 디렉터리(커밋 대상 아님). **`git add -A` 금지** — 명시 파일만.
- git 브랜치: `main` 보호, `develop` 통합, 작업→develop→main 2단계 PR. 상세 [[terminal-build-env]] 메모리.

## 4. 다음 작업 우선순위 — 다음 세션 이관

남은 즉시 우선순위는 Android 실기기 검증과 외부 릴리스 closeout이다. 진입 전 `develop`이 `df26c9a`
(또는 그 이후)인지 확인한다.

### 4.1 G1 T6 — 완료
- PR #109가 `develop`에 머지됐다. repeatable PowerShell startup smoke, transcript, GUI 화면 evidence,
  `pwsh.exe` child 확인까지 완료했다.
- 다음 Windows 기능 후보는 GT4(gated GUI 입력창 + `[RAW]` 라벨)이며 별도 범위다.

### 4.2 Android 실기기 openai 검증 (**실기기 `SM-F956N` 필요, device-only**)
- **상태**: 실 transport(OkHttp JNI) 구현 완료(#101). 실기기 실 openai 응답만 미검증(CI 불가).
- **필요 환경**: `SM-F956N` 실기기 + adb + 실 openai API 키.
- **절차**(정본 `docs/android-real-transport-device-verification.md`):
  1. DEBUG APK 빌드: `cd terminal/android && ./build-rust-jni.sh --profile release && ANDROID_HOME=~/AppData/Local/
     Android/Sdk ./gradlew :app:assembleDebug`
  2. 설치: `adb install -r app/build/outputs/apk/debug/app-debug.apk`
  3. config 배치(실 키): `adb push ai-terminal-ai-config.json /sdcard/Download/ai-terminal-ai-config.json`
     (`{"provider":"openai","model":"gpt-4o-mini","openai_url":"https://api.openai.com","api_key":"sk-..."}`)
- **완료 판정(체크리스트)**: AI 토글 ON→자연어 입력→**openai 실 응답**(mock echo 아님) / 제안만·자동실행 없음(§3-11)
  / 네트워크·키 오류 시 "unavailable" fail-soft·셸 계속(§3-3) / `adb logcat` api_key 평문 미노출(Debug redaction)
  / 여러 줄 입력 시 핸들 재사용(재구성 로그 없음). **정리**: `adb shell rm /sdcard/Download/ai-terminal-ai-config.json`.
- **주의**: API 키를 shell history·docs·evidence·스크린샷에 남기지 말 것.

### 4.3 외부 릴리스 follow-up evidence closeout (**외부 host 필요**)
- **상태**: `npm run check:release-followup` = **blocked**. MSI와 F-Droid build evidence는 ready이고,
  **Android signing secrets 1개 게이트만 미충족**. 정본 런북 `docs/releases/release-followup-runbook.md`.
- **핸드오프 패킷 생성**(secret-free, 외부 작업자용): `npm run export:release-followup-evidence-packet` →
  `artifacts/release-followup-evidence-packet/`(blocked 항목·다음 액션·정확한 외부 명령·안전규칙; secret 값 미포함).
- **MSI — 완료(2026-07-24)**: Windows-native MSVC + Tauri-managed WiX 3.14로
  `pwsh -File .\scripts\smoke-release-followup-preflight.ps1 -RunMsiBuild` 성공.
  `msi.status=ready`와 `msi.checks.{buildExitCodeZero,msiGenerated,msiSha256Recorded}=true`.
- **Android signing**(repo admin): 4개 secret 등록 `gh secret set AI_TERMINAL_ANDROID_{KEYSTORE_BASE64,
  KEYSTORE_PASSWORD,KEY_ALIAS,KEY_PASSWORD}` → `androidSigningSecrets.status=ready`. **secret 값은 읽거나 저장하지
  않는다.** (throwaway 검증: `android\smoke-github-signing-secrets.ps1 -UseThrowawayKeystore` — 실 signing 미완결.)
- **F-Droid — 완료(2026-07-24, Docker local build)**: 공식 fdroidserver 이미지 기반 Linux 환경에서
  `fdroid build -v -l dev.aiterminal.android:500`을 실행해 소스 커밋 `9c6e21868deff7e5991d0d2914ce96e21b41fb01`의
  unsigned APK를 생성했다. 패키지/버전/API 35/4 ABI/SHA256을 확인했고 combined preflight의
  `fdroidBuild.status=ready`와 모든 checks가 true다. 이는 실제 local fdroid build evidence이며 공식
  F-Droid VM buildserver 실행으로 과장하지 않는다. 현재 릴리스는
  **v0.5.0/versionCode 500**(F-Droid metadata 3곳 = `android/fdroid-version.properties`,
  `android/fdroiddata/metadata/dev.aiterminal.android.yml`, `.../changelogs/500.txt`).
- **완료 판정**: `npm run smoke:release-followup-preflight` 후 `closeout.canCloseDocs=true` & `blockedItems` 비어야
  함. 닫히면 `docs/{superpowers/plans/2026-07-01-remaining-work-priority,TROUBLESHOOTING,HANDOFF,HISTORY,TASK}.md`
  갱신. 릴리스 태그·기존 자산은 별도 결정 없으면 불변.

### 4.4 (참고) iOS TestFlight scaffold
- 공통 mobile JSON bridge/C ABI 위 SwiftUI `shellcore` REPL. **macOS/Xcode 필요** — 이 host 밖. (backlog Minor
  2건은 #108로 랜딩 완료.)

## 5. 비목표

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸지 않는다(이 파일은 CLI helper). Windows GUI 표면은 `ai-terminal.exe`.
- iOS/iPadOS는 Linux terminal·package manager·Termux-equivalent userland·downloaded functionality-changing
  code·arbitrary subprocess/PTY/background daemon을 약속하지 않는다(constrained local structured terminal).
- managed relay를 product default로 만들지 않는다(`live-loopback` 유지, managed는 explicit opt-in).
- **Android AI는 제안만** — 어떤 경로도 명령을 자동 실행하지 않는다(§3-11). shellcore·transport 계층 pure 유지.
