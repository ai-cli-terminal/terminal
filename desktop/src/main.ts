import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import "./styles.css";

type TerminalDataEvent = {
  id: string;
  data: string;
};

type TerminalExitEvent = {
  id: string;
  status: string;
};

type RuntimeId = "ash" | "ubuntu" | "docker" | "codex" | "claude" | "gemini";
type LayoutMode = "single" | "horizontal" | "vertical";

type PaneModel = {
  id: string;
  title: string;
  runtime: RuntimeId;
};

type TabModel = {
  id: string;
  title: string;
  layout: LayoutMode;
  activePaneId: string;
  panes: PaneModel[];
};

type FrontendSmokeConfig = {
  delayMilliseconds: number;
  selectionText: string;
  pasteText: string;
  pasteExpectedOutput: string;
  scrollbackLines: number;
};

type FrontendSmokeEvidence = {
  status: "passed" | "failed";
  timestamp: string;
  selection: {
    text: string;
    selected: boolean;
    selectedTextLength: number;
  };
  copy: {
    copied: boolean;
    copiedTextLength: number;
    usedEventClipboard: boolean;
  };
  paste: {
    text: string;
    expectedOutput: string;
    dispatched: boolean;
  };
  scrollback: {
    configuredScrollback: number | undefined;
    requestedLines: number;
    bufferLength: number;
    beforeBaseY: number;
    afterBaseY: number;
    viewportAfterTop: number;
    viewportAfterBottom: number;
    firstMarkerRetained: boolean;
    lastMarkerRetained: boolean;
    scrolled: boolean;
  };
};

const terminalElement = document.querySelector<HTMLDivElement>("#terminal");
const statusElement = document.querySelector<HTMLDivElement>("#status");
const restartButton = document.querySelector<HTMLButtonElement>("#restart");
const workspaceElement = document.querySelector<HTMLElement>("#workspace");
const tabBarElement = document.querySelector<HTMLElement>("#tab-bar");
const runtimeSelectElement = document.querySelector<HTMLSelectElement>("#runtime-select");
const paneStateElement = document.querySelector<HTMLSpanElement>("#pane-state");
const newTabButton = document.querySelector<HTMLButtonElement>("#new-tab");
const splitHorizontalButton = document.querySelector<HTMLButtonElement>("#split-horizontal");
const splitVerticalButton = document.querySelector<HTMLButtonElement>("#split-vertical");
const livePaneElement = document.querySelector<HTMLElement>('[data-pane-id="pane-1"]');
const livePaneRuntimeElement = livePaneElement?.querySelector<HTMLSpanElement>(".pane-runtime") ?? null;

if (
  !terminalElement ||
  !statusElement ||
  !restartButton ||
  !workspaceElement ||
  !tabBarElement ||
  !runtimeSelectElement ||
  !paneStateElement ||
  !newTabButton ||
  !splitHorizontalButton ||
  !splitVerticalButton ||
  !livePaneElement ||
  !livePaneRuntimeElement
) {
  throw new Error("terminal root is missing");
}

const status = statusElement;
const restart = restartButton;
const terminalRoot = terminalElement;
const workspace = workspaceElement;
const tabBar = tabBarElement;
const runtimeSelect = runtimeSelectElement;
const paneState = paneStateElement;
const livePane = livePaneElement;
const livePaneRuntime = livePaneRuntimeElement;
restart.disabled = true;

const runtimeLabels: Record<RuntimeId, string> = {
  ash: "ash",
  ubuntu: "Ubuntu",
  docker: "Docker",
  codex: "Codex",
  claude: "Claude",
  gemini: "Gemini"
};

const runtimeNotes: Record<RuntimeId, string> = {
  ash: "Bundled ash runtime is active.",
  ubuntu: "WSL2 Ubuntu management lands in the next runtime slice.",
  docker: "Docker install and image-first app management land in the next runtime slice.",
  codex: "Codex CLI auto-install and update checks land in the next runtime slice.",
  claude: "Claude CLI auto-install and update checks land in the next runtime slice.",
  gemini: "Gemini CLI auto-install and update checks land in the next runtime slice."
};

