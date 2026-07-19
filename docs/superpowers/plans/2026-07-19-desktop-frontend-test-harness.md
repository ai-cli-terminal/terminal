# desktop 프론트엔드 테스트 하네스 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `desktop/src` TypeScript 프론트엔드에 vitest 단위 테스트 하네스를 도입하고 런타임 순수 로직을 특성화(characterize) 테스트로 잠근 뒤, 이를 CI 게이트로 승격한다.

**Architecture:** Approach C(하이브리드) — vitest `environment: "happy-dom"` + 최소 DOM fixture + `@xterm/*`·`@tauri-apps/api/*` 표적 `vi.mock`. 대상 순수 함수들은 **호출 시점엔 DOM을 안 건드리고 모듈 로드 시점에만** DOM/xterm에 얽히므로, 하네스는 "모듈 그래프가 로드되게" 만드는 것이 핵심이다. `desktop/src`의 기존 소스는 수정하지 않는다(신규 파일 위주).

**Tech Stack:** TypeScript 5, Vite 8, vitest, happy-dom, Node v24, GitHub Actions(ubuntu-latest).

## Global Constraints

- **RuntimeId 집합은 `develop` 기준 6개**: `"ash" | "ubuntu" | "docker" | "codex" | "claude" | "gemini"` (`desktop/src/types.ts:14`). **`powershell`은 없다** — 그것은 미머지 Draft PR #109(`feat/wwp-g1-powershell-pane`)에만 있다. 이 플랜의 모든 테스트는 develop의 현재 동작을 특성화한다. (#109가 나중에 머지되면 이 테스트들이 의식적 갱신을 강제하는 것이 안전망의 정상 동작이다.)
- **`desktop/src/*.ts` 소스는 수정 금지**(§2 비목표). 유일한 예외 = Task 6의 일시적 mutation(즉시 `git checkout`으로 원복, 커밋하지 않음).
- **DOM 글루 함수 테스트 금지**(YAGNI): `updateAptActions`/`renderRuntimeInventory`/`installUbuntuRuntime` 등 버튼 `disabled`/`textContent` 세팅·`invoke()` 호출 함수는 대상 아님.
- **작업 검증은 이 host에서 완결**: Node v24 + `node_modules` 존재. `cd desktop && npm test`, `npm run build`.
- **git 브랜치**: 이미 `feat/desktop-frontend-test-harness`(develop 분기)에 있음. 작업→`develop` PR. `main` 직접 금지. 편집 전 `git rev-parse --abbrev-ref HEAD`로 브랜치 확인(공유 워킹트리).
- **스테이징은 명시 파일만**. `git add -A` 금지. `artifacts/` 커밋 금지.

---

## File Structure

**신규 파일**
- `desktop/vitest.config.ts` — vitest 설정(happy-dom 환경, setupFiles).
- `desktop/test/setup.ts` — DOM fixture 주입 + `@xterm/*`·`@tauri-apps/api/*` mock.
- `desktop/test/harness.smoke.test.ts` — 모듈 그래프 로드 성공 스모크.
- `desktop/test/runtime_guard.test.ts` — `isRuntimeId`.
- `desktop/test/runtime_constants.test.ts` — `runtimeLabels`/`runtimeNotes` exhaustiveness.
- `desktop/test/ai_cli_probes.test.ts` — `aiCliProbeIds`/`getAiCliProbes`/`missingAiCliLabels`.
- `desktop/test/pane_launch.test.ts` — `paneLaunchKey`.
- `desktop/test/runtime_summary.test.ts` — `runtimeLaunchSummary`(+ mutation 회귀증명).

**수정 파일**
- `desktop/package.json` — devDep `vitest`,`happy-dom` + `test`/`test:watch` 스크립트.
- `desktop/package-lock.json` — `npm install` 결과(생성물, 함께 커밋).
- `desktop/tsconfig.json` — `include`에 `"test"` 추가, `types: ["vitest/globals"]`.
- `.github/workflows/ci.yml` — `desktop-frontend` 잡 신규.

---

## Task 1: 하네스 부트스트랩 + 모듈 로드 스모크

