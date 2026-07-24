import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import "@xterm/xterm/css/xterm.css";
import "./styles.css";
import {
  applyWorkspaceDir,
  aptPackageSelect,
  closePane,
  closeTab,
  dockerAppSelect,
  eofSessionIds,
  installAiCli,
  installAptPackage,
  installDocker,
  installUbuntu,
  newTabButton,
  openWindow,
  pullDockerApp,
  pullDockerImage,
  refreshRuntimes,
  restart,
  runtimeLabels,
  runtimeSelect,
  splitHorizontalButton,
  splitVerticalButton,
  updateAiCli,
  updateApt,
  workspace,
  workspaceDirInput
} from "./app_context";
import { addTab, closeActivePane, closeActiveTab, openNewWindow, runtimeLaunchSummary, setActivePaneRuntime, splitActiveTab, syncShellUi } from "./layout";
import {
  findPaneById,
  findPaneSessionByBackendId,
  getActivePane,
  getActivePaneSession,
  paneLaunchKey,
  paneNeedsRestart,
  primarySession,
  setRunning,
  setStatus,
  term,
  writePaneLog,
  writeSessionLog
} from "./pane_session";
import {
  defaultDockerAppId,
  ensurePaneDockerAppId,
  getSelectedAptPackage,
  getSelectedDockerApp,
  installAiCliRuntime,
  installDockerRuntime,
  installSelectedAptPackage,
  installUbuntuRuntime,
  loadRuntimeInventory,
  pullManagedDockerImage,
  pullSelectedDockerApp,
  updateAiCliRuntime,
  updateUbuntuApt
} from "./runtimes";
import { scheduleFrontendSmokeIfConfigured, writeSmokeCommandIfConfigured } from "./smoke";
import { scheduleResize } from "./terminal_io";
import type { PaneModel, PaneSession, RuntimeId, TerminalDataEvent, TerminalExitEvent, WorkspaceProbe } from "./types";
import { currentDockerWorkspaceDir, isRuntimeId, saveWorkspaceState, updateDockerWorkspaceAction } from "./workspace_state";

let unlistenData: UnlistenFn | null = null;
let unlistenExit: UnlistenFn | null = null;

openWindow.addEventListener("click", openNewWindow);
newTabButton.addEventListener("click", addTab);
splitHorizontalButton.addEventListener("click", () => splitActiveTab("horizontal"));
splitVerticalButton.addEventListener("click", () => splitActiveTab("vertical"));
closePane.addEventListener("click", () => {
  void closeActivePane().catch((error: unknown) => {
    setStatus(String(error));
    getActivePaneSession()?.terminal.writeln(`\x1b[31m${String(error)}\x1b[0m`);
  });
});
closeTab.addEventListener("click", () => {
  void closeActiveTab().catch((error: unknown) => {
    setStatus(String(error));
    getActivePaneSession()?.terminal.writeln(`\x1b[31m${String(error)}\x1b[0m`);
  });
});
runtimeSelect.addEventListener("change", () => {
  setActivePaneRuntime(runtimeSelect.value as RuntimeId);
});
refreshRuntimes.addEventListener("click", () => {
  setStatus("refreshing runtime status");
  void loadRuntimeInventory(true);
});
aptPackageSelect.addEventListener("change", () => {
  const pane = getActivePane();
  pane.aptPackageId = aptPackageSelect.value;
  syncShellUi();
  if (pane.runtime === "ubuntu") {
    const pkg = getSelectedAptPackage(pane);
    setStatus(pkg
      ? `Ubuntu package selected: ${pkg.label}`
      : "Ubuntu package selected");
    if (paneNeedsRestart(pane)) {
      writePaneLog("Ubuntu package selection changed; restart this pane to apply.");
    }
  }
});
updateApt.addEventListener("click", () => {
  void updateUbuntuApt();
});
installAptPackage.addEventListener("click", () => {
  void installSelectedAptPackage();
});
dockerAppSelect.addEventListener("change", () => {
  const pane = getActivePane();
  pane.dockerAppId = dockerAppSelect.value;
  syncShellUi();
  if (pane.runtime === "docker") {
    const app = getSelectedDockerApp(pane);
    setStatus(app
      ? `Docker app selected: ${app.label}; restart the selected pane to apply`
      : "Docker app selected; restart the selected pane to apply");
    if (paneNeedsRestart(pane)) {
      writePaneLog("Docker app changed; restart this pane to apply.");
    }
  }
});
workspaceDirInput.addEventListener("input", updateDockerWorkspaceAction);
applyWorkspaceDir.addEventListener("click", () => {
  void applyPaneWorkspace().catch((error: unknown) => {
    setStatus(String(error));
    writePaneLog(`workspace apply failed: ${String(error)}`, "error");
  });
});

