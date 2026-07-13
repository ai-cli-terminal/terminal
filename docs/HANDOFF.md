# HANDOFF — ai-cli-terminal (2026-07-13)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`, `docs/HISTORY.md`,
`CHANGELOG.md`, `docs/INSTALL.md`, `docs/releases/`, `docs/superpowers/` 아래 spec/plan 문서다.
이 파일은 **재개 가이드와 다음 작업 우선순위만** 압축한다. 세부 이력은 위 정본 문서와 git 로그에 있다.

## 0. 재개점 (2026-07-13)

- **★ windows-wsl-parity (Approach A) 안전게이트 축 = 코어 완료. `develop`=`cbbfb5f`**(main은 여전히
  `9c6e218`·태그 v0.5.0). office-hours 감사에서 확정: 실행 표면이 2개로 갈림 — ① `ash` 표면(구조화 셸 +
  AI 안전게이트) ② 런타임 페인(WSL/Docker/AI-CLI raw ConPTY, **게이트 없음**, GUI 전용). "PowerShell/WSL을
  다 할 수 있나?" → 부분(WSL 페인은 GUI에 이미 있고 안전망 없음, PowerShell 페인은 부재). 정본
  `document/planning/builds/windows-wsl-parity/{DESIGN,PLAN,G3-PLAN}.md`.
  - **T1(F6a) 랜딩**(#105, `922c823`): `src/risk.rs`에 union-of-shells 위험 룰(PowerShell `Remove-Item
    -Recurse -Force`≥80·cmd `del /s /q`·`Format-Volume`/`format <drive>`·`Stop/Restart-Computer`, 대소문자
    무시, 오탐가드 `Get-ChildItem -Recurse`/`git format-patch`→Low).
  - **G3-C(gated backend 명령) 랜딩**(#106, `cbbfb5f`): 신규 `src/gated_backend.rs`(`Backend` enum·
    `host_wrap`·`gated_backend_run`) + CLI `ai exec --backend <pwsh|wsl|cmd>`. **assess-inner-then-wrap**:
    게이트는 raw 명령 평가(F6a 매칭), 실행만 `pwsh -c "..."`로 래핑(감싼 문자열 평가는 F6a 놓쳐 버그).
    `gated_runner`에서 `build_exec_environment()` DRY 추출. 이제 `ai exec --backend pwsh "Remove-Item -Recurse
    -Force C:\"`가 실차단 — T1 룰이 dormant→라이브. Approach C 확정(A=키스트로크버퍼 기각, B=셸훅 후속·cmd불가).
  - **다음 = desktop-capable 세션**: (a) G1 PowerShell 런타임 페인(PLAN.md T2~T6: `probe_powershell`/
    `powershell_command` pwsh7-only + `terminal_open_runtime` "powershell" arm + 리본 버튼 + GUI smoke),
    (b) GT4(gated 명령 GUI 입력창 + 기존 런타임 페인 `[RAW]` 라벨). **이 host의 WSL에선 desktop `src-tauri`
    (Tauri) `cargo check` 불가** — `libsoup-3.0`/webkit2gtk 미설치. SDD 레저 `.superpowers/sdd/{progress,g3-ledger}.md`.
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
  - backlog Minor(코드, 이 host 가능): `ShellAiConfig.toString()` apiKey redaction(Kotlin 미러 — Rust만
    redact됨, live sink는 없음), `ShellWorker.close()` idempotency 가드(`if(executor.isShutdown) return`).
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

- Rust 툴체인은 **WSL(Ubuntu) 전용**. Windows host는 cargo 없음(PowerShell smoke·release asset·NSIS만).
  검증: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`.
  멀티라인 금지(CRLF) → 스크립트 파일 경유. 종료코드는 `&&/||` 제어흐름으로만 판정(`$?` 문자열확장 무력화).
  파이프 뒤 `&& echo OK`는 거짓양성(exit는 파이프 끝) — `set -o pipefail` 또는 `if`.
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

1. **windows-wsl-parity desktop 표면(desktop-capable 세션 필요)**: (a) G1 PowerShell 런타임 페인
   (`PLAN.md` T2~T6), (b) GT4 gated 명령 GUI 입력창 + 런타임 페인 `[RAW]` 라벨(`G3-PLAN.md`). 코어 축
   (F6a·gated backend)은 완료(§0). 이 host WSL에선 Tauri `cargo check` 불가(libsoup/webkit).
2. **Android 실기기 openai 검증**: `SM-F956N`에서 실 transport 실 응답(§0, device-only — 이 host 밖).
   절차 `docs/android-real-transport-device-verification.md`.
3. **backlog Minor(코드)**: `ShellAiConfig.toString()` apiKey redaction, `ShellWorker.close()` idempotency 가드.
4. **릴리스 follow-up 외부 evidence closeout**: MSI/Android signing/F-Droid(§2). 외부 host 필요.
5. **iOS TestFlight scaffold**: 공통 mobile JSON bridge/C ABI 위 SwiftUI `shellcore` REPL(macOS/Xcode 필요).

## 5. 비목표

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸지 않는다(이 파일은 CLI helper). Windows GUI 표면은 `ai-terminal.exe`.
- iOS/iPadOS는 Linux terminal·package manager·Termux-equivalent userland·downloaded functionality-changing
  code·arbitrary subprocess/PTY/background daemon을 약속하지 않는다(constrained local structured terminal).
- managed relay를 product default로 만들지 않는다(`live-loopback` 유지, managed는 explicit opt-in).
- **Android AI는 제안만** — 어떤 경로도 명령을 자동 실행하지 않는다(§3-11). shellcore·transport 계층 pure 유지.
