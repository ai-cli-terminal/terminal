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

저장소(SQLite)를 켜는 빌드는 C 컴파일러가 필요하다(Linux/WSL/CI 권장):

```bash
cargo build --features storage
```

### Windows 개발 메모

대상 OS는 Linux이지만 순수 Rust 코어(CLI/TUI/PTY/async)는 Windows에서도 빌드된다. C 툴체인이 없는 Windows에서는 self-contained 링커를 포함한 **GNU 툴체인** 사용을 권장한다:

```powershell
rustup set default-host x86_64-pc-windows-gnu
rustup default stable
```

PTY·샌드박스 등 Linux 전용 동작은 WSL 또는 Linux CI에서 검증한다.

## 설정

설정 정본은 `~/.config/ai-terminal/config.toml`. 예시는 [`config.toml.example`](config.toml.example) 참조.

## 현재 상태

M0(부트스트랩) 완료 — `ai --version` / `ai doctor` 동작하는 CLI 골격. 다음은 [TASK.md](docs/TASK.md) M1.

## 라이선스

MIT OR Apache-2.0