let tabs: TabModel[] = [
  {
    id: "tab-1",
    title: "Terminal 1",
    layout: "single",
    activePaneId: "pane-1",
    panes: [{ id: "pane-1", title: "Pane 1", runtime: "ash" }]
  }
];
let activeTabId = "tab-1";
let nextTabNumber = 2;
let nextPaneNumber = 2;

const term = new Terminal({
  allowTransparency: false,
  convertEol: true,
  cursorBlink: true,
  cursorStyle: "block",
  fontFamily:
    "Cascadia Mono, CaskaydiaCove Nerd Font, Consolas, Menlo, monospace",
  fontSize: 14,
  letterSpacing: 0,
  lineHeight: 1.08,
  scrollback: 10000,
  tabStopWidth: 4,
  theme: {
    background: "#0c0d10",
    foreground: "#e5e7eb",
    cursor: "#f5f5f4",
    selectionBackground: "#3b4252",
    black: "#111318",
    red: "#ff6b6b",
    green: "#2dd4bf",
    yellow: "#f4bf75",
    blue: "#7aa2f7",
    magenta: "#c084fc",
    cyan: "#67e8f9",
    white: "#e5e7eb",
    brightBlack: "#4b5563",
    brightRed: "#fb7185",
    brightGreen: "#5eead4",
    brightYellow: "#fde68a",
    brightBlue: "#93c5fd",
    brightMagenta: "#d8b4fe",
    brightCyan: "#a5f3fc",
    brightWhite: "#ffffff"
  }
});

const fitAddon = new FitAddon();
term.loadAddon(fitAddon);
term.open(terminalRoot);

let sessionId: string | null = null;
let unlistenData: UnlistenFn | null = null;
let unlistenExit: UnlistenFn | null = null;
let resizeTimer: number | undefined;
let isRunning = false;
let isRestarting = false;
const eofSessionIds = new Set<string>();

function getActiveTab(): TabModel {
  return tabs.find((tab) => tab.id === activeTabId) ?? tabs[0];
}

function getActivePane(): PaneModel {
  const tab = getActiveTab();
  return tab.panes.find((pane) => pane.id === tab.activePaneId) ?? tab.panes[0];
}

function isLivePaneActive(): boolean {
  return activeTabId === "tab-1" && getActivePane().id === "pane-1";
}

function updateRestartDisabled(): void {
  restart.disabled = !isLivePaneActive() || isRunning || isRestarting;
}

function setStatus(value: string): void {
  status.textContent = value;
}

function setRunning(value: boolean): void {
  isRunning = value;
  updateRestartDisabled();
}

function renderTabs(): void {
  tabBar.textContent = "";
  for (const tab of tabs) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = `tab-button${tab.id === activeTabId ? " is-active" : ""}`;
    button.textContent = tab.title;
    button.addEventListener("click", () => {
      activeTabId = tab.id;
      syncShellUi();
    });
    tabBar.append(button);
  }
}

function createPlaceholderPane(pane: PaneModel, active: boolean): HTMLElement {
  const paneElement = document.createElement("section");
  paneElement.className = `pane is-placeholder${active ? " is-active" : ""}`;
  paneElement.dataset.paneId = pane.id;
  paneElement.addEventListener("click", () => {
    const tab = getActiveTab();
    tab.activePaneId = pane.id;
    syncShellUi();
  });

  const header = document.createElement("div");
  header.className = "pane-header";
  const title = document.createElement("span");
  title.textContent = pane.title;
  const runtime = document.createElement("span");
  runtime.className = "pane-runtime";
  runtime.textContent = runtimeLabels[pane.runtime];
  header.append(title, runtime);

  const body = document.createElement("div");
  body.className = "pane-placeholder";
  const heading = document.createElement("strong");
  heading.textContent = runtimeLabels[pane.runtime];
  const note = document.createElement("span");
  note.textContent = runtimeNotes[pane.runtime];
  body.append(heading, note);

  paneElement.append(header, body);
  return paneElement;
}

