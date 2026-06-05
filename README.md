# AI Terminal (`ai`)

일반 리눅스 터미널과 **완전 호환**되는 실행 환경을 유지하면서 AI 명령 생성·설명·디버깅·로그 분석·자동화를 **안전하게** 결합하는 단일 바이너리 터미널. 설계 철학은 **"AI는 명령 실행자가 아니라 의사결정 보조자."**

> 이 저장소는 **구현 repo**다. 설계 정본(source of truth)은 옆 디렉터리 [`../document/`](../document/)(v3.3, "MVP spec finalized — ready to build")에 있다.

## 문서 (`docs/`)

| 문서 | 내용 |
|---|---|
| [PRD](docs/PRD.md) | 제품 목표·범위·MVP 포함/제외·KPI |
| [TASK](docs/TASK.md) | MVP+ 구현 백로그(M0~M4 체크리스트) |
| [WORKFLOW](docs/WORKFLOW.md) | Git·커밋·PR·CI·빌드 명령 |
| [HISTORY](docs/HISTORY.md) | 변경/결정 로그 |
| [RULES](docs/RULES.md) | 구현·보안·코딩 규칙 |

## 기술 스택

Rust · ratatui · crossterm · tokio · portable-pty · rusqlite(SQLite WAL) · clap · tracing. (대안: Go)

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

## 플랫폼 지원 (v0.2.0)

| 플랫폼 | default·remote (C-free) | storage·tls (C 필요) | 비고 |
|---|---|---|---|
| Linux x86_64 | ✅ | ✅ | 1차 타깃, 셸 hook(bash/zsh) |
| Windows x86_64 | ✅ | ✅ (CI/MSVC) | wrapper 모드(`ai exec`), ConPTY |
| macOS | — | — | v0.2.0 범위 외 |

Windows에는 bash/zsh hook이 없어 `ai doctor`가 **wrapper 모드**를 안내한다 — 명령은 `ai exec "<cmd>"`로 게이트를 거친다. `storage`/`tls`는 MSVC C 툴체인이 필요하다(릴리즈 바이너리는 CI에서 빌드).

> 기본 빌드는 C-free(평문 `http://` provider만). `https://` URL은 `tls` feature 없이는 명확히 거부된다.

### Windows 개발 메모

대상 OS는 Linux이지만 순수 Rust 코어(CLI/TUI/PTY/async)는 Windows에서도 빌드된다. C 툴체인이 없는 Windows에서는 self-contained 링커를 포함한 **GNU 툴체인** 사용을 권장한다:

```powershell
rustup set default-host x86_64-pc-windows-gnu
rustup default stable
```

PTY·샌드박스 등 Linux 전용 동작은 WSL 또는 Linux CI에서 검증한다.

## 설치 (v0.2.0+)

릴리즈 바이너리는 [Releases](https://github.com/ai-cli-terminal/terminal/releases)에서 받는다. 다운로드 시 `.sha256` 체크섬을 함께 검증한다.

**Linux x86_64**

```bash
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | bash
# 특정 버전 고정:
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | AI_VERSION=v0.2.0 bash
ai --version   # PATH 추가 후 셸 재시작 필요할 수 있음(설치 스크립트가 안내)
```

**Windows x86_64** (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
# 특정 버전 고정:
$env:AI_VERSION = 'v0.2.0'; irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
ai --version   # PATH 추가(setx) 후 셸 재시작 필요할 수 있음(설치 스크립트가 안내)
```

**소스 빌드**: `cargo build --release --features remote`(C-free) 또는 C 툴체인이 있으면 `--features "storage tls remote"`. feature 설명은 위 Quickstart 참조.

> 서명 바이너리 검증은 Phase 3(트러스트 채널)에서 도입 예정 — v0.2.0은 SHA256 체크섬까지 제공한다.

## 설정

설정 정본은 `~/.config/ai-terminal/config.toml`. 예시는 [`config.toml.example`](config.toml.example) 참조.

## 현재 상태

**v0.2.0** — Phase 1(MVP+) 로컬 결정성 코어(M1~M4) + Phase 2 골격 + 원격 승인 기반(M0~M1 slice 4a) + 배포(Linux/Windows 바이너리·체크섬). 위험도·정책·마스킹·preview/undo/usage·컨텍스트·가드레일·provider 추상화 모듈과 `ai` 서브커맨드 동작. 변경 내역은 [CHANGELOG.md](CHANGELOG.md) 참조.

실제 클라우드 provider HTTP(S)·async 결합·실행 파이프라인 자동 연동(undo 자동 백업·usage 자동 기록·last-error 캡처)은 후속(M1~M3 잔여 / Phase 2). 다음 작업은 [TASK.md](docs/TASK.md) 참조.

## 라이선스

MIT OR Apache-2.0
