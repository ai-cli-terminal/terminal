# HISTORY — 변경 / 결정 로그

> **정본**: 설계 결정의 권위 기록은 `../document/`(특히 `00-overview-architecture.md` §0.2 불일치 해소, `03_프로젝트_아키텍처_정의서.md` ADR, `05-roadmap-enhancements-decisions.md` §30 결정안).
> 본 문서는 **구현 repo(`terminal/`)의 변경·결정 타임라인**이다. 최신 항목이 위로 온다.

---

## 2026-06-03 — hook chpwd → cwd + git branch 컨텍스트 (M1/W3, §31.10)

- **store**(`store.rs`): `record_context_snapshot`(context_snapshots INSERT) + `latest_context`(최근 스냅샷 조회) + `update_session_cwd`(세션 cwd 갱신). `NewContext`/`ContextRow`.
- **main**(`main.rs`): `ai __hook chpwd cwd=<path>` 처리 → 세션 cwd 갱신 + 해당 경로의 git branch(`context::git_branch`)를 context_snapshot으로 기록(best-effort, 셸 비중단).
- 검증: TDD(store 2: 스냅샷 record/latest, session cwd update), WSL e2e(git 레포 → `(chpwd, …/terminal, master)`, 비-git → branch None, sessions.cwd 갱신; python3 sqlite로 확인). storage 156 / default 139 통과, fmt·clippy clean.
- **범위**: zsh는 `chpwd` hook을 발생시킴. bash는 native chpwd가 없어(precmd `cwd` 보유) bash용 cwd/branch 연동은 후속.

## 2026-06-03 — 마스킹 고엔트로피 휴리스틱 (M2/W7, §31.8)

- **mask**(`mask.rs`): 명명 규칙(AWS/GitHub 등)이 놓친 generic secret을 Shannon 엔트로피로 탐지·마스킹. named 규칙 적용 후 → validation 전 후처리 패스로 `[HIGH_ENTROPY_REDACTED]` 치환.
- 판정(`is_high_entropy_secret`): 길이 ≥20 + 엔트로피 ≥4.0 bits/char + 영문·숫자 혼합 + `_REDACTED` 플레이스홀더 제외. 후보 문자셋 `[A-Za-z0-9_=+-]`(점·슬래시·콜론 제외)로 경로/URL/도메인/버전 오탐 회피.
- 차단(block)이 아니라 마스킹(redact) — 마스킹 자체가 안전 조치이고, 해시 등 비밀이 아닌 고엔트로피 문자열 과차단을 피함(보수적 over-mask 허용).
- 검증: TDD(고엔트로피 마스킹 / 자연어·경로·저엔트로피 비마스킹 / guards 3종), 합성 토큰은 선형 순열로 결정성 확보(리터럴 시크릿 회피). `ai mask` e2e 확인. storage 154 / default 139 통과, fmt·clippy clean.

## 2026-06-03 — hook precmd 종료코드 + last-error 분석 (M1/W3 + M3/W12)

- **store**(`store.rs`): `update_last_exit(session, exit)` — `preexec`에서 `exit_code=NULL`로 기록된 직전 명령에 `precmd`의 실제 종료 코드를 채움(미정 1건만 갱신). `last_error(session)` — 가장 최근 실패(exit≠0) 명령 조회. `OptionalExtension` 사용.
- **main**(`main.rs`): `ai __hook precmd exit=<n>` 처리 → `update_last_exit`(best-effort, 셸 비중단). `ai explain --last-error`가 저장소의 직전 실패 명령을 꺼내 분석(`command`를 Optional로 변경, storage 미빌드 시 명확한 안내).
- 셸 hook 스크립트는 이미 `precmd`에 `exit=$?`를 전달 중이었음 → Rust 쪽 처리만 추가하면 연결 완성.
- 검증: TDD(store 단위 4 + CLI 파싱 1), WSL e2e(`frobnicate` exit=127 → `explain --last-error`가 "명령을 찾을 수 없습니다" + 제안; 성공-only는 "실패 명령 없음"). storage 151+22+4 / default 136+22+4 통과, fmt·clippy(default+storage) clean.
- **TASK 정정**: W6 `ai policy set` 영속·W7 전화/카드/여권 패턴은 이미 구현됨을 반영(문서가 stale했음).

## 2026-06-02 — Phase 2 후속: Semantic Index + Tool Use Planner (P2-11~12)

