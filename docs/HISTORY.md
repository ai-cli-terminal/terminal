# HISTORY — 변경 / 결정 로그

> **정본**: 설계 결정의 권위 기록은 `../document/`(특히 `00-overview-architecture.md` §0.2 불일치 해소, `03_프로젝트_아키텍처_정의서.md` ADR, `05-roadmap-enhancements-decisions.md` §30 결정안).
> 본 문서는 **구현 repo(`terminal/`)의 변경·결정 타임라인**이다. 최신 항목이 위로 온다.

---

## 2026-06-04 — FU-1 리팩터 부채: 캐시 용량 상한 + `cmdparse` 공용화

- **캐시 LRU**(`cache.rs`): `ResponseCache`(HashMap)·`SemanticCache`(Vec) 둘 다 무한 증가(line 111 TODO)였다 → `DEFAULT_CACHE_CAPACITY=1024` + `with_capacity`. `put` 시 용량 초과면 가장 오래된(삽입 시각 최소) 항목 축출(Semantic은 만료 정리 후 앞쪽 제거). 장기 세션 메모리·선형 탐색 비용 제어.
- **`cmdparse` 공용화**(`cmdparse.rs` 신규): `program_token`이 preview·pipeline에 중복, 래퍼 스킵(`sudo|doas|env|nohup|nice`+`VAR=`)이 verify/risk/preview/pipeline에 흩어져 있었다 → `is_wrapper_token`/`is_env_assignment`/`program_token`/`args_after_program` 단일 진실원. preview(`program_token`·`path_args`·`extract_targets`)·pipeline(`program_token`·`candidate_paths`)·verify(`extract_program`)가 위임. 동작 보존 리팩터.
- 검증: cmdparse 단위(래퍼/환경 스킵), 캐시 축출 단위(용량 초과 시 oldest 축출·기존 키 갱신 무축출). default+storage 전체 통과(기존 테스트 무회귀), clippy/fmt clean.
- 계획: `docs/superpowers/plans/2026-06-04-phase2-followups.md` FU-1.

## 2026-06-04 — WI-5 TUI mid-exec 중단 + 라이브 스트리밍 (Phase 1 실사용 갭, §5/§31.5/§16.2)

- **배경**: TUI(`ui::run`)는 Submit 시 `dispatch::run`을 동기 블로킹 실행해 (1) 장기 명령 출력이 라이브로 안 보이고 (2) 실행 중 중단 불가했다. CLI `run_in_pty_streaming`은 프로세스 전역 ctrl_c라 raw-mode TUI(Ctrl+C=KeyEvent)에 부적합.
- **pty**(`pty.rs`): `run_in_pty_streaming_cancellable(shell, cmd, cancel: Arc<AtomicBool>, on_chunk)` 추가 — `child.clone_killer()`로 워처 스레드가 `cancel`을 20ms 폴링해 kill(출력 없는 silent 명령도 중단). 취소 시 130, 아니면 자식 exit.
- **ui**(`ui.rs`): Submit을 메인에서 `dispatch::dispatch`로 분류 → **셸만 `std::thread::scope` 워커**에서 `pipeline::execute`(게이트+취소 실행) 수행, `ChannelSink`로 청크를 메인에 송신. 메인 루프가 `try_recv`로 라이브 표시 + `event::poll(20ms)`로 Esc/Ctrl+C 중단 요청. **AI는 메인 스레드 동기**(GatewayResponder Send 비보장 회피, 요청 타임아웃이 상한). `render_shell_tail`로 완료 후 상태 꼬리만 append(라이브 출력 이중표시 방지).
- **위협/완화**: hook·셸 비중단 유지(워커 패닉/실패는 안내 후 루프 지속). 중단은 자식 프로세스 kill(고아 없음). AI 차단/타임아웃은 기존 경로 유지.
- 검증: pty 단위(취소→130 즉시 kill·정상→출력+exit, WSL), `render_shell_tail` 단위(Ran0/130/N·Blocked·Declined·BackupRefused). WSL 전체(lib 221) 통과, Windows default+storage 통과, clippy/fmt clean. TUI 루프 자체는 기존 `run`과 동일하게 단위 테스트 비대상.
- 설계: `docs/superpowers/specs/2026-06-04-tui-mid-exec-cancel-design.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-5).

## 2026-06-04 — WI-4 Native Wrapper fallback: 통합 모드 감지·표시 (Phase 1 실사용 갭, §30-1/§29.1)

- **배경**: §30-1 확정은 "Hook 기본 + Native Wrapper fallback"이나, hook 가용성 감지나 fallback 인지가 전혀 없었다.
- **조사**: `ai exec`/`ai dispatch`의 `Ran` 경로는 이미 `record_exec`로 명령+cwd+exit를 `commands`에 기록한다 → **wrapper 모드의 데이터 수집은 이미 기능**. 별도 wrapper 기록 신설은 중복(over-design)이라 추가하지 않음.
- **shell**(`shell.rs`): `ConfiguredMode{Hook,Wrapper,Auto}` + `IntegrationMode{Hook,Wrapper}` + `resolve_integration_mode`(Auto→hook 활성이면 Hook, 아니면 Wrapper). `hook_active(env_get)`가 `AI_TERMINAL_HOOK` 마커 확인(DI 테스트). bash/zsh hook이 `export AI_TERMINAL_HOOK=1` 설정.
- **doctor**(`main.rs`): `ai doctor`가 유효 통합 모드(hook 활성 / wrapper fallback) 표시 + wrapper 시 `ai exec` 사용·`ai init shell` 설치 안내.
- **범위 결정**: 영속 PTY 셸 런처(프롬프트 파싱·probe 동기화)는 무겁고 `ai tui`/WI-5와 중복 → Phase 2 이연. 심층 hook-health(활성 셸 수 등)는 T-RA5 이연.
- 검증: 단위(모드 해석 4케이스·hook_active·마커 export), `bash -n`/`zsh -n`(WSL 24 tests), doctor 양 모드 스모크(Windows: 마커 유무로 hook/wrapper 전환). default+storage 전체 통과, clippy/fmt clean.
- 설계: `docs/superpowers/specs/2026-06-04-wrapper-fallback-design.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-4).

