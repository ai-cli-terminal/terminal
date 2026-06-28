# AI Terminal (`ai`)

일반 리눅스 터미널과 **완전 호환**되는 실행 환경을 유지하면서 AI 명령 생성·설명·디버깅·로그 분석·자동화를 **안전하게** 결합하는 단일 바이너리 터미널. 설계 철학은 **"AI는 명령 실행자가 아니라 의사결정 보조자."**

> 이 저장소는 **구현 repo**다. `../document/`의 v3.3 설계는 Phase 1 기준 정본이고, 현재 제품 방향은 `docs/superpowers/specs/`의 독립 `ash` 셸 피벗 문서가 우선한다.

## 문서 (`docs/`)

| 문서 | 내용 |
|---|---|
| [PRD](docs/PRD.md) | 제품 목표·범위·MVP 포함/제외·KPI |
| [INSTALL](docs/INSTALL.md) | Linux/WSL/Windows native 설치와 검증 |
| [TASK](docs/TASK.md) | MVP+ 구현 백로그(M0~M4 체크리스트) |
| [WORKFLOW](docs/WORKFLOW.md) | Git·커밋·PR·CI·빌드 명령 |
| [HISTORY](docs/HISTORY.md) | 변경/결정 로그 |
| [RULES](docs/RULES.md) | 구현·보안·코딩 규칙 |
| [플랫폼 목표 매트릭스](docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md) | 독립 `ash` + Windows/mobile/PWA 목표 정본 |
| [Android 로컬 터미널 스파이크](docs/superpowers/specs/2026-06-23-android-local-terminal-spike.md) | PM-3 Android Kotlin/Compose + Rust core boundary |
| [Termux opt-in bridge design](docs/superpowers/specs/2026-06-25-termux-compatible-opt-in-bridge-design.md) | PM-3F Android external command bridge split into T0 probe and T1 helper |
| [플랫폼/모바일 workflow](docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md) | 플랫폼 Pivot 이후 Task별 상세 실행 흐름 |

## 기술 스택

Rust · ratatui · crossterm · reedline(`ash` 라인 에디터) · tokio · portable-pty · rusqlite(SQLite WAL) · clap · tracing. (대안: Go)

## 개발 (Quickstart)

```bash
# 툴체인은 rust-toolchain.toml(stable + rustfmt/clippy)로 자동 선택된다.
cargo build                 # 개발 빌드
cargo run -- doctor         # 환경 진단
cargo run -- doctor --guardrails
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

저장소(SQLite)·TLS를 켜는 빌드는 C 컴파일러가 필요하다(Linux/WSL/CI 권장). `storage`는 SQLite, `tls`는 HTTPS 클라우드 provider용 TLS transport(tokio-rustls/ring)를 켠다:

```bash
cargo build --features storage           # SQLite 저장소
cargo build --features tls               # HTTPS(https://) provider
cargo build --features "storage tls"     # 둘 다
```

## 플랫폼 지원 (v0.3.0)

| 플랫폼 | default·remote (C-free) | storage·tls (C 필요) | 비고 |
|---|---|---|---|
| Linux x86_64 | ✅ | ✅ | 1차 타깃, 셸 hook(bash/zsh) |
| Windows x86_64 | ✅ | storage ✅ / tls ⚠️¹ | wrapper 모드(`ai exec`), ConPTY |
| macOS | — | — | v0.2.0 범위 외 |

Windows native의 `ai`에는 bash/zsh hook이 없어 `ai doctor`가 **wrapper 모드**를 안내한다 — 명령은 `ai exec "<cmd>"`로 게이트를 거친다. 독립 셸 **`ash.exe`는 셸 자체가 안전 게이트를 적용**하므로 별도 wrapper 없이 외부 명령이 게이트를 통과한다(위 [`ai`와 `ash`](#ai와-ash) 참조). `storage`/`tls`는 MSVC C 툴체인이 필요하다(릴리즈 바이너리는 CI에서 빌드). WSL은 별도 Linux 런타임으로 취급한다.

> ¹ Windows 릴리즈 바이너리는 `storage remote`만 포함하고 **`tls`는 제외**한다(`ring`이 `nasm`을 요구해 기본 `windows-latest` 러너에서 빌드 불가). HTTPS(`tls`)가 필요하면 MSVC + `nasm` 환경에서 직접 빌드한다. Linux 릴리즈는 `storage tls remote` 전체 포함.

> 기본 빌드는 C-free(평문 `http://` provider만). `https://` URL은 `tls` feature 없이는 명확히 거부된다.