가장 리스크 높은 부분(xterm/DOM 모듈 로드 throw + vitest↔Vite8 호환)을 **스모크 테스트로 먼저 증명**한다. 이 Task가 green이면 이후 Task들은 순수 로직 검증만 남는다.

**Files:**
- Modify: `desktop/package.json`
- Create: `desktop/vitest.config.ts`
- Create: `desktop/test/setup.ts`
- Create: `desktop/test/harness.smoke.test.ts`
- Modify: `desktop/tsconfig.json`

**Interfaces:**
- Consumes: 기존 `desktop/src` 모듈들(`./src/workspace_state`, `./src/runtimes`, `./src/layout`, `./src/pane_session`, `./src/app_context`, `./src/types`).
- Produces: 동작하는 vitest 하네스. 이후 모든 테스트 파일은 `desktop/test/setup.ts`의 mock/fixture 위에서 `../src/*` 모듈을 import한다.

- [ ] **Step 1: 브랜치 확인**

Run: `cd /d/workspace/terminal-project/terminal && git rev-parse --abbrev-ref HEAD`
Expected: `feat/desktop-frontend-test-harness`

- [ ] **Step 2: vitest·happy-dom 설치**

Run:
```bash
cd /d/workspace/terminal-project/terminal/desktop && npm install -D vitest happy-dom
```
Expected: 설치 성공, `package.json` devDependencies에 `vitest`·`happy-dom` 추가, `package-lock.json` 갱신.
호환성 참고: Vite 8과 peer 충돌이 나면 에러 메시지의 권장 vitest 메이저를 확인해 `npm install -D vitest@<major> happy-dom` 재시도. Step 6 스모크가 최종 호환성 판정자다.

- [ ] **Step 3: `package.json` 스크립트 추가**

`desktop/package.json`의 `"scripts"` 블록에 `test`·`test:watch`를 추가한다(기존 스크립트 유지):

```json
  "scripts": {
    "dev": "vite --host 127.0.0.1 --port 1420",
    "build": "tsc && vite build",
    "test": "vitest run",
    "test:watch": "vitest",
    "package:windows-portable": "node scripts/package-windows-portable.mjs",
    "setup:wsl-nsis": "bash scripts/setup-wsl-nsis.sh",
    "stage:windows-sidecars": "node scripts/stage-windows-sidecars.mjs",
    "tauri": "tauri"
  },
```

- [ ] **Step 4: `desktop/vitest.config.ts` 생성**

```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "happy-dom",
    globals: true,
    setupFiles: ["./test/setup.ts"],
    include: ["test/**/*.test.ts"]
  }
});
```

- [ ] **Step 5: `desktop/test/setup.ts` 생성**

대상 함수는 호출 시점엔 DOM을 안 건드리지만, 모듈 로드 시점에 `app_context.ts`(`restart.disabled = true` → `#restart` 필요)와 `pane_session.ts`의 `createPaneSession`(→ `#terminal` + xterm 생성)이 실행된다. fixture는 `desktop/index.html`의 모든 element id를 담아 로드 부작용을 무해화하고, xterm/Tauri는 mock으로 격리한다.

