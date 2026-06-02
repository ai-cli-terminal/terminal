# TASK — MVP+ 구현 백로그

> **정본**: `../document/docs/06-mvp-implementation-spec.md` §31, `../document/planning/17_스케줄.md`(M1~M4), `../document/planning/05-...`(로드맵).
> 본 문서는 구현 체크리스트다. 완료 기준(DoD)은 각 §31 절의 **수용 기준**과 일치한다.
> 상태 표기: `[ ]` 대기 · `[~]` 진행 · `[x]` 완료. Phase 1(MVP+)은 약 16주(M1~M4).

---

## M0 — 부트스트랩 (repo 셋업) — 2026-06-02

- [x] `../document/` 설계 정본 검토
- [x] `docs/` working-set 5종 작성 (PRD/TASK/WORKFLOW/HISTORY/RULES)
- [x] Rust 환경 구성 (Cargo.toml, rust-toolchain.toml, rustfmt.toml, .editorconfig, .gitignore)
- [x] `ai` CLI 최소 골격 (`src/main.rs` — clap 기반 `--version`/`doctor`)
- [x] CI 스캐폴드 (`.github/workflows/ci.yml`: fmt/clippy/test/audit)
- [x] `config.toml.example` (§13 발췌)
- [x] `cargo build` / `cargo test` 검증

---

## M1 — 셸 Hook + 스토리지 (W1~W4) · §31.1, §31.2

### W1 프로젝트 셋업·아키텍처
- [~] Rust 라이브러리 크레이트 구조 착수 (`src/lib.rs` + `risk` 모듈). 나머지 도메인 모듈은 점진 추가
- [ ] Rust 워크스페이스/크레이트 구성 확정 (ratatui·tokio·portable-pty·clap·tracing)
- [ ] 5계층 + 일반 셸/AI 경로 분리 아키텍처 합의
- [ ] Git 규칙·CI 스캐폴드 확정 → `docs/WORKFLOW.md`

### W2 PTY Terminal Core
- [x] portable-pty 기반 PTY 실행 (`src/pty.rs` `run_in_pty` 단발 + `PtySession` 인터랙티브 write/read/kill). WSL에서 bash spawn·cat echo 검증
- [x] TUI 렌더링(`src/ui.rs`): ratatui 상태바(profile·cwd)/히스토리/입력(+실시간 위험도), `handle_key`, Esc·Ctrl-C, `ai tui`. `TestBackend` 검증. **Enter 제출 → PTY 실행 → 출력 히스토리 표시** 연결(`append_output`)
- [ ] 비동기 출력 스트리밍/backpressure(현재 동기 실행), 입력 지연 ≤10ms 벤치 → 후속

### W3 Shell Hook 통합 + rc UX — ✅ 대부분 구현 (2026-06-02, `src/shell.rs`)
- [x] `ai init shell` / `--dry-run` / `--diff` / `--uninstall` (rc 자동 수정 금지, 마커 기반 안전 제거)
- [x] `ai shell-hook bash|zsh` 생성 — preexec/precmd/chpwd, `command -v ai` 가드 + 에러 무시(셸 비중단). WSL에서 `bash -n`/`zsh -n` 문법 검증
- [x] 내부 `ai __hook` 진입점(현재 no-op) — hook이 무해하게 동작
- [ ] hook IPC 상태 기록(cwd/exit/git) — **W4 스토리지 + context(W13) 연동 후** (현재는 수집 호출만 wiring)
- [ ] Native Wrapper fallback 경로
- [x] **DoD(부분)**: `--dry-run`/`--diff` 미수정·`--uninstall` 블록만 제거(라운드트립 검증)·hook 실패가 셸 중단 안 함. (cd/git branch 반영은 W4 기록 후)

