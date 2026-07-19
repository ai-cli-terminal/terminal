# Design: desktop 프론트엔드 테스트 하네스

- **날짜**: 2026-07-19
- **상태**: 승인됨 (구현 대기)
- **브랜치**: `feat/desktop-frontend-test-harness` (develop 분기)
- **접근법**: C — 하이브리드 (happy-dom 인프라 + 표적 mock, 소스 변경 최소)

## 1. 배경 & 문제

`desktop/src`(TypeScript 프론트엔드, ~2,300줄)에 **자동 검증 장치가 전무**하다. 단위 테스트 파일이
0개이고 `desktop/package.json`에 test 스크립트도 없다. `HANDOFF.md`는 이 상태를 "CI에 TS 검사 스텝
없음 → 로컬이 authoritative"라고 명시한다. 즉 프론트엔드 로직의 회귀는 로컬에서 개발자가 `tsc`/`vite build`를
수동 실행할 때만 잡히고, 타입 검사만으로는 잡히지 않는 **런타임 분기 로직**(런치키 계산, 런타임 요약 문자열,
직렬화 타입 가드)의 회귀는 아무 안전망이 없다.

이 로직은 windows-wsl-parity **G1 T5**(PowerShell 런타임 페인 프론트)에서 최근 확장되었고, 향후 GT4(gated
GUI 입력창)에서 더 건드려질 예정이라 회귀 방지 가치가 크다.

## 2. 목표 / 비목표

**목표**
- `desktop/src`에 vitest 기반 단위 테스트 하네스를 도입한다.
- T5에서 도입/확장된 **런타임 순수 로직**에 단위 테스트를 건다(아래 §5).
- 프론트 테스트를 **CI authoritative**로 승격한다(로컬 전용 → CI 게이트).
- 이 host(WSL/Node v24)에서 검증까지 완결한다. desktop Rust 백엔드(`src-tauri`)는 MSVC 링커 부재로
  이 host에서 빌드 불가하므로 **대상에서 제외**한다 — 순수 TypeScript만 다룬다.

**비목표 (YAGNI)**
- DOM 글루 함수(`updateAptActions`, `renderRuntimeInventory`, `installUbuntuRuntime` 등 버튼
  `disabled`/`textContent`를 세팅하거나 `invoke()`로 Tauri 커맨드를 호출하는 함수)는 **테스트하지 않는다**.
  로직이 얕고(주로 상태→UI 반영) mock 비용 대비 가치가 낮다.
- E2E/통합 테스트, xterm 실제 렌더링 테스트, Playwright 류 브라우저 구동은 하지 않는다.
- 기존 소스의 리팩토링(순수 로직 추출 등)은 하지 않는다. 공유 워킹트리라 이동 편집은 병렬 세션과
  머지 충돌 위험이 크다. 신규 파일 위주로 간다.

## 3. 핵심 설계 제약

테스트 가치가 있는 순수 함수들이 **모듈 로드 시점 DOM 부작용**과 얽혀 있다:

- `app_context.ts`는 로드 즉시 `document.querySelector<...>("#terminal")` 등 20여 개를 실행한다.
  DOM이 없으면 `null`을 반환할 뿐 throw하지는 않는다.
- `pane_session.ts:112`는 로드 즉시 `primarySession = createPaneSession("pane-1", terminalRoot)`를
  실행한다. 이는 xterm `new Terminal()`을 실제로 생성하며, `#terminal` 요소/캔버스 환경이 없으면 **throw할 수
  있다**.

따라서 대상 함수(대부분 `pane_session.ts`/`layout.ts`/`runtimes.ts`에 있음)를 import하는 순간 이 부작용이
발동한다. 접근법 C는 이를 다음으로 해소한다:

1. **happy-dom 환경** — `document`/`window`/`localStorage`를 제공해 querySelector 부작용을 무해화.
2. **최소 DOM fixture** — `app_context.ts`가 찾는 element ID들과 `#terminal`을 셋업 파일에서 주입.
3. **표적 mock** — `@xterm/xterm`과 `@tauri-apps/api/core`를 `vi.mock`으로 대체해, 모듈 로드 시
   throw하는 경계(xterm Terminal 생성)와 IPC 호출을 격리.

## 4. 하네스 구성 (신규 파일 위주)

| 파일 | 종류 | 내용 |
|---|---|---|
| `desktop/package.json` | 수정 | devDep `vitest`, `happy-dom` 추가. scripts에 `"test": "vitest run"`, `"test:watch": "vitest"` |
| `desktop/vitest.config.ts` | 신규 | `test.environment = "happy-dom"`, `test.setupFiles = ["./test/setup.ts"]`, `test.include = ["test/**/*.test.ts"]` |
| `desktop/test/setup.ts` | 신규 | (a) 최소 DOM fixture 주입(app_context가 querySelect하는 ID + `#terminal`), (b) `vi.mock("@xterm/xterm")`, (c) `vi.mock("@tauri-apps/api/core")`로 `invoke` 스텁 |
| `desktop/tsconfig.json` | 수정 | `include`에 `"test"` 추가(vitest 글로벌/셋업 타입 인식). 필요 시 `types: ["vitest/globals"]` |
| `desktop/test/*.test.ts` | 신규 | §5 테스트 파일들 |

