# HANDOFF — ai-cli-terminal (2026-06-28)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`,
`docs/HISTORY.md`, `CHANGELOG.md`, `docs/superpowers/` 아래 spec/plan 문서다.
이 파일은 재개 가이드와 다음 작업 우선순위만 압축한다.

## 1. 현재 상태 — v0.3.0 릴리스 완료

작업 repo는 `D:\workspace\terminal-project\terminal`. **`main`·`develop`이 동기**(`main = develop`, 0 커밋 차)이고 **`v0.3.0` 태그가 발행**됐다(release.yml이 ai/ash Linux·Windows 바이너리 + SHA256을 공개 GitHub Release로 업로드 완료). 워킹트리는 clean(`.omc/`만 untracked).

제품 방향은 Windows에서 **완전한 독립 GUI 터미널 프로그램 `ai-terminal.exe`** 를 제공하는 것으로 정정됐다. 기존 Windows native `ash.exe` 기능 완성(로드맵 S1~S7) + 실 AI provider + 게이트 audit + AI usage 기록은 GUI 앱 내부 child runtime으로 재사용한다. 더 이상 Windows 완료 조건을 "Windows Terminal/PowerShell/Git Bash에서 `ash.exe` 수동 실행"으로 보지 않는다. 새 정본은 `docs/superpowers/specs/2026-06-27-windows-gui-terminal-pivot-design.md`다.

`ash`가 GUI 앱 내부 runtime으로 제공하는 것(0.3.0):

| 영역 | 모듈/동작 |
|---|---|
| config | `[general]`(history_limit/default_shell)·`[ai]`(provider/model/url) fail-soft 로드(`src/config.rs`), `ai doctor` 표시 |
| 안전 게이트 | 외부 실행이 risk→policy→preview→확인→undo 백업 통과(`src/gated_runner.rs` → `pipeline::execute`). Critical 차단/High 확인(비-TTY fail-closed) |
| 라인 에디터 | reedline 편집·↑↓ history·Ctrl-C/D, 비-TTY는 StdinLineReader 폴백(`src/line_editor.rs`, `shellcore::repl::LineReader`) |
| history | `<config_dir>/ash_history` 영속, secret/PII 명령 제외(`FilteringHistory` + mask) |
| AI 라우팅 | 자연어(`ai `/`?`/의문사/한글마커)→AI, 그 외→`eval_line`(`src/ai_router.rs`, `shellcore::repl::AiRouter`). 실패 fail-soft |
| AI provider | config `[ai] provider`로 ollama(기본)/openai/mock(`GatewayAiRouter::from_ai_config`). 키는 `OPENAI_API_KEY` env. openai-HTTPS는 `tls` feature |
| MSYS bridge | `AI_TERMINAL_WINDOWS_PROFILE=msys`+`MSYSTEM` 시 `sh -lc`(`shellcore::msys::{active_profile,bridge_invocation}`) |
| audit 기록 | 게이트 결과→storage(`src/shell_audit.rs`, Ran→commands, 비-Ran→audit_events, source="ash"). `ai exec`와 공유(DRY) |

**경계 규율(전 과정 유지)**: `shellcore`(`src/shellcore/*`)는 android cdylib에도 컴파일된다. 데스크톱 로직(게이트/에디터/AI/audit)은 trait 주입(`ExternalRunner`/`LineReader`/`AiRouter`)으로 분리하고, 데스크톱 전용 의존(reedline/portable-pty/crossterm 등)은 `[target.'cfg(not(target_os="android"))'.dependencies]`에 둔다. **모든 슬라이스에서 `cargo check --lib --target aarch64-linux-android` green을 유지했다.**

## 2. 이번 세션(2026-06-27) 머지 — PR #12~#24

- #12 Android Termux T1 helper(+Gradle 8.9 wrapper). #16 flaky `ShellWorkerTest` 수정.
- #13 S1 config · #14 S2 안전 게이트 · #15 S3 line editor · #17 S4 history · #18 S5 AI 통합 · #19 실 AI provider · #20 S6 MSYS bridge · #21 S7 문서.
- #22 S2 후속(ash gate audit). #23 chore(0.2.4→0.3.0 bump+CHANGELOG). #24 release(develop→main). 태그 `v0.3.0`.
- CI 회귀 2건 수정: android JNI의 termios target-gate, 에뮬레이터 KVM 활성화.

## 2.1. 현재 브랜치/PR 상태

- 브랜치: `codex/gui-terminal-pivot`.
- 이전 PR #26 `[codex] record AI usage from ask, dispatch, and ash`는 develop에 병합된 상태로 취급한다.
- 현재 작업: Windows GUI terminal pivot 정본 갱신 + Tauri/xterm desktop scaffold + PTY bridge + packaging/installer smoke 자동화.
- 로컬 워킹트리: `android/.omc/` untracked만 남긴 상태로 유지해야 한다. `git add -A` 금지.
- PR #26 문서 후속 커밋: `b75d66d docs: update ai usage pr26 handoff`.