### W4 SQLite 스토리지 + 파일 락 — ✅ 코어 구현 (2026-06-02, `src/store.rs`, `--features storage`)
- [x] `ai-terminal.db` WAL + PRAGMA(synchronous/foreign_keys/busy_timeout) + 7개 테이블 DDL(sessions/commands/ai_requests/usage_events/audit_events/context_snapshots/locks)
- [x] 기본 CRUD: create/get_or_create session, record_command(위험도 동반), recent_commands, FK 강제. `data_dir`/`open_default`
- [x] e2e: `ai __hook preexec`가 명령을 위험도와 함께 기록 → `ai history` 표시 (양 플랫폼 검증)
- [x] advisory 파일 락(`src/lock.rs`, `create_new` 원자적) + TTL + stale 판정·정리(PID 부재/TTL 초과) + RAII 해제
- [x] **DoD (M1 핵심)**: 동시 2 연결 무손상(WAL+busy_timeout, `integrity_check`=ok) + stale lock 회수 (테스트 검증)
- [x] `locks` 테이블 레지스트리(register/lock_owner/release) + `reclaim_if_stale`(audit 기록 후 제거) + `record_audit`. 파일 락↔DB 결합 오케스트레이션은 실제 연산 연결 시

## M2 — 위험도 + 정책 + 마스킹 (W5~W8) · §31.3, §31.4, §31.8

### W5 위험도 스코어링 (0~100) — ✅ 선구현 (2026-06-02, `src/risk.rs`)
> Windows 개발 환경에서 검증 가능한 순수 결정성 로직이라 M1보다 먼저 구현함(TDD).
- [x] 명령 유형 점수표(파일 삭제 +35 / 재귀 삭제 +30 / sudo +40 / 디스크 조작 +80 / 다운로드 후 실행 +50 …)
- [x] 경로 가중치(cwd +0 / `$HOME` +30 / `/etc`·`/usr`·`/bin` +50 / docker.sock +70) + 완화 요소(dry-run −20 / 명시적 파일 −10 / 임시 디렉터리 −10)
- [x] 등급 매핑(Low/Medium/High/Critical)
- [x] **DoD**: deterministic 점수, 동일 명령·환경 동일 점수 (§31.4 golden set 테스트 통과)
- [x] `ai risk "<command>"` CLI + 요인(factor) 분해 출력
- [ ] (후속) AI 분류 보조 신호 결합 — 로컬 점수 우선 유지. 정책 엔진(W6) 연동 후

### W6 정책 엔진 + 프로파일 — ✅ 선구현 (2026-06-02, `src/policy.rs`)
- [x] `balanced`(기본)·`paranoid` 전체 필드(§31.3 권위값)
- [x] 정책 액션 매핑(Critical 차단 / High: balanced 강한 확인·paranoid 차단 / paranoid 원격 AI 차단) — `decide(level)`
- [x] `ai policy show [--profile]` 표시 / `ai risk --profile`로 결정 연동
- [ ] `ai policy set paranoid` 영속 반영 — **config 저장(W4/store) 구현 후**
- [x] **DoD**: 두 프로파일 Critical 차단, 위험 등급은 로컬 `risk::assess`에서 산출(AI 미개입 → 로컬 우선 자동 충족)

### W7 Secret/PII 마스킹 파이프라인 — ✅ 코어 구현 (2026-06-02, `src/mask.rs`)
- [x] Secret 탐지(private key block/AWS/GitHub/Slack/Bearer/Authorization/Password)
- [x] PII 탐지(이메일/IPv4/한국 주민번호) + 규칙 테이블(baseline)
- [x] 파이프라인 순서(Secret → PII → Masking → Validation Scan → Remote Eligibility), private key fail-closed 차단
- [x] `is_sensitive_path`(.env/.pem/.key 등), `ai mask "<text>"` CLI
- [ ] 전화번호/신용카드/여권 등 추가 패턴, 엔트로피 휴리스틱 보완 — 후속
- [x] **DoD(부분)**: private key 감지 시 원격 차단, 마스킹 후 원문 secret 미잔존(검증 테스트). (`.env` 컨텍스트 제외 연결은 컨텍스트 수집 구현 시)