- **Semantic File Index**(`index.rs`): `FileIndex::build/search`(무시 디렉터리·대용량 제외 키워드 인덱스/랭킹). `ai index`.
- **Tool Use Planner**(`planner.rs`): `plan` 규칙 기반 명령 단계(복합 다단계/무매칭 AI 위임). `ai plan`.
- 환경: Windows `target/`가 3.8GB로 디스크 가득참 → `cargo clean` 후 재빌드(기본 feature로 검증, storage는 WSL/CI).
- 검증: Windows 기본 157개 통과, clippy/fmt clean. (storage 포함은 WSL에서 확인.)
- 남은 P2: async aitask 결합·HTTPS TLS·시맨틱 캐시 gateway 결합·데몬.

## 2026-06-02 — Phase 2 우선순위 진행: dispatcher/verify/skill/semcache/mcp (P2-6~10)

- **Hybrid dispatcher**(`dispatch.rs`): intent→Shell{risk,decision}/Ai/Empty. `ai route`.
- **Verification Agent**(`verify_agent.rs`): 환각+위험도+정책+secret 종합 Verdict. `ai verify`.
- **스킬 관리(§26)**(`skill.rs`): SKILL.md discover/parse/match. `ai skill`.
- **시맨틱 캐시**(`cache.rs`): Jaccard 유사도 `SemanticCache`.
- **MCP 관리(§27)**(`mcp.rs`): mcp.json 파싱 + mutate 도구 컨센트 판정. `ai mcp`.
- 검증: Windows 161개·Linux 동등, clippy(default+storage)/fmt clean. 커밋 5개 분리.
- 남은 P2: Tool Use Planner(AI 의존), async aitask 결합, HTTPS TLS, Semantic Index, 데몬.

## 2026-06-02 — Phase 2 진행: Intent/Cache/Ollama/OpenAI (P2-2~5)

- **P2-2 Intent**(`intent.rs`): `classify`(Shell/AiQuery/AiInline/Empty), `ai classify`.
- **P2-3 Cache**(`cache.rs`): TTL 정확 캐시 + Gateway 연동(히트 시 백엔드 생략, counting 테스트).
- **P2-4 Ollama**(`http.rs`+`ollama.rs`): `HttpTransport` 주입(+`TcpTransport` 무의존 평문 HTTP) + `OllamaBackend`(/api/generate, mock 테스트). `ai ask --backend ollama`.
- **P2-5 OpenAI**(`openai.rs`): bearer 인증 transport + `/v1/chat/completions` + `OpenAiBackend`($OPENAI_API_KEY). `ai ask --backend openai`.
- AI 백엔드 실패는 친절 고지 후 정상 종료(§3-3, exit 0). serde_json 추가.
- 검증: Windows 141개·Linux 동등 테스트 통과, clippy(default+storage) clean, fmt clean. 커밋 분리(intent/cache/ollama/openai).

## 2026-06-02 — Phase 2 착수: AI Model Gateway (P2-1)

- `src/gateway.rs` (TDD): `LlmBackend` 트레이트 + `EchoBackend`(mock), `Gateway::ask` 파이프라인 — prompt+context → **마스킹**(secret 치환/private key 차단 fail-closed) → 토큰 윈도(한도 초과 시 truncate) → 백엔드 → 토큰 추정.
- `ai ask "<prompt>"` CLI: 컨텍스트(cwd) 포함, 토큰 표시, storage feature 시 usage 자동 기록. echo 백엔드로 "secret이 백엔드 도달 전 마스킹됨" 검증.
- 이전 MVP 모듈(mask/tokenwin/provider/usage/context)을 AI 경로로 결합 — Phase 2의 토대.
- 검증: Windows 123개·(WSL 동등) 테스트 통과, clippy(default+storage) clean, fmt clean.
- 후속: 실제 provider HTTP 어댑터·로컬 LLM(Ollama), aitask 타임아웃/취소를 async 백엔드에 결합, Intent Classifier 등.

## 2026-06-02 — M4 구현 + MVP 진입 (context/guardrails/provider/호환성, W13~W16)

