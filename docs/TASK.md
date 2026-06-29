# TASK — MVP+ 구현 백로그

> **정본**: `../document/docs/06-mvp-implementation-spec.md` §31, `../document/planning/17_스케줄.md`(M1~M4), `../document/planning/05-...`(로드맵).
> 본 문서는 구현 체크리스트다. 완료 기준(완료 기준)은 각 §31 절의 **수용 기준**과 일치한다.
> 상태 표기: `[ ]` 대기 · `[~]` 진행 · `[x]` 완료. Phase 1(MVP+)은 약 16주(M1~M4).
>
> **진행 스냅샷(2026-06-04~2026-06-26)**: v0.1.0 이후 **Phase 1 실사용 갭 WI-1~5 + Phase 2 후속 FU-1~3 완료**. 이후 프로젝트는 "bash 위 AI 보조 레이어"에서 **독립 구조화 셸 `ash`**로 피벗했고, 플랫폼 목표도 **모바일 로컬 터미널**을 포함하도록 재정렬했다. 정본: `docs/superpowers/specs/2026-06-05-independent-shell-s0-core-design.md`, `docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md`.
>
> **다음 세션 인계**: (1) ✅ **FU-3 WSL e2e 재확인 완료(2026-06-04)** — 행(hang) 버그 발견·수정(readline이 probe 마커 `\x1f`=undo 가로챔 → bash `--noediting`으로 spawn). 상세 `HISTORY.md`. (2) ✅ **FU-4 / M0 인터셉트 제어점 완료(2026-06-04)** — WSL spike로 bash extdebug·zsh ZLE 차단 실증 후 in-repo 착지(`gate.rs`·`ai __gate`·`ai remote arm/disarm/status`·shell.rs hook 인터셉트). 대화형 e2e green. 설계/계획: `docs/superpowers/{specs,plans}/2026-06-04-remote-approval-m0-intercept*`. (3) ✅ **FU-4 / M0.5 와이어 프로토콜 + 크립토 코어 완료(2026-06-04)** — snow Noise_XX(순수 Rust, C-free)·ed25519-dalek 확정, `remote` feature, `remote.rs`(핸드셰이크 상호인증·transport 암복호·서명 검증 in-repo green). 스펙: `docs/superpowers/specs/2026-06-04-remote-approval-m05-wire-protocol-design.md`. (4) ✅ **FU-4 / M1 slice 1 로컬 게이트 데몬 완료(2026-06-04)** — `daemon.rs`(tokio Unix 소켓 `serve`/동기 `query`/`decide_request`), `ai remote daemon`, `ai __gate`가 데몬 질의+로컬 폴백. e2e: armed Critical=BLOCK via DAEMON, 데몬 종료 시 LOCAL 폴백. 설계: `docs/superpowers/specs/2026-06-04-remote-approval-m1-daemon-design.md`. (5) ✅ **FU-4 / M1 slice 2 승인 검증 상태머신 완료(2026-06-04)** — `approval.rs`(`validate` 보안-핵심 순수 검증 + `NonceStore` 1회용 + `gen_nonce`). ship 게이트 음성 케이스 9 단위 green(replay·expired·revoke·bad sig·TOCTOU·id/nonce mismatch). 설계: `docs/superpowers/specs/2026-06-04-remote-approval-m1-approval-validation-design.md`. (6) ✅ **FU-4 / M1 slice 3 Noise 세션 승인 왕복 완료(2026-06-04)** — `session.rs`(와이어 메시지 + encode/decode + 변환 + device_respond). e2e: 실제 Noise 암호문 위 승인 한 바퀴(approve→Approved+replay 차단, reject→Rejected). 설계: `docs/superpowers/specs/2026-06-04-remote-approval-m1-noise-session-design.md`. (7) ✅ **FU-4 / M1 slice 4a 전송 substrate 완료(2026-06-04)** — `session.rs`에 `send_frame`/`recv_frame`(제네릭, framing, DoS 가드) + `run_device`/`run_daemon_request`(역할 함수). **실제 `UnixStream::pair` 위 handshake+승인 왕복** e2e green. 설계: `docs/superpowers/specs/2026-06-04-remote-approval-m1-transport-design.md`. **다음(M1 후속)**: 실제 데몬 프로세스에서 디바이스 연결 리스너(device.sock/TCP) → 페어링 CLI/QR(daemon_pubkey 앵커)+디바이스 등록 영속화 → 게이트 플로우 결선(armed High opt-in 명령 → 데몬이 등록 디바이스로 `run_daemon_request` 트리거 → 결과로 통과/차단, fail-closed timeout) → 데몬 컨텍스트 스냅샷(§31.10)+context_hash 산출 → PWA(/approve,/pair) → relay(M2) → TTL/heartbeat/viz(#1·#2·#4). 잔여: bubblewrap/gVisor 격리, 영속 셸 입력 인터셉트, monthly 예산 시간창.
>
> **현재 우선순위(2026-06-28, Windows GUI 자동 완료 기준 충족)**: Windows 목표는 `ash.exe`를 Windows Terminal/PowerShell/Git Bash에서 수동 검증하는 모델이 아니라, 사용자가 더블클릭해 여는 **완전한 독립 GUI 터미널 프로그램 `ai-terminal.exe`** 로 전환했다. 기존 Windows native `ash.exe` S1~S7 구현은 GUI 앱 내부 PTY/ConPTY child runtime으로 재사용한다. 정본: `docs/superpowers/specs/2026-06-27-windows-gui-terminal-pivot-design.md`. GUI scaffold/PTY bridge/packaging/portable smoke/NSIS install-run-uninstall smoke와 GUI 내부 AI routing/safety gate/storage-audit evidence가 green이다. 남은 Windows 후속은 MSI 검토와 수동 더블클릭 smoke evidence이며, Android PM-3는 재개 가능 상태다.

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

### W2 PTY 터미널 코어
- [x] portable-pty 기반 PTY 실행 (`src/pty.rs` `run_in_pty` 단발 + `PtySession` 인터랙티브 write/read/kill). WSL에서 bash spawn·cat echo 검증
- [x] TUI 렌더링(`src/ui.rs`): ratatui 상태바(profile·cwd)/히스토리/입력(+실시간 위험도), `handle_key`, Esc·Ctrl-C, `ai tui`. `TestBackend` 검증. **Enter 제출 → PTY 실행 → 출력 히스토리 표시** 연결(`append_output`)
- [x] 중앙 실행 파이프라인 연결: `ai exec` + TUI가 위험도·정책·preview·백업 게이트를 거쳐 실행(`src/pipeline.rs`). **출력 스트리밍 완료(2026-06-03)**: `run_in_pty_streaming`(리더 스레드→bounded mpsc→ctrl_c select)로 청크 라이브 스트리밍 + CLI Ctrl+C 중단(exit 130, 취소 시 버퍼 드레인).
- [x] **TUI mid-exec 중단 + 라이브 스트리밍 (2026-06-04, WI-5)**: `pty::run_in_pty_streaming_cancellable`(명시적 `Arc<AtomicBool>` 취소 + `clone_killer` 워처 스레드 → silent 명령도 중단). TUI는 `dispatch::dispatch`로 분류 후 **셸만 워커 스레드**에서 실행(`std::thread::scope`+`ChannelSink`), 메인 루프가 청크 라이브 표시 + `event::poll`로 Esc/Ctrl+C 중단(exit 130). AI는 메인 동기(타임아웃 상한). `render_shell_tail`(이중 출력 방지). WSL 검증(취소→130 즉시). 설계: `docs/superpowers/specs/2026-06-04-tui-mid-exec-cancel-design.md`

### W3 Shell Hook 통합 + rc UX — ✅ 대부분 구현 (2026-06-02, `src/shell.rs`)
- [x] `ai init shell` / `--dry-run` / `--diff` / `--uninstall` (rc 자동 수정 금지, 마커 기반 안전 제거)
- [x] `ai shell-hook bash|zsh` 생성 — preexec/precmd/chpwd, `command -v ai` 가드 + 에러 무시(셸 비중단). WSL에서 `bash -n`/`zsh -n` 문법 검증
- [x] 내부 `ai __hook` 진입점(현재 no-op) — hook이 무해하게 동작
- [x] hook IPC 상태 기록(cwd/exit/git) (2026-06-03): exit_code(`precmd`→`update_last_exit`), cwd+git_branch(`chpwd`→`record_context_snapshot`/`update_session_cwd`).
- [x] **bash cwd 연동 (2026-06-04, WI-3)**: bash는 native chpwd 없음 → `BASH_HOOK` precmd가 셸 변수 `__ai_last_pwd`로 PWD 변화를 감지해 `ai __hook chpwd` 에뮬레이트(핸들러 재사용, exit 코드 보존). WSL e2e 검증: `cd` 2회→세션 cwd가 마지막 디렉터리로 갱신·context_snapshots 기록. 설계: `docs/superpowers/specs/2026-06-04-bash-cwd-hook-design.md`
- [x] **Native Wrapper fallback 경로 (2026-06-04, WI-4)**: `shell::{ConfiguredMode,IntegrationMode,resolve_integration_mode,hook_active}` — hook 마커(`AI_TERMINAL_HOOK=1`, 양 hook이 export) 부재 시 wrapper로 fallback 해석. `ai doctor`가 유효 모드 표시 + wrapper 시 `ai exec` 안내. wrapper 데이터 수집은 기존 `record_exec`(Ran 시 명령+cwd+exit 기록)로 이미 충족 → 중복 미추가. 영속 PTY 셸 런처는 Phase 2 이연. 설계: `docs/superpowers/specs/2026-06-04-wrapper-fallback-design.md`
- [x] **완료 기준(부분)**: `--dry-run`/`--diff` 미수정·`--uninstall` 블록만 제거(라운드트립 검증)·hook 실패가 셸 중단 안 함. (cd/git branch 반영은 W4 기록 후)

### W4 SQLite 스토리지 + 파일 락 — ✅ 코어 구현 (2026-06-02, `src/store.rs`, `--features storage`)
- [x] `ai-terminal.db` WAL + PRAGMA(synchronous/foreign_keys/busy_timeout) + 7개 테이블 DDL(sessions/commands/ai_requests/usage_events/audit_events/context_snapshots/locks)
- [x] 기본 CRUD: create/get_or_create session, record_command(위험도 동반), recent_commands, FK 강제. `data_dir`/`open_default`
- [x] e2e: `ai __hook preexec`가 명령을 위험도와 함께 기록 → `ai history` 표시 (양 플랫폼 검증)
- [x] advisory 파일 락(`src/lock.rs`, `create_new` 원자적) + TTL + stale 판정·정리(PID 부재/TTL 초과) + RAII 해제
- [x] **완료 기준 (M1 핵심)**: 동시 2 연결 무손상(WAL+busy_timeout, `integrity_check`=ok) + stale lock 회수 (테스트 검증)
- [x] `locks` 테이블 레지스트리(register/lock_owner/release) + `reclaim_if_stale`(audit 기록 후 제거) + `record_audit`. 파일 락↔DB 결합 오케스트레이션은 실제 연산 연결 시

## M2 — 위험도 + 정책 + 마스킹 (W5~W8) · §31.3, §31.4, §31.8

### W5 위험도 스코어링 (0~100) — ✅ 선구현 (2026-06-02, `src/risk.rs`)
> Windows 개발 환경에서 검증 가능한 순수 결정성 로직이라 M1보다 먼저 구현함(TDD).
- [x] 명령 유형 점수표(파일 삭제 +35 / 재귀 삭제 +30 / sudo +40 / 디스크 조작 +80 / 다운로드 후 실행 +50 …)
- [x] 경로 가중치(cwd +0 / `$HOME` +30 / `/etc`·`/usr`·`/bin` +50 / docker.sock +70) + 완화 요소(dry-run −20 / 명시적 파일 −10 / 임시 디렉터리 −10)
- [x] 등급 매핑(Low/Medium/High/Critical)
- [x] **완료 기준**: deterministic 점수, 동일 명령·환경 동일 점수 (§31.4 golden set 테스트 통과)
- [x] `ai risk "<command>"` CLI + 요인(factor) 분해 출력
- [ ] (후속) AI 분류 보조 신호 결합 — 로컬 점수 우선 유지. 정책 엔진(W6) 연동 후

### W6 정책 엔진 + 프로파일 — ✅ 선구현 (2026-06-02, `src/policy.rs`)
- [x] `balanced`(기본)·`paranoid` 전체 필드(§31.3 권위값)
- [x] 정책 액션 매핑(Critical 차단 / High: balanced 강한 확인·paranoid 차단 / paranoid 원격 AI 차단) — `decide(level)`
- [x] `ai policy show [--profile]` 표시 / `ai risk --profile`로 결정 연동
- [x] `ai policy set paranoid` 영속 반영 (`config.rs`, `active_profile` 저장)
- [x] **완료 기준**: 두 프로파일 Critical 차단, 위험 등급은 로컬 `risk::assess`에서 산출(AI 미개입 → 로컬 우선 자동 충족)

### W7 Secret/PII 마스킹 파이프라인 — ✅ 코어 구현 (2026-06-02, `src/mask.rs`)
- [x] Secret 탐지(private key block/AWS/GitHub/Slack/Bearer/Authorization/Password)
- [x] PII 탐지(이메일/IPv4/한국 주민번호) + 규칙 테이블(baseline)
- [x] 파이프라인 순서(Secret → PII → Masking → Validation Scan → Remote Eligibility), private key fail-closed 차단
- [x] `is_sensitive_path`(.env/.pem/.key 등), `ai mask "<text>"` CLI
- [x] 전화번호/신용카드/여권 추가 패턴 (`mask.rs`, IP 오탐 방지 포함)
- [x] 엔트로피 휴리스틱 보완 (2026-06-03, `is_high_entropy_secret`: 길이≥20·엔트로피≥4.0·영숫자 혼합, 경로/URL 오탐 회피)
- [x] **완료 기준(부분)**: private key 감지 시 원격 차단, 마스킹 후 원문 secret 미잔존(검증 테스트). (`.env` 컨텍스트 제외 연결은 컨텍스트 수집 구현 시)

### W8 환각 검증 게이트 + 통합 — ✅ 구현 (2026-06-02)
- [x] 바이너리 존재 검증(`src/verify.rs`, PATH/빌트인/PATHEXT), 미존재 시 `ai risk`에 UNKNOWN 표시 (플래그 검증은 P2)
- [x] AI 타임아웃(5/15/60/180s `Timeouts::defaults`) + Ctrl+C 취소 + Graceful Recovery(`src/aitask.rs` `run_cancellable`/`cancel_on_ctrl_c`, 실패·타임아웃·취소 모두 Err 반환 → 셸 비중단)
- [x] **완료 기준 (M2 핵심)**: 위험도+정책+마스킹+환각검증+타임아웃 모듈 동작, golden set·마스킹·Critical 차단 테스트 통과. (실제 provider 연동 후 end-to-end는 Phase 2)

## M3 — preview + undo + usage (W9~W12) · §31.5, §31.6, §31.7

### W9 미리보기 / Diff 엔진 — ✅ 분류 구현 (2026-06-02, `src/preview.rs`)
- [x] preview 전략 분류 `classify_preview`(dry-run 우선 / in-place→temp diff / 삭제·권한→대상목록 / 외부상태→불가 / 읽기→불필요)
- [x] dry-run 제안(`rsync --dry-run`, `git clean -n`, `terraform plan`, `kubectl --dry-run=client`, `helm --dry-run`)
- [x] `ai preview "<cmd>"` CLI (대상 목록·개수·불가 사유 표시)
- [x] 안전(실행 없는) 실제 미리보기 (2026-06-03): cp/mv 덮어쓰기 → 진짜 unified diff(읽기 전용), rm/truncate → content-at-risk 요약. `src/diff.rs`(LCS) + `preview::render_preview`. sed -i/perl -i 등 **실행 필요** diff는 샌드박스(§31.11, Phase 2+) 후속. 설계/계획: `docs/superpowers/{specs,plans}/2026-06-03-safe-preview-render*`
- [x] **완료 기준(부분)**: `rm -rf` 대상 목록·개수 표시, 외부상태 불가 사유 표시. diff 생성은 후속

### W10 실행 취소 / 트랜잭션 — ✅ 구현 (2026-06-02, `src/undo.rs`)
- [x] best-effort 파일 롤백: `create_backup`(파일 복사 + metadata.toml) / `restore` / `latest`
- [x] 백업 상한(500MB / 1000 files / 파일 20MB / TTL 7일) enforcement → 초과 시 `Refused(사유)`
- [x] `ai undo last` CLI (백업 없으면 안내)
- [x] 명령 실행 파이프라인에 백업 자동 트리거 연결(`pipeline::execute` → 삭제/덮어쓰기 시 `undo::create_backup`, Refused 시 실행 중단)
- [x] **완료 기준(부분)**: 한도 초과 시 Refused로 사전 차단(호출측 중단). 자동 트리거는 후속

### W11 사용량 / 비용 — ✅ 구현 (2026-06-02, `src/usage.rs` + store)
- [x] usage_event 기록(`store.record_usage`) + 누적 집계(`total_cost`), TokenSource/CostSource enum
- [x] 예산 평가 `evaluate`(session $2 / month $30, warn 80% / block 100%) → Ok/Warn/Block
- [x] `ai usage` CLI (누적 비용·예산·상태 표시)
- [x] **예산 게이트 결선 (2026-06-04, WI-1)**: `Gateway::with_budget`(주입식 `BudgetSnapshot`) → 캐시 미스 후 백엔드 호출 직전 `usage::evaluate`로 block 임계 시 `Blocked`. 캐시 히트·로컬(ollama)은 비용 0이라 차단 안 됨. `usage::estimate_cost`(per-token 단가) → `ai ask` estimated 비용 기록(0.0 하드코딩 제거)+배지. storage 통합테스트(지출 $2 초과→차단). 설계/계획: `docs/superpowers/{specs,plans}/2026-06-04-gateway-budget-gate*`
- [ ] (후속) 월 시간창(monthly window) 추적, provider-reported 실비용, `ai dispatch` 경로 예산 게이트
- [x] **완료 기준 (§31.7)**: 예산 100% 시 원격 AI 차단(게이트웨이+통합테스트), 모든 AI 요청 usage 기록, estimated 표기

### W12 에러 분석 + 히스토리 + 감사 — ✅ 구현 (2026-06-02, `src/explain.rs`)
- [x] 규칙 기반 에러 분석 `explain`(command not found/permission/no such file/generic) + `ai explain "<cmd>" --exit --stderr`
- [x] 세션 히스토리(`ai history`, W4), audit_events 기록(`record_audit`, W4/lock)
- [x] `last-error` 자동 캡처 (2026-06-03): `precmd` exit_code 기록 + `ai explain --last-error`(직전 실패 명령 분석). stderr 본문 캡처는 후속(hook은 stderr 미수집)
- [x] **완료 기준 (M3 핵심)**: preview/undo/usage/에러분석 모듈 동작 (CLI 제공)

## M4 — 컨텍스트 + 가드레일 + 호환성 (W13~W16) · §31.9, §31.10, §31.11

### W13 컨텍스트 일관성 관리자 — ✅ 구현 (2026-06-02, `src/context.rs`)
- [x] `SessionContext`(cwd/shell/user/hostname/git_branch) + `gather()`, `ai context` CLI
- [x] 상태 갱신 트리거 감지 `is_context_changing`(cd/pushd/export/alias/source/git checkout·switch·pull·reset)
- [x] env allowlist + denylist(TOKEN/SECRET/KEY/PASSWORD) + PATH hash-only(`filter_env_var`) → secret 미저장
- [x] `needs_refresh`(cwd/branch 불일치) + `git_branch`(.git/HEAD 파싱)
- [x] **민감 파일 컨텍스트 제외 가드 (2026-06-04, WI-2)**: `allow_file_in_context`/`filter_context_paths` — `.env`/`.pem`/`.key`/`id_rsa`/`credentials`를 원격 컨텍스트에서 제외(fail-closed). 패턴은 `mask::is_sensitive_path` 단일 진실원 위임. 향후 파일 본문 수집기가 통과해야 할 경계 게이트(경로 1차 + 본문 마스킹 2차 방어). 설계: `docs/superpowers/specs/2026-06-04-context-sensitive-path-guard-design.md`
- [x] **완료 기준**: git_branch 갱신·env secret 미저장·mismatch refresh 판정·민감 경로 제외(테스트). hook 자동 적용·파일 본문 수집기 결선은 후속

### W14 실행 가드레일 엔진 (baseline) — ✅ 구현 (2026-06-02, `src/guardrails.rs`)
- [x] Baseline 목록 `baseline()`(static analysis/risk scoring/preview/dry-run/timeout/confirmation/masking/policy enforcement)
- [x] 플랫폼 capability matrix `capabilities(Platform)` + `detect()`(Linux/WSL/macOS/Windows/Other)
- [x] `ai doctor --guardrails`가 platform·baseline·matrix 출력, 제한 플랫폼 High+ 강화 고지
- [ ] 실제 동적 감시(seccomp/cgroups 등) 구현 — Phase 2+ (MVP는 명시적 capability 고지)
- [x] **완료 기준**: 미지원 guardrail 명시(조용한 실패 금지), 제한 플랫폼 High+ 확인 강화 고지

### W15 Provider 추상화 + Token Window — ✅ 구현 (2026-06-02, `src/provider.rs`, `src/tokenwin.rs`)
- [x] `Provider`/`ModelCapability` capability map + `Provider::mock()`
- [x] fallback: `token_source`(→Estimated)/`cost_source`(→PricingTable)/`use_streaming`. tool-use MVP 제외
- [x] Token Window: `estimate_tokens`(char/4)/`chunk`(window·overlap)/`fits`
- [ ] 실제 provider 어댑터(HTTP) — Phase 2 Model Gateway
- [x] **완료 기준**: capability 기반 명시적 fallback, 불확실 토큰/비용 estimated (테스트)

### W16 호환성 테스트 + MVP 진입 결정 — ✅ 핵심 완료 (2026-06-02)
- [x] 셸 호환성(bash/zsh `-n` 문법, WSL), 플랫폼 감지(Linux/WSL/macOS/Windows/Other) — 양 플랫폼 테스트 통과
- [x] 속성/통합 테스트(`tests/integration.rs`): 위험도 결정성(50회)·Critical 차단 100%·마스킹 무유출
- [x] KPI(로컬): 결정성·Critical 100%·마스킹 0 검증. (지연/응답 KPI·커버리지 측정은 실행/provider 연동 후)
- [x] §31.12 9개 영역 체크리스트 + §31.13 확정값 → `docs/MVP-ENTRY.md`
- [x] **완료 기준 (M4 핵심)**: MVP+ 로컬 결정성 골격 완료. provider 의존 end-to-end는 Phase 2

---

## Phase 2 — Intelligent Workflow (착수)

### P2-1 AI Model Gateway — ✅ 골격 (2026-06-02, `src/gateway.rs`)
- [x] `LlmBackend` 트레이트 + `EchoBackend`(mock), `Gateway`(mask → token window → backend → usage)
- [x] 마스킹 fail-closed(private key 차단), secret은 백엔드 도달 전 치환, 토큰 추정. `ai ask "<prompt>"`
- [x] storage feature 시 usage 자동 기록
- [ ] 실제 provider 어댑터(HTTP), 로컬 LLM(Ollama) 백엔드 — 후속
- [ ] aitask 타임아웃/취소를 async 백엔드 경로에 결합 — 후속

### P2-2 Intent Classifier — ✅ (`src/intent.rs`)
- [x] `classify` → Empty/AiInline/AiQuery/Shell (질문어·물음표·한국어 마커). `ai classify`. Hybrid Mode 토대.

### P2-3 AI 응답 캐시 — ✅ (`src/cache.rs`)
- [x] `ResponseCache`(키=마스킹 프롬프트 해시, TTL) + Gateway 연동(히트 시 백엔드 생략). 시맨틱 캐시는 후속.

### P2-4 Ollama 로컬 LLM 백엔드 — ✅ (`src/http.rs`, `src/ollama.rs`)
- [x] `HttpTransport`(주입) + `TcpTransport`(평문 HTTP, 무의존) + `OllamaBackend`(/api/generate). `ai ask --backend ollama`. AI 실패 셸 비전파.

### P2-5 OpenAI 호환 HTTP 백엔드 — ✅ (`src/openai.rs`)
- [x] `/v1/chat/completions` build/parse + `OpenAiBackend`(bearer=`$OPENAI_API_KEY`). `ai ask --backend openai`. 평문 엔드포인트 지원; 클라우드 HTTPS는 TLS transport 후속.

### P2-6 Hybrid Mode dispatcher — ✅ (`src/dispatch.rs`)
- [x] `dispatch` intent→Shell{risk,decision}/Ai{prompt}/Empty. `ai route`.
### P2-7 Verification Agent — ✅ (`src/verify_agent.rs`)
- [x] `verify_command` → Verdict(환각/위험도/정책/secret 종합, safe_to_suggest). `ai verify`.
### P2-8 통합 스킬 관리(§26) — ✅ (`src/skill.rs`)
- [x] SKILL.md discover/parse(frontmatter)/match. `ai skill [--query]`. (콘텐츠=Zero-Trust)
### P2-9 시맨틱 캐시 — ✅ (`src/cache.rs`)
- [x] `similarity`(Jaccard) + `SemanticCache::get_similar`(임계값/TTL).
### P2-10 통합 MCP 관리(§27) — ✅ (`src/mcp.rs`)
- [x] `parse_servers`(mcp.json) + `is_mutating_tool`(컨센트 게이트). `ai mcp`.

### P2-11 Semantic File Index — ✅ (`src/index.rs`)
- [x] `FileIndex::build`(무시 디렉터리/대용량 제외 walk) + `search`(키워드 매칭 랭킹). `ai index`.
### P2-12 Tool Use Planner — ✅ (`src/planner.rs`)
- [x] `plan(request)` 규칙 매핑 명령 단계(복합→다단계, 무매칭→AI 위임). `ai plan`. (생성 명령은 게이트 대상)

### P2 나머지 (후속 — 리팩터/네트워크/이연)
- [x] aitask 타임아웃/취소 결합 (2026-06-03): `Gateway::ask_cancellable` + `ai ask` 런타임/Ctrl+C
- [x] 진짜 async transport (2026-06-03, 2a): `HttpTransport` async(AFIT) + `TcpTransport` tokio TcpStream, backend/gateway async. spawn_blocking 제거(future drop이 연결 취소)
- [x] HTTPS(TLS) transport (2026-06-03, 2b): `tls` feature — `tokio-rustls`(ring)+`webpki-roots`, scheme 분기, C-free 기본 빌드 유지. 실제 HTTPS e2e 확인
- [x] Shell/Ai 단일 dispatcher 통합 (2026-06-03): `dispatch::run` 오케스트레이터가 입력을 셸 pipeline / AI gateway로 분기(`AiResponder` 주입). `GatewayResponder`(sync↔async 브리지), TUI Submit 재배선(자연어 질의→AI), CLI `ai dispatch "<input>"`, audit source dispatch/exec 구분. 설계/계획: `docs/superpowers/{specs,plans}/2026-06-03-unified-dispatcher*`
- [x] 비-Ran 명령 결과 audit 기록 (2026-06-03): `command_blocked`/`command_declined`/`command_backup_refused`, 마스킹된 명령 포함. `shell_outcome_audit`(순수 매퍼) + `finish_shell_outcome`(공용 발산 헬퍼). run_exec/run_dispatch Shell arm 중복 제거.
- [x] gateway 시맨틱 캐시 2차 조회 결합 (2026-06-03): exact 미스 → `SemanticCache::get_similar`(임계값 0.85) 2차 조회, 히트 시 exact 승격. `CacheSource`(Backend/Exact/Semantic) 플래그를 `ai ask`/`ai dispatch` 배지로 표시. 설계/계획: `docs/superpowers/{specs,plans}/2026-06-03-gateway-semantic-cache*`
- [x] AI usage 기록 후속 (2026-06-27): `src/ai_usage.rs` helper와 `ai ask`/`ai dispatch`/ash AI 라우터 결선 구현. provider/model hard-code 제거, cache/local zero-cost, OpenAI estimated cost, ash budget snapshot 주입. PR #26 CI green(`fmt · clippy · test`, `cargo audit`, `android JNI packaging`, `windows build + self-contained check`). 설계/계획: `docs/superpowers/{specs,plans}/2026-06-27-ash-ai-usage-recording*`
- 데몬 아키텍처(설계상 조건부, P2 후반)

## 플랫폼 피벗 — 독립 `ash` + 모바일 로컬 터미널 (정렬 2026-06-23)

> 정본 설계: `docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md`. 세부 실행 workflow: `docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md`. 제품 정체성은 모든 지원 플랫폼에서 돌아가는 **독립 로컬 터미널**이다. PWA는 승인/모니터링 companion일 뿐, 모바일 제품의 본체가 아니다.

### 현재 진행 상태 (2026-06-26)

| 영역 | 상태 | 근거 | 다음 gap |
|---|---|---|---|
| `ai` 릴리즈 라인 | [x] | `Cargo.toml`/`VERSION` 0.2.2, Linux/Windows 설치·릴리즈 문서 | Windows `ash` 병행 배포 정책 정리 |
| Phase 1/2 안전 코어 | [x] | 위험도·정책·마스킹·preview/undo/usage·context·guardrails·gateway·dispatch 구현 | Windows `ash` 실행 경로에 안전 게이트 결선 |
| Remote approval 기반 | [~] | M0~M1 slice 4a 완료(게이트·Noise·검증·데몬 substrate·framing) | 실 리스너·페어링·게이트→디바이스 왕복·PWA companion |
| `ash`/`shellcore` | [~] | `[[bin]] name="ash"`, `src/shellcore/*`, REPL·값 모델·parser/evaluator·`where`·trait-backed 외부 실행 adapter·pure mode | Windows UX 기능 완성: line editor/history/config, AI/safety gate integration |
| 플랫폼 목표 매트릭스 | [x] | 2026-06-23 spec 작성 | 구현 slice별 계획/검증 |
| Windows GUI `ai-terminal.exe` | [x] | 독립 Tauri/xterm GUI, bundled `ash.exe` PTY bridge, terminal UX, AI/safety/storage/audit, portable/NSIS smoke evidence green | MSI 검토와 수동 더블클릭 smoke는 후속 |
| Android 로컬 터미널 | [~] 보류 | Kotlin/Compose skeleton, worker thread + stream/cancel JVM contract, Rust `MobileShell` pure core boundary, JNI bridge + instrumentation smoke, app-private workspace/cwd boundary, document import/export + text preview, full-ABI JNI packaging CI, shellcore-only MVP와 PM-3E 외부 명령 전략 결정, PM-3F Termux opt-in bridge design, T0 real-device smoke, T1 helper protocol/polling/cancel substrate, helper bootstrap UX + shared staging real-device smoke gate | Windows 완료 후 재개: SAF-backed staging UX decision, richer imported file readers |
| iOS/iPadOS 로컬 터미널 | [ ] | P2/research로 분리 | self-contained REPL·파일 컨테이너·정책-safe subset |
| PWA/모바일 companion | [~] | RA 설계/목업 계열 존재 | 로컬 터미널 대체가 아닌 승인·페어링·모니터링으로 재배치 |

### PM-0 — 방향 정렬
- [x] `ash`를 플랫폼 공통 독립 셸 런타임으로 확정(`shellcore` 공유)
- [x] 모바일 목표를 PWA 승인 화면에서 **온디바이스 로컬 터미널**로 전환
- [x] RA/PWA를 S4 companion 기능으로 재배치
- [x] Task별 세부 workflow 문서 작성: `docs/superpowers/plans/2026-06-23-platform-mobile-local-terminal-workflow.md`

### PM-1 — Desktop/Windows GUI Terminal — 자동 완료 기준 충족

> 2026-06-27 방향 정정: `ash.exe` 자체는 Windows 하위 런타임으로 유지한다. Windows 사용자의 1차 제품 표면은 외부 터미널 프로그램이 아니라 자체 창을 가진 `ai-terminal.exe`다. 기존 `docs/superpowers/plans/2026-06-27-windows-ash-manual-verification.md`는 GUI MVP 완료 조건이 아니라 `ash` 런타임 회귀 검증 문서로만 남긴다.

#### PM-1A — 기존 `ash.exe` runtime substrate
- [x] `shellcore` platform boundary 정의: pure evaluator와 외부 실행 adapter 분리, capability flags(`can_spawn`/`has_pty`/`has_conpty`/`has_userland`) 문서화 (`docs/superpowers/specs/2026-06-23-platform-execution-contract.md`)
- [x] Linux/WSL `ash` 스모크를 테스트에 추가(`[{size: 50} {size: 200}] | where size > 100`)
- [x] Windows `ash.exe` 스모크를 CI/로컬 smoke에 추가(`ash` 구조화 명령 + `.cmd`/`.ps1` 실행)
- [x] Windows execution adapter 정의: direct spawn / `cmd` / PowerShell / `.ps1` quoting·exit code·PATH/PATHEXT (`winexec` resolution + argv spawn-plan tests, native `.cmd`/`.ps1` exit-code smoke)
- [x] ConPTY 기반 interactive smoke 정의(portable-pty Windows 동작, `cmd.exe` marker round-trip)
- [x] Git Bash/MSYS profile 정의: path conversion, POSIX tool discovery, native `ash.exe`와 MSYS bridge 경계
- [x] WSL 설치/실행 문서 분리: Windows native `ash.exe`와 WSL `ash`를 혼동하지 않게 안내
- [x] Windows `ash.exe` line editor 구현: 입력 편집, history 탐색, Ctrl-C/Ctrl-D, EOF/interrupt 동작을 Windows 콘솔과 ConPTY에서 고정
- [x] Windows `ash.exe` history 저장/로드 구현: 기본 경로, 손상 파일 복구, 동시 실행 시 best-effort append, 민감 명령 저장 제외 정책
- [x] Windows `ash.exe` config 로딩 구현: 사용자 config path, profile/env override, 기본값 출력, 잘못된 config의 fail-soft 진단
- [x] Windows `ash` 실행 경로에 안전 게이트 결선: risk/policy/preview/undo/usage/audit를 shellcore external execution 앞에 연결
- [x] Windows AI integration 결선: 자연어 입력 dispatch, gateway timeout/cancel, 로컬 실패가 셸 세션을 깨지 않는지 검증
- [x] Git Bash/MSYS bridge runner 구현: `AI_TERMINAL_WINDOWS_PROFILE=msys`에서 path conversion, POSIX tool discovery, native `ash.exe` 호출 경계를 실제 실행으로 고정
- [x] Git Bash/MSYS smoke 추가: MSYS profile opt-in, path 변환, exit code, `.sh`/POSIX tool 호출의 성공·실패 케이스
- [x] Windows 문서/패키징 정리: README의 Windows native `ash.exe`, WSL `ash`, Git Bash/MSYS 사용 경계를 분리하고 `ai`/`ash` 역할을 명확히 작성
- [~] Windows `ash.exe` manual TTY 검증: GUI MVP의 완료 조건은 아니며, runtime regression evidence로 유지한다.

#### PM-1B — 독립 GUI terminal app
- [x] GUI terminal pivot 설계 추가: `docs/superpowers/specs/2026-06-27-windows-gui-terminal-pivot-design.md`
- [x] G0/G1 실행 계획 추가: `docs/superpowers/plans/2026-06-27-windows-gui-terminal-s0-s1.md`
- [x] Tauri v2 + xterm.js 앱 skeleton 추가(`desktop/`)
- [~] Rust backend `TerminalSession` manager 추가: open/write/resize/kill 구현, restart 후속
- [x] portable-pty/ConPTY로 bundled `ash.exe`를 child runtime으로 spawn
- [x] xterm.js `onData`/`onResize`와 backend event bridge 결선
- [x] copy/paste, selection, scrollback, Ctrl-C/Ctrl-D UX 결선 추가; resize, selection/copy/paste/scrollback, Ctrl-C recovery, Ctrl-D exit 자동 smoke evidence 확보
- [x] Windows packaging/smoke: portable package/zip 생성 완료(`ai-terminal.exe` + `ash.exe` + `ai.exe` + checksum + GUI smoke script), 자동 GUI smoke(창/`ash.exe` child/prompt·input·output screenshot+transcript/resize screenshot/frontend UX evidence/AI routing+safety gate+storage/audit evidence/Ctrl-C recovery screenshot/Ctrl-D exit screenshot/cleanup) green, NSIS installer artifact + SHA256 생성 완료, installer silent install/installed GUI smoke/uninstall cleanup green. MSI 재검토는 후속
- [x] GUI 완료 검증: `ai-terminal.exe` 실행 → 외부 터미널 창 없이 앱 내부에서 `ash` prompt/명령/AI/gate/storage/cleanup 통과. 자동 evidence: `gui-smoke-evidence.json`, `gui-smoke-ash-integration-evidence.json`, `artifacts/nsis-install-smoke/installed-gui-smoke-ash-integration-evidence.json`
- [x] Dev workbench S1 UI shell: 상단 리본바, 탭, Split H/V, pane-level runtime selector(`ash`/`Ubuntu`/`Docker`/`Codex`/`Claude`/`Gemini`) skeleton 추가. 첫 `ash` pane은 기존 live PTY/smoke 경로 유지. 정본: `docs/superpowers/specs/2026-06-29-windows-dev-runtime-workbench-design.md`
- [x] Dev workbench S2 runtime inventory: 앱 시작 시 read-only probe로 `ash`, WSL2 Ubuntu, Docker CLI, `codex`/`claude`/`gemini` 설치·버전 상태를 읽어 리본 chip에 표시. 설치/업데이트/실행 mutation 없음
- [x] Dev workbench S3 WSL2 Ubuntu manager 첫 실행 연결: `AI_TERMINAL_UBUNTU_DISTRO` 또는 기본 `Ubuntu` distro 감지, 리본의 명시적 Ubuntu install starter, live pane에서 `Ubuntu` 선택 후 Restart 시 `wsl.exe -d <distro> --exec bash -l` PTY 실행. apt update/import 세부 관리는 후속 S3 확장/S5 전 단계
- [x] Dev workbench S4 Docker manager 첫 연결: Docker CLI/Engine/managed image 상태 probe, 명시적 `winget install Docker.DockerDesktop` starter, `AI_TERMINAL_DOCKER_IMAGE` 또는 기본 `ubuntu:24.04` image pull/update, Docker runtime 선택 후 Restart 시 `docker run --rm -it <image> bash -l` PTY 실행. 내부 앱 카탈로그/compose 관리는 후속
- [x] Dev workbench S5 AI CLI manager 첫 연결: startup inventory가 managed Ubuntu 내부의 `codex`/`claude`/`gemini` 설치·버전을 probe하고, 리본의 명시적 `Install AI CLIs`/`Update AI CLIs` 액션이 Ubuntu 내부 사용자 npm prefix에 `@openai/codex`, `@anthropic-ai/claude-code`, `@google/gemini-cli`를 설치/업데이트한다. 각 CLI runtime 선택 후 Restart 시 `wsl.exe -d <distro> --exec bash -lc 'exec <cli>'`로 pane을 전환한다. 자격 증명은 복사하지 않으며 각 CLI의 자체 auth flow를 사용한다.
- [x] Dev workbench S6 Docker app catalog 첫 연결: 내부 앱 설치는 Docker image를 우선한다는 정책에 맞춰 built-in app catalog(`Ubuntu Base`, `Node.js Dev`, `Python Dev`, `Rust Dev`)를 리본에 노출하고, 각 image 상태 probe/명시적 `Pull App`/선택 앱 Docker pane 실행을 연결했다. Compose stack, 동적 카탈로그, per-app policy 파일은 후속
- [x] Dev workbench S7 Ubuntu apt manager 첫 연결: managed Ubuntu 안의 built-in apt package catalog(`git`, `curl`, `build-essential`, `python3`, `nodejs`, `npm`) 상태를 read-only probe하고, 리본의 명시적 `Apt Update`/`Install Pkg` 액션으로 `sudo -n apt-get update`와 선택 패키지 설치를 실행한다. Docker image 우선 정책은 유지하고 apt는 base dependency fallback으로 제한

**Windows 완료 기준**: `ai-terminal.exe`가 자체 GUI 창에서 PTY/ConPTY 기반 `ash` 세션을 실행하고, 외부 Windows Terminal/PowerShell/Git Bash 창 없이 terminal UX와 safety/AI/storage 경로가 통과해야 한다. 이 자동 완료 기준은 2026-06-28 portable/NSIS smoke에서 충족됐다. 남은 Windows 후속은 MSI 검토와 수동 더블클릭 smoke evidence다.

### PM-3 — Android 로컬 터미널 스파이크
> 현재 상태: Windows GUI 터미널 자동 완료 기준 이후 Android PM-3 UX와 direct APK/F-Droid release 입력 대부분을 닫았다. 다음 Android 후속은 fdroidserver/buildserver 실제 검증과 GitHub signing secrets 등록/검증이다.

- [x] Android 앱 shell 결정(Kotlin/Compose + Rust FFI 기본값, 대안은 spike에서만 변경)
- [x] Rust `shellcore`를 Android 앱에서 호출하는 최소 REPL core boundary (`src/mobile.rs`, `MobileShell`)
- [x] FFI boundary 정의: `eval_line(input, session_state) -> output + updated_state`, panic 격리, structured value JSON/typed bridge
- [x] JNI bridge 연결: `NativeShellBridge` → Rust `MobileShell`, `FakeShellBridge` 제거
- [x] Actual JNI instrumentation smoke: emulator/device에서 `NativeShellBridge`가 `MobileShell`을 호출하는 계약을 `androidTest`와 CI `connectedDebugAndroidTest`로 고정
- [x] terminal UI 입력/출력 + worker thread 분리 spike (`android/` Compose skeleton)
- [x] UI thread 차단 금지 1차 검증: `ShellWorker` single-thread executor + main-thread result posting
- [x] Worker behavior test: JVM unit test로 worker thread 평가, result poster callback, bridge failure 변환 계약 고정
- [x] Output streaming/cancel contract: `ShellStreamEvent`/`ShellRunHandle` 타입과 JVM event-ordering 테스트로 고정
- [x] 외부 명령 전략 비교: shellcore-only MVP 유지, 다음 후보는 Termux-compatible opt-in bridge, bundled minimal userland는 보류 (`docs/superpowers/specs/2026-06-24-android-external-command-strategy.md`)
- [x] Android Rust `.so` 전체 ABI 빌드/패키징 CI 자동화
- [x] Android 파일 접근/권한/스토리지 모델에서 workspace 개념 정의
- [x] 모바일 좁은 화면용 cwd/workspace/status 표현 결정
- [x] Android document picker 기반 import/export 구현
- [x] Android document import text preview: UTF-8 preview를 transcript에 표시하고 binary-like content는 건너뜀
- [x] Termux-compatible bridge design spike: availability, stream/cancel, non-zero exit, workspace sharing smoke 정의 (`docs/superpowers/specs/2026-06-25-termux-compatible-opt-in-bridge-design.md`)
- [x] Termux T0 `RUN_COMMAND` probe substrate: package visibility, permission detection, result receiver service, echo probe UI, pure result decoding tests
- [x] Termux T0 real-device smoke: `allow-external-apps`, final stdout/stderr/non-zero exit validation on installed Termux runtime
- [x] Termux T1 helper protocol substrate: argv request JSON and NDJSON event-to-`ShellStreamEvent` mapping tests
- [x] Termux T1 helper event file polling and cancel file-backed `ShellRunHandle.cancel()` tests
- [x] Termux T1 helper bootstrap UX and self-test gate: install `~/.ash-termux-bridge/helper.sh` through `RUN_COMMAND`, keep external commands disabled after helper self-test until shared staging smoke passes
- [x] Termux shared staging UI/path smoke gate: user-selected path app-write validation, helper event-file marker smoke, dynamic adapter attach only after success
- [x] Termux T1 helper protocol: real-device incremental event file, cancel token, stderr/non-zero, large output, shared staging workspace smoke (`SM_F956N`, `/sdcard/Download/ash-termux-bridge`, Termux storage permission granted)
- [x] Shared staging UX 결정: path input 유지 + primary shared-storage SAF picker 보조, Termux storage permission은 Verify smoke 실패 메시지로 안내
- [x] Imported file UX 확장: import된 workspace 파일을 `Open Last`로 read-only 재열람, workspace 밖/binary 거부
- [x] 배포 경로 결정: APK/GitHub Release → F-Droid 준비 우선, Play Store는 Termux-enabled build 정책 검토 후 (`docs/superpowers/specs/2026-06-28-android-distribution-route.md`)
- [x] Android release versioning/package gate: root `VERSION` 기반 `versionName`, semver-derived `versionCode`, universal APK, `:app:verifyFdroidReleaseInputs`
- [x] F-Droid/Fastlane metadata: title/short/full description, changelog `300.txt`, phone screenshots 2장, repo license files
- [x] Android signing smoke: throwaway keystore로 Gradle signing path와 `apksigner verify` 검증
- [x] GitHub Android signing secret preflight: base64 keystore secret decode path, Gradle signing, `apksigner --print-certs` 검증
- [x] fdroiddata submission draft: `android/fdroiddata/metadata/dev.aiterminal.android.yml` + regex update-check mirror `android/fdroid-version.properties`; local fdroidserver `lint`/`rewritemeta` green, build block disabled until next Android release tag includes this work
- [x] fdroiddata activation preflight: release commit hash로 disable 제거/TODO 교체/`rewritemeta`/lint를 source 수정 없이 dry-run

### PM-4 — iOS/iPadOS research
- [ ] self-contained `shellcore` REPL spike(TestFlight 기준)
- [ ] App Review 2.5.2 제약 아래 가능한 명령 subset 정의
- [ ] 파일 컨테이너/문서 picker 기반 workspace 모델 검증
- [ ] "완전 Linux 터미널"이 아니라 "제한적 로컬 구조화 터미널"로 사용자 약속 문구 확정
- [ ] iOS에서 외부 유저랜드/다운로드 코드/임의 프로세스 실행을 제품 약속에서 제외할지 결정

### PM-5 — Product packaging
- [ ] Windows 우선 `ai`(기존 CLI)와 `ash`(독립 셸)의 역할/이름/버전 정책 정리
- [ ] README 플랫폼 지원 표를 "현재 배포"와 "목표 매트릭스"로 분리하되, Windows native/WSL/Git Bash-MSYS 경계를 먼저 확정
- [ ] `document/` v3.3 설계와 `terminal/` 피벗 설계의 충돌을 정리하는 migration note 작성
- [x] 릴리즈 아티팩트에 `ai`/`ash`를 별도 바이너리 asset으로 함께 배포(v0.2.4, 각 checksum 포함)

### PM-6 — RA/PWA companion 재배치
- [ ] RA-1~RA-4를 desktop daemon/listener/pairing/gate-flow 기준으로 완주
- [ ] RA-5 PWA를 승인·페어링·모니터링 companion으로 한정
- [ ] Android/iOS 로컬 터미널이 준비되기 전에는 RA device identity를 모바일 터미널 본체와 결합하지 않음
- [ ] 사용자 문구 확정: "Mobile ash app = local terminal", "PWA companion = approve/pair/monitor/demo"

## Phase 3 — Team & Enterprise (상세화 2026-06-05)

> 정본 설계: `docs/superpowers/specs/2026-06-05-phase3-roadmap-design.md`, 플랫폼 우선순위는 `docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md`와 `docs/superpowers/specs/2026-06-27-windows-gui-terminal-pivot-design.md`. 현재 순서(2026-06-27): **R0 → Windows 독립 GUI 터미널 MVP → Android PM-3 재개 → RA companion → P3-1 → P3-2 → P3-3**. 각 마일스톤 착수 시 `writing-plans`로 슬라이스별 계획 생성. 동적 감시·gVisor는 Linux 우선.

### R0 — 현 상태 릴리즈 (v0.2.x, 최초 v0.2.0) · §29.11
- [x] R0-1 feature 매트릭스 빌드 확정(`default`+`remote` C-free 양 플랫폼 우선 / `storage`+`tls`는 Windows MSVC 검증) — 각 조합 release green, 실패 조합 명시
- [x] R0-2 Windows 네이티브 실사용 검증(ConPTY, wrapper 모드 안내, 경로/`\r\n`) — `ai doctor` 유효 모드 표시 + 핵심 명령 동작
- [x] R0-3 버전·릴리즈 메타(`Cargo.toml` 0.2.0, `CHANGELOG.md`, `VERSION`) — 버전 단조 증가
- [x] R0-4 배포 스크립트(Linux `install.sh` curl|sh / Windows `install.ps1`|zip) — 깨끗한 환경 설치→`ai --version` 동작
- [x] R0-5 크로스빌드 CI(GitHub Actions: ubuntu x86_64-gnu + windows x86_64-msvc) + 아티팩트 + **SHA256 체크섬** — 태그 push 시 Release 자동 첨부
- [x] R0-6 릴리즈 노트 + 설치 문서(README) — 문서만으로 설치 가능
- (검증 2026-06-05: lib 263 + version_sync + 통합 0 failed · fmt/clippy clean · 매트릭스 5조합 green · Windows 네이티브 SMOKE_OK · 브랜치 `feat/r0-release`. 실제 태그 push 릴리즈는 승인 후)
- **경계**: 서명 바이너리(§29.11 full)는 P3-1로 이연(R0는 체크섬까지).

### RA — remote-approval companion 완주 (M1 4b → PWA companion, relay M2 제외) · §28·§30-13
- [ ] RA-1 디바이스 연결 리스너(데몬이 `session::run_daemon_request` 호스팅) — 실 리스너 위 handshake+왕복
- [ ] RA-2 페어링 CLI/QR(`daemon_pubkey` 앵커 + `pairing_code`, `DeviceRecord` 영속화, TOFU·동시 페어링 거부) — `ai remote pair`
- [ ] RA-3 게이트 플로우 결선(armed High opt-in → 디바이스 승인 왕복 → `consume`+`validate`, **fail-closed timeout**, `NeedsApproval` 밴드 검토) — 승인/거부/타임아웃 e2e **← M1 데모 green 체크포인트**
- [ ] RA-4 데몬 컨텍스트 스냅샷(§31.10) + `context_hash`(env allowlist 해시 + realpath 타깃) — TOCTOU 실해시 재검증
- [ ] RA-5 PWA(`/approve`·`/pair`, `pwa-approval-mockup.html` 기반 + Noise 클라이언트 + 로컬/Tailscale 직결) — 실폰 페어링→승인/거부 반영
- [ ] RA-6 확장(arm TTL 자동 disarm #4 / heartbeat 최소판 #2 / 승인 상태 표시 #1)
- **경계**: relay(M2)·T-RA1~5는 완주 후 재평가(`TODOS.md`). 불변식 = §28(E2E·device revoke·replay 방지·signed approval·expiration).

### P3-1 — 트러스트 채널 + 조직 정책 · §30-7·§30-9·§29.11
- [ ] P3-1-1 공통 trust channel 코어(ed25519 manifest 검증, 공개키 앵커 OS trust store/MDM) — 위조·만료·다운그레이드 거부
- [ ] P3-1-2 signed `policy.d`(서명 필수, version monotonic, issued_at/expires_at, **readonly·최우선**) — 미서명·rollback 거부, 조직>사용자 e2e
- [ ] P3-1-3 스킬 서명 + 조직 레지스트리(§26.6, 외부 기본 비활성, update/revoke·감사) — 미서명 차단, revoke 즉시 반영
- [ ] P3-1-4 바이너리 서명(§29.11 full, R0 이연분) — 서명 검증 후만 설치/업데이트, 다운그레이드 방지

### P3-2 — 중앙 감사 + 팀 프로파일 + 엔터프라이즈 마스킹
- [ ] P3-2-1 중앙 감사 로그 export(`audit_events`→OTLP/syslog/파일, 명령 내용 미전송 옵션 §29.5) — 민감정보 미포함 검증
- [ ] P3-2-2 팀별 프로파일(balanced/paranoid 위 조직 레이어, policy.d 배포) — 적용·오버라이드 경계
- [ ] P3-2-3 엔터프라이즈 마스킹 규칙(조직 커스텀 패턴, `mask` 파이프라인 확장) — 기본 규칙과 병합
- [ ] P3-2-4 Debug Bundle(`ai doctor --bundle`, **마스킹 강제**) — 생성물에 secret 미잔존

### P3-3 — MCP 확장 + 고격리/가드레일 · §30-8·§30-10·§30-11
- [ ] P3-3-1 MCP mutate/external 컨센트(미선언=write/external 보수 분류, privileged 차단/강한 확인, 로컬 정책>서버 선언)
- [ ] P3-3-2 MCP OAuth(OS keyring 저장, scoped token, `ai mcp login/logout/status/rotate-token`) — silent refresh·재인증
- [ ] P3-3-3 Guardrails 동적 감시(seccomp/cgroups Linux 우선, eBPF Phase 3 한정, capability matrix 갱신) — WSL/Win 제한 명시
- [ ] P3-3-4 gVisor 샌드박스(FU-2 tmpdir→gVisor 승격, 가용성 고지) — 미가용 시 tmpdir 폴백

## Phase 4 — Advanced Automation (요약 — 추후 구체화)

- Cross-Session Knowledge, State Snapshot & Restore, Multi-agent workflow, Long-running task planner, IDE 연동, 웹 대시보드, Voice Input, Firecracker 고격리, **관리형 relay·멀티 디바이스**(RA에서 제외한 relay M2의 완성형이 여기로 합류).
- Phase 3 안정화 후 회고를 거쳐 상세화한다.