### W8 환각 검증 게이트 + 통합 — ✅ 구현 (2026-06-02)
- [x] 바이너리 존재 검증(`src/verify.rs`, PATH/빌트인/PATHEXT), 미존재 시 `ai risk`에 UNKNOWN 표시 (플래그 검증은 P2)
- [x] AI 타임아웃(5/15/60/180s `Timeouts::defaults`) + Ctrl+C 취소 + Graceful Recovery(`src/aitask.rs` `run_cancellable`/`cancel_on_ctrl_c`, 실패·타임아웃·취소 모두 Err 반환 → 셸 비중단)
- [x] **DoD (M2 핵심)**: 위험도+정책+마스킹+환각검증+타임아웃 모듈 동작, golden set·마스킹·Critical 차단 테스트 통과. (실제 provider 연동 후 end-to-end는 Phase 2)

## M3 — preview + undo + usage (W9~W12) · §31.5, §31.6, §31.7

### W9 Preview / Diff 엔진 — ✅ 분류 구현 (2026-06-02, `src/preview.rs`)
- [x] preview 전략 분류 `classify_preview`(dry-run 우선 / in-place→temp diff / 삭제·권한→대상목록 / 외부상태→불가 / 읽기→불필요)
- [x] dry-run 제안(`rsync --dry-run`, `git clean -n`, `terraform plan`, `kubectl --dry-run=client`, `helm --dry-run`)
- [x] `ai preview "<cmd>"` CLI (대상 목록·개수·불가 사유 표시)
- [ ] 실제 temp-copy 실행→diff(sed류) 생성 — WSL 연동 후속(현재 전략 표시까지)
- [x] **DoD(부분)**: `rm -rf` 대상 목록·개수 표시, 외부상태 불가 사유 표시. diff 생성은 후속

### W10 Undo / Transaction — ✅ 구현 (2026-06-02, `src/undo.rs`)
- [x] best-effort 파일 롤백: `create_backup`(파일 복사 + metadata.toml) / `restore` / `latest`
- [x] 백업 상한(500MB / 1000 files / 파일 20MB / TTL 7일) enforcement → 초과 시 `Refused(사유)`
- [x] `ai undo last` CLI (백업 없으면 안내)
- [ ] 명령 실행 파이프라인에 백업 자동 트리거 연결(현재 모듈/CLI까지) — 실행 연동 후속
- [x] **DoD(부분)**: 한도 초과 시 Refused로 사전 차단(호출측 중단). 자동 트리거는 후속

### W11 Usage / Cost — ✅ 구현 (2026-06-02, `src/usage.rs` + store)
- [x] usage_event 기록(`store.record_usage`) + 누적 집계(`total_cost`), TokenSource/CostSource enum
- [x] 예산 평가 `evaluate`(session $2 / month $30, warn 80% / block 100%) → Ok/Warn/Block
- [x] `ai usage` CLI (누적 비용·예산·상태 표시)
- [ ] AI 요청 파이프라인에서 자동 usage 기록(실제 provider 연동 시), estimated 배지 표기
- [x] **DoD(부분)**: usage 기록/집계·예산 평가 동작. 자동 기록·원격 차단 연동은 provider 연동 후

### W12 에러 분석 + 히스토리 + 감사 — ✅ 구현 (2026-06-02, `src/explain.rs`)
- [x] 규칙 기반 에러 분석 `explain`(command not found/permission/no such file/generic) + `ai explain "<cmd>" --exit --stderr`
- [x] 세션 히스토리(`ai history`, W4), audit_events 기록(`record_audit`, W4/lock)
- [ ] `last-error` 자동 캡처(직전 명령 stderr 저장) — hook stderr 캡처 연동 후속
- [x] **DoD (M3 핵심)**: preview/undo/usage/에러분석 모듈 동작 (CLI 제공)