## 플랫폼 목표 (독립 `ash` 피벗)

정본: [플랫폼 목표 매트릭스](docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md). Task별 상세 실행 흐름은 [플랫폼/모바일 workflow](docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md)를 따른다.

| 플랫폼 | 목표 |
|---|---|
| Linux/WSL | `ash` 독립 로컬 터미널 1급 |
| Windows native | `ai-terminal.exe` 독립 GUI 터미널 + 내부 `ash.exe`/ConPTY runtime. 현재 공개 릴리즈는 `ash.exe` CLI runtime 제공, GUI installer는 repo-local smoke green |
| Git Bash/MSYS | 별도 Windows POSIX profile, MSYS bridge는 명시 opt-in |
| Android | 모바일 **로컬 터미널** 1차 타깃 |
| iOS/iPadOS | 제한적 로컬 터미널 research |
| PWA | 승인/모니터링 companion, 로컬 터미널 대체 아님 |

### Windows 개발 메모

대상 OS는 Linux이지만 순수 Rust 코어(CLI/TUI/PTY/async)는 Windows에서도 빌드된다. C 툴체인이 없는 Windows에서는 self-contained 링커를 포함한 **GNU 툴체인** 사용을 권장한다:

```powershell
rustup set default-host x86_64-pc-windows-gnu
rustup default stable
```

POSIX PTY·샌드박스 등 Linux 전용 동작은 WSL 또는 Linux CI에서 검증한다. Windows native 터미널 동작은 ConPTY 경로로 별도 검증한다.

## 설치 (v0.2.0+)

