import type { RuntimeId } from "./types";

const terminalElement = document.querySelector<HTMLDivElement>("#terminal");
const statusElement = document.querySelector<HTMLDivElement>("#status");
const restartButton = document.querySelector<HTMLButtonElement>("#restart");
const workspaceElement = document.querySelector<HTMLElement>("#workspace");
const tabBarElement = document.querySelector<HTMLElement>("#tab-bar");
const runtimeSelectElement = document.querySelector<HTMLSelectElement>("#runtime-select");
const runtimeInventoryElement = document.querySelector<HTMLSpanElement>("#runtime-inventory");
const runtimeRefreshButton = document.querySelector<HTMLButtonElement>("#runtime-refresh");
const ubuntuInstallButton = document.querySelector<HTMLButtonElement>("#ubuntu-install");
const aptPackageSelectElement = document.querySelector<HTMLSelectElement>("#apt-package-select");
const aptUpdateButton = document.querySelector<HTMLButtonElement>("#apt-update");
const aptInstallButton = document.querySelector<HTMLButtonElement>("#apt-install");
const dockerInstallButton = document.querySelector<HTMLButtonElement>("#docker-install");
const dockerPullButton = document.querySelector<HTMLButtonElement>("#docker-pull");
const workspaceDirInputElement = document.querySelector<HTMLInputElement>("#workspace-dir");
const workspaceApplyButton = document.querySelector<HTMLButtonElement>("#workspace-apply");
const dockerAppSelectElement = document.querySelector<HTMLSelectElement>("#docker-app-select");
const dockerAppPullButton = document.querySelector<HTMLButtonElement>("#docker-app-pull");
const aiInstallButton = document.querySelector<HTMLButtonElement>("#ai-install");
const aiUpdateButton = document.querySelector<HTMLButtonElement>("#ai-update");
const paneStateElement = document.querySelector<HTMLSpanElement>("#pane-state");
const newWindowButton = document.querySelector<HTMLButtonElement>("#new-window");
const newTabButtonElement = document.querySelector<HTMLButtonElement>("#new-tab");
const splitHorizontalButtonElement = document.querySelector<HTMLButtonElement>("#split-horizontal");
const splitVerticalButtonElement = document.querySelector<HTMLButtonElement>("#split-vertical");
const closePaneButton = document.querySelector<HTMLButtonElement>("#close-pane");
const closeTabButton = document.querySelector<HTMLButtonElement>("#close-tab");
const livePaneElement = document.querySelector<HTMLElement>('[data-pane-id="pane-1"]');
const livePaneRuntimeElement = livePaneElement?.querySelector<HTMLSpanElement>(".pane-runtime") ?? null;

if (
  !terminalElement ||
  !statusElement ||
  !restartButton ||
  !workspaceElement ||
  !tabBarElement ||
  !runtimeSelectElement ||
  !runtimeInventoryElement ||
  !runtimeRefreshButton ||
  !ubuntuInstallButton ||
  !aptPackageSelectElement ||
  !aptUpdateButton ||
  !aptInstallButton ||
  !dockerInstallButton ||
  !dockerPullButton ||
  !workspaceDirInputElement ||
  !workspaceApplyButton ||
  !dockerAppSelectElement ||
  !dockerAppPullButton ||
  !aiInstallButton ||
  !aiUpdateButton ||
  !paneStateElement ||
  !newWindowButton ||
  !newTabButtonElement ||
  !splitHorizontalButtonElement ||
  !splitVerticalButtonElement ||
  !closePaneButton ||
  !closeTabButton ||
  !livePaneElement ||
  !livePaneRuntimeElement
) {
  throw new Error("terminal root is missing");
}

export const status = statusElement;
export const restart = restartButton;
export const terminalRoot = terminalElement;
export const workspace = workspaceElement;
export const tabBar = tabBarElement;
export const runtimeSelect = runtimeSelectElement;
export const runtimeInventoryStatus = runtimeInventoryElement;
export const refreshRuntimes = runtimeRefreshButton;
export const installUbuntu = ubuntuInstallButton;
export const aptPackageSelect = aptPackageSelectElement;
export const updateApt = aptUpdateButton;
export const installAptPackage = aptInstallButton;
export const installDocker = dockerInstallButton;
export const pullDockerImage = dockerPullButton;
export const workspaceDirInput = workspaceDirInputElement;
export const applyWorkspaceDir = workspaceApplyButton;
export const dockerAppSelect = dockerAppSelectElement;
export const pullDockerApp = dockerAppPullButton;
export const installAiCli = aiInstallButton;
export const updateAiCli = aiUpdateButton;
export const paneState = paneStateElement;
export const openWindow = newWindowButton;
export const newTabButton = newTabButtonElement;
export const splitHorizontalButton = splitHorizontalButtonElement;
export const splitVerticalButton = splitVerticalButtonElement;
export const closePane = closePaneButton;
export const closeTab = closeTabButton;
export const livePane = livePaneElement;
export const livePaneRuntime = livePaneRuntimeElement;
restart.disabled = true;

export const runtimeLabels: Record<RuntimeId, string> = {
  ash: "ash",
  ubuntu: "Ubuntu",
  powershell: "PowerShell",
  docker: "Docker",
  codex: "Codex",
  claude: "Claude",
  gemini: "Gemini"
};

export const runtimeNotes: Record<RuntimeId, string> = {
  ash: "Bundled ash runtime is active.",
  ubuntu: "Ubuntu runtime selected. Restart the selected pane to open WSL Ubuntu.",
  powershell: "PowerShell 7 (pwsh) runs on the Windows host. Restart the selected pane to open it.",
  docker: "Docker runtime selected. Pull the managed image, then restart the selected pane.",
  codex: "Codex CLI runs inside managed Ubuntu. Install or update AI CLIs, then restart the selected pane.",
  claude: "Claude CLI runs inside managed Ubuntu. Install or update AI CLIs, then restart the selected pane.",
  gemini: "Gemini CLI runs inside managed Ubuntu. Install or update AI CLIs, then restart the selected pane."
};

export const workspaceStateKey = "ai-terminal-workspace-state-v1";
export const dockerWorkspaceDirKey = "ai-terminal-docker-workspace-dir";

export const eofSessionIds = new Set<string>();
