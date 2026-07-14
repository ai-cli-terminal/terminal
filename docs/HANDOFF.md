# HANDOFF — ai-cli-terminal (2026-07-14)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`, `docs/HISTORY.md`,
`CHANGELOG.md`, `docs/INSTALL.md`, `docs/releases/`, `docs/superpowers/` 아래 spec/plan 문서다.
이 파일은 **재개 가이드와 다음 작업 우선순위만** 압축한다. 세부 이력은 위 정본 문서와 git 로그에 있다.

## 0. 재개점 (2026-07-14)

- **★ windows-wsl-parity G1 PowerShell 페인 = T2~T5 완료(Draft PR #109), T6만 남음. `develop`=`1b97e5d`**
  (main은 여전히 `9c6e218`·태그 v0.5.0). 정본 `document/planning/builds/windows-wsl-parity/{DESIGN,PLAN,G3-PLAN}.md`.
  안전게이트 축(F6a·G3-C)은 이미 완료; 이번은 GUI 런타임 페인으로 PowerShell 호스팅.
  - **완료(이전)**: **T1(F6a)** #105 `922c823` `src/risk.rs` union-of-shells 위험 룰(pwsh `Remove-Item -Recurse
    -Force`·cmd `del /s /q`·`Format-Volume`·`Stop/Restart-Computer`, 오탐가드). **G3-C** #106 `cbbfb5f` 신규
    `src/gated_backend.rs` + CLI `ai exec --backend <pwsh|wsl|cmd>`(assess-inner-then-wrap: raw 평가 후 실행만 래핑).
  - **★G1 T2~T5 구현(Draft PR #109, `feat/wwp-g1-powershell-pane`, develop 분기)**:
    - **T2~T4 백엔드**(커밋 `7449bfa`, `desktop/src-tauri`): `powershell_plan`(순수)+`powershell_command`(빌더,
      pwsh7 전용·workspace→Windows 호스트 cwd·winexec.rs 패턴+단위테스트 4)·`probe_powershell`+`runtime_inventory`
      배선(missing→`winget install --id Microsoft.PowerShell`)·`terminal_open_runtime` `"powershell"` arm(ConPTY).
    - **T5 프론트**(커밋 `047347b`, `desktop/src`·`index.html`): `RuntimeId` union+드롭다운 option+`runtimeLabels`/
      `runtimeNotes`(Record<RuntimeId> exhaustiveness)+`isRuntimeId` 가드+`runtimeLaunchSummary`(else fallback
      "managed Ubuntu CLI" 오분류 교정)+`paneLaunchKey`(pwsh는 workspaceDir=cwd라 런치키 포함). open 경로는 기존
      제네릭 `terminal_open_runtime` 재사용.
    - **검증**: 백엔드=CI windows 잡 `Desktop src-tauri check`(`cargo check --manifest-path desktop/src-tauri/
      Cargo.toml`)가 MSVC 컴파일 검증(PR #109 4잡 green). 프론트=로컬 `tsc --noEmit`+`npm run build`(tsc && vite
      build) exit 0. **주의: CI cargo check는 --all-targets 아니라 desktop 단위테스트는 CI 미실행·CI에 TS 검사 스텝
      없음** → 백엔드 단위테스트 `cargo test` + 프론트는 로컬 빌드가 authoritative.
  - **다음 = T6 GUI 스모크(desktop-capable 세션 필요)**: PowerShell 페인 열고 `Write-Output PWSH_SMOKE_OK` 마커
    확인(`AI_TERMINAL_GUI_SMOKE_TRANSCRIPT` 트랜스크립트 ash smoke 패턴, `smoke.rs` 또는 `scripts/smoke-gui.ps1`)
    + 실제 페인 열림 수동 확인. **실제 GUI 실행 필요→MSVC 빌드 필수**. 이후 Draft #109 Ready 전환·develop 머지.
  - **★빌드 환경(이 host, 2026-07-14 실측)**: **Windows 네이티브 rustup 설치됨**(`winget install Rustlang.Rustup`,
    무관리자·`~\.cargo\bin`·`stable-x86_64-pc-windows-msvc`). **그러나 MSVC 링커(VS Build Tools)는 관리자 UAC 필요→
    리모트 세션이라 설치 불가**(winget은 WindowsApps 앱별칭이라 `Start-Process -Verb RunAs` 승격 구조적 실패, exit
    1602). `cargo check`도 build.rs 링크로 `link.exe` 필요→로컬 desktop 빌드 불가. **로컬 복귀 시 관리자 터미널서**:
    `winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.22621" --accept-package-agreements --accept-source-agreements`
    설치하면 이 host가 완전한 desktop 빌드 환경(rustup 기설치)→T6·desktop 단위테스트·Tauri 빌드 가능. 프론트 Node 빌드는
    이미 가능(node v24·node_modules). WebView2 런타임 기설치. **WSL은 여전히 desktop 불가**(webkit2gtk/libsoup 미설치).
  - **G3(GT4 gated GUI 입력창+런타임 페인 `[RAW]` 라벨)**: 설계만(`G3-PLAN.md`), G1 이후 별도. SDD 레저
    `.superpowers/sdd/{progress,g3-ledger}.md`.
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

## 2. 외부 릴리스 blocker (이 host에서 못 닫음)

`npm run check:release-followup` 기준 blocked: **Windows MSI**(MSVC+WiX native host 필요),
**Android signing secrets**(repo secret names 비어 있음), **F-Droid build/buildserver evidence**.
외부 작업자 handoff packet은 `npm run export:release-followup-evidence-packet`. 절차는
`docs/releases/release-followup-runbook.md`. secret 값은 읽거나 저장하지 않는다.

## 3. 빌드·검증 환경 메모

- 코어(`terminal/`) Rust 툴체인은 **WSL(Ubuntu)**. 검증: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd
  /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`.
  멀티라인 금지(CRLF) → 스크립트 파일 경유. 종료코드는 `&&/||` 제어흐름으로만 판정(`$?` 문자열확장 무력화).
  파이프 뒤 `&& echo OK`는 거짓양성(exit는 파이프 끝) — `set -o pipefail` 또는 `if`.
- **desktop(`desktop/src-tauri`, Tauri) 빌드(2026-07-14 갱신)**: **Windows 네이티브 rustup 기설치**(`~\.cargo\bin`,
  msvc 툴체인)이나 **MSVC 링커(VS Build Tools) 미설치→로컬 빌드/`cargo check` 불가**(관리자 UAC 필요, §0 설치 명령).
  WSL은 webkit2gtk/libsoup 미설치라 여전히 불가. **desktop Rust 검증 우회 = CI windows 잡 `Desktop src-tauri check`
  (`cargo check --manifest-path desktop/src-tauri/Cargo.toml`)** — PR만 올리면 MSVC 컴파일 검증(단 --all-targets
  아니라 단위테스트 미실행). **프론트(`desktop/src`)는 Node로 로컬 검증 가능**: `cd desktop && npx tsc --noEmit`,
  `npm run build`(tsc && vite build). CI엔 TS 검사 스텝 없음.
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

## 4. 다음 작업 우선순위

1. **★windows-wsl-parity G1 마무리(desktop-capable 세션)**: T2~T5 완료·Draft PR #109(§0). 남은 **T6 GUI
   스모크**(pwsh 페인 열고 `PWSH_SMOKE_OK` 마커) + 실제 페인 수동 확인 → Draft #109 Ready·develop 머지. **먼저 이
   host에 VS Build Tools 설치**(§0·§3 명령)하면 desktop 빌드/T6/단위테스트 가능. 이후 GT4(gated GUI 입력창+`[RAW]`
   라벨 `G3-PLAN.md`).
2. **Android 실기기 openai 검증**: `SM-F956N`에서 실 transport 실 응답(§0, device-only — 이 host 밖).
   절차 `docs/android-real-transport-device-verification.md`.
3. **릴리스 follow-up 외부 evidence closeout**: MSI/Android signing/F-Droid(§2). 외부 host 필요.
4. **iOS TestFlight scaffold**: 공통 mobile JSON bridge/C ABI 위 SwiftUI `shellcore` REPL(macOS/Xcode 필요).
   (backlog Minor 2건은 #108로 랜딩 완료 — §0.)

## 5. 비목표

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸지 않는다(이 파일은 CLI helper). Windows GUI 표면은 `ai-terminal.exe`.
- iOS/iPadOS는 Linux terminal·package manager·Termux-equivalent userland·downloaded functionality-changing
  code·arbitrary subprocess/PTY/background daemon을 약속하지 않는다(constrained local structured terminal).
- managed relay를 product default로 만들지 않는다(`live-loopback` 유지, managed는 explicit opt-in).
- **Android AI는 제안만** — 어떤 경로도 명령을 자동 실행하지 않는다(§3-11). shellcore·transport 계층 pure 유지.