자세한 플랫폼별 안내는 [INSTALL](docs/INSTALL.md)을 따른다. 릴리즈 바이너리는 [Releases](https://github.com/ai-cli-terminal/terminal/releases)에서 받는다. 다운로드 시 `.sha256` 체크섬을 함께 검증한다. v0.2.4부터 설치 스크립트는 `ai`와 독립 셸 `ash`를 함께 설치한다.

**Linux x86_64 또는 WSL**

```bash
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | bash
# 특정 버전 고정:
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | AI_VERSION=v0.3.0 bash
ai --version   # PATH 추가 후 셸 재시작 필요할 수 있음(설치 스크립트가 안내)
ash            # 독립 구조화 셸
```

**Windows x86_64** (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
# 특정 버전 고정:
$env:AI_VERSION = 'v0.3.0'; irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
ai --version   # PATH 추가(setx) 후 셸 재시작 필요할 수 있음(설치 스크립트가 안내)
ash            # 독립 구조화 셸
```

Windows native와 WSL은 설치 대상과 실행 adapter가 다르다. Windows native의 최종 사용자 표면은 독립 GUI 터미널 `ai-terminal.exe`이며, 내부 런타임으로 `ai.exe`/`ash.exe`와 ConPTY를 사용한다. 현재 공개 릴리즈의 `ash.exe`는 CLI/runtime 산출물이고, repo-local GUI installer/portable smoke는 통과했다. WSL은 Linux용 `ai`/`ash`와 POSIX PTY/hook 경로를 사용한다. Git Bash/MSYS에서 `ash.exe`를 실행해도 기본값은 Windows native이며, MSYS POSIX bridge는 별도 profile로만 다룬다.

**소스 빌드**: `cargo build --release --features remote`(C-free) 또는 C 툴체인이 있으면 `--features "storage tls remote"`. feature 설명은 위 Quickstart 참조.

> 서명 바이너리 검증은 Phase 3(트러스트 채널)에서 도입 예정 — v0.2.x 릴리즈는 SHA256 체크섬까지 제공한다.

## 설정

설정 정본은 `~/.config/ai-terminal/config.toml`. 예시는 [`config.toml.example`](config.toml.example) 참조.

## `ai`와 `ash`

이 저장소는 두 바이너리를 제공한다.

- **`ai`** — 서브커맨드 CLI(`doctor`/`risk`/`policy`/`mask`/`preview`/`exec`/`ask`/`dispatch`/`undo`/`history`/`explain` 등)와 기존 셸(bash/zsh) hook·wrapper 통합. 일반 셸 위에 AI 보조 레이어를 얹는다.
- **`ash`** — 안전 게이트·라인 에디터·AI 라우팅을 내장한 **독립 구조화 인터랙티브 셸**. `ai exec`를 거치지 않고 셸 자체가 게이트를 적용한다.

### `ash`가 제공하는 것

| 영역 | 동작 |
|---|---|
| 안전 게이트 | 외부 명령이 risk→policy→preview→확인→undo 백업을 거쳐 실행된다. Critical은 차단, High는 확인(TTY, 비-TTY는 fail-closed), 파일 변경은 undo 백업(`ai undo last`로 복구) |
| 라인 에디터 | TTY에서 reedline 편집·커서 이동·↑↓ history 회상·Ctrl-C(라인 취소)·Ctrl-D(EOF). 비-TTY(파이프/스크립트)는 라인 단위 입력으로 폴백 |
| history | `~/.config/ai-terminal/ash_history`에 세션 간 영속. secret/PII가 탐지된 명령은 저장 제외 |
| AI 라우팅 | 자연어 입력(`ai <질문>`·`?`로 끝·의문사·한글 요청마커)은 AI로, 그 외는 구조화 셸 평가로 분기. AI 실패/타임아웃/취소는 fail-soft(셸을 막지 않음) |
| AI provider | `[ai] provider`로 `ollama`(기본, `localhost:11434`)·`openai`·`mock` 선택. OpenAI 키는 `OPENAI_API_KEY` 환경변수로만. `openai`(HTTPS)는 `tls` feature 빌드 필요 |
| 설정 | `~/.config/ai-terminal/config.toml`의 `[general]`(`history_limit`·`default_shell`)·`[ai]`. 손상/부재는 fail-soft(기본값+경고). `ai doctor`가 로드된 값을 표시 |
| Git Bash/MSYS | `AI_TERMINAL_WINDOWS_PROFILE=msys`(+`MSYSTEM` 존재) 시 외부 명령을 MSYS POSIX host(`sh -lc`)로 실행한다. 기본은 Windows native(`.exe`/`.cmd`/`.ps1`) |

## 현재 상태

**v0.3.0** — Phase 1(MVP+) 로컬 결정성 코어(M1~M4) + Phase 2 골격 + 원격 승인 기반(M0~M1 slice 4a) 위에 독립 구조화 셸 `ash`를 config·외부 실행 안전 게이트·reedline 라인 에디터·파일 영속 history·자연어 AI 라우팅(ollama/openai/mock)·Git Bash/MSYS bridge·게이트 audit 기록까지 완성했다. Linux/Windows 릴리즈는 `ai`와 `ash` 바이너리·체크섬을 함께 제공한다. 위험도·정책·마스킹·preview/undo/usage·컨텍스트·가드레일·provider 추상화 모듈과 `ai` 서브커맨드 동작. 변경 내역은 [CHANGELOG.md](CHANGELOG.md) 참조.

플랫폼 진행 기준으로 독립 셸 **`ash`는 config 로딩·외부 실행 안전 게이트·reedline 라인 에디터·파일 영속 history(민감명령 제외)·자연어 AI 라우팅(ollama/openai/mock)·Git Bash/MSYS bridge까지 결선**됐다(위 [`ai`와 `ash`](#ai와-ash) 참조). Android는 모바일 로컬 터미널 1차 타깃, iOS/iPadOS는 제한적 로컬 터미널 research, PWA는 승인·페어링·모니터링 companion으로 재배치됐다. AI usage/audit 자동 기록·env 정책 좁히기·인터랙티브 mid-exec 세부는 후속. 다음 작업은 [TASK.md](docs/TASK.md)와 [플랫폼/모바일 workflow](docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md)를 참조.

## 라이선스

MIT OR Apache-2.0