- **W13 context**(`src/context.rs`): `SessionContext`/`gather`/`is_context_changing`/`filter_env_var`(allowlist+denylist+PATH hash, secret 미저장)/`needs_refresh`/`git_branch`(.git/HEAD). `ai context`.
- **W14 guardrails**(`src/guardrails.rs`): `detect`(Linux/WSL/macOS/Other)·`baseline`·`capabilities` 매트릭스·`dynamic_monitoring_limited`. `ai doctor --guardrails` 리팩터링.
- **W15 provider**(`src/provider.rs`, `src/tokenwin.rs`): capability map + fallback(token_source/cost_source/use_streaming) + mock, `estimate_tokens`/`chunk`/`fits`.
- **W16 호환성+진입**: `tests/integration.rs`(결정성 50회·Critical 차단 100%·마스킹 무유출), `docs/MVP-ENTRY.md`(§31.12 9영역 + §31.13 확정값).
- 검증: 단계별 TDD, Windows 118개(94 lib + 20 bin + 4 integration)·Linux 동등, 양쪽 clippy(default+storage) clean, fmt clean. 커밋 W13~W16 분리.
- **M1~M4 로컬 결정성 핵심 완료.** provider 의존 원격 경로는 Phase 2(Model Gateway).

## 2026-06-02 — M3 구현 (preview/undo/usage/explain, W9~W12)

- **W9 preview**(`src/preview.rs`): `classify_preview`(dry-run 제안 / in-place→temp diff / 삭제·권한→대상목록 / 외부상태→불가 / 읽기→불필요), `ai preview`.
- **W10 undo**(`src/undo.rs`): `create_backup`(상한 enforcement→Refused) / `restore` / `latest`, `ai undo last`.
- **W11 usage**(`src/usage.rs` + store): `BudgetConfig`/`evaluate`(80% 경고/100% 차단), `record_usage`/`total_cost`, `ai usage`.
- **W12 explain**(`src/explain.rs`): 규칙 기반 에러 분석(not found/permission/no such file/generic), `ai explain`.
- 검증: 단계별 TDD, Windows 100개·Linux 104개 테스트 통과, 양쪽 clippy(default+storage) clean, fmt clean. 커밋 W9~W12 분리.
- M3 핵심 완료. 실행 파이프라인 자동 연동(백업 트리거·usage 자동기록·last-error stderr 캡처)은 provider/실행 연동 후속.

## 2026-06-02 — AI 요청 타임아웃 + Ctrl+C 취소 (M2/W8, §13·§16.2)

- `src/aitask.rs` 추가 (TDD, tokio): `Timeouts::defaults`(5/15/60/180s), `run_cancellable`(작업/타임아웃/취소 3-way select), `RequestError`(TimedOut/Cancelled/Failed), `cancel_on_ctrl_c`(SIGINT→취소).
- 실패·타임아웃·취소는 모두 `Err` 반환 → **AI 장애가 셸을 막지 않음**(Graceful Recovery, §16.2). tokio `sync` feature 추가.
- 검증: Windows 77개·Linux 81개 테스트 통과(async 테스트 포함), 양쪽 clippy clean, fmt clean.
- W8 완료 → M2 핵심(위험도·정책·마스킹·환각검증·타임아웃) 모듈 구현 완료. 실제 provider end-to-end는 Phase 2.

## 2026-06-02 — M1 잔여 항목 마무리 (5종, TDD + 커밋별 정리)

순차 진행한 M1 마무리 작업:
1. **마스킹 패턴 확장**(§31.8): 전화(KR)/신용카드/여권 추가, IP 오탐 방지.
2. **환각 검증 게이트**(§29.2, `src/verify.rs`): 바이너리 존재 검증(sudo/env/VAR= 건너뜀, 빌트인 인식, 경로/PATHEXT), `ai risk`에 binary 상태 표시.
3. **config 영속화**(§31.3, `src/config.rs`): 활성 프로파일을 `~/.config/ai-terminal/active_profile`에 저장. `ai policy set`, show/risk/tui는 활성 프로파일 사용.
4. **locks 레지스트리 + audit**(§31.2): `store`에 register/lock_owner/release/`reclaim_if_stale`(audit)/`record_audit`. 파일 락(lock.rs)과 함께 2층 구조 완성.
5. **TUI↔PTY 연결**(§5): TUI Enter 제출 → `pty::run_in_pty` 실행 → `append_output`로 히스토리 표시.

- 검증: Windows 72개·Linux 76개 테스트 통과, 양쪽 clippy(`--features storage`) clean, fmt clean.