async function applyPaneWorkspace(): Promise<void> {
  const pane = getActivePane();
  const workspaceDir = workspaceDirInput.value.trim();
  const probe = await invoke<WorkspaceProbe>("workspace_probe", {
    workspaceDir: workspaceDir.length > 0 ? workspaceDir : null
  });
  if (probe.status !== "ready") {
    setStatus(probe.detail);
    writePaneLog(`workspace rejected: ${probe.detail}`, "error");
    return;
  }

  pane.workspaceDir = workspaceDir;
  if (workspaceDir) {
    setStatus(probe.detail);
    writePaneLog(
      `${pane.title} workspace set: ${probe.hostPath ?? workspaceDir}` +
        (probe.ubuntuPath ? `; Ubuntu ${probe.ubuntuPath}` : "")
    );
  } else {
    setStatus(`${pane.title} workspace reset to runtime default`);
    writePaneLog(`${pane.title} workspace reset to runtime default`);
  }
  saveWorkspaceState();
  syncShellUi();
  if (paneNeedsRestart(pane)) {
    writePaneLog("workspace changed; restart this pane to apply.");
  }
  updateDockerWorkspaceAction();
  void loadRuntimeInventory(true);
}
installUbuntu.addEventListener("click", () => {
  void installUbuntuRuntime();
});
installDocker.addEventListener("click", () => {
  void installDockerRuntime();
});
pullDockerImage.addEventListener("click", () => {
  void pullManagedDockerImage();
});
pullDockerApp.addEventListener("click", () => {
  void pullSelectedDockerApp();
});
installAiCli.addEventListener("click", () => {
  void installAiCliRuntime();
});
updateAiCli.addEventListener("click", () => {
  void updateAiCliRuntime();
});

const resizeObserver = new ResizeObserver(scheduleResize);
resizeObserver.observe(workspace);

async function ensureTerminalEventListeners(): Promise<void> {
  if (!unlistenData) {
    unlistenData = await listen<TerminalDataEvent>("terminal-data", (event) => {
      const session = findPaneSessionByBackendId(event.payload.id);
      session?.terminal.write(event.payload.data);
    });
  }

  if (!unlistenExit) {
    unlistenExit = await listen<TerminalExitEvent>("terminal-exit", (event) => {
      const session = findPaneSessionByBackendId(event.payload.id);
      if (!session) {
        return;
      }
      session.sessionId = null;
      session.runningRuntime = null;
      session.runningLaunchKey = null;
      const expectedEof = eofSessionIds.delete(event.payload.id);
      setStatus(expectedEof || event.payload.status === "exited" ? "exited" : event.payload.status);
      setRunning(session, false);
      syncShellUi();
    });
  }
}