## 2.2. 이번 세션 검증 결과 — 완료/미완료 구분

완료:

- AI usage 기록 후속은 PR #26 CI green으로 구현 검증 완료.
- Linux `ash` 경로에서 격리 config/data로 config fail-soft, mock AI routing, Ollama 미실행 fail-soft, OpenAI no-key fail-soft, Critical 차단, High-risk 비대화형 거부, storage usage/audit 기록 확인.
- Windows/Tauri Rust toolchain blocker는 user-local apt extraction으로 해소했다. MinGW와 WebKitGTK dev packages는 `~/.local/opt/ai-terminal-deps/root` 아래에 풀려 있다.
- Desktop GUI checks green:
  `npm run build`,
  `cargo check --manifest-path desktop/src-tauri/Cargo.toml`,
  `cargo check --manifest-path desktop/src-tauri/Cargo.toml --target x86_64-pc-windows-gnu`,
  `npm run tauri -- build --target x86_64-pc-windows-gnu --no-bundle --ci`.
- Windows target build 산출물 확인: `desktop/src-tauri/target/x86_64-pc-windows-gnu/release/ai-terminal.exe` (`PE32+ executable (GUI) x86-64`).
- Sidecar build/staging green:
  `cargo build --release --bins --features "storage tls remote" --target x86_64-pc-windows-gnu`,
  `npm run stage:windows-sidecars`,
  `npm run tauri -- build --target x86_64-pc-windows-gnu --no-bundle --ci --config src-tauri/tauri.windows.conf.example.json`.
- Portable smoke package generated at
  `desktop/src-tauri/target/x86_64-pc-windows-gnu/release/portable/ai-terminal-windows-x86_64-pc-windows-gnu`
  with `ai-terminal.exe`, `WebView2Loader.dll`, `ash.exe`, `ai.exe`, `smoke-gui.ps1`, and `SHA256SUMS.txt`.
- Portable release archive generated at
  `desktop/src-tauri/target/x86_64-pc-windows-gnu/release/portable/ai-terminal-windows-x86_64-pc-windows-gnu.zip`
  with `.zip.sha256`.
- Windows GUI smoke script added: `scripts/smoke-gui.ps1`. It verifies portable checksums, launches `ai-terminal.exe`, waits for a visible main window, checks that `ash.exe` is a child process, checks for unexpected external shell descendants, closes the app, verifies child cleanup, and writes `gui-smoke-evidence.json`.
- `scripts/smoke-gui.ps1` now also writes `gui-smoke-screenshot.png`, `gui-smoke-resize-screenshot.png`, `gui-smoke-frontend-evidence.json`, `gui-smoke-frontend-screenshot.png`, `gui-smoke-ash-integration-evidence.json`, `gui-smoke-ash-integration-screenshot.png`, `gui-smoke-ctrl-c-screenshot.png`, `gui-smoke-ctrl-d-screenshot.png`, and `gui-smoke-transcript.txt`, then records those paths in the evidence JSON. Capture waits for the target window to reach a real size and uses `PrintWindow` so screenshots target `ai-terminal.exe`, not whatever window is foreground. The default smoke command is `print AI_TERMINAL_GUI_SMOKE_OK`; the frontend reads it through `terminal_smoke_command`, writes it through the normal `terminal_write` path, and the script requires transcript output `AI_TERMINAL_GUI_SMOKE_OK` with no `error:`. The smoke also resizes the actual app window with `SetWindowPos`, records before/target/after bounds, verifies xterm selection/copy/paste/scrollback through frontend evidence (`frontend.status=passed`, `frontend.copy.copied=true`, `frontend.scrollback.scrolled=true`), uses an isolated mock config/data root to verify GUI-internal ash AI routing/safety gate/storage/audit (`ashIntegration.transcriptEvidence.aiRouted=true`, `safetyGateBlocked=true`, `externalCommandRan=true`, `ashIntegration.database.usagePersisted=true`, `commandHistoryPersisted=true`, `auditBlockedPersisted=true`), uses `AI_TERMINAL_GUI_SMOKE_CTRL_C_DELAY_MS` to verify pending input can be interrupted and recovered in the same `ash.exe` child (`ctrlC.recovered=true`, `ctrlC.ashStillRunning=true`), then uses `AI_TERMINAL_GUI_SMOKE_CTRL_D_DELAY_MS` to verify `ash.exe` exits and records `ctrlD.ashExited=true`. Latest portable/installed screenshots show the `AI Terminal` window with prompt/input/output, resize evidence, frontend UX evidence, ash integration evidence, Ctrl-C recovery evidence, and Ctrl-D/exit evidence.
- Windows GUI automated smoke passed on real Windows host:
  `pwsh -NoProfile -ExecutionPolicy Bypass -File ..\scripts\smoke-gui.ps1`
  from `desktop/` after packaging. Evidence:
  `desktop/src-tauri/target/x86_64-pc-windows-gnu/release/portable/ai-terminal-windows-x86_64-pc-windows-gnu/gui-smoke-evidence.json`.
