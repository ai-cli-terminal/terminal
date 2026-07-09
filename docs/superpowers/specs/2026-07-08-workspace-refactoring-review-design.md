# 워크스페이스 구조 파악 및 리팩토링 검토 — 설계 (2026-07-08)

기준 스냅샷: `codex/release-manifest-bootstrap-evidence-gate` tip `f35b01d`
(trust 스택 DRAFT PR #68~#80 적층 상태, main `990883b`에서 분기).
검토 동기: **유지보수성·AI 세션 효율** — Claude/Codex 등 여러 AI 도구가 반복
작업하는 환경에서 컨텍스트 파악·안전한 부분 수정이 쉬운 구조를 만든다.

## 1. 목표

1. **구조 지도 문서화** — 워크스페이스 전체(루트 + terminal/ 표면들)를 한 장으로
   파악할 수 있는 아키텍처 문서를 신설해, 세션마다 반복되는 재탐색 비용을 없앤다.
2. **문서 정본 체계 정리** — HANDOFF는 "현재 상태 스냅샷"만, 세션 서사는 HISTORY,
   백로그는 TASK로 역할을 고정하고 그 계약을 문서 상단에 명시한다.
3. **거대 파일 분할** — 도구(AI 포함)가 안전하게 부분 수정할 수 있는 단위로:
   `src/main.rs` 4,240줄 · `src/daemon.rs` 3,230줄 · `src/remote_transport.rs` 2,546줄 ·
   `desktop/src/main.ts` 2,163줄 · `desktop/src-tauri/src/main.rs` 1,660줄.
4. **잔재·중복 제거** — 오진 유발원(빈 android/ 스텁, 깨진 루트 .git 등) 제거.

## 2. 비목표

- 기능·동작 변경. 모든 슬라이스는 behavior-preserving이다.
- 도메인 계층 전면 재편(src/ 물리 디렉토리 재구성, crate 분리). 부록 B에 기록만 남기고
  다음 라운드에서 별도 브레인스토밍으로 다룬다.
- trust 스택(DRAFT PR 13개)과 파일이 겹치는 코드 변경을 스택 머지 전에 실행하는 것.
- 브랜치 전략 재설계. 드리프트 현황·권고만 부록 A에 기록하고 결정은 별도 세션.

## 3. 현황 파악 결과 (2026-07-08 조사)

### 3.1 워크스페이스 표면

| 표면 | 위치 | 규모/상태 |
|---|---|---|
| Rust 코어 (`ai` CLI + `ash` 셸) | `terminal/src/` | 평면 모듈 55개 + `bin/` + `shellcore/`, 총 ~27.9k줄 |
| Tauri 데스크톱 GUI | `terminal/desktop/` | main.ts 2,163줄 + src-tauri main.rs 1,660줄 (각 단일 파일) |
| PWA 승인 컴패니언 | `terminal/pwa/` | 바닐라 JS, 소규모 |
| Android 앱 | `terminal/android/` | Kotlin 23파일 (Compose + JNI) |
| iOS 기판 | `terminal/src/mobile_ffi.rs` 등 | C ABI만, Xcode 프로젝트 없음 |
| 문서 | `terminal/docs/` | HISTORY 1,699줄 · HANDOFF 530줄 · TASK 375줄 + superpowers specs 44·plans 157 |
| 스크립트·게이트 | `terminal/scripts/` + package.json | 스크립트 71개, npm check:/smoke: 게이트 다수 |
| CI/릴리스 | `.github/workflows/` | ci.yml + release.yml |

### 3.2 발견된 문제 (우선순위 판단 근거)

1. **거대 파일 5개** — 위 목표 3의 파일들. 상위 3개가 Rust 코드의 36%.
   main.rs는 clap 서브커맨드 ~25개+중첩 커맨드와 핸들러·인라인 테스트가 한 파일.
2. **lib.rs 평면 선언 55개** — 모듈마다 `#[cfg(not(target_os = "android"))]` ×
   feature 게이트가 반복되고 도메인 구분이 없다. 파일 상단 doc comment는 초기 3개
   모듈(risk/policy/pty)만 언급하는 스테일 상태.
3. **문서 비대·역할 혼합** — HANDOFF 530줄에 세션 서사가 축적("## 1." 중복 섹션 2개,
   장문 run-on 문단). superpowers 문서 201개가 평면 나열로 활성/완료 구분 없음.
4. **루트 잔재** — `android/`(파일 0개 빈 트리), `ai-terminal/`(옛 DB 1개),
   `.git/`(info/만 남은 껍데기 — "not a git repository" 오류 유발),
   `HANDOFF.md`(2026-06-04 스테일), `test_audit_step*.sh` 2개.
5. **브랜치 전략 드리프트** — 글로벌 규칙(develop 경유 2단계 PR) vs 실제
   관행(main squash-merge + codex 스택). develop은 main과 58/8 커밋 분기.
   스테일 PR #11(6/5)·#27, 미머지 로컬 브랜치 다수.

### 3.3 trust 스택 충돌 지도 (main...HEAD, 40개 파일)

- **겹침 있음 → Wave 2**: `src/main.rs`, `src/lib.rs`, trust 계열 모듈
  (binary_manifest/policy_d/skill_registry/trust/skill/ai_router/gated_runner/
  shell_audit), `docs/HANDOFF.md`·`HISTORY.md`·`TASK.md`, `package.json`,
  `scripts/install.*`, CI/release 워크플로.
- **겹침 없음 → Wave 1 가능**: `src/daemon.rs`, `src/remote_transport.rs`,
  `desktop/`, `pwa/`.

## 4. 원칙

- 슬라이스당 브랜치·PR 1개, 수명은 짧게(장기 오픈이 충돌 리스크의 근원).
- 코드 이동은 모듈 추출·re-export 위주, 로직 수정 금지. 리뷰 기준은
  "diff가 이동·재수출뿐인가"이다.
- trust 스택과 겹치는 슬라이스는 "스택 머지 후" 태그로 명시적 스테이징.
- 슬라이스 착수 직전 열린 PR 목록을 재확인해 충돌 지도를 갱신한다.

## 5. Track 1 — 워크스페이스·문서 위생

### T1-1 루트 잔재 제거 (Wave 1)
- 삭제: 루트 `android/` 빈 트리, `ai-terminal/`(ai-terminal.db 1개),
  루트 `.git/` 껍데기, `test_audit_step1.sh`·`test_audit_step2.sh`.
- 대체: 스테일 루트 `HANDOFF.md` 삭제 → 간결한 루트 `README.md` 신설
  (워크스페이스 개요 + 정본 문서 위치 포인터 ~10줄).
- 루트는 git 밖이라 복구 불가. **실행 시 삭제 목록을 먼저 제시하고 사용자 확인 후
  진행한다** (플랜에 게이트 단계로 명시).

### T1-2 HANDOFF 슬리밍 (Wave 2 — 스택이 HANDOFF 수정 중)
- `docs/HANDOFF.md` 530줄 → ~100줄: 현재 재개점, 다음 작업 후보, 환경 메모만 유지.
- 세션 서사·완료 기록은 `docs/HISTORY.md`로 이관(요약 유지, 원문 손실 없음).
- 문서 상단에 역할 계약 명시: HANDOFF=최신 스냅샷 / HISTORY=세션 로그 / TASK=백로그.

### T1-3 구조 지도 신설 (Wave 1)
- `docs/ARCHITECTURE.md` 신설: §3.1 표면 표 확장(표면별 1~2문단), 모듈 클러스터 표
  (안전코어/AI/셸·PTY/원격승인/trust/모바일/인프라), feature gate 매트릭스
  (default/storage/tls/remote/trust × android 게이트), mermaid 다이어그램 1개.
- README 문서 표에 링크 추가(README는 스택과 겹치므로 1줄 충돌은 감수 또는 후행).

### T1-4 superpowers 인덱스 (Wave 1)
- `docs/superpowers/INDEX.md` 신설: specs 44·plans 157을 주제 클러스터·상태
  (활성/완료)로 분류한 목록. **파일 이동은 하지 않는다**(상호 링크 보존).
- 유지 규칙 1줄을 `docs/WORKFLOW.md`에 추가: 새 spec/plan 추가 시 INDEX에도 1줄.

### T1-5 브랜치·PR 위생 권고 (분석만 — 부록 A)
- 변경 없음. 부록 A의 권고안을 사용자가 결정하면 별도 작업으로 실행.

## 6. Track 2 — 코드 정리

### T2-1 desktop 분할 (Wave 1)
- `desktop/src/main.ts` 2,163줄 → 기능 모듈로 분할. 실제 경계는 플랜 단계에서
  심볼 지도를 뜬 뒤 확정하되, 후보: 터미널 렌더링 / PTY IPC / AI·승인 패널 /
  설정·테마 / 부트스트랩.
- `desktop/src-tauri/src/main.rs` 1,660줄 → Tauri command 핸들러 모듈 분리.
- 검증: tsc/빌드 + `scripts/smoke-gui.ps1` 스모크.

### T2-2 daemon.rs 분할 (Wave 1)
- `src/daemon.rs` 3,230줄(unix 전용) → `src/daemon/` 디렉토리.
- `mod.rs`가 기존 pub API를 그대로 re-export해 외부 `daemon::…` 경로 불변.
- 내부 후보 경계(플랜에서 확정): runtime / listener / gate bridge / relay /
  companion live endpoint.

### T2-3 remote_transport.rs 분할 (Wave 1)
- `src/remote_transport.rs` 2,546줄 → `src/remote_transport/` 하위 모듈화.
  T2-2와 동일 원칙(공개 경로 불변).

### T2-4 main.rs 분할 (Wave 2 — 스택 겹침)
- `src/main.rs` 4,240줄 → `src/cli/` 신설: `mod.rs`(clap Command enum + 디스패치)
  + 도메인 핸들러 파일(safety / ai / remote / release / skill / shell / info).
- `main.rs`는 진입점 ~50줄만 유지.
- 안전망: `ai --help`(및 주요 서브커맨드 `--help`) 출력 스냅샷 테스트를 추가해
  CLI 표면 불변을 기계적으로 보증.

### T2-5 lib.rs 정리 (Wave 2 — T2-4와 연속 실행)
- 55개 선언을 도메인 그룹 순으로 재배열 + 그룹 주석(안전코어/AI/셸/원격/trust/
  모바일/인프라). 물리 이동 없음.
- 스테일 doc comment를 현재 모듈 구성 기준으로 갱신.

## 7. 실행 순서·검증

**Wave 1 (즉시)**: T1-1 → T1-3 → T1-4 → T2-1 → T2-2 → T2-3
**Wave 2 (trust 스택 머지 후)**: T1-2 → T2-4 → T2-5

공통 검증 게이트(코드 슬라이스):
- Rust: WSL에서 `cargo fmt --all` → `cargo clippy --all-targets -D warnings` →
  feature 매트릭스 테스트(default / storage / tls / remote / trust) →
  lib.rs 관여 시 `cargo check --lib --target aarch64-linux-android`.
- desktop: tsc/빌드 + `smoke-gui.ps1`.
- 판정은 파이프 없이 exit 제어흐름으로(빌드 환경 메모의 파이프 마스킹 함정 준수).

## 8. 리스크·에러 처리

| 리스크 | 대응 |
|---|---|
| 루트 잔재 삭제는 비가역 | 삭제 목록 제시 → 사용자 확인 게이트 후 실행 |
| 분할 PR 오픈 중 동일 파일을 다른 세션이 수정 | 슬라이스 수명 단축(빠른 머지) + 착수 전 열린 PR 재확인 |
| daemon.rs는 Windows에서 컴파일 불가 | 검증은 WSL `--features remote` 필수 명시 |
| 분할 중 동작 변화 유입 | 이동만 허용 원칙 + 기존 테스트 green + T2-4 help 스냅샷 |
| 실패 시 복구 | 코드 슬라이스는 PR 단위 revert. 문서 슬라이스는 git 이력으로 복원 |

## 9. 부록 A — 브랜치·PR 위생 권고 (결정 대기)

1. **develop 재동기 (추천)**: develop을 main 기준으로 재동기하고 글로벌 규칙
   (develop 경유 2단계 PR)을 복원. 대안: main 단일 + PR 관행을 공식화하고
   글로벌 규칙의 이 레포 예외를 문서화. 어느 쪽이든 현재의 이중 상태는 해소 필요.
2. **스테일 PR**: #11(2026-06-05, S1b-1 셸 문법)은 리베이스 또는 close 결정.
   #27(windows verification artifacts)은 trust 스택의 release asset 작업과 중복
   가능성 — 스택 머지 후 재평가·close 후보.
3. **로컬 브랜치**: main에 머지된 `codex/ai-usage-recording`·
   `codex/product-packaging-companion-docs` 삭제 가능.
   `release/v0.3.4-android-reader`(+11커밋 미머지)·`ios-mobile-*` 3개는
   보존/PR화/폐기 결정 필요.
4. 리팩토링 PR의 대상 브랜치는 1번 결정을 따른다. 결정 전 기본값은 현 관행
   (main 대상 PR).

## 10. 부록 B — 검토했던 대안

- **B. 도메인 계층 재편**: src/ 평면 55모듈을 도메인 디렉토리로 물리 재구성 +
  crate 분리 검토. use 경로 전면 변경으로 열린 PR 13개 전부와 충돌해 이번 라운드
  제외. 이 설계(Track 1·2) 완료 후, 스택이 정리된 시점에 별도 브레인스토밍으로
  재평가한다. 그때 lib.rs 그룹(T2-5)이 디렉토리 경계의 초안이 된다.
- **C. 검토 보고서만**: 실행 설계 없이 백로그만 남기는 안 — 실행 동력이 약해 기각.
