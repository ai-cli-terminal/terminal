# HANDOFF — ai-cli-terminal (2026-07-09)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`, `docs/HISTORY.md`,
`CHANGELOG.md`, `docs/INSTALL.md`, `docs/releases/`, `docs/superpowers/` 아래 spec/plan 문서다.
이 파일은 **재개 가이드와 다음 작업 우선순위만** 압축한다. 세부 이력은 위 정본 문서와 git 로그에 있다.

## 0. 재개점 (2026-07-09)

- **v0.4.0 릴리스 완료.** `develop→main` 정비(managed relay·Wave1 리팩토링·iOS C ABI 흡수)를
  마쳤다. `main`은 `fcb6627`, 태그 `v0.4.0` 발행, 공개 Release에 자산 14개(Linux/Windows `ai`·`ash`,
  Windows GUI zip + NSIS installer, `ai-terminal-android-universal-unsigned.apk`, 전부 `.sha256`).
  정본 계획: `docs/superpowers/plans/2026-07-08-develop-main-release-reconciliation.md`.
  - 교훈: **버전 범프 시 Android F-Droid metadata 3곳 동반 갱신** 필수 —
    `android/fdroid-version.properties`, `android/fdroiddata/metadata/dev.aiterminal.android.yml`
    (Builds versionName/versionCode + CurrentVersion), `android/fastlane/.../changelogs/<versionCode>.txt`
    신규. `verifyFdroidReleaseInputs`가 강제하며 release.yml android APK 잡에서만 터진다
    (로컬 `cd android && ./gradlew :app:verifyFdroidReleaseInputs`로 선검증).
- **Wave2 워크스페이스 리팩토링 진행 중** (move-only). 정본: `docs/superpowers/{specs,plans}/2026-07-09-wave2-*`.
  `develop` base 3 PR: **#93** `refactor/wave2-main-split`(`src/main.rs` 3041→15줄, `src/cli/` 10모듈),
  **#94** `refactor/wave2-lib-domains`(`lib.rs` 도메인 클러스터, 경로 불변), **#95** `docs/wave2-handoff-slim`
  (이 문서 + 설계/계획 doc). 세 PR은 파일이 안 겹쳐 독립 머지 가능. 각 태스크 WSL 매트릭스 green(test 487).
- **trust 스택(#69~80)**: codex/trust-* 12 DRAFT, `main` 기반 적층(P3-1 signed policy/skill/binary manifest).
  Wave2가 `develop→main` 랜딩된 뒤 **새 main 위로 재-rebase**(Phase 6). Wave2 공격적 분할이라 충돌은
  `cli/command.rs`·`cli/dispatch.rs`로 국한.

## 1. 플랫폼 현황 (요약)

- **Linux/WSL·Windows**: `ai`(CLI) + `ash`(독립 구조화 셸, 안전 게이트·reedline·history·AI 라우팅·MSYS
  bridge) + Windows 독립 GUI `ai-terminal.exe`(portable zip·NSIS installer, 내부 ConPTY runtime). 정본
  `docs/superpowers/specs/2026-06-27-windows-gui-terminal-pivot-design.md`.
- **Android(PM-3)**: shellcore-only 로컬 터미널. imported workspace document reader(content kind·byte/line
  metadata·safe summary), `Export Last`(SAF), selected-file helper(`List Files`/`Find Last`), Termux shared
  staging 진단(app-write/helper-marker), UTF-8 preview 경계. 실기기 smoke(`SM-F956N`) green. 무서명 universal
  APK 배포. Termux external은 explicit opt-in.
- **iOS/iPadOS(PM-4)**: 제한적 로컬 터미널 research 경계 고정
  (`docs/superpowers/plans/2026-07-06-ios-ipados-local-terminal-research-boundary.md`). 공통 Rust `mobile`
  JSON eval/state bridge + `cdylib` C ABI(`src/mobile_ffi.rs`) 추가(Swift/ObjC wrapper용, `mobile_jni`는
  Android 타깃 전용). 다음 후보 = TestFlight SwiftUI `shellcore` REPL scaffold(macOS/Xcode 환경 필요).
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
  검증: `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`.
  멀티라인 금지(CRLF) → 스크립트 파일 경유. 종료코드는 `&&/||` 제어흐름으로만 판정(`$?` 문자열확장 무력화).
  파이프 뒤 `&& echo OK`는 거짓양성(exit는 파이프 끝) — `set -o pipefail` 또는 `if`.
- feature gate: 기본 C-free. `storage`(rusqlite)·`tls`(ring/nasm) C 필요 → 게이트. 검증은 `storage tls remote`
  조합 + 무피처 build + `--target aarch64-linux-android` check.
- Android 실제 프로젝트는 `terminal/android`(repo 밖 `terminal-project/android` 스텁과 혼동 금지).
  gradle은 Windows(JDK21 + `ANDROID_HOME=~/AppData/Local/Android/Sdk`)에서 `./gradlew :app:testDebugUnitTest`.
- `artifacts/`는 smoke evidence 작업 디렉터리(커밋 대상 아님). **`git add -A` 금지** — 명시 파일만.
- git 브랜치: `main` 보호, `develop` 통합, 작업→develop→main 2단계 PR. 상세 [[terminal-build-env]] 메모리.

## 4. 다음 작업 우선순위

1. **Wave2 마무리**: PR #93·#94·#95 CI green 확인 → develop 머지 → `develop→main` 릴리스 PR.
2. **trust 스택 재-rebase(Phase 6)**: Wave2가 main에 랜딩된 뒤 codex/trust-* 12 스택을 새 main 위로
   #69부터 순차 rebase·force-push(`--update-refs`, 문서 충돌은 v0.4.0 우선).
3. **릴리스 follow-up 외부 evidence closeout**: MSI/Android signing/F-Droid(§2). 외부 host 필요.
4. **iOS TestFlight scaffold**: 공통 mobile JSON bridge/C ABI 위 SwiftUI `shellcore` REPL(macOS/Xcode 필요).

## 5. 비목표

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸지 않는다(이 파일은 CLI helper). Windows GUI 표면은 `ai-terminal.exe`.
- iOS/iPadOS는 Linux terminal·package manager·Termux-equivalent userland·downloaded functionality-changing
  code·arbitrary subprocess/PTY/background daemon을 약속하지 않는다(constrained local structured terminal).
- managed relay를 product default로 만들지 않는다(`live-loopback` 유지, managed는 explicit opt-in).