- NSIS installer packaging now works from WSL via user-local `makensis`/`proot` setup:
  `npm run setup:wsl-nsis`, then
  `npm run tauri -- build --target x86_64-pc-windows-gnu --ci --config src-tauri/tauri.windows.conf.example.json`.
  Artifact:
  `desktop/src-tauri/target/x86_64-pc-windows-gnu/release/bundle/nsis/AI Terminal_0.1.0_x64-setup.exe`
  (`SHA256 c90253b6b2f08a097114094ef84240c304ca8fe6c5aa73e7e1b9e194d1d776bd`, refreshed 2026-06-28).
- NSIS installer smoke script added and passed:
  `pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-nsis.ps1`.
  It silently installs to `artifacts\nsis-install-smoke\AITerminal`, waits briefly for the fresh install tree to settle, verifies installed files, runs installed GUI smoke with `-SkipChecksums`, silently uninstalls, and checks no install dir/shortcut/uninstall registry leftovers remain. Evidence:
  `artifacts/nsis-install-smoke/nsis-smoke-evidence.json`.
- Repository gate green:
  `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`,
  `cargo test --features "storage tls remote"`,
  `cargo test`,
  `cargo check --lib --target aarch64-linux-android`.

미완료/보류:

- `ai-terminal.exe` 독립 GUI 앱은 1차 skeleton/PTY bridge와 G2 hardening(restart UX, 종료 시 child cleanup, sidecar path policy/staging, portable package + checksums + GUI smoke script)이 추가됐고, Windows target executable build, 자동 GUI smoke(창/`ash.exe` child/외부 shell descendant 없음/prompt·input·output screenshot/transcript/resize screenshot/selection-copy-paste-scrollback evidence/GUI 내부 AI routing/safety gate/storage/audit evidence/Ctrl-C recovery/Ctrl-D exit/cleanup), NSIS installer artifact 생성, installer silent install/installed GUI smoke/uninstall cleanup까지 통과했다.
- 실제 Windows native `ash.exe` 직접 실행 검증은 미완료지만, 이제 GUI 완료 조건이 아니라 runtime regression evidence다.
- 제공 PTY가 reedline cursor-position query(`ESC[6n`)에 응답하지 않아 line editor TTY 검증을 완료할 수 없었다.
- `docs/TASK.md` PM-1의 완료 기준은 `ai-terminal.exe` GUI 검증으로 바뀌었다.

## 3. 빌드·검증 환경 (필수 숙지)