export async function startTerminal(session: PaneSession | null): Promise<void> {
  if (!session || session.sessionId) {
    return;
  }

  await ensureTerminalEventListeners();
  session.fitAddon.fit();
  setStatus("starting");
  setRunning(session, false);

  const pane = findPaneById(session.paneId);
  const runtime = pane?.runtime ?? "ash";
  writeSessionLog(session, runtimeLaunchSummary(runtime, pane));
  try {
    session.sessionId = await openRuntimeSession(session, runtime, pane);
    session.runningRuntime = runtime;
    session.runningLaunchKey = pane ? paneLaunchKey(pane) : runtime;
    setStatus(`${runtimeLabels[runtime]} running`);
    setRunning(session, true);
    writeSessionLog(session, `${runtimeLabels[runtime]} session attached`, "success");
    syncShellUi();
  } catch (error) {
    session.runningRuntime = null;
    session.runningLaunchKey = null;
    setRunning(session, false);
    setStatus(String(error));
    writeSessionLog(
      session,
      `${runtimeLabels[runtime]} start failed: ${String(error)}`,
      "error"
    );
    throw error;
  }
  session.terminal.focus();
  if (session !== primarySession) {
    return;
  }
  void writeSmokeCommandIfConfigured().catch((error: unknown) => {
    setStatus(String(error));
  });
  if (runtime !== "ash") {
    return;
  }
  void scheduleFrontendSmokeIfConfigured().catch((error: unknown) => {
    setStatus(String(error));
  });
}

async function openRuntimeSession(
  session: PaneSession,
  runtime: RuntimeId,
  pane: PaneModel | null
): Promise<string> {
  if (runtime === "ash") {
    return invoke<string>("terminal_open", {
      rows: session.terminal.rows,
      cols: session.terminal.cols
    });
  }

  if (runtime === "docker") {
    return invoke<string>("terminal_open_docker_app", {
      rows: session.terminal.rows,
      cols: session.terminal.cols,
      appId: pane ? ensurePaneDockerAppId(pane) : defaultDockerAppId(),
      workspaceDir: pane ? currentDockerWorkspaceDir(pane) : null
    });
  }

  return invoke<string>("terminal_open_runtime", {
    rows: session.terminal.rows,
    cols: session.terminal.cols,
    runtime,
    workspaceDir: pane ? currentDockerWorkspaceDir(pane) : null
  });
}

async function restartTerminal(session: PaneSession | null): Promise<void> {
  if (!session || session.isRestarting) {
    return;
  }

  session.isRestarting = true;
  setRunning(session, false);
  setStatus("restarting");

  const previousSessionId = session.sessionId;
  session.sessionId = null;
  if (previousSessionId) {
    await invoke("terminal_kill", { id: previousSessionId });
  }

  session.runningRuntime = null;
  session.runningLaunchKey = null;
  session.terminal.reset();
  await startTerminal(session);
  session.isRestarting = false;
  setRunning(session, true);
}

restart.addEventListener("click", () => {
  const session = getActivePaneSession();
  void restartTerminal(session).catch((error: unknown) => {
    if (session) {
      session.isRestarting = false;
      setRunning(session, false);
      session.terminal.writeln(`\x1b[31m${String(error)}\x1b[0m`);
    }
    setStatus(String(error));
  });
});

window.addEventListener("beforeunload", () => {
  void invoke("terminal_kill_all");
  unlistenData?.();
  unlistenExit?.();
});

async function startInitialSessions(): Promise<void> {
  const smokeRuntime = await invoke<string | null>("terminal_smoke_runtime");
  if (smokeRuntime !== null) {
    if (!isRuntimeId(smokeRuntime)) {
      throw new Error(`invalid GUI smoke runtime: ${smokeRuntime}`);
    }
    getActivePane().runtime = smokeRuntime;
  }

  syncShellUi();
  void loadRuntimeInventory();

  const startupActiveSession = getActivePaneSession();
  await startTerminal(primarySession);
  if (startupActiveSession && startupActiveSession !== primarySession) {
    await startTerminal(startupActiveSession);
  }
}

void startInitialSessions().catch((error: unknown) => {
  setStatus(String(error));
  term.writeln(`\x1b[31m${String(error)}\x1b[0m`);
  const startupActiveSession = getActivePaneSession();
  if (startupActiveSession && startupActiveSession !== primarySession) {
    setStatus(String(error));
    startupActiveSession.terminal.writeln(`\x1b[31m${String(error)}\x1b[0m`);
  }
});
