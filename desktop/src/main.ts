import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
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
  dockerAppId: string;
};

type TabModel = {
  id: string;
  title: string;
  layout: LayoutMode;
  activePaneId: string;
  panes: PaneModel[];
};

type PaneSession = {
  paneId: string;
  terminal: Terminal;
  fitAddon: FitAddon;
  root: HTMLElement;
  sessionId: string | null;
  isRunning: boolean;
  isRestarting: boolean;
};

type RuntimeProbeStatus = "ready" | "missing" | "unavailable" | "unknown";

type RuntimeProbe = {
  id: string;
  label: string;
  status: RuntimeProbeStatus;
  detail: string;
  version?: string;
  path?: string;
};

type RuntimeInventory = {
  checkedAtEpochSeconds: number;
  probes: RuntimeProbe[];
};

type DockerAppStatus = "ready" | "missing" | "unavailable";

type DockerAppProbe = {
  id: string;
  label: string;
  image: string;
  status: DockerAppStatus;
  detail: string;
  shell: string[];
};

type AptPackageStatus = "ready" | "missing" | "unavailable";

type AptPackageProbe = {
  id: string;
  label: string;
  packageName: string;
  status: AptPackageStatus;
  detail: string;
  version?: string;
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
const runtimeInventoryElement = document.querySelector<HTMLSpanElement>("#runtime-inventory");
const ubuntuInstallButton = document.querySelector<HTMLButtonElement>("#ubuntu-install");
const aptPackageSelectElement = document.querySelector<HTMLSelectElement>("#apt-package-select");
const aptUpdateButton = document.querySelector<HTMLButtonElement>("#apt-update");
const aptInstallButton = document.querySelector<HTMLButtonElement>("#apt-install");
const dockerInstallButton = document.querySelector<HTMLButtonElement>("#docker-install");
const dockerPullButton = document.querySelector<HTMLButtonElement>("#docker-pull");
const dockerAppSelectElement = document.querySelector<HTMLSelectElement>("#docker-app-select");
const dockerAppPullButton = document.querySelector<HTMLButtonElement>("#docker-app-pull");
const aiInstallButton = document.querySelector<HTMLButtonElement>("#ai-install");
const aiUpdateButton = document.querySelector<HTMLButtonElement>("#ai-update");
const paneStateElement = document.querySelector<HTMLSpanElement>("#pane-state");
const newWindowButton = document.querySelector<HTMLButtonElement>("#new-window");
const newTabButton = document.querySelector<HTMLButtonElement>("#new-tab");
const splitHorizontalButton = document.querySelector<HTMLButtonElement>("#split-horizontal");
const splitVerticalButton = document.querySelector<HTMLButtonElement>("#split-vertical");
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
  !ubuntuInstallButton ||
  !aptPackageSelectElement ||
  !aptUpdateButton ||
  !aptInstallButton ||
  !dockerInstallButton ||
  !dockerPullButton ||
  !dockerAppSelectElement ||
  !dockerAppPullButton ||
  !aiInstallButton ||
  !aiUpdateButton ||
  !paneStateElement ||
  !newWindowButton ||
  !newTabButton ||
  !splitHorizontalButton ||
  !splitVerticalButton ||
  !closePaneButton ||
  !closeTabButton ||
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
const runtimeInventoryStatus = runtimeInventoryElement;
const installUbuntu = ubuntuInstallButton;
const aptPackageSelect = aptPackageSelectElement;
const updateApt = aptUpdateButton;
const installAptPackage = aptInstallButton;
const installDocker = dockerInstallButton;
const pullDockerImage = dockerPullButton;
const dockerAppSelect = dockerAppSelectElement;
const pullDockerApp = dockerAppPullButton;
const installAiCli = aiInstallButton;
const updateAiCli = aiUpdateButton;
const paneState = paneStateElement;
const openWindow = newWindowButton;
const closePane = closePaneButton;
const closeTab = closeTabButton;
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
  ubuntu: "Ubuntu runtime selected. Restart the selected pane to open WSL Ubuntu.",
  docker: "Docker runtime selected. Pull the managed image, then restart the selected pane.",
  codex: "Codex CLI runs inside managed Ubuntu. Install or update AI CLIs, then restart the selected pane.",
  claude: "Claude CLI runs inside managed Ubuntu. Install or update AI CLIs, then restart the selected pane.",
  gemini: "Gemini CLI runs inside managed Ubuntu. Install or update AI CLIs, then restart the selected pane."
};