**제약 준수**: 기존 `desktop/src/*.ts` 소스는 **수정하지 않는다**(§2 비목표). fixture ID 목록은
`desktop/index.html` 및 `app_context.ts`의 querySelector 대상과 일치시켜야 하며, 이 둘이 하네스가 동기
유지해야 하는 유일한 결합점이다(setup.ts 주석에 근거를 남긴다).

## 5. 테스트 대상 (첫 파스)

| 테스트 파일 | 대상 함수 | 위치 | 검증 포인트 |
|---|---|---|---|
| `test/pane_launch.test.ts` | `paneLaunchKey(pane)` | `pane_session.ts:156` | docker → `docker\|<appId>\|<dir>`; ubuntu → `ubuntu\|<pkgId>\|<dir>`; codex/claude/gemini → `<runtime>\|<dir>`; 기타(powershell 등) → `<runtime>` |
| `test/runtime_summary.test.ts` | `runtimeLaunchSummary(runtime, pane)` | `layout.ts:350` | 각 RuntimeId별 요약 문자열 + **else fallback 오분류 교정** 케이스(T5 회귀점 — 되돌리면 red) |
| `test/runtime_guard.test.ts` | `isRuntimeId(value)` | `workspace_state.ts:33` | 유효 RuntimeId → true; 오타/비문자열/null/객체 → false |
| `test/runtime_constants.test.ts` | `runtimeLabels`, `runtimeNotes` | `app_context.ts:98,107` | 모든 RuntimeId 키에 엔트리 존재(런타임 exhaustiveness 검증 — compile-time Record 보증의 런타임 백스톱) |
| `test/ai_cli_probes.test.ts` | `getAiCliProbes`, `missingAiCliLabels`, `aiCliProbeIds` | `runtimes.ts:415~452` | inventory에서 codex/claude/gemini 프로브 추출; `status !== "ready"`인 것만 missing 라벨로 |

**RuntimeId 집합**은 `desktop/src/types.ts`의 union이 단일 근거(source of truth). 테스트는 이 타입에서
파생된 값 배열을 사용하고, 새 RuntimeId 추가 시 exhaustiveness 테스트가 신규 항목의 라벨/노트 누락을 잡도록
한다.

**TDD 정신**: 최소 1개 테스트(runtime_summary의 fallback 교정 케이스)는 해당 로직을 되돌렸을 때 red가 되도록
작성해, 테스트가 실제로 회귀를 잡는지 증명한다.

## 6. CI 통합

`.github/workflows/ci.yml`에 프론트 테스트 스텝을 추가한다. 프론트 테스트는 happy-dom 기반이라 **OS 무관**
이므로, 기존 Windows 검증 잡이 아니라 **Linux 잡 기준으로 node 스텝**을 추가하는 것이 비용이 낮다:

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: 24
    cache: npm
    cache-dependency-path: desktop/package-lock.json
- name: Desktop frontend tests
  working-directory: desktop
  run: |
    npm ci
    npm test
```

배치 위치(기존 Linux 잡에 스텝 추가 vs 신규 잡)는 구현 시 `ci.yml`의 잡 구조를 보고 결정하되, **프론트
테스트가 PR 머지 게이트에 포함**되는 것을 보장한다. `desktop/package-lock.json`이 devDep 추가로 갱신되므로
함께 커밋한다.

## 7. 검증 (이 host에서 완결)

1. `cd desktop && npm ci && npm test` → 전 테스트 green (Node v24, 이 host 가능).
2. `cd desktop && npx tsc --noEmit` exit 0, `npm run build`(tsc && vite build) exit 0 →
   하네스가 프로덕션 빌드/타입검사를 깨뜨리지 않음.
3. **회귀 증명**: `runtimeLaunchSummary`의 fallback 교정 로직을 일시적으로 되돌려 해당 테스트가 red가 됨을
   확인 후 원복(TDD 근거 남김).
4. `ci.yml` 변경은 로컬에서 실행 불가하므로 PR 후 CI 잡 green으로 검증(프론트 테스트 스텝 포함 확인).

## 8. 브랜치 / PR 전략

- `develop`에서 `feat/desktop-frontend-test-harness` 분기(완료).
- 작업 완료 후 → `develop`으로 PR. CI green 확인 후 머지.
- `main` 직접 커밋/PR 금지(릴리스 경로 아님). `git-branch-flow` 규칙 준수.
- `artifacts/` 커밋 금지, `git add -A` 금지 — 명시 파일만 스테이징.

## 9. 리스크 & 완화

| 리스크 | 완화 |
|---|---|
| xterm `new Terminal()`이 happy-dom에서 mock 없이 throw | `vi.mock("@xterm/xterm")`로 Terminal 클래스 스텁(§3-3). 구현 첫 단계에서 "빈 테스트 파일 import만" 스모크로 로드 성공 선확인 |
| DOM fixture ID가 `index.html`/`app_context.ts`와 어긋남 | fixture ID를 app_context.ts querySelector 대상과 1:1 대조, setup.ts 주석에 근거 명시. 어긋나면 import 시 관련 함수가 null 참조로 실패 → 테스트가 즉시 드러냄 |
| 공유 워킹트리에서 병렬 세션이 브랜치 스위치 | 편집 전 `git rev-parse --abbrev-ref HEAD` 확인. 신규 파일 위주라 소스 충돌 표면 최소 |
| vitest/happy-dom 버전이 Node v24/Vite v8과 비호환 | 구현 시 `npm ls` 및 스모크 실행으로 조기 확인. 비호환 시 happy-dom↔jsdom 대체 검토 |