- **Rust 툴체인은 WSL(Ubuntu)에만**. Windows엔 cargo 없음. 검증은 WSL 경유:
  `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **feature gate**: default는 C-free. `storage`(SQLite)·`tls`(HTTPS, ring→nasm)·`remote`(Noise). 전체 검증은 `--features "storage tls remote"` + default 둘 다.
- **검증 게이트**: `cargo fmt --all -- --check`(실제 `cargo fmt --all` 후) · `cargo clippy --all-targets --features "storage tls remote" -- -D warnings` · `cargo test --features "storage tls remote"` + default `cargo test`.
- **android 경계**: `cargo check --lib --target aarch64-linux-android`(rustup target add 필요, NDK 불필요).
- **Android 앱**: 진짜 프로젝트는 `terminal/android`(레포 루트 밖 `terminal-project/android`는 빈 스텁 — 혼동 금지). Gradle 8.9 wrapper 커밋됨: `cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest`. PM-3 shared staging UX는 path input 유지 + primary shared-storage SAF picker 보조로 결정됐고, imported file UX는 `Open Last` read-only reopen으로 닫았다. 배포 경로는 direct APK/GitHub Release → F-Droid 준비 우선, Google Play는 Termux-enabled build 정책 검토 후로 결정했다. WSL용 `android/build-rust-jni.sh --profile release` + NDK r28c Linux prebuilt로 4개 ABI JNI staging을 통과했고, `:app:assembleRelease :app:verifyNativeLibraries` green이다. 현재 산출물은 unsigned `app-release-unsigned.apk`; signing은 `AI_TERMINAL_ANDROID_KEYSTORE*` env vars로 opt-in. CI/release Android jobs use the checked-in Gradle wrapper, API 35/build-tools 35.0.0/NDK r28c. Release workflow uploads Android universal APK(+SHA256) as tag release assets.
- **함정**: ① `$?`/`echo $?`로 종료코드 못 잼 → `cmd && echo OK || echo FAIL` 또는 `set -o pipefail`(파이프 마스킹 주의). ② git-bash `/tmp` ≠ WSL `/tmp` → 스크립트는 `/mnt/d/...`에 Write 후 `MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/d/.../x.sh`. ③ `git add -A` 금지(.omc 오커밋). ④ config에 필드 추가 Task는 `--lib`만이 아니라 `cargo build --bins`까지(bin/테스트의 Config 리터럴 깨짐). ⑤ **ash 빌트인(`echo`/`cd`/`where` 등 `shellcore::builtins`)은 GatedRunner 외부실행 경로 미경유** → 게이트/audit/MSYS 대상 아님(e2e Ran 검증은 `/usr/bin/true` 같은 외부명령). ⑥ spawn_task가 메인 워킹트리 브랜치를 바꿀 수 있음 → 커밋 전 `git rev-parse --abbrev-ref HEAD` 확인.

## 4. 워크플로

브랜치 전략: `main` 보호, **develop 경유 2단계 PR**(작업브랜치→develop, 릴리스만 develop→main). gh 인증됨(계정 `VelkaressiaBlutkrone`). 슬라이스 흐름: brainstorm→spec(`docs/superpowers/specs/`)→writing-plans(`plans/`)→subagent-driven TDD→**컨트롤러 직접검증(범위·테스트·android·전체게이트 직접 재실행)**→최종 whole-branch 리뷰(opus)→PR→CI green→머지. **서브에이전트 보고는 신뢰하지 말고 직접 재검증**(clippy 오보고·리뷰어 빈응답 flaky 사례 다수). 리뷰어 빈응답 시 4줄 평결 포맷을 명시하면 회수율↑.

## 5. 다음 작업 후보 (우선순위)

1. **G4 Windows installer follow-up**: NSIS artifact와 install/run/uninstall smoke는 green이다. 남은 것은 MSI 가능 여부 검토다. Linux cross-host에서는 MSI가 ignored 처리됐고, NSIS는 `desktop/scripts/setup-wsl-nsis.sh`로 user-local toolchain을 준비한다.
2. **Real Windows GUI manual smoke**: 더블클릭 실행, 외부 terminal window 미생성, `ash` prompt/명령/AI/gate/storage 통과를 수동 evidence로 남긴다. 자동 process/window/terminal UX/AI/gate/storage/audit/cleanup smoke는 green.
3. **Toolchain persistence**: 새 셸에서는 `export PATH="$HOME/.local/bin:$HOME/.local/opt/ai-terminal-deps/root/usr/bin:$PATH"`를 Windows GNU target/NSIS 빌드 전에 설정한다. `~/.local/bin/node`/`npm`은 WSL Tauri CLI 실행용 user-local Node다. Linux Tauri check에는 `PKG_CONFIG_PATH="$HOME/.local/opt/ai-terminal-deps/root/usr/lib/x86_64-linux-gnu/pkgconfig:$HOME/.local/opt/ai-terminal-deps/root/usr/share/pkgconfig:$PKG_CONFIG_PATH"`가 필요하다.
4. **Android 후속**: PM-3 UX/배포 결정, unsigned release APK packaging, GitHub Release APK asset 자동화는 닫혔다. 초기 fastlane/F-Droid metadata도 있다. 다음 후보는 실제 signing secrets 등록/검증과 F-Droid reproducibility 준비(`docs/superpowers/specs/2026-06-28-android-distribution-route.md`).
5. **잔여 리뷰 후속**: SemanticCache/exact 캐시 LRU·용량 상한, `command_executed` audit payload serde_json·source 통일(기존 불일치), preview↔pipeline `cmd_parse` 중복.
6. **원격 승인(RA) 완주**: M1 slice 4b(디바이스 리스너·페어링·게이트→디바이스 왕복) → PWA companion(`docs/TASK.md` RA, `docs/superpowers/specs/2026-06-04-remote-approval-*`).

## 6. 비목표(의도적 제외 — 재논의 전 구현 금지)

- **env 실행 좁히기**: 데스크톱 셸은 자식에게 full env 상속해야 도구(`gh`/`aws`)가 동작 → 해롭다. secret-to-AI 우려는 `context::gather`(raw env 미포함)+mask로 이미 차단. (이번 세션에 발견·문서화.)
- AI 생성 명령 자동 실행(auto_execute=false 유지), `provider="local_or_remote"` 폴백, MSYS PTY/signal·명시 cygpath/tool-discovery(sh가 담당).