## 2026-06-02 — 파일 락 + stale 정리 + DB 동시성 (M1/W4 잔여, §31.2)

- `src/lock.rs` 추가 (TDD): advisory 파일 락(`create_new` 원자적 상호배제), 락 파일에 pid/timestamp 기록, `LockGuard` RAII 해제. stale 판정(TTL 초과 / Linux는 `/proc` PID 부재) → 제거 → 재시도(§31.2).
- `store`: `integrity_ok`(`PRAGMA integrity_check`) 추가. **동시성 테스트**: 같은 파일 DB에 두 연결이 교대 write(30건) 후 무손상·integrity=ok 검증 → M1 DoD "동시 터미널 무손상"(WAL+busy_timeout) 충족.
- 검증: Windows 58개·Linux 62개 테스트 통과, 양쪽 clippy clean, fmt clean.
- 후속: `locks` 테이블 heartbeat 레지스트리 + stale audit 기록(진단/복구 고도화).

## 2026-06-02 — Secret/PII 마스킹 (M1/W7, §31.8)

- `src/mask.rs` 추가 (TDD, regex): `Masker::baseline()` 규칙 테이블(Secret: private_key_block(hard block)/AWS/GitHub/Slack/Bearer/Authorization/Password, PII: email/kr_rrn/ipv4), `mask()`가 Secret→PII 순 적용 후 validation scan.
- fail-closed: private key block 감지 또는 validation 재매치 시 `blocked=true`(원격 전송 차단). 원문 secret 미잔존 검증 테스트.
- `is_sensitive_path`(.env/.pem/.key/id_rsa), CLI `ai mask "<text>"`(leading-dash 허용).
- authorization 치환문이 자기 패턴에 재매치되어 오탐 차단 → 치환문을 `[AUTHORIZATION_REDACTED]`로 수정.
- 검증: Windows 54개·(WSL 동일) 테스트 통과, clippy clean, fmt clean.

> 다음 단계: 2층 파일 락 + stale 정리(W4 잔여, M1 DoD).

## 2026-06-02 — TUI 렌더링 착수 (M1/W2, §5)

- `src/ui.rs` 추가 (TDD): `UiState`(입력 편집/submit/히스토리), `current_risk`(실시간 위험도), `handle_key`(Char/Backspace/Enter/Esc→Action), `render`(상태바 profile·cwd / 히스토리 / 입력+위험도).
- `ratatui::TestBackend`로 헤드리스 렌더 검증(상태바 profile, 입력 위험 등급 표시 확인). `run` 이벤트 루프(crossterm raw mode + alt screen, Esc/Ctrl-C 종료)는 TTY 필요로 단위 테스트 제외.
- CLI: `ai tui [--profile]`.
- 검증: Windows 45개·Linux 49개 테스트 통과, 양쪽 clippy clean, fmt clean.

> 다음 단계: Secret/PII 마스킹(W7, §31.8).

## 2026-06-02 — SQLite 스토리지 + PTY 인터랙티브 (M1/W4·W2, §31.2)