function renderWorkspace(): void {
  const activeTab = getActiveTab();
  const activePane = getActivePane();
  workspace.dataset.layout = activeTab.layout;
  workspace
    .querySelectorAll<HTMLElement>(".pane.is-placeholder")
    .forEach((pane) => pane.remove());

  livePane.hidden = activeTab.id !== "tab-1";
  livePane.classList.toggle(
    "is-active",
    activeTab.id === "tab-1" && activePane.id === "pane-1"
  );
  livePaneRuntime.textContent =
    activeTab.id === "tab-1"
      ? runtimeLabels[activeTab.panes[0]?.runtime ?? "ash"]
      : "ash";

  for (const pane of activeTab.panes) {
    if (activeTab.id === "tab-1" && pane.id === "pane-1") {
      continue;
    }
    workspace.append(createPlaceholderPane(pane, pane.id === activePane.id));
  }
}

function syncShellUi(): void {
  const activeTab = getActiveTab();
  const activePane = getActivePane();
  renderTabs();
  renderWorkspace();
  runtimeSelect.value = activePane.runtime;
  paneState.textContent =
    `${activeTab.title} · ${activePane.title} · ${runtimeLabels[activePane.runtime]}`;
  updateRestartDisabled();
  if (isLivePaneActive()) {
    scheduleResize();
    term.focus();
  }
}

function addTab(): void {
  const tabNumber = nextTabNumber;
  nextTabNumber += 1;
  const paneNumber = nextPaneNumber;
  nextPaneNumber += 1;
  const pane: PaneModel = {
    id: `pane-${paneNumber}`,
    title: "Pane 1",
    runtime: "ash"
  };
  const tab: TabModel = {
    id: `tab-${tabNumber}`,
    title: `Terminal ${tabNumber}`,
    layout: "single",
    activePaneId: pane.id,
    panes: [pane]
  };
  tabs = [...tabs, tab];
  activeTabId = tab.id;
  setStatus("tab created; runtime launch wiring pending");
  syncShellUi();
}

function splitActiveTab(layout: Exclude<LayoutMode, "single">): void {
  const tab = getActiveTab();
  tab.layout = layout;
  if (tab.panes.length === 1) {
    const paneNumber = nextPaneNumber;
    nextPaneNumber += 1;
    tab.panes.push({
      id: `pane-${paneNumber}`,
      title: `Pane ${tab.panes.length + 1}`,
      runtime: getActivePane().runtime
    });
  }
  tab.activePaneId = tab.panes[tab.panes.length - 1].id;
  setStatus(`${layout} split staged; runtime launch wiring pending`);
  syncShellUi();
}

function setActivePaneRuntime(runtime: RuntimeId): void {
  const pane = getActivePane();
  pane.runtime = runtime;
  setStatus(runtime === "ash" ? "ash runtime selected" : runtimeNotes[runtime]);
  syncShellUi();
}

function fitTerminal(): void {
  if (livePane.hidden) {
    return;
  }
  fitAddon.fit();
}

async function resizeBackend(): Promise<void> {
  if (!sessionId) {
    return;
  }

  await invoke("terminal_resize", {
    id: sessionId,
    rows: term.rows,
    cols: term.cols
  });
}

function scheduleResize(): void {
  window.clearTimeout(resizeTimer);
  resizeTimer = window.setTimeout(() => {
    fitTerminal();
    void resizeBackend().catch((error: unknown) => {
      setStatus(String(error));
    });
  }, 40);
}