## 2026-06-04 — WI-3 bash cwd hook 연동 (chpwd 에뮬레이션, Phase 1 실사용 갭, §31.1/§31.10)

- **배경**: zsh는 native `chpwd`로 디렉터리 변경 시 세션 cwd·git branch를 갱신하나, bash는 native chpwd가 없어 `cd`/`git switch` 후 컨텍스트가 갱신되지 않았다. bash `precmd` 핸들러는 `exit=`만 처리하고 받은 `cwd=`를 무시했다.
- **shell**(`shell.rs` BASH_HOOK): 셸 변수 `__ai_last_pwd`로 직전 PWD 보관. precmd에서 `$PWD != $__ai_last_pwd`이면 변수 갱신 후 `ai __hook chpwd "cwd=$PWD"` 호출(zsh와 동일 이벤트로 수렴 → `record_hook_chpwd` 핸들러 재사용, Rust 측 무변경). 초기값 빈 문자열 → 첫 prompt에서 초기 cwd 1회 기록.
- **불변식 유지**: `command -v ai` 가드 + `>/dev/null 2>&1 || true`, precmd가 `local __ai_ec=$?` 캡처 후 `return $__ai_ec`로 종료 코드 보존(chpwd 에뮬레이션은 그 사이에서 수행).
- 검증: 단위(BASH_HOOK가 `__ai_last_pwd`·`__hook chpwd`·`return $__ai_ec` 포함), `bash -n` 문법(WSL), **WSL e2e**: hook source 후 `cd` 2회→세션 cwd가 마지막 디렉터리로 갱신·context_snapshots에 chpwd 2건. default+storage 전체 통과, clippy/fmt clean.
- 설계: `docs/superpowers/specs/2026-06-04-bash-cwd-hook-design.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-3).

## 2026-06-04 — WI-2 `.env`/민감 경로 컨텍스트 제외 가드 (Phase 1 실사용 갭, §31.8)

- **배경**: `mask::is_sensitive_path`는 있으나 컨텍스트 경계에서 미사용. 현재 `context::gather`는 파일 본문을 수집하지 않지만, Phase 2 파일 본문 수집기 추가 시 `.env`/`.pem` 본문이 원격 AI로 유출될 면이 열린다(§31.8 미보장).
- **context**(`context.rs`): `allow_file_in_context(path)`(민감 경로면 false) + `filter_context_paths(paths)`(민감 경로 제거·순서 보존) 추가. 패턴은 `mask::is_sensitive_path` 단일 진실원에 위임.
- **계약**: 향후 파일 본문 수집기는 원격 전송 전 반드시 이 게이트를 통과 → 경로 게이트(1차) + 본문 마스킹(기존, 2차)의 이중 방어. fail-closed.
- **위협/완화**: `.env`/`*.pem`/`*.key`/`id_rsa`/`credentials` 본문의 원격 노출 차단. 경로 기준 결정적 제외.
- 검증: 단위(민감 경로 제외·일반 소스 포함·필터 순서 보존). default+storage 전체 통과, clippy/fmt clean.
- 설계: `docs/superpowers/specs/2026-06-04-context-sensitive-path-guard-design.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-2).

