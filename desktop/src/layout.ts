import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import {
  aptPackageSelect,
  dockerAppSelect,
  livePane,
  livePaneRuntime,
  paneState,
  runtimeLabels,
  runtimeNotes,
  runtimeSelect,
  tabBar,
  workspace,
  workspaceDirInput
} from "./app_context";
import { startTerminal } from "./main";
import {
  createPaneSession,
  getActivePane,
  getActivePaneSession,
  getActiveTab,
  getPaneSession,
  killPaneSession,
  paneNeedsRestart,
  setStatus,
  updateLayoutActions,
  updateRestartDisabled,
  writePaneLog
} from "./pane_session";
import {
  defaultAptPackageId,
  defaultDockerAppId,
  ensurePaneAptPackageId,
  ensurePaneDockerAppId,
  getSelectedAptPackage,
  getSelectedDockerApp,
  updateAptActions,
  updateDockerAppActions
} from "./runtimes";
import { scheduleResize } from "./terminal_io";
import type { LayoutMode, PaneModel, RuntimeId, TabModel } from "./types";
import { currentDockerWorkspaceDir, loadWorkspaceState, saveWorkspaceState, updateDockerWorkspaceAction } from "./workspace_state";

const initialWorkspaceState = loadWorkspaceState();

export let tabs: TabModel[] = initialWorkspaceState.tabs;
export let activeTabId = initialWorkspaceState.activeTabId;
export let nextTabNumber = initialWorkspaceState.nextTabNumber;
export let nextPaneNumber = initialWorkspaceState.nextPaneNumber;