## M4 — 컨텍스트 + 가드레일 + 호환성 (W13~W16) · §31.9, §31.10, §31.11

### W13 Context Consistency Manager — ✅ 구현 (2026-06-02, `src/context.rs`)
- [x] `SessionContext`(cwd/shell/user/hostname/git_branch) + `gather()`, `ai context` CLI
- [x] 상태 갱신 트리거 감지 `is_context_changing`(cd/pushd/export/alias/source/git checkout·switch·pull·reset)
- [x] env allowlist + denylist(TOKEN/SECRET/KEY/PASSWORD) + PATH hash-only(`filter_env_var`) → secret 미저장
- [x] `needs_refresh`(cwd/branch 불일치) + `git_branch`(.git/HEAD 파싱)
- [x] **DoD**: git_branch 갱신·env secret 미저장·mismatch refresh 판정(테스트). hook 자동 적용은 후속

### W14 Execution Guardrails Engine (baseline) — ✅ 구현 (2026-06-02, `src/guardrails.rs`)
- [x] Baseline 목록 `baseline()`(static analysis/risk scoring/preview/dry-run/timeout/confirmation/masking/policy enforcement)
- [x] 플랫폼 capability matrix `capabilities(Platform)` + `detect()`(Linux/WSL/macOS/Other)
- [x] `ai doctor --guardrails`가 platform·baseline·matrix 출력, 제한 플랫폼 High+ 강화 고지
- [ ] 실제 동적 감시(seccomp/cgroups 등) 구현 — Phase 2+ (MVP는 명시적 capability 고지)
- [x] **DoD**: 미지원 guardrail 명시(조용한 실패 금지), 제한 플랫폼 High+ 확인 강화 고지

### W15 Provider 추상화 + Token Window — ✅ 구현 (2026-06-02, `src/provider.rs`, `src/tokenwin.rs`)
- [x] `Provider`/`ModelCapability` capability map + `Provider::mock()`
- [x] fallback: `token_source`(→Estimated)/`cost_source`(→PricingTable)/`use_streaming`. tool-use MVP 제외
- [x] Token Window: `estimate_tokens`(char/4)/`chunk`(window·overlap)/`fits`
- [ ] 실제 provider 어댑터(HTTP) — Phase 2 Model Gateway
- [x] **DoD**: capability 기반 명시적 fallback, 불확실 토큰/비용 estimated (테스트)

### W16 호환성 테스트 + MVP 진입 결정 — ✅ 핵심 완료 (2026-06-02)
- [x] 셸 호환성(bash/zsh `-n` 문법, WSL), 플랫폼 감지(Linux/WSL/macOS/Other) — 양 플랫폼 테스트 통과
- [x] 속성/통합 테스트(`tests/integration.rs`): 위험도 결정성(50회)·Critical 차단 100%·마스킹 무유출
- [x] KPI(로컬): 결정성·Critical 100%·마스킹 0 검증. (지연/응답 KPI·커버리지 측정은 실행/provider 연동 후)
- [x] §31.12 9개 영역 체크리스트 + §31.13 확정값 → `docs/MVP-ENTRY.md`
- [x] **DoD (M4 핵심)**: MVP+ 로컬 결정성 골격 완료. provider 의존 end-to-end는 Phase 2

---

## Phase 2~4 (요약 — MVP 회고 후 구체화)

- **P2 Intelligent Workflow**: Hybrid Mode, Intent Classifier, Tool Use Planner, Semantic Index, 로컬 LLM(Ollama), 스킬·MCP 로컬 기본, 데몬 아키텍처.
- **P3 Team & Enterprise**: 조직 정책(signed policy.d), 중앙 감사, gVisor, 스킬 서명, MCP mutate/external, 리모트 모니터링→승인.
- **P4 Advanced Automation**: Cross-Session Knowledge, Multi-agent, Firecracker, 웹 대시보드, 관리형 릴레이.
