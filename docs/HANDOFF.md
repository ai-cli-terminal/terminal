# HANDOFF — ai-cli-terminal (2026-06-27)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`,
`docs/HISTORY.md`, `CHANGELOG.md`, `docs/superpowers/` 아래 spec/plan 문서다.
이 파일은 재개 가이드와 다음 작업 우선순위만 압축한다.

## 1. 현재 상태 — v0.3.0 릴리스 완료

작업 repo는 `D:\workspace\terminal-project\terminal`. **`main`·`develop`이 동기**(`main = develop`, 0 커밋 차)이고 **`v0.3.0` 태그가 발행**됐다(release.yml이 ai/ash Linux·Windows 바이너리 + SHA256을 공개 GitHub Release로 업로드 완료). 워킹트리는 clean(`.omc/`만 untracked).

제품 방향은 플랫폼별 독립 로컬 터미널 `ash`다. **Windows native `ash.exe` 기능 완성(로드맵 S1~S7) + 실 AI provider + 게이트 audit + AI usage 기록까지 구현 및 CI 검증 완료**됐다. 남은 완료 조건은 실제 Windows/TTY 수동 검증이다. 2026-06-27 현재 Linux `ash` 경로의 부분 수동 검증과 repository gate는 green이지만, Windows native/TTY/MSYS 검증은 환경 부재로 아직 완료되지 않았다.

`ash`가 제공하는 것(0.3.0):

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

## 2.1. 현재 브랜치/PR 상태 — PR #26

- 브랜치: `codex/ai-usage-recording`.
- PR: #26 `[codex] record AI usage from ask, dispatch, and ash` — draft/open/merge clean.
- CI: 최신 head에서 `fmt · clippy · test`, `cargo audit`, `android JNI packaging`, `windows build + self-contained check` 전부 green.
- 로컬 워킹트리: `android/.omc/` untracked만 남긴 상태로 유지해야 한다. `git add -A` 금지.
- PR #26 문서 후속 커밋: `b75d66d docs: update ai usage pr26 handoff`.

## 2.2. 이번 세션 검증 결과 — 완료/미완료 구분

완료:

- AI usage 기록 후속은 PR #26 CI green으로 구현 검증 완료.
- Linux `ash` 경로에서 격리 config/data로 config fail-soft, mock AI routing, Ollama 미실행 fail-soft, OpenAI no-key fail-soft, Critical 차단, High-risk 비대화형 거부, storage usage/audit 기록 확인.
- Repository gate green:
  `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`,
  `cargo test --features "storage tls remote"`,
  `cargo test`,
  `cargo check --lib --target aarch64-linux-android`.

미완료/보류:

- 실제 Windows native `ash.exe` 실행 검증은 미완료. 현재 환경에 `powershell.exe`, `cmd.exe`, `wsl.exe`, Windows `ash.exe`가 없었다.
- 제공 PTY가 reedline cursor-position query(`ESC[6n`)에 응답하지 않아 line editor TTY 검증을 완료할 수 없었다.
- 실제 Windows Terminal/PowerShell TTY에서 reedline 편집, ↑↓ history recall, Ctrl-C/Ctrl-D, history persistence/secret filtering, 실제 Ollama 응답, ConPTY smoke, Git Bash/MSYS `AI_TERMINAL_WINDOWS_PROFILE=msys`의 `sh -lc` 실행은 다음 세션으로 넘긴다.
- `docs/TASK.md` PM-1의 `Windows 완료 검증`은 아직 `[ ]`가 맞다.

## 3. 빌드·검증 환경 (필수 숙지)