async function writeToBackend(data: string): Promise<void> {
  if (!sessionId || !isRunning) {
    return;
  }

  await invoke("terminal_write", {
    id: sessionId,
    data
  });
}

async function requestTerminalEof(): Promise<void> {
  if (!sessionId || !isRunning) {
    return;
  }

  const id = sessionId;
  eofSessionIds.add(id);
  setStatus("exiting");
  await invoke("terminal_eof", { id });
}

async function handleTerminalInput(data: string): Promise<void> {
  if (data === "\x04") {
    await requestTerminalEof();
    return;
  }

  await writeToBackend(data);
}

function copySelection(event?: ClipboardEvent): string {
  const selection = term.getSelection();
  if (!selection) {
    return "";
  }

  if (event?.clipboardData) {
    event.preventDefault();
    event.clipboardData.setData("text/plain", selection);
    return selection;
  }

  if (navigator.clipboard) {
    void navigator.clipboard
      .writeText(selection)
      .catch((error: unknown) => setStatus(String(error)));
  }
  return selection;
}

async function pasteText(data: string): Promise<boolean> {
  if (!isRunning || data.length === 0) {
    return false;
  }

  await writeToBackend(data);
  return true;
}

function bufferContains(text: string): boolean {
  const buffer = term.buffer.active;
  for (let index = 0; index < buffer.length; index += 1) {
    const line = buffer.getLine(index);
    if (line?.translateToString(true).includes(text)) {
      return true;
    }
  }
  return false;
}

function findBufferText(text: string): { column: number; row: number } | null {
  const buffer = term.buffer.active;
  for (let row = 0; row < buffer.length; row += 1) {
    const line = buffer.getLine(row)?.translateToString(true);
    if (!line) {
      continue;
    }
    const column = line.indexOf(text);
    if (column >= 0) {
      return { column, row };
    }
  }
  return null;
}

function dispatchPasteEvent(data: string): boolean {
  if (typeof DataTransfer === "undefined" || typeof ClipboardEvent === "undefined") {
    return false;
  }

  const clipboardData = new DataTransfer();
  clipboardData.setData("text/plain", data);
  const event = new ClipboardEvent("paste", {
    bubbles: true,
    cancelable: true,
    clipboardData
  });
  terminalRoot.dispatchEvent(event);
  return true;
}

function readCopyEventData(): { copiedText: string; usedEventClipboard: boolean } {
  if (typeof DataTransfer === "undefined" || typeof ClipboardEvent === "undefined") {
    return { copiedText: copySelection(), usedEventClipboard: false };
  }

  const clipboardData = new DataTransfer();
  const event = new ClipboardEvent("copy", {
    bubbles: true,
    cancelable: true,
    clipboardData
  });
  terminalRoot.dispatchEvent(event);
  return {
    copiedText: clipboardData.getData("text/plain"),
    usedEventClipboard: true
  };
}

async function writeFrontendSmokeEvidence(evidence: FrontendSmokeEvidence): Promise<void> {
  await invoke("terminal_write_smoke_frontend_evidence", {
    evidence: JSON.stringify(evidence, null, 2)
  });
}