- `src/store.rs` 추가 (TDD, `storage` feature/rusqlite): `Store`(open/open_in_memory/open_default), §31.2 7테이블 스키마 + WAL/PRAGMA, CRUD(create/get_or_create session, record_command w/ 위험도, recent_commands, count), FK 강제, `data_dir`(XDG/HOME).
- e2e 배선: `ai __hook preexec`가 명령을 위험도와 함께 `sess-default`에 기록(best-effort, 재진입 가드) → `ai history`로 표시. 셸 hook → risk → SQLite → 조회 전 구간 동작. (storage feature, 기본 빌드는 C-free 유지.)
- `src/pty.rs` 확장: `PtySession`(spawn/write_input/read_chunk/kill) — 인터랙티브 입출력 프리미티브. WSL에서 `cat` echo 라운드트립 검증.
- SQL 다중행 리터럴의 `\` 줄잇기가 식별자를 붙여(`risk_scoreFROM`) 버그 유발 → 일반 개행으로 수정.
- 검증: Windows 40개(lib 27 + bin 13)·Linux 44개(lib 31 incl pty 3 + bin 13) 테스트 통과, 양쪽 clippy(`--features storage`) clean, fmt clean. PtySession은 Windows(ConPTY) 컴파일 확인.

> 다음 단계: 2층 파일 락 + stale 정리(W4 잔여, M1 DoD) 또는 TUI 렌더링(ratatui, W2 잔여) 또는 마스킹(W7).

## 2026-06-02 — 셸 Hook 생성/설치 UX (M1/W3, §31.1)

- `src/shell.rs` 추가 (TDD, 2 cycle): `Shell`(bash/zsh, 경로 파싱), `hook_script`(preexec/precmd/chpwd, `command -v ai` 가드 + 에러 무시), `rc_block`(마커 래핑 가드 블록), `is_installed`/`apply_install`(idempotent)/`apply_uninstall`(블록만 제거)/`unified_diff`(공통 prefix/suffix).
- CLI: `ai shell-hook <bash|zsh>`, `ai init shell [--shell --rc --dry-run --diff --uninstall]`, 내부 `ai __hook`(hide, no-op). 순수 `plan_init_shell`로 파일 I/O와 분리해 테스트.
- WSL 검증: 생성 hook이 `bash -n`/`zsh -n` 문법 통과, rc 라운드트립(install→`bash -n` OK→uninstall이 사용자 라인 정확 복원).
- §31.1 수용 기준 충족: `--dry-run`/`--diff` 미수정, `--uninstall` 블록만 제거, hook 실패가 셸 중단 안 함. (cd/exit/git 실제 기록은 W4 스토리지 연동 후 — 현재 `__hook` no-op로 wiring만.)
- 검증: Windows 34개(lib 21 + bin 13)·Linux 37개(lib 24 + bin 13) 테스트 통과, clippy clean, fmt clean.

> 다음 단계: SQLite 스토리지(W4, §31.2) — `ai-terminal.db` + 락. 정책 `set` 영속화·hook 상태 기록의 선행조건.

## 2026-06-02 — PTY Terminal Core 착수 (M1/W2, WSL 검증)

- `src/pty.rs` 추가 (TDD): `run_in_pty(shell, command) -> PtyOutput{output, exit_code}` — portable-pty로 PTY를 열고 `shell -c command` 실행, 출력/종료코드 수집.
- 테스트는 `#[cfg(all(test, unix))]` — 실제 bash spawn이 필요해 **WSL(Ubuntu-Dev)** 에서 검증(`echo` 출력 포함, 종료코드 3 전파).
- 환경: WSL에 Linux Rust 툴체인 설치(rustup), 빌드는 `CARGO_TARGET_DIR=~/targets/ai-terminal`로 분리(/mnt/c 느림·Windows 산출물 충돌 회피). 소스는 `/mnt/c/...` 공유.
- 검증: Linux 21개(lib 14 incl pty 2 + bin 7)·Windows 19개(unix 테스트 제외) 통과, 양쪽 clippy clean, fmt clean. pty 모듈은 Windows(ConPTY)에서도 컴파일.

> 다음 단계: PTY 인터랙티브 세션 + 입출력 렌더링(W2 잔여) 또는 셸 Hook 생성/설치 UX(W3, §31.1).

## 2026-06-02 — 정책 엔진 + 프로파일 선구현 (W6, §31.3·§31.4)

- `src/policy.rs` 추가 (TDD): `PolicyProfile`(balanced 기본 / paranoid) 전체 필드(§31.3 권위값), `Decision`(Allow/Confirm/StrongConfirm/Block), `decide(level)` 액션 매핑(§31.4).
- 매핑: Critical→Block(두 프로파일), High→StrongConfirm(balanced)/Block(paranoid), Medium→Confirm, Low→Allow(balanced)/Confirm(paranoid).
- 위험 등급을 로컬 `risk::assess`에서 받으므로 "로컬 정책 우선"(§31.4)이 구조적으로 보장됨.
- CLI: `ai policy show [--profile]`, `ai risk --profile <p>`(결정 표시 추가). 미지원 프로파일은 명확히 오류.
- `set`(영속 변경)은 config 저장 모듈(W4) 구현 후로 보류.
- 검증: lib 12 + bin 7 = 19 테스트 통과, clippy clean, fmt clean.

> 다음 단계: WSL에서 M1 PTY/Hook 착수.

## 2026-06-02 — 위험도 스코어링 엔진 선구현 (W5, §31.4)

