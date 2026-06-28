# HANDOFF — ai-cli-terminal (2026-06-29)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`,
`docs/HISTORY.md`, `CHANGELOG.md`, `docs/INSTALL.md`, `docs/superpowers/` 아래
spec/plan 문서다. 이 파일은 재개 가이드와 다음 작업 우선순위만 압축한다.

## 1. 현재 상태 — v0.3.3 릴리스 완료

작업 repo는 `D:\workspace\terminal-project\terminal`. `main`과 `develop`은
`c3aa63a Release v0.3.3`에서 동기화된 상태였고, 이번 후속 작업은
`codex/v0.3.3-release-smoke-handoff` 브랜치에서 진행 중이다.

공개 릴리스: <https://github.com/ai-cli-terminal/terminal/releases/tag/v0.3.3>

v0.3.3은 Windows 사용자가 더블클릭해 여는 독립 GUI 터미널
`ai-terminal.exe`를 릴리스 자산으로 배포한다. `ai-windows-x86_64.exe`는
GUI가 아니라 CLI helper이며, 더블클릭 안내 문구는 GUI 자산
(`ai-terminal-windows-*.zip` 또는 `AI.Terminal_*_x64-setup.exe`)을 가리키도록
수정되어 있다.

릴리스 자산 핵심:

| 자산 | 역할 |
|---|---|
| `ai-terminal-windows-x86_64-pc-windows-msvc.zip` | Windows portable GUI package |
| `AI.Terminal_0.3.3_x64-setup.exe` | Windows NSIS installer |
| `ai-windows-x86_64.exe` | CLI helper `ai.exe` |
| `ash-windows-x86_64.exe` | CLI/runtime shell `ash.exe` |
| `ai-terminal-android-universal-unsigned.apk` | Android unsigned universal APK |
| Linux `ai`/`ash` binaries | Linux CLI/runtime assets |

## 2. v0.3.3 릴리스 산출물 검증 — 완료

2026-06-29에 GitHub Release에서 실제 공개 자산을 다시 내려받아 검증했다.
검증 디렉터리: `artifacts/release-v0.3.3-smoke` (작업 산출물, 커밋 대상 아님).

체크섬:

- `ai-terminal-windows-x86_64-pc-windows-msvc.zip`:
  `d46fdbf9a5e3557d40ea7d46184212b42513575e518cf886cbba720d93e70f24`
- `AI.Terminal_0.3.3_x64-setup.exe`:
  `89054f320280eb87336b769b4f621adc33ce798819f78ea2b1f92439b0b987cc`

Portable GUI smoke:

- 명령:
  `pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-gui.ps1 -PackageDir <extracted-msvc-package> -StartupTimeoutSeconds 30`
- 결과: `GUI_SMOKE_OK`
- Evidence:
  `artifacts/release-v0.3.3-smoke/zip-extracted/ai-terminal-windows-x86_64-pc-windows-msvc/gui-smoke-evidence.json`
- 확인 내용: package manifest checksum 4개 일치, visible `ai-terminal.exe` window,
  `ash.exe` child, 외부 terminal descendant 없음, transcript output, resize,
  Ctrl-C recovery, Ctrl-D exit, frontend selection/copy/paste/scrollback,
  GUI 내부 AI routing/safety gate/storage-audit 모두 통과.

NSIS installer smoke:

- 최초 실행은 `scripts/smoke-nsis.ps1`가 `WebView2Loader.dll`을 필수 파일로
  요구해 실패했다. MSVC/Tauri 릴리스 산출물은 별도 DLL 없이 동작하므로
  portable packaging과 동일하게 optional로 정정했다.
- 재실행 명령:
  `pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-nsis.ps1 -InstallerPath artifacts\release-v0.3.3-smoke\AI.Terminal_0.3.3_x64-setup.exe`
- 결과: `NSIS_SMOKE_OK`
- Evidence: `artifacts/nsis-install-smoke/nsis-smoke-evidence.json`
- 설치 파일: `ai-terminal.exe`, `ash.exe`, `ai.exe`, `uninstall.exe`
- Optional missing: `WebView2Loader.dll`
- install/uninstall exit code: `0`/`0`; installed GUI smoke도 통과.

## 3. 이번 후속 변경

- `scripts/smoke-nsis.ps1`: `WebView2Loader.dll`을 optional installed file로
  처리하고, 실제 설치된 optional 파일/누락 optional 파일을 evidence JSON에 기록한다.
- `docs/HANDOFF.md`: v0.3.0 중심의 stale 인계를 v0.3.3 릴리스/실 자산 smoke 기준으로 갱신.
- `docs/HISTORY.md`: 2026-06-29 v0.3.3 릴리스 자산 smoke 결과 추가.

## 4. 빌드·검증 환경 메모

- Rust 툴체인은 WSL(Ubuntu) 중심이다. Windows host에서는 PowerShell smoke,
  release asset download, NSIS install smoke를 실행했다.
- 일반 Rust 검증은 WSL에서:
  `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; <cmd>'`
- Android 실제 프로젝트는 `terminal/android`다. repo 루트 밖
  `terminal-project/android` 스텁과 혼동 금지.
- `artifacts/`는 smoke evidence 작업 디렉터리이며 커밋 대상이 아니다.
- `git add -A` 금지. 필요한 파일만 명시 stage한다.

## 5. 다음 작업 후보

1. **v0.3.2 superseded 처리**: v0.3.2 릴리스는 Windows GUI packaging 실패로
   v0.3.3에 대체됐다. 태그를 고쳐 쓰지 말고 GitHub Release note에 superseded
   안내를 남기는 방식이 안전하다.
2. **Explorer double-click evidence**: 자동 smoke는 release zip/installer 모두 green이다.
   엄밀한 "Explorer에서 더블클릭" evidence가 필요하면 수동 operator 단계로 캡처한다.
3. **Android/F-Droid 후속**: 실제 `fdroid build`/buildserver 검증 또는 GitHub Android
   signing secrets 등록/검증을 진행한다.
4. **Windows MSI 후속**: MSI는 여전히 Windows-native Rust+MSVC+WiX packaging host에서
   재검토해야 한다. 현재 release gate는 portable zip + NSIS installer다.
5. **잔여 리뷰 후속**: SemanticCache LRU/용량 상한, audit payload/source 통일,
   preview/pipeline `cmd_parse` 중복 정리.

## 6. 비목표

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸는 것은 비목표다. 이 파일은 CLI helper다.
- GUI 완료 기준을 Windows Terminal/PowerShell/Git Bash에서 `ash.exe` 수동 실행으로
  되돌리지 않는다. `ash.exe`는 GUI 내부 runtime 및 별도 CLI asset이다.