```ts
import { vi } from "vitest";

// pane_session.ts 는 모듈 로드 시 new Terminal() / new FitAddon() 을 실행한다. 실제 xterm 은
// canvas/DOM 렌더링이 필요해 happy-dom 에서 throw 할 수 있으므로 표적 mock 으로 대체한다.
vi.mock("@xterm/xterm", () => {
  class Terminal {
    open() {}
    write() {}
    focus() {}
    scrollToBottom() {}
    clear() {}
    resize() {}
    loadAddon() {}
    dispose() {}
    onData() { return { dispose() {} }; }
    onResize() { return { dispose() {} }; }
    onSelectionChange() { return { dispose() {} }; }
    get buffer() { return { active: { baseY: 0, cursorY: 0, length: 0 } }; }
  }
  return { Terminal };
});

vi.mock("@xterm/addon-fit", () => {
  class FitAddon {
    activate() {}
    fit() {}
    dispose() {}
  }
  return { FitAddon };
});

// runtimes.ts 등은 @tauri-apps/api/core 의 invoke 를 import 한다(호출은 함수 내부에서만 발생).
// IPC 사고 방지를 위해 mock 한다.
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => undefined)
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
  emit: vi.fn(async () => undefined)
}));

// DOM fixture: app_context.ts 의 querySelector 대상과 pane_session 의 #terminal 을 제공한다.
// 목록은 desktop/index.html 의 id 집합과 동기 유지한다(근거: app_context.ts 상단 querySelector).
const FIXTURE_IDS = [
  "terminal", "terminal-shell", "status", "status-bar", "restart", "workspace",
  "tab-bar", "ribbon-bar", "runtime-select", "runtime-inventory", "runtime-refresh",
  "ubuntu-install", "apt-package-select", "apt-update", "apt-install",
  "docker-install", "docker-pull", "docker-app-select", "docker-app-pull",
  "workspace-dir", "workspace-apply", "ai-install", "ai-update", "pane-state",
  "new-window", "new-tab", "split-horizontal", "split-vertical",
  "close-pane", "close-tab", "pane-1"
];

document.body.innerHTML = FIXTURE_IDS.map((id) => `<div id="${id}"></div>`).join("");
```

- [ ] **Step 6: `desktop/test/harness.smoke.test.ts` 생성 (모듈 로드 증명)**

```ts
import { describe, expect, it } from "vitest";
import { isRuntimeId } from "../src/workspace_state";
import { aiCliProbeIds } from "../src/runtimes";
import { runtimeLaunchSummary } from "../src/layout";
import { paneLaunchKey } from "../src/pane_session";
import { runtimeLabels } from "../src/app_context";

describe("harness smoke", () => {
  it("loads the source module graph under happy-dom + mocks", () => {
    expect(typeof isRuntimeId).toBe("function");
    expect(typeof aiCliProbeIds).toBe("function");
    expect(typeof runtimeLaunchSummary).toBe("function");
    expect(typeof paneLaunchKey).toBe("function");
    expect(typeof runtimeLabels).toBe("object");
  });
});
```

- [ ] **Step 7: `desktop/tsconfig.json` 수정**

`compilerOptions`에 `"types": ["vitest/globals"]` 추가, `include`를 `["src", "test"]`로 변경:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "allowJs": false,
    "skipLibCheck": true,
    "esModuleInterop": true,
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "forceConsistentCasingInFileNames": true,
    "moduleResolution": "Bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "types": ["vitest/globals"]
  },
  "include": ["src", "test"]
}
```

- [ ] **Step 8: 스모크 실행 (하네스 검증)**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npm test`
Expected: PASS — `harness smoke > loads the source module graph...` 1 passed.
만약 import 단계에서 throw하면(`Cannot set properties of null` 등) fixture에 누락된 id를 추가하거나 xterm mock 메서드를 보강한다. `Cannot resolve @tauri-apps/...`류면 해당 모듈 mock을 추가한다. green이 될 때까지 setup.ts만 조정한다(소스 수정 금지).

- [ ] **Step 9: 프로덕션 빌드 무결성 확인**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npm run build`
Expected: exit 0 — 하네스 추가가 `tsc && vite build`를 깨뜨리지 않음. (`test/`가 tsc include에 들어갔으므로 테스트 파일 타입 에러도 여기서 잡힌다.)

- [ ] **Step 10: 커밋**

```bash
cd /d/workspace/terminal-project/terminal
git add desktop/package.json desktop/package-lock.json desktop/vitest.config.ts desktop/tsconfig.json desktop/test/setup.ts desktop/test/harness.smoke.test.ts
git commit -m "test(desktop): vitest+happy-dom 하네스 부트스트랩 + 모듈 로드 스모크"
```

---

## Task 2: `isRuntimeId` 타입 가드 테스트

**Files:**
- Create: `desktop/test/runtime_guard.test.ts`

**Interfaces:**
- Consumes: `isRuntimeId(value: unknown): value is RuntimeId` (`../src/workspace_state`).
- Produces: 없음(리프 테스트).

- [ ] **Step 1: 테스트 작성**

```ts
import { describe, expect, it } from "vitest";
import { isRuntimeId } from "../src/workspace_state";