## 2026-06-04 — WI-1 Gateway 예산 게이트 + estimated 비용 (Phase 1 실사용 갭, §31.7)

- **배경**: `gateway::ask`가 백엔드(원격 AI) 호출 전 예산을 평가하지 않았고, `ai ask`는 비용을 `0.0`으로 하드코딩해 지출이 누적되지 않았다(§31.7 미충족). `usage::evaluate`는 순수 함수로 존재했으나 미연결.
- **usage**(`usage.rs`): `estimate_cost(input, output)` 추가 — per-token 단가 테이블로 비용 추정, 항상 `CostSource::Estimated`(provider 미보고 표시).
- **gateway**(`gateway.rs`): `BudgetSnapshot{spent_usd, cfg}` + `Gateway::with_budget(spent, cfg)`(주입식 — 게이트웨이는 storage 비의존). `ask`에서 **exact·semantic 캐시 미스 이후·`backend.generate()` 직전**에 `evaluate` 평가 → Block 임계 시 `Blocked("예산 초과 …")`. 캐시 히트·로컬 결과는 원격 비용이 없어 위에서 이미 통과(예산 무관).
- **cli**(`main.rs`): `ai ask`가 storage 시 `total_cost(None)`를 읽어 `with_budget` 주입. 응답 비용은 *원격 호출 시에만*(캐시 히트·ollama 로컬=$0) `estimate_cost`로 기록(0.0 하드코딩 제거)+`(cost ~ $X estimated)` 배지.
- **위협/완화**: 예산 초과 시 원격 전송 차단(fail-closed 비용 통제). 캐시/로컬은 비용 0이라 차단되지 않아 가용성 보존. 게이트는 *원격 호출 직전*에만 평가해 캐시된 답의 가용성을 해치지 않음.
- 검증: usage 단위(estimate_cost 양수·estimated·스케일), gateway 단위(초과→차단·백엔드 미호출, 캐시 히트 바이패스, 정상), storage 통합테스트(지출 $2 초과→`ask` 차단). default+storage 전체 통과, clippy/fmt clean. `ai ask` 런타임 배지 확인.
- 설계/계획: `docs/superpowers/specs/2026-06-04-gateway-budget-gate-design.md`, `docs/superpowers/plans/2026-06-04-gateway-budget-gate.md`(상위: `plans/2026-06-04-phase1-usability-gaps.md` WI-1).

## 2026-06-03 — W9 안전(실행 없는) 실제 미리보기: unified diff + content-at-risk

- **diff**(`diff.rs` 신규): 순수 LCS 라인 unified diff(`unified_diff`, 외부 의존성 없음).
- **preview**(`preview.rs`): `render_preview`/`PreviewRender` 추가 — cp/mv 덮어쓰기(dst 기존)는 read-only로 진짜 unified diff, rm/shred/unlink·`> file` truncate는 content-at-risk(행·바이트·head). sed -i/perl -i/formatter 등 실행 필요 diff는 보류(샌드박스 후속). 크기 상한(diff 64KiB/risk 1MiB)·비UTF8 lossy·미존재/디렉터리 안전 처리. **대상 파일 절대 미수정**(e2e 확인).
- **cli**(`main.rs`): `ai preview`가 실제 diff/content-at-risk 출력(기존 분류 메시지는 Info로 유지).
- 검증: diff 단위(추가/삭제/변경/엣지), preview 단위(temp 파일: cp diff·rm risk·redirect·sed 보류·미존재), WSL e2e(dst 미수정 확인). clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-safe-preview-render-design.md`, `docs/superpowers/plans/2026-06-03-safe-preview-render.md`.

## 2026-06-03 — PTY 출력 스트리밍 + CLI Ctrl+C 중단 (W2 완료)

- **pty**(`pty.rs`): `run_in_pty_streaming` 추가 — 리더 스레드가 PTY를 블로킹 read해 bounded `tokio::mpsc`(cap 64)로 보내고(backpressure), current-thread 런타임이 `select!{ recv, ctrl_c }`로 청크를 `on_chunk`에 흘리며 Ctrl+C 시 자식 kill·버퍼 드레인·exit 130. 기존 `run_in_pty`/`PtySession` 유지.
- **pipeline**(`pipeline.rs`): `PtyExecutor::run`을 `run_in_pty_streaming(..|c| sink.write(c))`로 제자리 교체 → `ai exec`/`ai dispatch`/TUI 3경로가 라이브 스트리밍·CLI 중단 자동 적용. 트레이트 시그니처 불변.
- 검증: pty 단위 테스트(스트리밍 누적·종료코드 전파), WSL e2e(printf 라이브 출력·exit 전파·`sleep` SIGINT 즉시 중단). clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-pty-streaming-cancel-design.md`, `docs/superpowers/plans/2026-06-03-pty-streaming-cancel.md`.

