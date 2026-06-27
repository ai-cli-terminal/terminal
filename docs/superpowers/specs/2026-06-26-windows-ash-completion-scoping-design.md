# Windows native `ash.exe` 기능 완성 — 스코핑 / 분해 설계

> **작성일**: 2026-06-26
> **유형**: 분해(decomposition) 설계 — 7개 후속 슬라이스의 로드맵. 각 슬라이스는 자체 spec→plan→구현 사이클을 갖는다.
> **정본 상위 문서**: `2026-06-23-platform-target-matrix-design.md`, `2026-06-23-platform-execution-contract.md`, `../../TASK.md` PM-1.
> **범위**: TASK.md PM-1의 미완 항목(현재 최우선). Linux/WSL·ConPTY·MSYS profile **계약**은 이미 존재하며, 본 문서는 그 위에 Windows 네이티브 `ash.exe`를 **사용 가능하고 안전한 셸**로 완성하는 실행 순서를 고정한다.

## 1. 배경과 현재 상태

제품은 모든 지원 플랫폼에서 도는 **독립 로컬 터미널**이며, `ash`(`src/bin/ash.rs` → `shellcore::repl::run`)가 그 런타임이다. `shellcore`는 순수 언어(lexer/parser/ast/engine/value/`where`…)를 담당하고, 호스트 실행은 `shellcore::external::ExternalRunner` trait 뒤에 둔다(`DesktopRunner`=process spawn, `DisabledRunner`=pure/mobile).

코드 근거로 확인한 **미구현 갭**:

| # | 항목 | 현재 상태(파일:근거) |
|---|---|---|
| 1 | line editor | `repl.rs`가 `stdin.read_line`만 사용. 라인 편집·history 탐색·Ctrl-C/Ctrl-D 처리 없음(주석에 "S2"로 이연 명시) |
| 2 | history 저장/로드 | 없음 |
| 3 | config 로딩 | ash가 소비하는 경로 없음(`src/config.rs`는 존재하나 `cfg(not(target_os="android"))` 데스크톱 모듈, ash REPL과 미결선) |
| 4 | 안전 게이트 결선 | `external.rs DesktopRunner::run`이 `std::process::Command`로 **직접 spawn** — risk/policy/preview/undo/usage/audit를 전부 우회. `src/pipeline.rs`의 게이트는 `ai exec` 전용이고 ash 경로와 분리됨 |
| 5 | AI 통합 결선 | `shellcore`는 순수 언어만 평가. 자연어 입력을 `dispatch`/`gateway`로 보내는 경로 없음 |
| 6 | Git Bash/MSYS bridge runner | `shellcore::msys::select_profile` 순수 함수만 존재. 실제 bridge runner(POSIX path 변환·tool discovery 실행) 미구현 |

**이미 된 것(재사용 자산)**: Windows 실행 해석(`shellcore::winexec`: `.exe`/PATHEXT `.cmd|.bat`/`.ps1` resolution + `spawn_plan`, exit code) 구현·테스트됨. ConPTY interactive smoke·MSYS profile **계약**·`config.rs`/`risk.rs`/`policy.rs`/`pipeline.rs`/`preview.rs`/`undo.rs`/`usage.rs`/`dispatch.rs`/`gateway.rs` 본체는 모두 존재 → 대부분은 **신규 구현이 아니라 shellcore 경로로의 결선**이다.

## 2. 설계 원칙

1. **shellcore 순수성 유지**: line editor·history·config·safety·AI는 모두 `ExternalRunner`/REPL **호스트 어댑터 계층**에 둔다. 순수 evaluator(모바일/PWA 임베드 대상)는 오염시키지 않는다. pure core는 `DisabledRunner`로 외부 실행이 막힘을 계속 증명한다.
2. **결선 우선, 재구현 금지**: risk/policy/preview/undo/usage/audit·dispatch/gateway는 이미 있다. ash 실행 경로가 이들을 **호출**하게 만드는 것이 핵심이며, 로직 중복 구현은 금지(YAGNI).
3. **fail-soft**: config 손상·AI 백엔드 실패·history 파일 손상이 **셸 세션을 깨지 않는다**. 진단만 남기고 동작은 지속한다(기존 hook/게이트 fail-closed/지속 원칙과 정합).
4. **슬라이스 독립 검증**: 각 슬라이스는 `cargo fmt/clippy/test` + 해당 동작의 e2e로 단독 검증 후 다음으로 간다.

## 3. 슬라이스 순서와 근거

의존(하드 제약)·제품가치·기술위험 3축으로 정렬한다.

- **의존**: #2→#1(history 탐색은 편집기 필요), #5→#4(AI 제안 명령도 게이트 통과 필수), #1·#2·#4·#5는 #3(config)를 읽음.
- **가치**: #4 안전 게이트 = "안전한 AI 보조" 정체성의 핵심. #1 = daily-driver 기본 사용성.
- **위험**: #1 line editor가 최고(Windows 콘솔+ConPTY raw mode·Ctrl-C/D·EOF) → 너무 늦지 않게 배치해 de-risk. #4는 기존 컴포넌트 통합이라 위험 낮음.