describe("isRuntimeId", () => {
  it("accepts every develop RuntimeId", () => {
    for (const id of ["ash", "ubuntu", "docker", "codex", "claude", "gemini"]) {
      expect(isRuntimeId(id)).toBe(true);
    }
  });

  it("rejects unknown or non-string values", () => {
    expect(isRuntimeId("powershell")).toBe(false); // develop 미포함(Draft #109 전용)
    expect(isRuntimeId("bash")).toBe(false);
    expect(isRuntimeId("")).toBe(false);
    expect(isRuntimeId(null)).toBe(false);
    expect(isRuntimeId(undefined)).toBe(false);
    expect(isRuntimeId(42)).toBe(false);
    expect(isRuntimeId({ runtime: "ash" })).toBe(false);
  });
});
```

- [ ] **Step 2: 실행 (기존 동작 특성화 → PASS)**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npx vitest run test/runtime_guard.test.ts`
Expected: PASS (2 passed). 코드가 이미 존재하므로 특성화 테스트는 즉시 통과한다(실제 RED-GREEN 회귀증명은 Task 6에서 mutation으로 수행).

- [ ] **Step 3: 커밋**

```bash
cd /d/workspace/terminal-project/terminal
git add desktop/test/runtime_guard.test.ts
git commit -m "test(desktop): isRuntimeId 타입 가드 특성화 테스트"
```

---

## Task 3: `runtimeLabels`/`runtimeNotes` exhaustiveness 테스트

컴파일 타임 `Record<RuntimeId, string>` 보증의 **런타임 백스톱**. 새 RuntimeId 추가 시 라벨/노트 누락을 잡는다.

**Files:**
- Create: `desktop/test/runtime_constants.test.ts`

**Interfaces:**
- Consumes: `runtimeLabels: Record<RuntimeId, string>`, `runtimeNotes: Record<RuntimeId, string>` (`../src/app_context`).
- Produces: 없음.

- [ ] **Step 1: 테스트 작성**

```ts
import { describe, expect, it } from "vitest";
import { runtimeLabels, runtimeNotes } from "../src/app_context";

const RUNTIME_IDS = ["ash", "ubuntu", "docker", "codex", "claude", "gemini"] as const;

describe("runtime label/note maps", () => {
  it("has a non-empty label for every RuntimeId", () => {
    for (const id of RUNTIME_IDS) {
      expect(runtimeLabels[id]).toBeTruthy();
      expect(typeof runtimeLabels[id]).toBe("string");
    }
    expect(Object.keys(runtimeLabels).sort()).toEqual([...RUNTIME_IDS].sort());
  });

  it("has a non-empty note for every RuntimeId", () => {
    for (const id of RUNTIME_IDS) {
      expect(runtimeNotes[id]).toBeTruthy();
      expect(typeof runtimeNotes[id]).toBe("string");
    }
    expect(Object.keys(runtimeNotes).sort()).toEqual([...RUNTIME_IDS].sort());
  });
});
```

- [ ] **Step 2: 실행**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npx vitest run test/runtime_constants.test.ts`
Expected: PASS (2 passed).

- [ ] **Step 3: 커밋**

```bash
cd /d/workspace/terminal-project/terminal
git add desktop/test/runtime_constants.test.ts
git commit -m "test(desktop): runtimeLabels/Notes exhaustiveness 테스트"
```

---

## Task 4: AI CLI 프로브 필터 테스트

**Files:**
- Create: `desktop/test/ai_cli_probes.test.ts`

**Interfaces:**
- Consumes(모두 `../src/runtimes`):
  - `aiCliProbeIds(): RuntimeId[]` → `["codex", "claude", "gemini"]`
  - `getAiCliProbes(inventory: RuntimeInventory): RuntimeProbe[]`
  - `missingAiCliLabels(inventory: RuntimeInventory): string[]`
- `RuntimeInventory` 형태(`../src/types`): `{ checkedAtEpochSeconds: number; probes: RuntimeProbe[] }`. `RuntimeProbe`: `{ id: string; label: string; status: "ready"|"missing"|"unavailable"|"unknown"; detail: string; version?: string; path?: string }`.
- Produces: 없음.

- [ ] **Step 1: 테스트 작성**

```ts
import { describe, expect, it } from "vitest";
import { aiCliProbeIds, getAiCliProbes, missingAiCliLabels } from "../src/runtimes";
import type { RuntimeInventory, RuntimeProbe } from "../src/types";