## 2026-06-03 — gateway 시맨틱 캐시 2차 조회 결합

- **gateway**(`gateway.rs`): `ask`가 exact 캐시 미스 후 `SemanticCache::get_similar`(TTL 24h, Jaccard 임계값 0.85) 2차 조회. 시맨틱 히트는 그 답을 exact 캐시에 승격 저장(다음 동일 프롬프트는 exact 히트). 백엔드 응답은 exact+semantic 양쪽 저장(await 후 시각으로 TTL 기록). 시맨틱 키도 마스킹된 텍스트(RULES §2).
- **cache source 플래그**(`cache.rs`): `CacheSource { Backend, Exact, Semantic }`를 `GatewayOutcome::Answered`→`AiOutcome::Answered`로 전파. `ai ask`/`ai dispatch`가 캐시 히트 시 배지(`[cache: exact]`/`[cache: semantic ~근사]`) 표시.
- 검증: gateway 단위 테스트(시맨틱 히트→exact 승격, source 계층), `cache_badge` 라벨 테스트. clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-gateway-semantic-cache-design.md`, `docs/superpowers/plans/2026-06-03-gateway-semantic-cache.md`.

## 2026-06-03 — 비-Ran 명령 결과 audit 기록 (run_exec/run_dispatch, storage feature)

- **CLI**(`main.rs`): `shell_outcome_audit`(순수 매퍼 — 비-Ran `ExecOutcome`→`Option<AuditRecord>` 변환, 단위 테스트 가능) + `finish_shell_outcome`(공용 발산 헬퍼 — audit 기록 + 안내 후 `process::exit`) 추출. `run_exec`/`run_dispatch` Shell arm이 이 헬퍼를 공유하도록 중복 제거. `pipeline.rs`는 storage-free 유지(기록은 호출측).
- **audit 기록**: `Blocked`→`command_blocked`(Critical), `Declined`→`command_declined`(High 등 실제 등급 재산출), `BackupRefused`→`command_backup_refused`(해당 등급) — 마스킹된 명령(`mask::Masker::baseline().mask(...)`) 포함, `serde_json` payload(`command`/`source`/`factors`|`reason`). `Ran`은 기존 `record_exec`/`command_executed` 경로 유지(변경 없음).
- **storage 게이팅**: `record_outcome_audit`가 `#[cfg(feature = "storage")]` 게이트 안에서만 활성화 — 기본 빌드 C-free 유지.
- 검증: 단위 테스트 5개(Ran→None, 각 비-Ran 타입/level, BackupRefused reason, 마스킹 무유출). WSL e2e — `rm -rf /` → `('command_blocked','Critical','{…"command"…}')` 행 확인; `sudo systemctl restart nginx` + `n` 입력 → `('command_declined','High')` 행 확인. clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-audit-non-ran-outcomes-design.md`, `docs/superpowers/plans/2026-06-03-audit-non-ran-outcomes.md`.

## 2026-06-03 — Shell/Ai 단일 디스패처 통합

- **dispatch**(`dispatch.rs`): `run` 오케스트레이터 추가 — 입력 intent를 판정해 셸 경로(위험도→정책→preview→백업→실행 `pipeline`)와 AI 경로(주입된 `AiResponder`)로 라우팅한다. 셸/AI 양쪽 진입점을 하나로 일원화.
- **GatewayResponder**(lib 모듈, 신규): async `Gateway`를 동기 디스패처에 연결하는 브리지 — 내부 런타임에서 `ask_cancellable`을 구동해 타임아웃 + Ctrl+C 취소를 적용하고 동기 `AiResponder` 인터페이스로 노출. AI 경로 실패는 셸을 깨지 않음(graceful).
- **TUI**(`ui.rs`): Submit(Enter)를 `pipeline` 직접 호출 대신 `dispatch::run`을 거치도록 재배선 — 자연어 질의가 이제 AI 경로로 라우팅된다(명령은 셸 경로 유지).
- **CLI**(`main.rs`): `ai dispatch "<input>"` 원샷 명령 추가 — 디스패처를 직접 호출(셸/AI 자동 라우팅). audit 기록은 source를 "dispatch"와 "exec"로 구분.
- 설계/계획: `docs/superpowers/specs/2026-06-03-unified-dispatcher-design.md`, `docs/superpowers/plans/2026-06-03-unified-dispatcher.md`.
- 검증: 전체 테스트 default/storage/`storage tls` 모두 통과(0 failed; storage tls 합산 217), WSL e2e(셸 `echo` exit 0 / AI mock echo `(tokens ~ in:.. out:..)` exit 0 / `rm -rf /` Critical 차단 exit 1), fmt·clippy(`storage tls`, `-D warnings`) clean.

## 2026-06-03 — 그룹 C 백로그: 리다이렉트 인식 백업 대상 (W10 보완)

- **pipeline**(`pipeline.rs`): `strip_redirect_op`/`redirect_targets` 추가 — 셸 리다이렉트(`>f`/`>>f`/`N>f`/`&>f`/`> f`) 대상을 추출. `backup_targets`가 (삭제/덮어쓰기 프로그램 인자 ∪ 리다이렉트 대상)을 dedup 후 기존 일반 파일만 백업. `command.contains('>')` 거친 트리거 제거 → 붙은 `>out.txt`도 정확히 백업.
- **이유**: 기존엔 `echo x >out.txt`의 대상이 `is_file(">out.txt")`로 걸러져 덮어쓰기 전 백업이 안 됨(조용한 갭). 리뷰 LOW 보완.
- **한계**: 공백 분리된 인용 내 `>`(`echo "a > b"`)는 여전히 오인 가능하나 `is_file` 필터로 무해. 완전 정확성은 shell-words 토크나이저 영역 — 이연.
- 검증: TDD(strip_redirect_op/redirect_targets 단위 + backup_targets 통합 4), WSL e2e(`echo > f` 덮어쓰기→백업→`undo last` 복구). pipeline 11 + 전체 통과, clippy(default+storage)·fmt clean.

## 2026-06-03 — 그룹 C: 중앙 실행 파이프라인 (W10/W11/W2 키스톤)

- **pipeline**(`pipeline.rs`, 신규): `execute`가 위험도→정책(Block/Confirm)→preview→undo 백업(W10 자동 트리거, Refused 시 실행 중단)→실행→결과를 묶는다. I/O는 `Executor`/`Confirmer`/`OutputSink` 트레이트로 주입(PTY 없이 단위 테스트). `PtyExecutor`가 `run_in_pty` 래핑 — 청크 sink 모양이 W2 스트리밍을 수용(후속에 impl 교체).
- **CLI**(`main.rs`): `ai exec "<cmd>" [--yes] [--profile]` — stdin y/N 확인(`--yes`로 생략, Block은 우회 불가), 종료코드 전파. storage 시 명령+종료코드+audit 기록.
- **TUI**(`ui.rs`): Enter가 `run_in_pty` 직접 호출 대신 `pipeline::execute`를 거친다. 이번 증분은 위험(확인 필요) 명령을 거부+안내, Allow 명령은 실행.
- **백업 범위**: 삭제(rm/unlink/shred)·덮어쓰기/in-place(sed -i, `>`, cp/mv/tee/touch)의 기존 일반 파일만. 권한 변경(chmod/chown)은 내용 백업 무의미로 제외(한계 고지). W11은 셸 경로 토큰비용 없음 → AI 경로 기존 기록 재사용.
- 검증: TDD(pipeline 7: Allow/Block/Declined/Confirmed/백업생성/백업거부중단/종료코드), `ai exec` WSL e2e(rm 백업→undo 복구, `rm -rf /` 차단 exit 1). storage/default 통과, clippy(default+storage)·fmt clean.
- **후속**: W2 실제 async 스트리밍, W9 실제 diff, Shell/Ai 단일 dispatcher 통합, TUI 인라인 확인 모달.

## 2026-06-03 — 그룹 C 2b: HTTPS(TLS) transport (`tls` feature, Phase 2)

- **http**(`http.rs`): scheme 인식 `parse_url`(http/https) + `host_header`(기본 포트 생략) + 요청/응답 헬퍼 추출. `TcpTransport`가 스킴에 따라 평문/TLS로 분기.
- **TLS**(`#[cfg(feature = "tls")]`): `tokio-rustls`(ring) + `webpki-roots`로 `post_json_tls` — `RootCertStore` + `ClientConfig::builder_with_provider(ring)` + `TlsConnector`. tls 미빌드 시 https는 명확히 거부(조용한 실패 금지).
- **Cargo.toml**: `tls` feature + optional `tokio-rustls`(default-features off, ring/logging/tls12) + `webpki-roots`. rustls crypto provider가 C 툴체인을 요구하므로 `storage`처럼 게이트 → **기본 빌드 C-free 유지**.
- **CI**: `--features tls` clippy + `storage tls` build 추가. **README**: feature 빌드 안내.
- 검증: 단위(parse_url http/https·host_header·build_request), **실제 TLS e2e**(`ai ask --backend ollama --ollama-url https://postman-echo.com/post` → TLS 핸드셰이크+HTTPS 왕복 성공으로 JSON 수신; tls 없는 빌드는 거부). tls/default/storage 모두 144 통과, 양쪽 clippy clean.