async function runFrontendSmoke(config: FrontendSmokeConfig): Promise<void> {
  const evidence: FrontendSmokeEvidence = {
    status: "failed",
    timestamp: new Date().toISOString(),
    selection: {
      text: config.selectionText,
      selected: false,
      selectedTextLength: 0
    },
    copy: {
      copied: false,
      copiedTextLength: 0,
      usedEventClipboard: false
    },
    paste: {
      text: config.pasteText,
      expectedOutput: config.pasteExpectedOutput,
      dispatched: false
    },
    scrollback: {
      configuredScrollback: term.options.scrollback,
      requestedLines: config.scrollbackLines,
      bufferLength: term.buffer.active.length,
      beforeBaseY: term.buffer.active.baseY,
      afterBaseY: term.buffer.active.baseY,
      viewportAfterTop: term.buffer.active.viewportY,
      viewportAfterBottom: term.buffer.active.viewportY,
      firstMarkerRetained: false,
      lastMarkerRetained: false,
      scrolled: false
    }
  };

  try {
    term.writeln(config.selectionText);
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));
    const selectionPosition = findBufferText(config.selectionText);
    if (selectionPosition) {
      term.select(
        selectionPosition.column,
        selectionPosition.row,
        config.selectionText.length
      );
    } else {
      term.selectAll();
    }
    const selectedText = term.getSelection();
    evidence.selection.selectedTextLength = selectedText.length;
    evidence.selection.selected =
      selectionPosition !== null &&
      selectedText.length === config.selectionText.length;

    const copyResult = readCopyEventData();
    evidence.copy.copiedTextLength = copyResult.copiedText.length;
    evidence.copy.usedEventClipboard = copyResult.usedEventClipboard;
    evidence.copy.copied =
      copyResult.copiedText.length > 0 &&
      copyResult.copiedText === selectedText;
    term.clearSelection();

    const pasteDispatched = dispatchPasteEvent(config.pasteText);
    evidence.paste.dispatched = pasteDispatched || await pasteText(config.pasteText);

    const firstMarker = "AI_TERMINAL_GUI_SMOKE_SCROLLBACK_000";
    const lastMarker =
      `AI_TERMINAL_GUI_SMOKE_SCROLLBACK_${String(config.scrollbackLines - 1).padStart(3, "0")}`;
    evidence.scrollback.beforeBaseY = term.buffer.active.baseY;
    for (let index = 0; index < config.scrollbackLines; index += 1) {
      term.writeln(`AI_TERMINAL_GUI_SMOKE_SCROLLBACK_${String(index).padStart(3, "0")}`);
    }
    await new Promise<void>((resolve) => window.requestAnimationFrame(() => resolve()));

    evidence.scrollback.afterBaseY = term.buffer.active.baseY;
    evidence.scrollback.bufferLength = term.buffer.active.length;
    evidence.scrollback.firstMarkerRetained = bufferContains(firstMarker);
    evidence.scrollback.lastMarkerRetained = bufferContains(lastMarker);
    term.scrollToTop();
    evidence.scrollback.viewportAfterTop = term.buffer.active.viewportY;
    term.scrollToBottom();
    evidence.scrollback.viewportAfterBottom = term.buffer.active.viewportY;
    evidence.scrollback.scrolled =
      evidence.scrollback.afterBaseY > evidence.scrollback.beforeBaseY &&
      evidence.scrollback.viewportAfterTop !== evidence.scrollback.viewportAfterBottom;

    evidence.status =
      evidence.selection.selected &&
      evidence.copy.copied &&
      evidence.paste.dispatched &&
      evidence.scrollback.firstMarkerRetained &&
      evidence.scrollback.lastMarkerRetained &&
      evidence.scrollback.scrolled
        ? "passed"
        : "failed";
  } finally {
    await writeFrontendSmokeEvidence(evidence);
  }
}

async function scheduleFrontendSmokeIfConfigured(): Promise<void> {
  const config = await invoke<FrontendSmokeConfig | null>("terminal_smoke_frontend_config");
  if (!config) {
    return;
  }

  window.setTimeout(() => {
    void runFrontendSmoke(config).catch((error: unknown) => {
      setStatus(String(error));
    });
  }, config.delayMilliseconds);
}

async function writeSmokeCommandIfConfigured(): Promise<void> {
  const [command, ctrlDDelayMs] = await Promise.all([
    invoke<string | null>("terminal_smoke_command"),
    invoke<number | null>("terminal_smoke_ctrl_d_delay_ms")
  ]);

  if (!command && ctrlDDelayMs === null) {
    return;
  }

  if (command) {
    const data = command.endsWith("\r") || command.endsWith("\n")
      ? command
      : `${command}\r`;
    window.setTimeout(() => {
      void writeToBackend(data).catch((error: unknown) => {
        setStatus(String(error));
      });
    }, 250);
  }

  if (ctrlDDelayMs !== null) {
    window.setTimeout(() => {
      void handleTerminalInput("\x04").catch((error: unknown) => {
        setStatus(String(error));
      });
    }, ctrlDDelayMs);
  }
}