function probe(over: Partial<RuntimeProbe> & Pick<RuntimeProbe, "id" | "label" | "status">): RuntimeProbe {
  return { detail: "", ...over };
}

function inventory(probes: RuntimeProbe[]): RuntimeInventory {
  return { checkedAtEpochSeconds: 0, probes };
}

describe("AI CLI probe helpers", () => {
  it("aiCliProbeIds is codex/claude/gemini", () => {
    expect(aiCliProbeIds()).toEqual(["codex", "claude", "gemini"]);
  });

  it("getAiCliProbes returns only present AI CLI probes, ignoring non-AI ones", () => {
    const inv = inventory([
      probe({ id: "ubuntu", label: "Ubuntu", status: "ready" }),
      probe({ id: "codex", label: "Codex", status: "ready" }),
      probe({ id: "claude", label: "Claude", status: "missing" })
      // gemini 없음
    ]);
    expect(getAiCliProbes(inv).map((p) => p.id)).toEqual(["codex", "claude"]);
  });

  it("missingAiCliLabels lists labels of non-ready AI CLI probes", () => {
    const inv = inventory([
      probe({ id: "codex", label: "Codex", status: "ready" }),
      probe({ id: "claude", label: "Claude", status: "missing" }),
      probe({ id: "gemini", label: "Gemini", status: "unavailable" })
    ]);
    expect(missingAiCliLabels(inv)).toEqual(["Claude", "Gemini"]);
  });

  it("missingAiCliLabels is empty when all AI CLIs are ready", () => {
    const inv = inventory([
      probe({ id: "codex", label: "Codex", status: "ready" }),
      probe({ id: "claude", label: "Claude", status: "ready" }),
      probe({ id: "gemini", label: "Gemini", status: "ready" })
    ]);
    expect(missingAiCliLabels(inv)).toEqual([]);
  });
});
```

- [ ] **Step 2: 실행**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npx vitest run test/ai_cli_probes.test.ts`
Expected: PASS (4 passed).

- [ ] **Step 3: 커밋**

```bash
cd /d/workspace/terminal-project/terminal
git add desktop/test/ai_cli_probes.test.ts
git commit -m "test(desktop): AI CLI 프로브 필터(getAiCliProbes/missingAiCliLabels) 테스트"
```

---

## Task 5: `paneLaunchKey` 런치키 분기 테스트

런타임별 재시작 판정(`paneNeedsRestart`)이 의존하는 핵심 로직. 모듈 state(`dockerApps`/`aptPackages`)는 테스트에서 비어 있으므로 `ensurePane*Id`가 `pane`의 값을 그대로 반환한다(결정적).

**Files:**
- Create: `desktop/test/pane_launch.test.ts`

**Interfaces:**
- Consumes: `paneLaunchKey(pane: PaneModel): string` (`../src/pane_session`). `PaneModel`(`../src/types`): `{ id; title; runtime: RuntimeId; dockerAppId: string; aptPackageId: string; workspaceDir: string }`.
- 동작(develop `pane_session.ts:156`): docker → `runtime|dockerAppId|dir`; ubuntu → `runtime|aptPackageId|dir`; codex/claude/gemini → `runtime|dir`; 그 외(ash) → `runtime`. `dir`은 `workspaceDir.trim()`이 비면 `""`.
- Produces: 없음.

- [ ] **Step 1: 테스트 작성**