- **Rust 툴체인은 WSL(Ubuntu)에만**. Windows엔 cargo 없음. 검증은 WSL 경유:
  `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
- **feature gate**: default는 C-free. `storage`(SQLite)·`tls`(HTTPS, ring→nasm)·`remote`(Noise). 전체 검증은 `--features "storage tls remote"` + default 둘 다.
- **검증 게이트**: `cargo fmt --all -- --check`(실제 `cargo fmt --all` 후) · `cargo clippy --all-targets --features "storage tls remote" -- -D warnings` · `cargo test --features "storage tls remote"` + default `cargo test`.
- **android 경계**: `cargo check --lib --target aarch64-linux-android`(rustup target add 필요, NDK 불필요).
- **Android 앱**: 진짜 프로젝트는 `terminal/android`(레포 루트 밖 `terminal-project/android`는 빈 스텁 — 혼동 금지). Gradle 8.9 wrapper 커밋됨: `cd terminal/android && ANDROID_HOME=~/AppData/Local/Android/Sdk ./gradlew :app:testDebugUnitTest`.
- **함정**: ① `$?`/`echo $?`로 종료코드 못 잼 → `cmd && echo OK || echo FAIL` 또는 `set -o pipefail`(파이프 마스킹 주의). ② git-bash `/tmp` ≠ WSL `/tmp` → 스크립트는 `/mnt/d/...`에 Write 후 `MSYS_NO_PATHCONV=1 wsl.exe -- bash /mnt/d/.../x.sh`. ③ `git add -A` 금지(.omc 오커밋). ④ config에 필드 추가 Task는 `--lib`만이 아니라 `cargo build --bins`까지(bin/테스트의 Config 리터럴 깨짐). ⑤ **ash 빌트인(`echo`/`cd`/`where` 등 `shellcore::builtins`)은 GatedRunner 외부실행 경로 미경유** → 게이트/audit/MSYS 대상 아님(e2e Ran 검증은 `/usr/bin/true` 같은 외부명령). ⑥ spawn_task가 메인 워킹트리 브랜치를 바꿀 수 있음 → 커밋 전 `git rev-parse --abbrev-ref HEAD` 확인.

## 4. 워크플로

브랜치 전략: `main` 보호, **develop 경유 2단계 PR**(작업브랜치→develop, 릴리스만 develop→main). gh 인증됨(계정 `VelkaressiaBlutkrone`). 슬라이스 흐름: brainstorm→spec(`docs/superpowers/specs/`)→writing-plans(`plans/`)→subagent-driven TDD→**컨트롤러 직접검증(범위·테스트·android·전체게이트 직접 재실행)**→최종 whole-branch 리뷰(opus)→PR→CI green→머지. **서브에이전트 보고는 신뢰하지 말고 직접 재검증**(clippy 오보고·리뷰어 빈응답 flaky 사례 다수). 리뷰어 빈응답 시 4줄 평결 포맷을 명시하면 회수율↑.

## 5. 다음 작업 후보 (우선순위)

1. **PR #26 후속 처리**: 현재 draft/open/merge-clean이고 CI는 전부 green이다(`fmt · clippy · test`, `cargo audit`, `android JNI packaging`, `windows build + self-contained check`). Draft 해제와 머지 여부를 결정한다.
2. **인터랙티브/Windows 수동 검증(Windows 전용 잔여)**: Linux `ash` 경로와 repository gate는 부분 통과했지만, 실제 Windows/TTY + Ollama 환경에서 `ash.exe` 직접 확인이 필요하다. 실행 계획: `docs/superpowers/plans/2026-06-27-windows-ash-manual-verification.md`.
3. **Windows 완료 처리**: 수동 검증 통과 후 `docs/TASK.md`의 PM-1 Windows 완료 검증을 `[x]`로 바꾸고, `docs/HISTORY.md`에 evidence를 기록하고, 이 HANDOFF 우선순위를 갱신한다.
4. **Android PM-3 재개**: Windows 완료 후 보류 해제. shared staging UX(path input 유지 vs SAF-backed directory picker), imported file UX(read-only builtin 또는 structured table reader), APK/F-Droid 우선 배포 경로 결정이 첫 후보(`docs/TASK.md` PM-3).
5. **잔여 리뷰 후속**: SemanticCache/exact 캐시 LRU·용량 상한, `command_executed` audit payload serde_json·source 통일(기존 불일치), preview↔pipeline `cmd_parse` 중복.
6. **원격 승인(RA) 완주**: M1 slice 4b(디바이스 리스너·페어링·게이트→디바이스 왕복) → PWA companion(`docs/TASK.md` RA, `docs/superpowers/specs/2026-06-04-remote-approval-*`).

## 6. 비목표(의도적 제외 — 재논의 전 구현 금지)

- **env 실행 좁히기**: 데스크톱 셸은 자식에게 full env 상속해야 도구(`gh`/`aws`)가 동작 → 해롭다. secret-to-AI 우려는 `context::gather`(raw env 미포함)+mask로 이미 차단. (이번 세션에 발견·문서화.)
- AI 생성 명령 자동 실행(auto_execute=false 유지), `provider="local_or_remote"` 폴백, MSYS PTY/signal·명시 cygpath/tool-discovery(sh가 담당).