- `src/lib.rs` 라이브러리 크레이트 착수 + `src/risk.rs` 위험도 엔진 추가 (TDD, red-green-refactor).
- 0~100 rule-based 스코어링: 명령 유형 점수 → (액션 존재 시) 경로 가중치 최댓값 → 완화 요소. 등급 매핑 Low/Medium/High/Critical(§31.4).
- 결정성 보장(순수 함수). §31.4 "예시 분류" golden set 테스트로 고정: `ls -al`=Low … `rm -rf /`/`dd …=/dev/sda`=Critical, `chmod -R 777 .`/`curl|sh`/`sudo systemctl restart`=High.
- 순수 read-only 명령은 경로 가중치 미적용(`cat /etc/hostname`이 High로 오분류되지 않도록).
- `ai risk "<command>"` CLI 추가 — 점수·등급·요인(factor) 분해 출력(감사/설명용, RULES §2).
- 검증: lib 6 + bin 4 = 10 테스트 통과, clippy `-D warnings` clean, fmt clean.
- **순서 결정**: PTY(W2)·셸 Hook(W3)은 Linux 전용이라 Windows 개발 머신에서 검증이 어려워, 크로스플랫폼·결정성 보안 핵심인 위험도 엔진(W5)을 먼저 구현. 정책 엔진(W6)이 이 엔진에 의존한다.

> 다음 단계: 정책 엔진 + 프로파일(W6, §31.3) — balanced/paranoid에서 위험 등급별 액션(Critical 차단 등) 매핑. 또는 WSL 환경에서 M1 PTY/Hook 착수.

## 2026-06-02 — 구현 repo 부트스트랩 (M0)

- `../document/` 설계 정본(v3.3) 검토 완료.
- `docs/` working-set 5종 작성: PRD · TASK · WORKFLOW · HISTORY · RULES (한국어 압축형, 설계 repo §번호 참조).
- 기술 스택 확정: **Rust** (설계 1순위). ratatui · crossterm · tokio · portable-pty · serde/toml · clap · tracing · rusqlite.
- Rust 개발 환경 구성: `Cargo.toml` · `rust-toolchain.toml`(stable + rustfmt/clippy) · `rustfmt.toml` · `.editorconfig` · `.gitignore` · `config.toml.example` · `.github/workflows/ci.yml`.
- `ai` CLI 최소 골격(`src/main.rs`): clap 기반 `--version` / `doctor` 서브커맨드 (스켈레톤).
- `cargo build` / `cargo test` 검증 (개발 머신: Windows 11). Linux 전용 동작(PTY·샌드박스)은 추후 `#[cfg(target_os)]` 분기 + Linux CI에서 검증.

> 다음 단계: `docs/TASK.md` M1(W1) — Rust 워크스페이스/크레이트 구성 확정 및 5계층 아키텍처 합의.

---

## 채택된 핵심 설계 결정 (요약 — 정본은 설계 repo §0.2 / §30)

부트스트랩 시점에 확정되어 구현이 따르는 결정들. 상세 근거·대안은 정본 참조.

| 결정 | 채택안 | 정본 |
|---|---|---|
| 셸 통합 | **Hook 기반 기본 + Native Wrapper fallback** (rc 자동 수정 금지) | §29.1, §30-1, §31.1 |
| 저장 아키텍처 | **데몬 없음** — SQLite WAL `ai-terminal.db` + 파일 락 + stale cleanup | §30-2, §31.2 |
| 위험도 스케일 | **0~100 rule-based** (소가산 안 폐기), 로컬 정책 우선, AI는 보조 | §31.4 |
| 저장 DB 통일 | `history.db` → **`ai-terminal.db` 단일 스키마** | §0.2, §15.2 |
| 마스킹 | Secret/PII 기본 ON, **마스킹 실패 시 원격 AI 차단(fail-closed)** | §31.8 |
| 정책 프로파일 | **balanced(기본) + paranoid** 필수, poweruser/dev는 P2 | §31.3 |
| 자가 치유 | 자동 *분석/제안* 허용, 자동 *실행* 항상 금지 | §16.3 |
| 로컬 LLM | Phase 2로 이연 | §30-3 |
| 기술 스택 | **Rust** 1순위 (Go 대안) | §24.1 |

---

<!-- 새 항목 추가 시 이 위에 날짜 역순으로 기록. 형식:
## YYYY-MM-DD — <제목> (마일스톤)
- 변경/결정 요약 (왜 중심). 보안 관련은 위협/완화 명시.
-->