```ts
import { describe, expect, it } from "vitest";
import { paneLaunchKey } from "../src/pane_session";
import type { PaneModel, RuntimeId } from "../src/types";

function pane(over: Partial<PaneModel> & { runtime: RuntimeId }): PaneModel {
  return {
    id: "pane-1",
    title: "Pane 1",
    dockerAppId: "ubuntu-base",
    aptPackageId: "git",
    workspaceDir: "",
    ...over
  };
}

describe("paneLaunchKey", () => {
  it("docker key includes dockerAppId and workspace dir", () => {
    expect(paneLaunchKey(pane({ runtime: "docker", dockerAppId: "myapp", workspaceDir: "/w" })))
      .toBe("docker|myapp|/w");
  });

  it("ubuntu key includes aptPackageId and workspace dir", () => {
    expect(paneLaunchKey(pane({ runtime: "ubuntu", aptPackageId: "curl", workspaceDir: "/w" })))
      .toBe("ubuntu|curl|/w");
  });

  it("AI CLI keys include runtime and workspace dir only", () => {
    expect(paneLaunchKey(pane({ runtime: "codex", workspaceDir: "/w" }))).toBe("codex|/w");
    expect(paneLaunchKey(pane({ runtime: "claude", workspaceDir: "/w" }))).toBe("claude|/w");
    expect(paneLaunchKey(pane({ runtime: "gemini", workspaceDir: "/w" }))).toBe("gemini|/w");
  });

  it("blank workspace dir collapses to empty segment", () => {
    expect(paneLaunchKey(pane({ runtime: "docker", dockerAppId: "myapp", workspaceDir: "   " })))
      .toBe("docker|myapp|");
  });

  it("ash (fallthrough) key is just the runtime", () => {
    expect(paneLaunchKey(pane({ runtime: "ash", workspaceDir: "/w" }))).toBe("ash");
  });
});
```

- [ ] **Step 2: 실행**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npx vitest run test/pane_launch.test.ts`
Expected: PASS (5 passed).

- [ ] **Step 3: 커밋**

```bash
cd /d/workspace/terminal-project/terminal
git add desktop/test/pane_launch.test.ts
git commit -m "test(desktop): paneLaunchKey 런타임별 런치키 분기 테스트"
```

---

## Task 6: `runtimeLaunchSummary` 테스트 + mutation 회귀증명

이 Task가 하네스의 **실제 RED-GREEN 회귀증명**을 담당한다(Step 4). 테스트는 모듈 state가 빈 상태(dockerApps/aptPackages 비어 있음)에서 결정적으로 동작한다: `getSelectedAptPackage`/`getSelectedDockerApp`이 `null`을 반환하고, `workspaceDir`이 비어 `formatPaneWorkspace`가 default 문자열을 낸다.

**Files:**
- Create: `desktop/test/runtime_summary.test.ts`
- Mutation(일시적, 커밋 금지): `desktop/src/layout.ts`

**Interfaces:**
- Consumes: `runtimeLaunchSummary(runtime: RuntimeId, pane: PaneModel | null): string` (`../src/layout`).
- 동작(develop `layout.ts:350`):
  - `ash` → `starting ash (<workspace>)`
  - `ubuntu` → `starting Ubuntu (<workspace>[; selected package: <label>])`
  - `docker` → `starting Docker app: <appLabel> (<workspace>)`, app 미선택 시 appLabel=`selected Docker app`
  - 그 외(codex/claude/gemini) → `starting <runtimeLabels[runtime]> CLI in managed Ubuntu (<workspace>)`
  - `<workspace>`: pane null/빈 dir → docker=`workspace: app working directory -> /workspace`, 그 외=`workspace: runtime default`
- Produces: 없음.

- [ ] **Step 1: 테스트 작성**

```ts
import { describe, expect, it } from "vitest";
import { runtimeLaunchSummary } from "../src/layout";
import type { PaneModel, RuntimeId } from "../src/types";

function pane(runtime: RuntimeId): PaneModel {
  return { id: "pane-1", title: "Pane 1", runtime, dockerAppId: "ubuntu-base", aptPackageId: "git", workspaceDir: "" };
}