**확정 순서**: **#3 → #4 → #1 → #2 → #5 → #6 → #7**

#4를 #1보다 앞에 둔 이유: #1의 기술위험은 3순위로 충분히 일찍 해소되고, 안전 게이트를 먼저 박아야 "예쁘지만 안전하지 않은 셸"을 먼저 내보내는 일을 피한다(제품 약속 우선).

## 4. 슬라이스 명세 개요

각 항목은 후속 spec에서 상세화한다. 여기서는 목표·경계·수용기준 골격만 고정한다.

### S1 — Config 로딩 (#3)
- **목표**: ash가 사용자 config(경로·profile/env override)를 로드해 이후 슬라이스가 읽을 단일 소스를 만든다. `src/config.rs` 재사용, ash REPL/runner에 주입.
- **경계**: 기본값 출력, 잘못된 config는 fail-soft 진단(세션 비중단). 비밀값은 `mask` 정책을 따른다.
- **수용기준**: 기본 경로 로드, override 반영, 손상 config→기본값+경고, 단위테스트.

### S2 — 안전 게이트 결선 (#4)
- **목표**: shellcore external 실행 **앞단**에 risk→policy(Block/preview/confirm)→undo 백업→실행→usage/audit를 연결. `DesktopRunner`를 게이트 통과 runner로 교체하되 `pipeline.rs` 로직 재사용.
- **경계**: pure/mobile은 영향 없음(`DisabledRunner` 유지). 게이트는 config(S1)의 활성 profile을 읽는다. env 정책 좁히기(계약 §5)를 포함.
- **수용기준**: 위험 명령이 정책대로 Block/확인/preview/undo를 거침, audit 기록, e2e(rm→백업→undo, 위험명령 차단 exit code).

### S3 — Line editor (#1)
- **목표**: Windows 콘솔/ConPTY에서 입력 편집·history 탐색·Ctrl-C/Ctrl-D·EOF/interrupt 동작을 고정. 라인에디터 크레이트 도입 여부 포함(후속 spec에서 결정).
- **경계**: shellcore 순수성 유지(REPL 어댑터 계층). 비Windows 동작 회귀 없음.
- **수용기준**: 편집·history 키, Ctrl-C(라인 취소)·Ctrl-D(EOF 종료) 동작, Windows 콘솔+ConPTY 양쪽 smoke.

### S4 — History 저장/로드 (#2)
- **목표**: 기본 경로 저장/로드, 손상 파일 복구, 동시 실행 best-effort append, 민감 명령 저장 제외 정책. config(S1) 읽음, editor(S3) 위에 얹힘.
- **수용기준**: 저장·로드 라운드트립, 손상 복구, 민감 명령 제외, 동시성 무손상.

### S5 — AI 통합 결선 (#5)
- **목표**: ash 자연어 입력을 `dispatch`로 분류 → `gateway`로 질의(timeout/cancel). AI 제안 명령은 S2 게이트를 통과. 로컬/백엔드 실패가 세션을 깨지 않음.
- **수용기준**: NL→AI 경로, 셸 명령은 그대로 실행, gateway timeout/cancel, 실패 fail-soft, e2e.

### S6 — Git Bash/MSYS bridge runner (#6)
- **목표**: `AI_TERMINAL_WINDOWS_PROFILE=msys`에서 path 변환·POSIX tool discovery·native `ash.exe` 호출 경계를 **실제 실행**으로 고정(`select_profile` 위에 runner 구현).
- **수용기준**: MSYS opt-in, path 변환, exit code, `.sh`/POSIX tool 성공·실패 케이스 smoke. native script host와 혼합 금지.

### S7 — Windows 문서/패키징 (#7)
- **목표**: README의 Windows native `ash.exe`·WSL `ash`·Git Bash/MSYS 사용 경계 분리, `ai`/`ash` 역할 명확화.

## 5. 비목표 (이 로드맵 밖)

- 샌드박스/고격리(bubblewrap/gVisor 등)는 Phase 2+.
- iOS/Android 신규 기능(PM-3 보류 유지).
- remote-approval companion(PM-6)·relay(M2).

## 6. 다음 단계

본 분해 doc 승인 후 **S1(Config)** 부터 개별 spec→plan→구현 사이클을 시작한다. 각 슬라이스 완료 시 `cargo fmt --all -- --check` · `cargo clippy --all-targets --features "storage tls remote" -D warnings` · `cargo test --features "storage tls remote"` + 해당 e2e가 green이어야 한다. 전 슬라이스 완료 시 PM-1 "Windows 완료 기준" 충족 → Android PM-3 재개 가능.