let tabs: TabModel[] = [
  {
    id: "tab-1",
    title: "Terminal 1",
    layout: "single",
    activePaneId: "pane-1",
    panes: [{ id: "pane-1", title: "Pane 1", runtime: "ash", dockerAppId: "ubuntu-base" }]
  }
];
let activeTabId = "tab-1";
let nextTabNumber = 2;
let nextPaneNumber = 2;
let currentInventory: RuntimeInventory | null = null;
let isInstallingUbuntu = false;
let isUpdatingApt = false;
let isInstallingAptPackage = false;
let isInstallingDocker = false;
let isPullingDockerImage = false;
let isPullingDockerApp = false;
let isInstallingAiCli = false;
let isUpdatingAiCli = false;
let dockerApps: DockerAppProbe[] = [];
let aptPackages: AptPackageProbe[] = [];
let selectedAptPackageId = "git";
let hasRunStartupAiCliEnsure = false;

const aiCliAutoInstallDateKey = "ai-terminal-ai-cli-auto-install-date";
const aiCliAutoUpdateDateKey = "ai-terminal-ai-cli-auto-update-date";

function createTerminal(): Terminal {
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

const paneSessions = new Map<string, PaneSession>();
let unlistenData: UnlistenFn | null = null;
let unlistenExit: UnlistenFn | null = null;
let resizeTimer: number | undefined;
const eofSessionIds = new Set<string>();

function createPaneSession(paneId: string, root: HTMLElement): PaneSession {
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

const primarySession = createPaneSession("pane-1", terminalRoot);
const term = primarySession.terminal;

livePane.addEventListener("click", () => {
  const tab = getActiveTab();
  if (tab.id !== "tab-1") {
    return;
  }
  tab.activePaneId = "pane-1";
  syncShellUi();
});

function getActiveTab(): TabModel {
  return tabs.find((tab) => tab.id === activeTabId) ?? tabs[0];
}

function getActivePane(): PaneModel {
  const tab = getActiveTab();
  return tab.panes.find((pane) => pane.id === tab.activePaneId) ?? tab.panes[0];
}

function findPaneById(paneId: string): PaneModel | null {
  return tabs
    .flatMap((tab) => tab.panes)
    .find((pane) => pane.id === paneId) ?? null;
}

function getPaneSession(paneId: string): PaneSession | null {
  return paneSessions.get(paneId) ?? null;
}

function getActivePaneSession(): PaneSession | null {
  return getPaneSession(getActivePane().id);
}

function findPaneSessionByBackendId(sessionId: string): PaneSession | null {
  for (const session of paneSessions.values()) {
    if (session.sessionId === sessionId) {
      return session;
    }
  }
  return null;
}

function updateRestartDisabled(): void {
  restart.disabled = getActivePaneSession()?.isRestarting ?? true;
}

function updateLayoutActions(): void {
  const activeTab = getActiveTab();
  const activePane = getActivePane();
  closePane.disabled = activePane.id === "pane-1" || activeTab.panes.length <= 1;
  closeTab.disabled = activeTab.id === "tab-1" || tabs.length <= 1;
}

function setStatus(value: string): void {
  status.textContent = value;
}

function setRunning(session: PaneSession, value: boolean): void {
  session.isRunning = value;
  updateRestartDisabled();
  updateLayoutActions();
}

async function killPaneSession(paneId: string): Promise<void> {
  const session = paneSessions.get(paneId);
  if (!session) {
    return;
  }

  const backendSessionId = session.sessionId;
  session.sessionId = null;
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

function renderRuntimeInventory(inventory: RuntimeInventory): void {
  currentInventory = inventory;
  runtimeInventoryStatus.textContent = "";
  for (const probe of inventory.probes) {
    const chip = document.createElement("span");
    chip.className = "runtime-chip";
    chip.dataset.status = probe.status;
    chip.textContent = probe.label;
    chip.title = [
      probe.detail,
      probe.version ? `Version: ${probe.version}` : undefined,
      probe.path ? `Path: ${probe.path}` : undefined
    ]
      .filter(Boolean)
      .join("\n");
    runtimeInventoryStatus.append(chip);
  }
  updateUbuntuInstallAction();
  updateAptActions();
  updateDockerActions();
  updateDockerAppActions();
  updateAiCliActions();
}

async function loadRuntimeInventory(): Promise<void> {
  runtimeInventoryStatus.textContent = "Checking runtimes...";
  try {
    const [inventory, apps] = await Promise.all([
      invoke<RuntimeInventory>("runtime_inventory"),
      invoke<DockerAppProbe[]>("docker_app_catalog")
    ]);
    const aptPackages = await invoke<AptPackageProbe[]>("apt_package_catalog");
    renderAptPackages(aptPackages);
    renderDockerApps(apps);
    renderRuntimeInventory(inventory);
    void ensureAiCliOnStartup(inventory).catch((error: unknown) => {
      setStatus(String(error));
    });
  } catch (error) {
    runtimeInventoryStatus.textContent = "Runtime check failed";
    runtimeInventoryStatus.title = String(error);
    updateUbuntuInstallAction();
    updateAptActions();
    updateDockerActions();
    updateDockerAppActions();
    updateAiCliActions();
  }
}

function getRuntimeProbe(id: string): RuntimeProbe | null {
  return currentInventory?.probes.find((probe) => probe.id === id) ?? null;
}

function updateUbuntuInstallAction(): void {
  const ubuntuProbe = getRuntimeProbe("ubuntu");
  const isReady = ubuntuProbe?.status === "ready";
  installUbuntu.disabled = isInstallingUbuntu || isReady;
  installUbuntu.textContent = isInstallingUbuntu ? "Installing..." : "Install Ubuntu";
  installUbuntu.title = isReady
    ? ubuntuProbe?.detail ?? "Ubuntu is available."
    : "Install Ubuntu through WSL.";
}

async function installUbuntuRuntime(): Promise<void> {
  if (isInstallingUbuntu) {
    return;
  }

  isInstallingUbuntu = true;
  updateUbuntuInstallAction();
  setStatus("starting WSL Ubuntu install");
  try {
    const message = await invoke<string>("wsl_ubuntu_install");
    setStatus(message);
  } catch (error) {
    setStatus(String(error));
  } finally {
    isInstallingUbuntu = false;
    await loadRuntimeInventory();
  }
}

function getSelectedAptPackage(): AptPackageProbe | null {
  return aptPackages.find((pkg) => pkg.id === selectedAptPackageId) ?? null;
}

function renderAptPackages(packages: AptPackageProbe[]): void {
  aptPackages = packages;
  if (!aptPackages.some((pkg) => pkg.id === selectedAptPackageId)) {
    selectedAptPackageId = aptPackages[0]?.id ?? "git";
  }
  aptPackageSelect.textContent = "";
  for (const pkg of aptPackages) {
    const option = document.createElement("option");
    option.value = pkg.id;
    option.textContent = pkg.label;
    option.title = [
      pkg.packageName,
      pkg.detail,
      pkg.version ? `Version: ${pkg.version}` : undefined
    ]
      .filter(Boolean)
      .join("\n");
    aptPackageSelect.append(option);
  }
  aptPackageSelect.value = selectedAptPackageId;
  updateAptActions();
}

function updateAptActions(): void {
  const ubuntuReady = getRuntimeProbe("ubuntu")?.status === "ready";
  const pkg = getSelectedAptPackage();
  const packageReady = pkg?.status === "ready";
  aptPackageSelect.disabled = aptPackages.length === 0 || isUpdatingApt || isInstallingAptPackage;
  updateApt.disabled = isUpdatingApt || isInstallingAptPackage || !ubuntuReady;
  installAptPackage.disabled =
    isUpdatingApt || isInstallingAptPackage || !ubuntuReady || packageReady || pkg === null;
  updateApt.textContent = isUpdatingApt ? "Updating..." : "Apt Update";
  installAptPackage.textContent = isInstallingAptPackage ? "Installing..." : "Install Pkg";
  updateApt.title = ubuntuReady
    ? "Run apt-get update in managed Ubuntu."
    : "Install or enable Ubuntu before running apt update.";
  installAptPackage.title = pkg
    ? packageReady
      ? `${pkg.label} is installed in managed Ubuntu.`
      : `Install Ubuntu apt package: ${pkg.packageName}`
    : "No apt package is selected.";
}

async function updateUbuntuApt(): Promise<void> {
  if (isUpdatingApt) {
    return;
  }

  isUpdatingApt = true;
  updateAptActions();
  setStatus("running apt update in managed Ubuntu");
  try {
    const message = await invoke<string>("apt_update");
    setStatus(message);
  } catch (error) {
    setStatus(String(error));
  } finally {
    isUpdatingApt = false;
    await loadRuntimeInventory();
  }
}

async function installSelectedAptPackage(): Promise<void> {
  if (isInstallingAptPackage) {
    return;
  }

  const pkg = getSelectedAptPackage();
  if (!pkg) {
    return;
  }

  isInstallingAptPackage = true;
  updateAptActions();
  setStatus(`installing apt package: ${pkg.label}`);
  try {
    const message = await invoke<string>("apt_package_install", {
      packageId: pkg.id
    });
    setStatus(message);
  } catch (error) {
    setStatus(String(error));
  } finally {
    isInstallingAptPackage = false;
    await loadRuntimeInventory();
  }
}

function updateDockerActions(): void {
  const dockerProbe = getRuntimeProbe("docker");
  const isReady = dockerProbe?.status === "ready";
  const hasDocker = dockerProbe?.status === "ready" || dockerProbe?.status === "missing";
  installDocker.disabled = isInstallingDocker || hasDocker;
  pullDockerImage.disabled = isPullingDockerImage || !hasDocker || isReady;
  installDocker.textContent = isInstallingDocker ? "Installing..." : "Install Docker";
  pullDockerImage.textContent = isPullingDockerImage ? "Pulling..." : "Pull Image";
  installDocker.title = hasDocker
    ? dockerProbe?.detail ?? "Docker is available."
    : "Install Docker Desktop through winget.";
  pullDockerImage.title = isReady
    ? dockerProbe?.detail ?? "Managed Docker image is ready."
    : "Pull or update the managed Docker image.";
}

function defaultDockerAppId(): string {
  return dockerApps[0]?.id ?? "ubuntu-base";
}

function ensurePaneDockerAppId(pane: PaneModel): string {
  if (!dockerApps.some((app) => app.id === pane.dockerAppId)) {
    pane.dockerAppId = defaultDockerAppId();
  }
  return pane.dockerAppId;
}

function getSelectedDockerApp(pane = getActivePane()): DockerAppProbe | null {
  const appId = ensurePaneDockerAppId(pane);
  return dockerApps.find((app) => app.id === appId) ?? null;
}

function renderDockerApps(apps: DockerAppProbe[]): void {
  dockerApps = apps;
  tabs.flatMap((tab) => tab.panes).forEach(ensurePaneDockerAppId);
  dockerAppSelect.textContent = "";
  for (const app of dockerApps) {
    const option = document.createElement("option");
    option.value = app.id;
    option.textContent = app.label;
    option.title = `${app.image}\n${app.detail}`;
    dockerAppSelect.append(option);
  }
  dockerAppSelect.value = ensurePaneDockerAppId(getActivePane());
  updateDockerAppActions();
}

function updateDockerAppActions(): void {
  const app = getSelectedDockerApp();
  const dockerProbe = getRuntimeProbe("docker");
  const hasDocker = dockerProbe?.status === "ready" || dockerProbe?.status === "missing";
  const appReady = app?.status === "ready";
  dockerAppSelect.disabled = dockerApps.length === 0 || isPullingDockerApp;
  pullDockerApp.disabled = isPullingDockerApp || !hasDocker || appReady || app === null;
  pullDockerApp.textContent = isPullingDockerApp ? "Pulling..." : "Pull App";
  pullDockerApp.title = app
    ? appReady
      ? `${app.label} image is ready: ${app.image}`
      : `Pull Docker app image: ${app.image}`
    : "No Docker app is selected.";
}

async function installDockerRuntime(): Promise<void> {
  if (isInstallingDocker) {
    return;
  }

  isInstallingDocker = true;
  updateDockerActions();
  setStatus("starting Docker Desktop install");
  try {
    const message = await invoke<string>("docker_desktop_install");
    setStatus(message);
  } catch (error) {
    setStatus(String(error));
  } finally {
    isInstallingDocker = false;
    await loadRuntimeInventory();
  }
}

async function pullSelectedDockerApp(): Promise<void> {
  if (isPullingDockerApp) {
    return;
  }

  const app = getSelectedDockerApp();
  if (!app) {
    return;
  }

  isPullingDockerApp = true;
  updateDockerAppActions();
  setStatus(`pulling Docker app image: ${app.label}`);
  try {
    const message = await invoke<string>("docker_app_pull", {
      appId: app.id
    });
    setStatus(message);
  } catch (error) {
    setStatus(String(error));
  } finally {
    isPullingDockerApp = false;
    await loadRuntimeInventory();
  }
}

async function pullManagedDockerImage(): Promise<void> {
  if (isPullingDockerImage) {
    return;
  }

  isPullingDockerImage = true;
  updateDockerActions();
  setStatus("pulling managed Docker image");
  try {
    const message = await invoke<string>("docker_image_pull");
    setStatus(message);
  } catch (error) {
    setStatus(String(error));
  } finally {
    isPullingDockerImage = false;
    await loadRuntimeInventory();
  }
}

function aiCliProbeIds(): RuntimeId[] {
  return ["codex", "claude", "gemini"];
}

function todayLocalDateKey(): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

function readLocalStorage(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writeLocalStorage(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Storage can be unavailable in restricted webviews; startup automation still proceeds.
  }
}

function getAiCliProbes(inventory: RuntimeInventory): RuntimeProbe[] {
  return aiCliProbeIds()
    .map((id) => inventory.probes.find((probe) => probe.id === id))
    .filter((probe): probe is RuntimeProbe => probe !== undefined);
}

function missingAiCliLabels(inventory: RuntimeInventory): string[] {
  return getAiCliProbes(inventory)
    .filter((probe) => probe.status !== "ready")
    .map((probe) => probe.label);
}

async function ensureAiCliOnStartup(inventory: RuntimeInventory): Promise<void> {
  if (hasRunStartupAiCliEnsure) {
    return;
  }
  hasRunStartupAiCliEnsure = true;

  const ubuntuReady = inventory.probes.some(
    (probe) => probe.id === "ubuntu" && probe.status === "ready"
  );
  if (!ubuntuReady) {
    setStatus("Ubuntu not ready; AI CLI startup ensure skipped");
    return;
  }

  const today = todayLocalDateKey();
  const missingLabels = missingAiCliLabels(inventory);
  if (missingLabels.length > 0) {
    if (readLocalStorage(aiCliAutoInstallDateKey) === today) {
      setStatus(`AI CLI startup install already attempted: ${missingLabels.join(", ")}`);
      return;
    }

    writeLocalStorage(aiCliAutoInstallDateKey, today);
    setStatus(`installing missing AI CLIs: ${missingLabels.join(", ")}`);
    await installAiCliRuntime();
    return;
  }

  if (readLocalStorage(aiCliAutoUpdateDateKey) === today) {
    setStatus("AI CLIs checked today");
    return;
  }

  writeLocalStorage(aiCliAutoUpdateDateKey, today);
  setStatus("updating AI CLIs on startup");
  await updateAiCliRuntime();
}

function updateAiCliActions(): void {
  const ubuntuReady = getRuntimeProbe("ubuntu")?.status === "ready";
  const aiProbes = aiCliProbeIds()
    .map((id) => getRuntimeProbe(id))
    .filter((probe): probe is RuntimeProbe => probe !== null);
  const allReady = aiProbes.length === aiCliProbeIds().length &&
    aiProbes.every((probe) => probe.status === "ready");
  installAiCli.disabled = isInstallingAiCli || isUpdatingAiCli || !ubuntuReady || allReady;
  updateAiCli.disabled = isInstallingAiCli || isUpdatingAiCli || !ubuntuReady;
  installAiCli.textContent = isInstallingAiCli ? "Installing..." : "Install AI CLIs";
  updateAiCli.textContent = isUpdatingAiCli ? "Updating..." : "Update AI CLIs";
  installAiCli.title = ubuntuReady
    ? "Install Codex, Claude, and Gemini into the managed Ubuntu runtime."
    : "Install or enable Ubuntu before installing AI CLIs.";
  updateAiCli.title = ubuntuReady
    ? "Update Codex, Claude, and Gemini inside the managed Ubuntu runtime."
    : "Install or enable Ubuntu before updating AI CLIs.";
}

async function installAiCliRuntime(): Promise<void> {
  if (isInstallingAiCli) {
    return;
  }

  isInstallingAiCli = true;
  updateAiCliActions();
  setStatus("installing AI CLIs in managed Ubuntu");
  try {
    const message = await invoke<string>("ai_cli_install");
    setStatus(message);
  } catch (error) {
    setStatus(String(error));
  } finally {
    isInstallingAiCli = false;
    await loadRuntimeInventory();
  }
}

async function updateAiCliRuntime(): Promise<void> {
  if (isUpdatingAiCli) {
    return;
  }

  isUpdatingAiCli = true;
  updateAiCliActions();
  setStatus("updating AI CLIs in managed Ubuntu");
  try {
    const message = await invoke<string>("ai_cli_update");
    setStatus(message);
  } catch (error) {
    setStatus(String(error));
  } finally {
    isUpdatingAiCli = false;
    await loadRuntimeInventory();
  }
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

function createRuntimePane(pane: PaneModel, active: boolean): HTMLElement {
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

function renderWorkspace(): void {
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
  livePaneRuntime.textContent =
    activeTab.id === "tab-1"
      ? runtimeLabels[activeTab.panes[0]?.runtime ?? "ash"]
      : "ash";

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
      runtime.textContent = runtimeLabels[pane.runtime];
    }
  }
}

function syncShellUi(): void {
  const activeTab = getActiveTab();
  const activePane = getActivePane();
  const activeDockerApp = getSelectedDockerApp(activePane);
  renderTabs();
  renderWorkspace();
  runtimeSelect.value = activePane.runtime;
  dockerAppSelect.value = ensurePaneDockerAppId(activePane);
  paneState.textContent =
    activePane.runtime === "docker" && activeDockerApp
      ? `${activeTab.title} · ${activePane.title} · Docker · ${activeDockerApp.label}`
      : `${activeTab.title} · ${activePane.title} · ${runtimeLabels[activePane.runtime]}`;
  updateRestartDisabled();
  updateLayoutActions();
  const activeSession = getActivePaneSession();
  if (activeSession) {
    scheduleResize();
    activeSession.terminal.focus();
  }
}

function openNewWindow(): void {
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

function addTab(): void {
  const tabNumber = nextTabNumber;
  nextTabNumber += 1;
  const paneNumber = nextPaneNumber;
  nextPaneNumber += 1;
  const pane: PaneModel = {
    id: `pane-${paneNumber}`,
    title: "Pane 1",
    runtime: "ash",
    dockerAppId: defaultDockerAppId()
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

function splitActiveTab(layout: Exclude<LayoutMode, "single">): void {
  const tab = getActiveTab();
  tab.layout = layout;
  if (tab.panes.length === 1) {
    const paneNumber = nextPaneNumber;
    nextPaneNumber += 1;
    tab.panes.push({
      id: `pane-${paneNumber}`,
      title: `Pane ${tab.panes.length + 1}`,
      runtime: getActivePane().runtime,
      dockerAppId: ensurePaneDockerAppId(getActivePane())
    });
  }
  tab.activePaneId = tab.panes[tab.panes.length - 1].id;
  syncShellUi();
  void startTerminal(getActivePaneSession()).catch((error: unknown) => {
    setStatus(String(error));
  });
}

async function closeActivePane(): Promise<void> {
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

async function closeActiveTab(): Promise<void> {
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

function setActivePaneRuntime(runtime: RuntimeId): void {
  const pane = getActivePane();
  pane.runtime = runtime;
  setStatus(
    runtime === "ash"
      ? "ash runtime selected; restart the selected pane to apply"
      : runtimeNotes[runtime]
  );
  syncShellUi();
}

function fitTerminal(): void {
  const session = getActivePaneSession();
  if (!session || session.root.closest<HTMLElement>(".pane")?.hidden) {
    return;
  }
  session.fitAddon.fit();
}

async function resizeBackend(): Promise<void> {
  const session = getActivePaneSession();
  if (!session?.sessionId) {
    return;
  }

  await invoke("terminal_resize", {
    id: session.sessionId,
    rows: session.terminal.rows,
    cols: session.terminal.cols
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

async function writeToBackend(session: PaneSession, data: string): Promise<void> {
  if (!session.sessionId || !session.isRunning) {
    return;
  }

  await invoke("terminal_write", {
    id: session.sessionId,
    data
  });
}

async function requestTerminalEof(session: PaneSession): Promise<void> {
  if (!session.sessionId || !session.isRunning) {
    return;
  }

  const id = session.sessionId;
  eofSessionIds.add(id);
  setStatus("exiting");
  await invoke("terminal_eof", { id });
}

async function handleTerminalInput(session: PaneSession, data: string): Promise<void> {
  if (data === "\x04") {
    await requestTerminalEof(session);
    return;
  }

  await writeToBackend(session, data);
}

function copySelection(session: PaneSession, event?: ClipboardEvent): string {
  const selection = session.terminal.getSelection();
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

async function pasteText(session: PaneSession, data: string): Promise<boolean> {
  if (!session.isRunning || data.length === 0) {
    return false;
  }

  await writeToBackend(session, data);
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
    return { copiedText: copySelection(primarySession), usedEventClipboard: false };
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
    evidence.paste.dispatched =
      pasteDispatched || await pasteText(primarySession, config.pasteText);

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
      void writeToBackend(primarySession, data).catch((error: unknown) => {
        setStatus(String(error));
      });
    }, 250);
  }

  if (ctrlDDelayMs !== null) {
    window.setTimeout(() => {
      void handleTerminalInput(primarySession, "\x04").catch((error: unknown) => {
        setStatus(String(error));
      });
    }, ctrlDDelayMs);
  }
}

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
aptPackageSelect.addEventListener("change", () => {
  selectedAptPackageId = aptPackageSelect.value;
  updateAptActions();
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
  updateDockerAppActions();
  if (pane.runtime === "docker") {
    const app = getSelectedDockerApp(pane);
    setStatus(app
      ? `Docker app selected: ${app.label}; restart the selected pane to apply`
      : "Docker app selected; restart the selected pane to apply");
  }
});
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
      const expectedEof = eofSessionIds.delete(event.payload.id);
      setStatus(expectedEof || event.payload.status === "exited" ? "exited" : event.payload.status);
      setRunning(session, false);
    });
  }
}

async function startTerminal(session: PaneSession | null): Promise<void> {
  if (!session || session.sessionId) {
    return;
  }

  await ensureTerminalEventListeners();
  session.fitAddon.fit();
  setStatus("starting");
  setRunning(session, false);

  const pane = findPaneById(session.paneId);
  const runtime = pane?.runtime ?? "ash";
  session.sessionId = await openRuntimeSession(session, runtime, pane);
  setStatus(`${runtimeLabels[runtime]} running`);
  setRunning(session, true);
  session.terminal.focus();
  if (session !== primarySession || runtime !== "ash") {
    return;
  }
  void writeSmokeCommandIfConfigured().catch((error: unknown) => {
    setStatus(String(error));
  });
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
      appId: pane ? ensurePaneDockerAppId(pane) : defaultDockerAppId()
    });
  }

  return invoke<string>("terminal_open_runtime", {
    rows: session.terminal.rows,
    cols: session.terminal.cols,
    runtime
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

syncShellUi();
void loadRuntimeInventory();

void startTerminal(primarySession).catch((error: unknown) => {
  setStatus(String(error));
  term.writeln(`\x1b[31m${String(error)}\x1b[0m`);
});