describe("runtimeLaunchSummary", () => {
  it("ash summary", () => {
    expect(runtimeLaunchSummary("ash", pane("ash")))
      .toBe("starting ash (workspace: runtime default)");
  });

  it("ubuntu summary without a resolvable package", () => {
    expect(runtimeLaunchSummary("ubuntu", pane("ubuntu")))
      .toBe("starting Ubuntu (workspace: runtime default)");
  });

  it("docker summary falls back to 'selected Docker app' and docker workspace phrasing", () => {
    expect(runtimeLaunchSummary("docker", pane("docker")))
      .toBe("starting Docker app: selected Docker app (workspace: app working directory -> /workspace)");
  });

  it("AI CLI runtimes route through the managed-Ubuntu branch with their label", () => {
    expect(runtimeLaunchSummary("codex", pane("codex")))
      .toBe("starting Codex CLI in managed Ubuntu (workspace: runtime default)");
    expect(runtimeLaunchSummary("claude", pane("claude")))
      .toBe("starting Claude CLI in managed Ubuntu (workspace: runtime default)");
    expect(runtimeLaunchSummary("gemini", pane("gemini")))
      .toBe("starting Gemini CLI in managed Ubuntu (workspace: runtime default)");
  });

  it("null pane still yields a default-workspace summary", () => {
    expect(runtimeLaunchSummary("codex", null))
      .toBe("starting Codex CLI in managed Ubuntu (workspace: runtime default)");
  });
});
```

- [ ] **Step 2: 실행 (특성화 → PASS)**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npx vitest run test/runtime_summary.test.ts`
Expected: PASS (5 passed).

- [ ] **Step 3: mutation 주입 (테스트의 teeth 증명)**

`desktop/src/layout.ts`의 `runtimeLaunchSummary` 마지막 `return`을 일시적으로 손상시킨다. 아래 원본을

```ts
  return `starting ${runtimeLabels[runtime]} CLI in managed Ubuntu (${workspace})`;
```

임시로 다음으로 바꾼다:

```ts
  return `starting ${runtimeLabels[runtime]} CLI (${workspace})`; // TEMP MUTATION
```

- [ ] **Step 4: 재실행하여 RED 확인**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npx vitest run test/runtime_summary.test.ts`
Expected: FAIL — "AI CLI runtimes route through the managed-Ubuntu branch" 케이스가 `starting Codex CLI in managed Ubuntu ...` 기대와 불일치로 실패. 이로써 테스트가 회귀를 실제로 잡음을 증명.

- [ ] **Step 5: mutation 원복**

Run: `cd /d/workspace/terminal-project/terminal && git checkout desktop/src/layout.ts`
그런 다음 재실행하여 GREEN 복귀 확인:
Run: `cd /d/workspace/terminal-project/terminal/desktop && npx vitest run test/runtime_summary.test.ts`
Expected: PASS (5 passed). `git status`에 `desktop/src/layout.ts` 변경이 없어야 한다(원복 확인).

- [ ] **Step 6: 커밋 (테스트 파일만)**

```bash
cd /d/workspace/terminal-project/terminal
git add desktop/test/runtime_summary.test.ts
git commit -m "test(desktop): runtimeLaunchSummary 테스트 + mutation 회귀증명"
```

---

## Task 7: 전체 스위트 통과 + CI 게이트 승격

**Files:**
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: Task 1의 `npm test` 스크립트.
- Produces: PR 머지 게이트에 포함되는 `desktop-frontend` CI 잡.

- [ ] **Step 1: 전체 프론트 스위트 실행**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npm test`
Expected: PASS — 6개 테스트 파일 전부 통과(smoke + guard + constants + probes + pane_launch + summary).

- [ ] **Step 2: 빌드 무결성 재확인**

Run: `cd /d/workspace/terminal-project/terminal/desktop && npm run build`
Expected: exit 0.

- [ ] **Step 3: `.github/workflows/ci.yml`에 `desktop-frontend` 잡 추가**

`jobs:` 아래(예: `audit:` 잡 뒤, `android-native:` 앞)에 신규 잡을 추가한다. 들여쓰기는 기존 잡과 동일(잡 키 2 스페이스):

```yaml
  desktop-frontend:
    name: desktop frontend tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 24
          cache: npm
          cache-dependency-path: desktop/package-lock.json

      - name: Install desktop dependencies
        working-directory: desktop
        run: npm ci

      - name: Frontend unit tests
        working-directory: desktop
        run: npm test

      - name: Frontend typecheck & build
        working-directory: desktop
        run: npm run build
```

