# Changelog

이 프로젝트의 주요 변경 사항을 기록한다. 형식은 [Keep a Changelog](https://keepachangelog.com/ko/1.1.0/)를
따르고, 버전은 [SemVer](https://semver.org/lang/ko/)를 따른다. 분류는 Conventional Commits 기반이며
보안 변경은 별도 **Security** 섹션으로 묶는다(WORKFLOW §8.4).

## [Unreleased]

## [0.3.1] - 2026-06-28

Android and Windows packaging evidence patch release.

### Added

- Android direct APK/F-Droid release metadata: Fastlane descriptions, changelog, phone screenshots, fdroiddata draft, version mirror, and repository license files.
- Android release preflight scripts for F-Droid metadata lint/rewritemeta, F-Droid activation, local signing, and GitHub base64 signing secrets.
- Windows GUI/NSIS smoke evidence documentation and locked SQLite helper-file tolerance in GUI smoke binary scans.

### Changed

- GitHub release workflow now runs Android F-Droid release-input verification before packaging APK assets.

## [0.3.0] - 2026-06-27

독립 셸 `ash`를 **설정·외부 실행 안전 게이트·라인 에디터·history·자연어 AI**까지 결선한 기능 릴리스.

### Added

- **`ash` config 로딩**: `~/.config/ai-terminal/config.toml`의 `[general]`(`history_limit`·`default_shell`)·`[ai]`(`provider`/`model`/`ollama_url`/`openai_url`)를 fail-soft로 로드하고 `ai doctor`가 표시한다.
- **`ash` 외부 실행 안전 게이트**: 외부 명령이 risk→policy→preview→확인→undo 백업을 거쳐 실행된다. Critical 차단, High 확인(비-TTY는 fail-closed), 파일 변경은 undo 백업.
- **`ash` 라인 에디터(reedline)**: TTY에서 입력 편집·커서 이동·↑↓ history 회상·Ctrl-C(라인 취소)·Ctrl-D(EOF). 비-TTY는 라인 단위 입력으로 폴백.
- **`ash` 파일 영속 history**: `~/.config/ai-terminal/ash_history`에 세션 간 저장. secret/PII가 탐지된 명령은 저장에서 제외한다.
- **`ash` 자연어 AI 라우팅**: 자연어 입력(`ai <질문>`·`?`·의문사·한글 요청마커)을 AI로, 그 외는 구조화 셸로 분기. `[ai] provider`로 `ollama`(기본)·`openai`·`mock` 선택, OpenAI 키는 `OPENAI_API_KEY` 환경변수. AI 실패/타임아웃/취소는 fail-soft(셸 비중단).
- **`ash` Git Bash/MSYS bridge**: `AI_TERMINAL_WINDOWS_PROFILE=msys`(+`MSYSTEM` 존재) 시 외부 명령을 MSYS POSIX host(`sh -lc`)로 실행한다.
- **`ash` 게이트 audit 기록**: 게이트 결과를 storage에 기록한다(실행→`commands`, 차단/거부→`audit_events`). `shell_audit` 모듈로 `ai exec`와 단일화.
- **Android Termux T1 helper 브리지**(spike): opt-in shared staging 경유 외부 명령 stream/cancel.

### Fixed

- **Android JNI 빌드 회귀**: 데스크톱 전용 의존(`portable-pty`→`serial`→`termios`)이 android cdylib 빌드에 새던 문제를 타깃 게이트로 해결하고, 에뮬레이터 smoke 잡에 KVM 활성화를 추가했다.
- **flaky `ShellWorkerTest`**: 비스레드안전 이벤트 수집으로 인한 동시성 테스트 flakiness를 hardening했다.

## [0.2.4] - 2026-06-23

### Added

- **`ash` 릴리즈 asset**: Linux/Windows 릴리즈에 `ash-linux-x86_64`, `ash-windows-x86_64.exe`와 각 SHA256 checksum을 `ai` 바이너리와 함께 업로드한다.
- **설치 스크립트 `ash` 동시 설치**: `install.sh`와 `install.ps1`이 새 릴리즈에서는 `ai`와 `ash`를 함께 설치하고, 예전 릴리즈처럼 `ash` asset이 없는 경우에는 경고 후 `ai`만 설치한다.

### Fixed

- Windows CI의 `ash` 테이블 smoke가 출력 표의 헤더/행 배치를 너무 엄격하게 매칭하던 문제를 완화했다.

## [0.2.3] - 2026-06-23

### Added

- **독립 `ash` 셸 코어**: 값 모델, 렉서/파서, AST, REPL, 테이블 포매터, `print`/`cd`/`ls`/`get`/`first`/`length` 등 기본 builtin과 외부 실행 경로를 추가했다.
- **표현식과 `where` 파이프라인**: 비교 연산자, `and`/`or`/`not`, 행 조건 필터링을 지원해 구조화 데이터 파이프라인의 첫 실사용 흐름을 만들었다.
- **플랫폼 실행 경계**: `shellcore::external::ExternalRunner` 기반으로 순수 평가와 host process 실행을 분리하고, `Engine::pure()`로 모바일/PWA 임베딩에서 process spawn 없이 실행할 수 있게 했다.
- **Windows `ash.exe` 실행 해석**: `.exe` 직접 실행, PATHEXT 기반 `.cmd/.bat`, `.ps1` PowerShell 실행 target을 분리해 Windows 네이티브 실행을 예측 가능하게 했다.
- **Windows `ash.exe` smoke**: CI와 로컬 `scripts/smoke.ps1`에 구조화 셸 smoke와 `.cmd`/`.ps1` 외부 실행 검증을 추가했다.

### Changed

- 플랫폼 목표를 데스크톱과 모바일이 공유하는 로컬 `ash` 터미널로 재정렬하고, Android/iOS/PWA/Windows 목표 매트릭스와 workflow 문서를 추가했다.
- 계획, 핸드오프, PRD, 백로그, superpowers 문서의 사용자-facing 영문 설명과 라벨을 한글 문서 흐름으로 정리했다.

## [0.2.2] - 2026-06-05

### Added

- **Windows 더블클릭 실행 가드**: 탐색기에서 `ai.exe`를 더블클릭하면 콘솔이 즉시 닫혀
  "실행 안 됨"으로 오인되던 문제. `GetConsoleProcessList==1`(자기 콘솔 단독 점유)로 더블클릭을
  감지해 사용법 안내를 보여주고 Enter 입력까지 창을 유지한다. 터미널 실행(부모 셸 attach,
  count≥2)에는 영향 없음 — CI/스크립트 비행. Windows 전용, 새 의존성 없음(kernel32 extern).

## [0.2.1] - 2026-06-05

### Fixed

- **Windows 바이너리 실행 실패 수정**: 깨끗한 Windows(VC++ 재배포 패키지 미설치)에서 `ai.exe`가
  `VCRUNTIME140.dll` 부재로 실행 실패(`0xC0000135`)하던 문제. MSVC 타깃에 CRT 정적 링크
  (`.cargo/config.toml` `crt-static`)로 self-contained 빌드 → 런타임 의존 제거. 근본 원인은
  CI(`windows-latest`=MSVC) 빌드가 CRT를 동적 링크했고 로컬 검증이 GNU 툴체인이라 미검출된 것.
  CI에 Windows 빌드 잡 + `VCRUNTIME140` 의존 없음 가드, 릴리즈에 self-contained 가드 추가(재발 방지).

## [0.2.0] - 2026-06-05

Phase 1(MVP+) + Phase 2(Intelligent Workflow) 코어에 더해, 원격 승인 기반(M0~M1 slice 4a:
셸 인터셉트·Noise 와이어 프로토콜·로컬 게이트 데몬·승인 검증·세션 전송 substrate)을 담은
**첫 배포 가능 릴리즈**. Linux x86_64 + Windows 네이티브 바이너리 + SHA256 체크섬 제공.

### Added

- **통합 디스패처** (`dispatch::run`): 셸/AI 경로 일원화, TUI 자연어 질의 AI 라우팅, `ai dispatch`.
- **원격 승인 기반(remote, feature)**: 셸 인터셉트 제어점(`gate`), Noise_XX/Ed25519 크립토 코어
  (`remote`), 로컬 게이트 데몬(`daemon`), 승인 검증 상태머신·nonce 저장소(`approval`),
  Noise 세션 승인 왕복·전송 substrate(`session`). (M0~M1 slice 4a — 디바이스/PWA는 RA에서 후속)
- **배포**: feature 매트릭스 빌드 스크립트, `install.sh`/`install.ps1`, 태그 기반 릴리즈 CI(SHA256).

### 비고

- 빌드: `default`·`remote`는 C-free(전 플랫폼), `storage`(SQLite)·`tls`(ring)는 C 툴체인 필요
  (Linux/WSL/CI 또는 MSVC). 실폰 원격 승인(디바이스 리스너·페어링·PWA)은 RA 마일스톤 후속.

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
- **미리보기/Diff 분류** (`preview`): `classify_preview`, dry-run 제안(rsync/git clean/terraform/kubectl/helm),
  `ai preview "<cmd>"` (M3/W9, §31.5).
- **실행 취소** (`undo`): best-effort 파일 롤백 + metadata, 백업 상한(500MB/1000 files/20MB/TTL 7일),
  `ai undo last` (M3/W10, §31.6).
- **사용량/비용** (`usage`): usage_event 기록 + 누적 집계 + 예산 평가(session $2/month $30, 80% warn/100% block),
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

### 비고

- 로컬 결정성 코어 기준선. **실제 클라우드 provider HTTP(S) 어댑터·async 결합·실행 파이프라인 자동
  연동은 미포함**(후속 M1~M3 잔여 / Phase 2 네트워크).
- 빌드: 기본 feature는 C 컴파일러 불필요(전 플랫폼), `storage`는 rusqlite(bundled) — Linux/WSL/CI 권장.

[Unreleased]: https://github.com/ai-cli-terminal/terminal/compare/v0.2.4...HEAD
[0.2.4]: https://github.com/ai-cli-terminal/terminal/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/ai-cli-terminal/terminal/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/ai-cli-terminal/terminal/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/ai-cli-terminal/terminal/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/ai-cli-terminal/terminal/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ai-cli-terminal/terminal/releases/tag/v0.1.0
