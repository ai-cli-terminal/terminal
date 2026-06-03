# Changelog

이 프로젝트의 주요 변경 사항을 기록한다. 형식은 [Keep a Changelog](https://keepachangelog.com/ko/1.1.0/)를
따르고, 버전은 [SemVer](https://semver.org/lang/ko/)를 따른다. 분류는 Conventional Commits 기반이며
보안 변경은 별도 **Security** 섹션으로 묶는다(WORKFLOW §8.4).

## [Unreleased]

### Changed

- **통합 디스패처**: `dispatch::run`으로 셸/AI 경로 일원화. TUI 자연어 질의가 AI로 라우팅되고, CLI `ai dispatch "<input>"` 추가. async 게이트웨이를 동기 디스패처에 잇는 `GatewayResponder`(타임아웃/Ctrl+C) 도입, audit source를 `dispatch`/`exec`로 구분.

## [0.1.0] - 2026-06-03

Phase 1(MVP+) 로컬 결정성 코어 + Phase 2(Intelligent Workflow) 골격을 담은 첫 체크포인트
릴리즈. 실제 클라우드 provider 연동(HTTPS/async) 이전의 **로컬·결정성 기능 기준선**이다.
일부 실행 파이프라인 자동 연동(undo 자동 백업, usage 자동 기록, last-error 캡처, hook IPC
기록)은 후속(M1~M3 잔여)으로 남아 있다.

### Added

- **CLI 골격** (`ai`): clap 기반 `--version` / `doctor` / `doctor --guardrails` (M0).
- **위험도 엔진** (`risk`): 0~100 rule-based 결정성 스코어링 — 명령 유형 점수표 + 경로 가중치 +
  완화 요소, Low/Medium/High/Critical 등급 매핑, `ai risk "<cmd>"` 요인 분해 출력 (M2/W5, §31.4).
- **정책 엔진** (`policy`): `balanced`(기본)·`paranoid` 프로파일, 등급별 액션 매핑(Critical 차단 등),
  `ai policy show/set`, 활성 프로파일 영속화 (M2/W6, §31.3).
- **PTY 코어** (`pty`): portable-pty 기반 `run_in_pty` + 인터랙티브 `PtySession`(write/read/kill).
- **TUI** (`ui`): ratatui 상태바·히스토리·입력(실시간 위험도), Enter 제출 → PTY 실행 연결, `ai tui`.
- **셸 Hook 통합** (`shell`): `ai init shell`(`--dry-run`/`--diff`/`--uninstall`, rc 자동 수정 금지),
  `ai shell-hook bash|zsh`(preexec/precmd/chpwd, 셸 비중단 가드), 내부 `ai __hook` (M1/W3, §31.1).
- **SQLite 스토리지** (`store`, `--features storage`): `ai-terminal.db` WAL + 7테이블 DDL, 세션/명령/
  usage/audit CRUD, FK 강제 (M1/W4, §31.2).
- **2층 파일 락** (`lock`): advisory 락 + TTL + stale 판정/회수 + RAII 해제, `locks` 레지스트리 (M1/W4).
- **환각 검증** (`verify`): 바이너리 존재 검증(PATH/빌트인/PATHEXT), 미존재 시 UNKNOWN 표시 (M2/W8, §29.2).
- **AI 타임아웃/취소** (`aitask`): `Timeouts`(5/15/60/180s) + `run_cancellable` + Ctrl+C 취소 (M2/W8, §16.2).
- **Preview/Diff 분류** (`preview`): `classify_preview`, dry-run 제안(rsync/git clean/terraform/kubectl/helm),
  `ai preview "<cmd>"` (M3/W9, §31.5).
- **Undo** (`undo`): best-effort 파일 롤백 + metadata, 백업 상한(500MB/1000 files/20MB/TTL 7일),
  `ai undo last` (M3/W10, §31.6).
- **Usage/Cost** (`usage`): usage_event 기록 + 누적 집계 + 예산 평가(session $2/month $30, 80% warn/100% block),
  `ai usage` (M3/W11, §31.7).
- **에러 분석** (`explain`): 규칙 기반 분석(not found/permission/no such file/generic), `ai explain` (M3/W12).
- **컨텍스트 동기화** (`context`): `SessionContext`/`gather`, `is_context_changing`, env allowlist/denylist +
  PATH hash-only, `needs_refresh`, git branch 파싱, `ai context` (M4/W13, §31.10).
- **가드레일 baseline** (`guardrails`): baseline 목록 + 플랫폼 capability matrix + `detect`,
  `ai doctor --guardrails` (M4/W14, §31.11).
- **Provider 추상화** (`provider`, `tokenwin`): capability map + fallback(token/cost/streaming),
  토큰 추정/chunk/fits (M4/W15, §31.9).
- **Phase 2 골격**: AI Model Gateway(`gateway`, mask→token→backend→usage), Intent Classifier(`intent`),
  응답·시맨틱 캐시(`cache`, Jaccard), Ollama·OpenAI 백엔드(`http`/`ollama`/`openai`),
  Hybrid dispatcher(`dispatch`), Verification Agent(`verify_agent`), 스킬 관리(`skill`, §26),
  MCP 관리(`mcp`, §27), Semantic File Index(`index`), Tool Use Planner(`planner`).
- 통합 테스트(`tests/integration.rs`): 위험도 결정성(50회)·Critical 차단 100%·마스킹 무유출.
- MVP 진입 문서 `docs/MVP-ENTRY.md` (§31.12 9영역 + §31.13 확정값).

### Security

- **Secret/PII 마스킹** (`mask`): Secret(private key/AWS/GitHub/Slack/Bearer/Authorization/Password) +
  PII(이메일/IPv4/한국 주민번호/전화/신용카드/여권) 탐지, 파이프라인(Secret→PII→Masking→Validation→Eligibility),
  **private key fail-closed 차단**, 마스킹 실패 시 원격 전송 차단, `ai mask "<text>"` (M2/W7, §31.8).
- AI Model Gateway가 백엔드 전송 전 마스킹을 강제하고 private key 감지 시 fail-closed로 차단(Phase 2).
- 컨텍스트 수집 시 env denylist(TOKEN/SECRET/KEY/PASSWORD) + PATH hash-only로 secret 디스크 미저장.

### Fixed

- `shell::generated_hooks_pass_syntax_check` 테스트가 셸 바이너리(zsh 등) 미설치 환경에서
  spawn 실패로 panic하던 문제 — 부재 시 graceful skip으로 변경. CI에 zsh 설치 단계 추가해
  zsh hook 문법 검증 커버리지 유지.

### Notes

- 로컬 결정성 코어 기준선. **실제 클라우드 provider HTTP(S) 어댑터·async 결합·실행 파이프라인 자동
  연동은 미포함**(후속 M1~M3 잔여 / Phase 2 네트워크).
- 빌드: 기본 feature는 C 컴파일러 불필요(전 플랫폼), `storage`는 rusqlite(bundled) — Linux/WSL/CI 권장.

[Unreleased]: https://github.com/ai-cli-terminal/terminal/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ai-cli-terminal/terminal/releases/tag/v0.1.0