- [ ] **Step 4: 워크플로 YAML 문법 검증**

Run:
```bash
cd /d/workspace/terminal-project/terminal
python -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml',encoding='utf-8')); print('YAML OK')"
```
Expected: `YAML OK` (파서 에러 없음). python이 없으면 `node -e "require('js-yaml')"` 대신, git 커밋 후 PR의 GitHub Actions 파싱으로 검증한다.

- [ ] **Step 5: 커밋**

```bash
cd /d/workspace/terminal-project/terminal
git add .github/workflows/ci.yml
git commit -m "ci: desktop 프론트엔드 테스트 잡 추가(프론트 검증 CI authoritative화)"
```

- [ ] **Step 6: 푸시 + PR 생성**

```bash
cd /d/workspace/terminal-project/terminal
git push -u origin feat/desktop-frontend-test-harness
gh pr create --base develop --head feat/desktop-frontend-test-harness \
  --title "test(desktop): 프론트엔드 테스트 하네스(vitest+happy-dom) + CI 게이트" \
  --body "$(cat <<'BODY'
## 요약
desktop/src 프론트엔드에 vitest 단위 테스트 하네스(Approach C: happy-dom + 표적 mock)를 도입하고, T5 런타임 순수 로직을 특성화 테스트로 잠근 뒤 CI 게이트로 승격.

## 테스트 대상
- `isRuntimeId` 타입 가드
- `runtimeLabels`/`runtimeNotes` exhaustiveness
- `getAiCliProbes`/`missingAiCliLabels`/`aiCliProbeIds`
- `paneLaunchKey` 런타임별 런치키 분기
- `runtimeLaunchSummary`(+ mutation 회귀증명)

## 검증
- 로컬: `cd desktop && npm test` green, `npm run build` exit 0 (Node v24)
- CI: 신규 `desktop-frontend` 잡(ubuntu-latest)에서 `npm ci && npm test && npm run build`

## 정본
설계: docs/superpowers/specs/2026-07-19-desktop-frontend-test-harness-design.md
플랜: docs/superpowers/plans/2026-07-19-desktop-frontend-test-harness.md

## 비고
develop의 RuntimeId는 6개(powershell 없음 — Draft #109 전용). 테스트는 develop 현재 동작을 특성화한다.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
BODY
)"
```
Expected: PR 생성됨. CI 4+1잡(check·audit·android-native·windows + desktop-frontend) 트리거. `desktop-frontend` green 확인 후 리뷰/머지.

---

## Self-Review (작성자 체크)

**Spec 커버리지:**
- §2 하네스 도입 → Task 1 ✅ / §5 테스트 대상 5종 → Task 2~6 ✅ (pane_launch, runtime_summary, runtime_guard, runtime_constants, ai_cli_probes 모두 매핑) / §6 CI 통합 → Task 7 ✅ / §7 검증(npm test·tsc·build·mutation) → Task 1 Step 8~9, Task 6 Step 3~5, Task 7 Step 1~2 ✅ / §8 브랜치·PR → Task 7 Step 6 ✅.
- **정정 반영**: spec §1/§5가 참조한 "T5 fallback 오분류 교정(powershell)"은 develop 미포함(Draft #109 전용)이므로, 회귀증명(Task 6)은 develop에 실재하는 **codex/claude/gemini의 managed-Ubuntu 라우팅**을 mutation 대상으로 삼도록 조정함. Global Constraints에 명시.

**Placeholder 스캔:** "TBD"/"적절히 처리"/코드 없는 "테스트 작성" 없음 — 모든 코드 스텝에 실제 코드/명령/기대출력 포함. ✅

**Type 일관성:** `RuntimeId`(6-union), `PaneModel`, `RuntimeInventory`/`RuntimeProbe`(`RuntimeProbeStatus`), 함수 시그니처(`paneLaunchKey`/`runtimeLaunchSummary`/`isRuntimeId`/`getAiCliProbes`/`missingAiCliLabels`/`aiCliProbeIds`)를 실제 소스와 대조 확인. 테스트 헬퍼(`probe`/`inventory`/`pane`)의 필드가 타입 정의와 일치. ✅
