import { applyWorkspaceDir, dockerWorkspaceDirKey, workspaceDirInput, workspaceStateKey } from "./app_context";
import { activeTabId, nextPaneNumber, nextTabNumber, tabs } from "./layout";
import { getActivePane } from "./pane_session";
import { readLocalStorage, writeLocalStorage } from "./runtimes";
import type { LayoutMode, PaneModel, RuntimeId, TabModel, WorkspaceState } from "./types";

export function defaultWorkspaceState(): WorkspaceState {
  return {
    tabs: [
      {
        id: "tab-1",
        title: "Terminal 1",
        layout: "single",
        activePaneId: "pane-1",
        panes: [
          {
            id: "pane-1",
            title: "Pane 1",
            runtime: "ash",
            dockerAppId: "ubuntu-base",
            aptPackageId: "git",
            workspaceDir: defaultDockerWorkspaceDir()
          }
        ]
      }
    ],
    activeTabId: "tab-1",
    nextTabNumber: 2,
    nextPaneNumber: 2
  };
}

export function isRuntimeId(value: unknown): value is RuntimeId {
  return value === "ash" ||
    value === "ubuntu" ||
    value === "docker" ||
    value === "codex" ||
    value === "claude" ||
    value === "gemini";
}

export function isLayoutMode(value: unknown): value is LayoutMode {
  return value === "single" || value === "horizontal" || value === "vertical";
}

export function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export function parsePaneModel(value: unknown): PaneModel | null {
  if (!isPlainObject(value) ||
    typeof value.id !== "string" ||
    typeof value.title !== "string" ||
    !isRuntimeId(value.runtime)
  ) {
    return null;
  }

  return {
    id: value.id,
    title: value.title,
    runtime: value.runtime,
    dockerAppId: typeof value.dockerAppId === "string" ? value.dockerAppId : "ubuntu-base",
    aptPackageId: typeof value.aptPackageId === "string" ? value.aptPackageId : "git",
    workspaceDir: typeof value.workspaceDir === "string"
      ? value.workspaceDir
      : defaultDockerWorkspaceDir()
  };
}

export function parseTabModel(value: unknown): TabModel | null {
  if (!isPlainObject(value) ||
    typeof value.id !== "string" ||
    typeof value.title !== "string" ||
    typeof value.activePaneId !== "string" ||
    !isLayoutMode(value.layout) ||
    !Array.isArray(value.panes)
  ) {
    return null;
  }

  const panes = value.panes
    .map(parsePaneModel)
    .filter((pane): pane is PaneModel => pane !== null);
  if (panes.length === 0 || !panes.some((pane) => pane.id === value.activePaneId)) {
    return null;
  }

  return {
    id: value.id,
    title: value.title,
    layout: panes.length === 1 ? "single" : value.layout,
    activePaneId: value.activePaneId,
    panes
  };
}

export function nextIdNumber(items: string[], prefix: string): number {
  return items.reduce((next, id) => {
    if (!id.startsWith(prefix)) {
      return next;
    }
    const value = Number(id.slice(prefix.length));
    return Number.isInteger(value) && value >= next ? value + 1 : next;
  }, 1);
}

export function loadWorkspaceState(): WorkspaceState {
  const fallback = defaultWorkspaceState();
  const raw = readLocalStorage(workspaceStateKey);
  if (!raw) {
    return fallback;
  }

  try {
    const parsed: unknown = JSON.parse(raw);
    if (!isPlainObject(parsed) ||
      !Array.isArray(parsed.tabs) ||
      typeof parsed.activeTabId !== "string"
    ) {
      return fallback;
    }

    const parsedTabs = parsed.tabs
      .map(parseTabModel)
      .filter((tab): tab is TabModel => tab !== null);
    const primaryTab = parsedTabs.find((tab) => tab.id === "tab-1");
    const primaryPane = primaryTab?.panes.find((pane) => pane.id === "pane-1");
    if (!primaryTab || !primaryPane || !parsedTabs.some((tab) => tab.id === parsed.activeTabId)) {
      return fallback;
    }

    const parsedNextTabNumber = typeof parsed.nextTabNumber === "number" && parsed.nextTabNumber > 1
      ? Math.floor(parsed.nextTabNumber)
      : fallback.nextTabNumber;
    const parsedNextPaneNumber = typeof parsed.nextPaneNumber === "number" && parsed.nextPaneNumber > 1
      ? Math.floor(parsed.nextPaneNumber)
      : fallback.nextPaneNumber;

    return {
      tabs: parsedTabs,
      activeTabId: parsed.activeTabId,
      nextTabNumber: Math.max(parsedNextTabNumber, nextIdNumber(parsedTabs.map((tab) => tab.id), "tab-")),
      nextPaneNumber: Math.max(
        parsedNextPaneNumber,
        nextIdNumber(parsedTabs.flatMap((tab) => tab.panes.map((pane) => pane.id)), "pane-")
      )
    };
  } catch {
    return fallback;
  }
}

export function saveWorkspaceState(): void {
  writeLocalStorage(workspaceStateKey, JSON.stringify({
    tabs,
    activeTabId,
    nextTabNumber,
    nextPaneNumber
  }));
}

export function defaultDockerWorkspaceDir(): string {
  return readLocalStorage(dockerWorkspaceDirKey) ?? "";
}

export function currentDockerWorkspaceDir(pane = getActivePane()): string | null {
  const value = pane.workspaceDir.trim();
  return value.length > 0 ? value : null;
}

export function updateDockerWorkspaceAction(): void {
  const workspaceDir = workspaceDirInput.value.trim();
  applyWorkspaceDir.title = workspaceDir
    ? `Apply pane workspace directory: ${workspaceDir}`
    : "Use each runtime's default workspace directory.";
  workspaceDirInput.title = workspaceDir
    ? `Ubuntu starts here; Docker mounts this host directory at /workspace: ${workspaceDir}`
    : "Leave blank to use Ubuntu home and the app working directory for Docker workspace mounts.";
}