term.onData((data) => {
  void handleTerminalInput(data).catch((error: unknown) => {
    setStatus(String(error));
  });
});

term.attachCustomKeyEventHandler((event) => {
  if (event.type !== "keydown" || !event.ctrlKey || !event.shiftKey) {
    return true;
  }

  if (event.code === "KeyC") {
    copySelection();
    return false;
  }

  if (event.code === "KeyV") {
    void navigator.clipboard
      .readText()
      .then((text) => pasteText(text))
      .catch((error: unknown) => setStatus(String(error)));
    return false;
  }

  return true;
});

terminalRoot.addEventListener("copy", (event) => {
  copySelection(event);
});

terminalRoot.addEventListener("paste", (event) => {
  event.preventDefault();
  const data = event.clipboardData?.getData("text/plain") ?? "";
  void pasteText(data).catch((error: unknown) => {
    setStatus(String(error));
  });
});

newTabButton.addEventListener("click", addTab);
splitHorizontalButton.addEventListener("click", () => splitActiveTab("horizontal"));
splitVerticalButton.addEventListener("click", () => splitActiveTab("vertical"));
runtimeSelect.addEventListener("change", () => {
  setActivePaneRuntime(runtimeSelect.value as RuntimeId);
});

const resizeObserver = new ResizeObserver(scheduleResize);
resizeObserver.observe(terminalRoot);

async function startTerminal(): Promise<void> {
  unlistenData?.();
  unlistenExit?.();
  unlistenData = null;
  unlistenExit = null;

  fitTerminal();
  setStatus("starting");
  setRunning(false);

  unlistenData = await listen<TerminalDataEvent>("terminal-data", (event) => {
    if (event.payload.id === sessionId) {
      term.write(event.payload.data);
    }
  });

  unlistenExit = await listen<TerminalExitEvent>("terminal-exit", (event) => {
    if (event.payload.id === sessionId) {
      sessionId = null;
      const expectedEof = eofSessionIds.delete(event.payload.id);
      setStatus(expectedEof || event.payload.status === "exited" ? "exited" : event.payload.status);
      setRunning(false);
    }
  });

  sessionId = await invoke<string>("terminal_open", {
    rows: term.rows,
    cols: term.cols
  });
  setStatus("running");
  setRunning(true);
  term.focus();
  void writeSmokeCommandIfConfigured().catch((error: unknown) => {
    setStatus(String(error));
  });
  void scheduleFrontendSmokeIfConfigured().catch((error: unknown) => {
    setStatus(String(error));
  });
}

async function restartTerminal(): Promise<void> {
  if (isRestarting) {
    return;
  }

  isRestarting = true;
  setRunning(false);
  setStatus("restarting");

  const previousSessionId = sessionId;
  sessionId = null;
  if (previousSessionId) {
    await invoke("terminal_kill", { id: previousSessionId });
  }

  term.reset();
  await startTerminal();
  isRestarting = false;
  setRunning(true);
}

restart.addEventListener("click", () => {
  void restartTerminal().catch((error: unknown) => {
    isRestarting = false;
    setRunning(false);
    setStatus(String(error));
    term.writeln(`\x1b[31m${String(error)}\x1b[0m`);
  });
});

window.addEventListener("beforeunload", () => {
  void invoke("terminal_kill_all");
  unlistenData?.();
  unlistenExit?.();
});

syncShellUi();

void startTerminal().catch((error: unknown) => {
  setStatus(String(error));
  term.writeln(`\x1b[31m${String(error)}\x1b[0m`);
});