export function renderTabs(): void {
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

export function createRuntimePane(pane: PaneModel, active: boolean): HTMLElement {
  const paneElement = document.createElement("section");
  paneElement.className = `pane is-live${active ? " is-active" : ""}`;
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
  body.className = "terminal-host";

  paneElement.append(header, body);
  createPaneSession(pane.id, body);
  return paneElement;
}

export function paneRuntimeDisplay(pane: PaneModel): {
  label: string;
  title: string;
  pendingRestart: boolean;
} {
  const session = getPaneSession(pane.id);
  const runningRuntime = session?.runningRuntime ?? null;
  const pendingRestart = paneNeedsRestart(pane, session);
  if (pendingRestart) {
    if (runningRuntime === pane.runtime) {
      return {
        label: `${runtimeLabels[pane.runtime]} *`,
        title: `Running ${runtimeLabels[pane.runtime]}. Selected runtime settings changed. Restart this pane to apply.`,
        pendingRestart
      };
    }

    return {
      label: `${runtimeLabels[runningRuntime ?? pane.runtime]} -> ${runtimeLabels[pane.runtime]}`,
      title: `Running ${runtimeLabels[runningRuntime ?? pane.runtime]}. Selected ${runtimeLabels[pane.runtime]}. Restart this pane to apply.`,
      pendingRestart
    };
  }

  return {
    label: runtimeLabels[pane.runtime],
    title: runningRuntime
      ? `Running ${runtimeLabels[runningRuntime]}.`
      : `Selected ${runtimeLabels[pane.runtime]}.`,
    pendingRestart
  };
}

export function renderWorkspace(): void {
  const activeTab = getActiveTab();
  const activePane = getActivePane();
  workspace.dataset.layout = activeTab.layout;
  workspace
    .querySelectorAll<HTMLElement>('.pane.is-live:not([data-pane-id="pane-1"])')
    .forEach((paneElement) => {
      paneElement.hidden = true;
    });

  livePane.hidden = activeTab.id !== "tab-1";
  livePane.classList.toggle(
    "is-active",
    activeTab.id === "tab-1" && activePane.id === "pane-1"
  );
  const primaryPane = activeTab.panes[0] ?? activePane;
  livePaneRuntime.textContent =
    activeTab.id === "tab-1"
      ? paneRuntimeDisplay(primaryPane).label
      : "ash";
  if (activeTab.id === "tab-1") {
    const display = paneRuntimeDisplay(primaryPane);
    livePaneRuntime.title = display.title;
    livePaneRuntime.classList.toggle("is-pending", display.pendingRestart);
  } else {
    livePaneRuntime.title = "Selected ash.";
    livePaneRuntime.classList.remove("is-pending");
  }

  for (const pane of activeTab.panes) {
    if (activeTab.id === "tab-1" && pane.id === "pane-1") {
      continue;
    }
    let paneElement = workspace.querySelector<HTMLElement>(
      `.pane.is-live[data-pane-id="${pane.id}"]`
    );
    if (!paneElement) {
      paneElement = createRuntimePane(pane, pane.id === activePane.id);
      workspace.append(paneElement);
    }

    paneElement.hidden = false;
    paneElement.classList.toggle("is-active", pane.id === activePane.id);
    const runtime = paneElement.querySelector<HTMLSpanElement>(".pane-runtime");
    if (runtime) {
      const display = paneRuntimeDisplay(pane);
      runtime.textContent = display.label;
      runtime.title = display.title;
      runtime.classList.toggle("is-pending", display.pendingRestart);
    }
  }
}

export function syncShellUi(): void {
  const activeTab = getActiveTab();
  const activePane = getActivePane();
  const activeAptPackage = getSelectedAptPackage(activePane);
  const activeDockerApp = getSelectedDockerApp(activePane);
  renderTabs();
  renderWorkspace();
  runtimeSelect.value = activePane.runtime;
  aptPackageSelect.value = ensurePaneAptPackageId(activePane);
  dockerAppSelect.value = ensurePaneDockerAppId(activePane);
  workspaceDirInput.value = activePane.workspaceDir;
  const runtimeDisplay = paneRuntimeDisplay(activePane);
  paneState.textContent =
    runtimeDisplay.pendingRestart
      ? `${activeTab.title} · ${activePane.title} · ${runtimeDisplay.label} · restart required`
      : activePane.runtime === "ubuntu" && activeAptPackage
      ? `${activeTab.title} · ${activePane.title} · Ubuntu · ${activeAptPackage.label}`
      : activePane.runtime === "docker" && activeDockerApp
      ? `${activeTab.title} · ${activePane.title} · Docker · ${activeDockerApp.label}`
      : `${activeTab.title} · ${activePane.title} · ${runtimeLabels[activePane.runtime]}`;
  paneState.title = runtimeDisplay.title;
  updateRestartDisabled();
  updateLayoutActions();
  updateAptActions();
  updateDockerAppActions();
  updateDockerWorkspaceAction();
  const activeSession = getActivePaneSession();
  if (activeSession) {
    scheduleResize();
    activeSession.terminal.focus();
  }
  saveWorkspaceState();
}

export function openNewWindow(): void {
  const label = `terminal-window-${Date.now()}`;
  const webview = new WebviewWindow(label, {
    url: "index.html",
    title: "AI Terminal",
    width: 1200,
    height: 760,
    minWidth: 720,
    minHeight: 480,
    resizable: true,
    focus: true
  });

  setStatus("opening window");
  void webview.once("tauri://created", () => {
    setStatus("window opened");
  });
  void webview.once<string>("tauri://error", (event) => {
    setStatus(`window open failed: ${event.payload}`);
  });
}

export function addTab(): void {
  const tabNumber = nextTabNumber;
  nextTabNumber += 1;
  const paneNumber = nextPaneNumber;
  nextPaneNumber += 1;
  const pane: PaneModel = {
    id: `pane-${paneNumber}`,
    title: "Pane 1",
    runtime: "ash",
    dockerAppId: defaultDockerAppId(),
    aptPackageId: defaultAptPackageId(),
    workspaceDir: getActivePane().workspaceDir
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
  syncShellUi();
  void startTerminal(getActivePaneSession()).catch((error: unknown) => {
    setStatus(String(error));
  });
}

export function splitActiveTab(layout: Exclude<LayoutMode, "single">): void {
  const tab = getActiveTab();
  tab.layout = layout;
  if (tab.panes.length === 1) {
    const paneNumber = nextPaneNumber;
    nextPaneNumber += 1;
    tab.panes.push({
      id: `pane-${paneNumber}`,
      title: `Pane ${tab.panes.length + 1}`,
      runtime: getActivePane().runtime,
      dockerAppId: ensurePaneDockerAppId(getActivePane()),
      aptPackageId: ensurePaneAptPackageId(getActivePane()),
      workspaceDir: getActivePane().workspaceDir
    });
  }
  tab.activePaneId = tab.panes[tab.panes.length - 1].id;
  syncShellUi();
  void startTerminal(getActivePaneSession()).catch((error: unknown) => {
    setStatus(String(error));
  });
}

export async function closeActivePane(): Promise<void> {
  const tab = getActiveTab();
  const pane = getActivePane();
  if (pane.id === "pane-1" || tab.panes.length <= 1) {
    return;
  }

  const paneIndex = tab.panes.findIndex((candidate) => candidate.id === pane.id);
  await killPaneSession(pane.id);
  tab.panes = tab.panes.filter((candidate) => candidate.id !== pane.id);
  tab.layout = tab.panes.length === 1 ? "single" : tab.layout;
  const nextPane = tab.panes[Math.max(0, paneIndex - 1)] ?? tab.panes[0];
  tab.activePaneId = nextPane.id;
  setStatus(`${pane.title} closed`);
  syncShellUi();
}

export async function closeActiveTab(): Promise<void> {
  const tab = getActiveTab();
  if (tab.id === "tab-1" || tabs.length <= 1) {
    return;
  }

  const tabIndex = tabs.findIndex((candidate) => candidate.id === tab.id);
  await Promise.all(tab.panes.map((pane) => killPaneSession(pane.id)));
  tabs = tabs.filter((candidate) => candidate.id !== tab.id);
  const nextTab = tabs[Math.max(0, tabIndex - 1)] ?? tabs[0];
  activeTabId = nextTab.id;
  setStatus(`${tab.title} closed`);
  syncShellUi();
}

export function setActivePaneRuntime(runtime: RuntimeId): void {
  const pane = getActivePane();
  pane.runtime = runtime;
  const session = getActivePaneSession();
  const runningRuntime = session?.runningRuntime ?? null;
  const needsRestart = paneNeedsRestart(pane, session);
  setStatus(
    needsRestart
      ? `${runtimeLabels[runtime]} selected; restart ${pane.title} to apply`
      : runtime === "ash"
      ? "ash runtime selected"
      : runtimeNotes[runtime]
  );
  if (needsRestart) {
    writePaneLog(
      `${runtimeLabels[runtime]} selected; currently running ${runtimeLabels[runningRuntime ?? runtime]}. Restart this pane to apply.`
    );
  }
  syncShellUi();
}

export function formatPaneWorkspace(pane: PaneModel | null, runtime: RuntimeId): string {
  const workspaceDir = pane ? currentDockerWorkspaceDir(pane) : null;
  if (!workspaceDir) {
    return runtime === "docker"
      ? "workspace: app working directory -> /workspace"
      : "workspace: runtime default";
  }

  return runtime === "docker"
    ? `workspace: ${workspaceDir} -> /workspace`
    : `workspace: ${workspaceDir}`;
}

export function runtimeLaunchSummary(runtime: RuntimeId, pane: PaneModel | null): string {
  const workspace = formatPaneWorkspace(pane, runtime);
  if (runtime === "ash") {
    return `starting ash (${workspace})`;
  }

  if (runtime === "ubuntu") {
    const pkg = pane ? getSelectedAptPackage(pane) : null;
    return `starting Ubuntu (${workspace}${pkg ? `; selected package: ${pkg.label}` : ""})`;
  }

  if (runtime === "docker") {
    const app = pane ? getSelectedDockerApp(pane) : null;
    const appLabel = app ? `${app.label} (${app.image})` : "selected Docker app";
    return `starting Docker app: ${appLabel} (${workspace})`;
  }

  return `starting ${runtimeLabels[runtime]} CLI in managed Ubuntu (${workspace})`;
}
