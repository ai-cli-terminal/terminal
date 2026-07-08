import type { FitAddon } from "@xterm/addon-fit";
import type { Terminal } from "@xterm/xterm";

export type TerminalDataEvent = {
  id: string;
  data: string;
};

export type TerminalExitEvent = {
  id: string;
  status: string;
};

export type RuntimeId = "ash" | "ubuntu" | "docker" | "codex" | "claude" | "gemini";
export type LayoutMode = "single" | "horizontal" | "vertical";

export type PaneModel = {
  id: string;
  title: string;
  runtime: RuntimeId;
  dockerAppId: string;
  aptPackageId: string;
  workspaceDir: string;
};

export type TabModel = {
  id: string;
  title: string;
  layout: LayoutMode;
  activePaneId: string;
  panes: PaneModel[];
};

export type WorkspaceState = {
  tabs: TabModel[];
  activeTabId: string;
  nextTabNumber: number;
  nextPaneNumber: number;
};

export type PaneSession = {
  paneId: string;
  terminal: Terminal;
  fitAddon: FitAddon;
  root: HTMLElement;
  sessionId: string | null;
  runningRuntime: RuntimeId | null;
  runningLaunchKey: string | null;
  isRunning: boolean;
  isRestarting: boolean;
};

export type RuntimeProbeStatus = "ready" | "missing" | "unavailable" | "unknown";

export type RuntimeProbe = {
  id: string;
  label: string;
  status: RuntimeProbeStatus;
  detail: string;
  version?: string;
  path?: string;
};

export type RuntimeInventory = {
  checkedAtEpochSeconds: number;
  probes: RuntimeProbe[];
};

export type DockerAppStatus = "ready" | "missing" | "unavailable";

export type DockerAppProbe = {
  id: string;
  label: string;
  image: string;
  status: DockerAppStatus;
  detail: string;
  shell: string[];
};

export type AptPackageStatus = "ready" | "missing" | "unavailable";

export type AptPackageProbe = {
  id: string;
  label: string;
  packageName: string;
  status: AptPackageStatus;
  detail: string;
  version?: string;
};

export type WorkspaceProbeStatus = "ready" | "unavailable";

export type WorkspaceProbe = {
  status: WorkspaceProbeStatus;
  detail: string;
  hostPath?: string;
  ubuntuPath?: string;
  dockerTarget?: string;
};

export type AiCliActionSource = "manual" | "startup";

export type FrontendSmokeConfig = {
  delayMilliseconds: number;
  selectionText: string;
  pasteText: string;
  pasteExpectedOutput: string;
  scrollbackLines: number;
};

export type FrontendSmokeEvidence = {
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
