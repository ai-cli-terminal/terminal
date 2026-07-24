import { invoke } from "@tauri-apps/api/core";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import { closePane, closeTab, eofSessionIds, livePane, restart, status, terminalRoot } from "./app_context";
import { activeTabId, syncShellUi, tabs } from "./layout";
import { ensurePaneAptPackageId, ensurePaneDockerAppId } from "./runtimes";
import { copySelection, handleTerminalInput, pasteText } from "./terminal_io";
import type { PaneModel, PaneSession, TabModel } from "./types";
import { currentDockerWorkspaceDir } from "./workspace_state";

export function createTerminal(): Terminal {
  return new Terminal({
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
}

export const paneSessions = new Map<string, PaneSession>();

export function createPaneSession(paneId: string, root: HTMLElement): PaneSession {
  const terminal = createTerminal();
  const fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(root);

  const session: PaneSession = {
    paneId,
    terminal,
    fitAddon,
    root,
    sessionId: null,
    runningRuntime: null,
    runningLaunchKey: null,
    isRunning: false,
    isRestarting: false
  };

  terminal.onData((data) => {
    void handleTerminalInput(session, data).catch((error: unknown) => {
      setStatus(String(error));
    });
  });

  terminal.attachCustomKeyEventHandler((event) => {
    if (event.type !== "keydown" || !event.ctrlKey || !event.shiftKey) {
      return true;
    }

    if (event.code === "KeyC") {
      copySelection(session);
      return false;
    }

    if (event.code === "KeyV") {
      void navigator.clipboard
        .readText()
        .then((text) => pasteText(session, text))
        .catch((error: unknown) => setStatus(String(error)));
      return false;
    }

    return true;
  });

  root.addEventListener("copy", (event) => {
    copySelection(session, event);
  });

  root.addEventListener("paste", (event) => {
    event.preventDefault();
    const data = event.clipboardData?.getData("text/plain") ?? "";
    void pasteText(session, data).catch((error: unknown) => {
      setStatus(String(error));
    });
  });

  paneSessions.set(paneId, session);
  return session;
}

export const primarySession = createPaneSession("pane-1", terminalRoot);
export const term = primarySession.terminal;

livePane.addEventListener("click", () => {
  const tab = getActiveTab();
  if (tab.id !== "tab-1") {
    return;
  }
  tab.activePaneId = "pane-1";
  syncShellUi();
});

export function getActiveTab(): TabModel {
  return tabs.find((tab) => tab.id === activeTabId) ?? tabs[0];
}

export function getActivePane(): PaneModel {
  const tab = getActiveTab();
  return tab.panes.find((pane) => pane.id === tab.activePaneId) ?? tab.panes[0];
}

export function findPaneById(paneId: string): PaneModel | null {
  return tabs
    .flatMap((tab) => tab.panes)
    .find((pane) => pane.id === paneId) ?? null;
}

export function getPaneSession(paneId: string): PaneSession | null {
  return paneSessions.get(paneId) ?? null;
}

export function getActivePaneSession(): PaneSession | null {
  return getPaneSession(getActivePane().id);
}

export function findPaneSessionByBackendId(sessionId: string): PaneSession | null {
  for (const session of paneSessions.values()) {
    if (session.sessionId === sessionId) {
      return session;
    }
  }
  return null;
}

export function paneLaunchKey(pane: PaneModel): string {
  const workspaceDir = currentDockerWorkspaceDir(pane) ?? "";
  if (pane.runtime === "docker") {
    return [
      pane.runtime,
      ensurePaneDockerAppId(pane),
      workspaceDir
    ].join("|");
  }

  if (pane.runtime === "ubuntu") {
    return [
      pane.runtime,
      ensurePaneAptPackageId(pane),
      workspaceDir
    ].join("|");
  }

  // PowerShell은 workspaceDir을 Windows 호스트 cwd로 쓰므로(백엔드 powershell_command),
  // workspace가 바뀌면 재시작이 필요하다 → 런치키에 workspaceDir 포함.
  if (pane.runtime === "powershell") {
    return [
      pane.runtime,
      workspaceDir
    ].join("|");
  }

  if (
    pane.runtime === "codex" ||
    pane.runtime === "claude" ||
    pane.runtime === "gemini"
  ) {
    return [
      pane.runtime,
      workspaceDir
    ].join("|");
  }

  return pane.runtime;
}

export function paneNeedsRestart(pane: PaneModel, session = getPaneSession(pane.id)): boolean {
  return (
    session?.isRunning === true &&
    session.runningLaunchKey !== null &&
    session.runningLaunchKey !== paneLaunchKey(pane)
  );
}

export function updateRestartDisabled(): void {
  const session = getActivePaneSession();
  const pane = getActivePane();
  const needsRestart = paneNeedsRestart(pane, session);
  restart.disabled = session?.isRestarting ?? true;
  restart.textContent = needsRestart ? "Apply" : "Restart";
  restart.title = needsRestart
    ? `Restart ${pane.title} to apply selected runtime settings.`
    : `Restart ${pane.title}.`;
}

export function updateLayoutActions(): void {
  const activeTab = getActiveTab();
  const activePane = getActivePane();
  closePane.disabled = activePane.id === "pane-1" || activeTab.panes.length <= 1;
  closeTab.disabled = activeTab.id === "tab-1" || tabs.length <= 1;
}

export function setStatus(value: string): void {
  status.textContent = value;
}

export type PaneLogTone = "info" | "success" | "error";

export function writeSessionLog(
  session: PaneSession | null,
  message: string,
  tone: PaneLogTone = "info"
): void {
  const color = tone === "success" ? "32" : tone === "error" ? "31" : "36";
  session?.terminal.writeln(`\x1b[${color}m[ai-terminal]\x1b[0m ${message}`);
}

export function writePaneLog(message: string, tone: PaneLogTone = "info"): void {
  writeSessionLog(getActivePaneSession(), message, tone);
}

export function setRunning(session: PaneSession, value: boolean): void {
  session.isRunning = value;
  updateRestartDisabled();
  updateLayoutActions();
}

export async function killPaneSession(paneId: string): Promise<void> {
  const session = paneSessions.get(paneId);
  if (!session) {
    return;
  }

  const backendSessionId = session.sessionId;
  session.sessionId = null;
  session.runningRuntime = null;
  session.runningLaunchKey = null;
  session.isRunning = false;
  if (backendSessionId) {
    eofSessionIds.delete(backendSessionId);
    await invoke("terminal_kill", { id: backendSessionId });
  }

  if (session !== primarySession) {
    session.terminal.dispose();
    session.root.closest<HTMLElement>(".pane")?.remove();
    paneSessions.delete(paneId);
  }
}