## 2026-06-03 — 그룹 C 2a: 진짜 async transport + AI 경로 async 전환 (Phase 2)

- **http**(`http.rs`): `HttpTransport`를 async 트레이트(AFIT)로, `TcpTransport`를 `tokio::net::TcpStream` 기반 **비동기 평문 HTTP/1.1**로 전환. 진짜 async I/O라 상위에서 future drop(타임아웃/취소) 시 연결도 함께 취소(고아 호출 없음).
- **gateway**(`gateway.rs`): `LlmBackend`를 dyn 호환 async(박싱 future `GenerateFuture`)로, `Gateway::ask`를 async로. `ask_cancellable`이 `spawn_blocking` 없이 `run_cancellable(self.ask(...))`로 단순화 — #5의 워커-스레드 우회 제거.
- **backends**(`ollama.rs`/`openai.rs`): async generate(transport await). **Send 바운드 제거**(current-thread `block_on` 구동이라 불필요) → 새 의존성 0, C-free 유지.
- **main**(`main.rs`): ask 핸들러가 async gateway를 직접 await(Arc 불필요).
- 검증: 테스트 async 전환(gateway/ollama/openai #[tokio::test]), **로컬 mock HTTP 서버 e2e**(`ai ask --backend ollama` → tokio async TCP 실연결·응답 파싱). storage 158 / default 141 통과, fmt·clippy clean.
- **다음(2b)**: `tls` feature 게이트로 `tokio-rustls`(ring) + `webpki-roots` → HTTPS 클라우드 provider. 기본 빌드는 C-free 유지(rustls crypto provider가 C 툴체인 요구하므로 게이트).

## 2026-06-03 — 그룹 C 착수: AI 게이트웨이 타임아웃/취소 결합 (Phase 2, §16.2)

- **gateway**(`gateway.rs`): `Gateway` 스레드 안전화(`RefCell→Mutex`, `LlmBackend: Send+Sync`) + `ask_cancellable`(async) 추가 — 동기 `ask`를 `spawn_blocking`으로 옮겨 `aitask::run_cancellable`(타임아웃 + Ctrl+C)로 감싼다. 캐시 락은 백엔드 호출 전 해제.
- **http**(`http.rs`): `HttpTransport: Send+Sync`(transport를 워커 스레드로 이동 가능하게).
- **main**(`main.rs`): `ai ask`가 current-thread tokio 런타임에서 `ask_cancellable` 실행 + `cancel_on_ctrl_c`. 실패·타임아웃·취소 모두 graceful 고지(exit 0, §16.2).
- 검증: TDD(gateway: 느린 백엔드 타임아웃 / 정상 응답 통과 / 캐시; mock transport `RefCell→Mutex`), `ai ask` e2e(echo + 마스킹 유지). storage 158 / default 141 통과, fmt·clippy clean.
- **한계/다음**: 동기 백엔드는 타임아웃 시 호출자만 제어 복귀(고아 호출은 백그라운드 종료). 진짜 async transport(tokio TcpStream/TLS)·gateway 시맨틱 캐시 2차 조회는 다음 증분.

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
