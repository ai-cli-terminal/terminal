import { vi } from "vitest";

// ---------------------------------------------------------------------------
// Xterm mocks
// pane_session.ts はモジュールロード時に new Terminal() / new FitAddon() を実行する。
// 実際の xterm は canvas/DOM レンダリングが必要で happy-dom で throw する可能性があるため
// 表的 mock で代替する。
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
// IPC 事故防止のため mock する。
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
// Circular-dependency break
//
// The source modules form a circular import graph:
//   layout → workspace_state → runtimes → workspace_state  (cycle)
//   layout → pane_session → app_context → (DOM queries)
//   layout → main → (heavy side effects)
//
// Vitest SSR transform hits TDZ errors on these cycles at module-load time.
// We mock the modules that are both (a) imported by others at load time and
// (b) part of the cycle. The smoke test only checks `typeof <export>` so
// function stubs satisfy it.
// ---------------------------------------------------------------------------

// workspace_state.ts imports runtimes AND is imported by runtimes → cycle.
// layout.ts calls loadWorkspaceState() at line 43 (module-load side effect).
vi.mock("../src/workspace_state", () => ({
  isRuntimeId: vi.fn(() => false),
  defaultWorkspaceState: vi.fn(() => ({
    tabs: [{
      id: "tab-1", title: "Terminal 1", layout: "single",
      activePaneId: "pane-1",
      panes: [{ id: "pane-1", title: "Pane 1", runtime: "ash",
        dockerAppId: "ubuntu-base", aptPackageId: "git", workspaceDir: "" }]
    }],
    activeTabId: "tab-1", nextTabNumber: 2, nextPaneNumber: 2
  })),
  loadWorkspaceState: vi.fn(() => ({
    tabs: [{
      id: "tab-1", title: "Terminal 1", layout: "single",
      activePaneId: "pane-1",
      panes: [{ id: "pane-1", title: "Pane 1", runtime: "ash",
        dockerAppId: "ubuntu-base", aptPackageId: "git", workspaceDir: "" }]
    }],
    activeTabId: "tab-1", nextTabNumber: 2, nextPaneNumber: 2
  })),
  saveWorkspaceState: vi.fn(),
  currentDockerWorkspaceDir: vi.fn(() => ""),
  defaultDockerWorkspaceDir: vi.fn(() => ""),
  updateDockerWorkspaceAction: vi.fn(),
}));

// runtimes.ts imports workspace_state AND is imported by workspace_state → cycle.
// Provide the symbols needed by other modules at load time.
vi.mock("../src/runtimes", () => ({
  aiCliProbeIds: vi.fn(() => []),
  readLocalStorage: vi.fn(() => null),
  writeLocalStorage: vi.fn(),
  currentInventory: null,
  isRefreshingRuntimeInventory: false,
  isInstallingUbuntu: false,
  isUpdatingApt: false,
  isInstallingAptPackage: false,
  isInstallingDocker: false,
  isPullingDockerImage: false,
  isPullingDockerApp: false,
  isInstallingAiCli: false,
  isUpdatingAiCli: false,
  isRunningStartupAiCliEnsure: false,
  dockerApps: [],
  aptPackages: [],
  hasRunStartupAiCliEnsure: false,
  aiCliAutoInstallDateKey: "ai-terminal-ai-cli-auto-install-date",
  aiCliAutoUpdateDateKey: "ai-terminal-ai-cli-auto-update-date",
  dockerWorkspaceDirKey: "ai-terminal-docker-workspace-dir",
  workspaceStateKey: "ai-terminal-workspace-state-v1",
  renderRuntimeInventory: vi.fn(),
  loadRuntimeInventory: vi.fn(async () => undefined),
  getRuntimeProbe: vi.fn(() => null),
  updateRuntimeRefreshAction: vi.fn(),
  updateUbuntuInstallAction: vi.fn(),
  installUbuntuRuntime: vi.fn(async () => undefined),
  defaultAptPackageId: vi.fn(() => "git"),
  ensurePaneAptPackageId: vi.fn((p: { aptPackageId: string }) => p.aptPackageId),
  getSelectedAptPackage: vi.fn(() => null),
  renderAptPackages: vi.fn(),
  updateAptActions: vi.fn(),
  updateUbuntuApt: vi.fn(async () => undefined),
  installSelectedAptPackage: vi.fn(async () => undefined),
  updateDockerActions: vi.fn(),
  defaultDockerAppId: vi.fn(() => "ubuntu-base"),
  ensurePaneDockerAppId: vi.fn((p: { dockerAppId: string }) => p.dockerAppId),
  getSelectedDockerApp: vi.fn(() => null),
  renderDockerApps: vi.fn(),
  updateDockerAppActions: vi.fn(),
  installDockerRuntime: vi.fn(async () => undefined),
  pullSelectedDockerApp: vi.fn(async () => undefined),
  pullManagedDockerImage: vi.fn(async () => undefined),
  todayLocalDateKey: vi.fn(() => ""),
  getAiCliProbes: vi.fn(() => []),
  missingAiCliLabels: vi.fn(() => []),
  ensureAiCliOnStartup: vi.fn(async () => undefined),
  updateAiCliActions: vi.fn(),
  installAiCliRuntime: vi.fn(async () => undefined),
  updateAiCliRuntime: vi.fn(async () => undefined),
}));

// pane_session.ts imports app_context (DOM queries at load) and is imported by many.
// Provide all symbols consumed at load time by layout.ts and runtimes.ts.
vi.mock("../src/pane_session", () => ({
  paneLaunchKey: vi.fn(() => ""),
  paneSessions: new Map(),
  createPaneSession: vi.fn(),
  createTerminal: vi.fn(),
  getActivePane: vi.fn(() => ({ id: "pane-1", title: "Pane 1", runtime: "ash",
    dockerAppId: "ubuntu-base", aptPackageId: "git", workspaceDir: "" })),
  getActivePaneSession: vi.fn(() => null),
  getActiveTab: vi.fn(() => null),
  getPaneSession: vi.fn(() => null),
  killPaneSession: vi.fn(async () => undefined),
  paneNeedsRestart: vi.fn(() => false),
  setStatus: vi.fn(),
  updateLayoutActions: vi.fn(),
  updateRestartDisabled: vi.fn(),
  writePaneLog: vi.fn(),
  writeSessionLog: vi.fn(),
  findPaneById: vi.fn(() => null),
  findPaneSessionByBackendId: vi.fn(() => null),
  primarySession: null,
  setRunning: vi.fn(),
  term: { writeln: vi.fn(), write: vi.fn(), focus: vi.fn() },
  eofSessionIds: new Set(),
}));

// main.ts has heavy module-load side effects (addEventListener, ResizeObserver, etc.)
// layout.ts imports startTerminal from main.
vi.mock("../src/main", () => ({
  startTerminal: vi.fn(async () => undefined)
}));

// smoke.ts may be imported transitively.
vi.mock("../src/smoke", () => ({
  scheduleFrontendSmokeIfConfigured: vi.fn(async () => undefined),
  writeSmokeCommandIfConfigured: vi.fn(async () => undefined),
}));

// terminal_io.ts may be imported transitively.
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
