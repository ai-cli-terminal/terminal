import { vi } from "vitest";

// ---------------------------------------------------------------------------
// Xterm mocks
// pane_session.ts はモジュールロード時に new Terminal() / new FitAddon() を実行する。
// 実際の xterm は canvas/DOM レンダリングが必要で happy-dom で throw する可能性があるため
// 表的 mock で代替する。これらはテスト対象ではなく外部依存なので mock は正当。
// ---------------------------------------------------------------------------
vi.mock("@xterm/xterm", () => {
  class Terminal {
    open() {}
    write() {}
    writeln() {}
    focus() {}
    scrollToBottom() {}
    clear() {}
    resize() {}
    reset() {}
    loadAddon() {}
    dispose() {}
    attachCustomKeyEventHandler() {}
    onData() { return { dispose() {} }; }
    onResize() { return { dispose() {} }; }
    onSelectionChange() { return { dispose() {} }; }
    get buffer() { return { active: { baseY: 0, cursorY: 0, length: 0 } }; }
    get rows() { return 24; }
    get cols() { return 80; }
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

// CSS imports — vite 環境外では解決できないため空 mock にする。
vi.mock("@xterm/xterm/css/xterm.css", () => ({}));

// ---------------------------------------------------------------------------
// Tauri API mocks
// IPC 事故防止のため mock する。これらは外部依存でテスト対象外なので mock は正当。
// ---------------------------------------------------------------------------
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => undefined)
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
  emit: vi.fn(async () => undefined)
}));

// layout.ts は WebviewWindow を import する。
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  WebviewWindow: class WebviewWindow {
    constructor() {}
    once() {}
    emit() {}
  }
}));

// ---------------------------------------------------------------------------
// 非テスト対象の副作用モジュール mock
//
// 以下は「テスト対象ではない」進入点・副作用モジュールであり、ロード時の重い
// side effect(addEventListener / ResizeObserver / 自動 IPC 等)を避けるためだけに
// mock する。テスト対象の純粋関数モジュール
// (workspace_state / runtimes / pane_session / layout / app_context) は
// 決して mock しない。実装が実際にロードされて検証される。
// ---------------------------------------------------------------------------

// main.ts has heavy module-load side effects (addEventListener, ResizeObserver, etc.)
// layout.ts imports startTerminal from main.
vi.mock("../src/main", () => ({
  startTerminal: vi.fn(async () => undefined)
}));

// smoke.ts may be imported transitively; avoid its load-time side effects.
vi.mock("../src/smoke", () => ({
  scheduleFrontendSmokeIfConfigured: vi.fn(async () => undefined),
  writeSmokeCommandIfConfigured: vi.fn(async () => undefined),
}));

// terminal_io.ts may be imported transitively; avoid its load-time side effects.
vi.mock("../src/terminal_io", () => ({
  scheduleResize: vi.fn(),
  copySelection: vi.fn(),
  handleTerminalInput: vi.fn(async () => undefined),
  pasteText: vi.fn(async () => undefined),
}));

// ---------------------------------------------------------------------------
// DOM fixture
// app_context.ts queries these ids at module load time.
// livePaneElement uses [data-pane-id="pane-1"] and needs a .pane-runtime child.
// ---------------------------------------------------------------------------
const FIXTURE_IDS = [
  "terminal", "terminal-shell", "status", "status-bar", "restart", "workspace",
  "tab-bar", "ribbon-bar", "runtime-select", "runtime-inventory", "runtime-refresh",
  "ubuntu-install", "apt-package-select", "apt-update", "apt-install",
  "docker-install", "docker-pull", "docker-app-select", "docker-app-pull",
  "workspace-dir", "workspace-apply", "ai-install", "ai-update", "pane-state",
  "new-window", "new-tab", "split-horizontal", "split-vertical",
  "close-pane", "close-tab"
];

document.body.innerHTML =
  FIXTURE_IDS.map((id) => `<div id="${id}"></div>`).join("") +
  `<div data-pane-id="pane-1"><span class="pane-runtime"></span></div>`;

// ---------------------------------------------------------------------------
// Warm the circular source module graph in dependency order (after the DOM
// fixture above) so test files can import the REAL modules with plain static
// imports without hitting Vitest's SSR TDZ at layout.ts:43 (loadWorkspaceState
// runs at module load inside the layout↔workspace_state↔runtimes cycle).
// ---------------------------------------------------------------------------
await import("../src/app_context");
await import("../src/runtimes");
await import("../src/pane_session");
await import("../src/workspace_state");
await import("../src/layout");
