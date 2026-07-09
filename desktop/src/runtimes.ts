import { invoke } from "@tauri-apps/api/core";
import {
  aptPackageSelect,
  dockerAppSelect,
  installAiCli,
  installAptPackage,
  installDocker,
  installUbuntu,
  pullDockerApp,
  pullDockerImage,
  refreshRuntimes,
  runtimeInventoryStatus,
  updateAiCli,
  updateApt
} from "./app_context";
import { tabs } from "./layout";
import { getActivePane, setStatus, writePaneLog } from "./pane_session";
import type { AiCliActionSource, AptPackageProbe, DockerAppProbe, PaneModel, RuntimeId, RuntimeInventory, RuntimeProbe } from "./types";
import { currentDockerWorkspaceDir, saveWorkspaceState } from "./workspace_state";

export let currentInventory: RuntimeInventory | null = null;
export let isRefreshingRuntimeInventory = false;
export let isInstallingUbuntu = false;
export let isUpdatingApt = false;
export let isInstallingAptPackage = false;
export let isInstallingDocker = false;
export let isPullingDockerImage = false;
export let isPullingDockerApp = false;
export let isInstallingAiCli = false;
export let isUpdatingAiCli = false;
export let isRunningStartupAiCliEnsure = false;
export let dockerApps: DockerAppProbe[] = [];
export let aptPackages: AptPackageProbe[] = [];
export let hasRunStartupAiCliEnsure = false;

export const aiCliAutoInstallDateKey = "ai-terminal-ai-cli-auto-install-date";
export const aiCliAutoUpdateDateKey = "ai-terminal-ai-cli-auto-update-date";

export function renderRuntimeInventory(inventory: RuntimeInventory): void {
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
  updateRuntimeRefreshAction();
}

export async function loadRuntimeInventory(logToPane = false): Promise<void> {
  if (isRefreshingRuntimeInventory) {
    if (logToPane) {
      writePaneLog("runtime status check is already running");
    }
    return;
  }

  isRefreshingRuntimeInventory = true;
  updateRuntimeRefreshAction();
  runtimeInventoryStatus.textContent = "Checking runtimes...";
  if (logToPane) {
    writePaneLog("refreshing WSL Ubuntu, Docker, apt, Docker app, and AI CLI status");
  }
  try {
    const [inventory, apps] = await Promise.all([
      invoke<RuntimeInventory>("runtime_inventory", {
        workspaceDir: currentDockerWorkspaceDir()
      }),
      invoke<DockerAppProbe[]>("docker_app_catalog", {
        workspaceDir: currentDockerWorkspaceDir()
      })
    ]);
    const aptPackages = await invoke<AptPackageProbe[]>("apt_package_catalog");
    renderAptPackages(aptPackages);
    renderDockerApps(apps);
    renderRuntimeInventory(inventory);
    void ensureAiCliOnStartup(inventory).catch((error: unknown) => {
      setStatus(String(error));
    });
    if (logToPane) {
      writePaneLog("runtime status refreshed", "success");
    }
  } catch (error) {
    runtimeInventoryStatus.textContent = "Runtime check failed";
    runtimeInventoryStatus.title = String(error);
    if (logToPane) {
      writePaneLog(`runtime status refresh failed: ${String(error)}`, "error");
    }
    updateUbuntuInstallAction();
    updateAptActions();
    updateDockerActions();
    updateDockerAppActions();
    updateAiCliActions();
  } finally {
    isRefreshingRuntimeInventory = false;
    updateRuntimeRefreshAction();
  }
}

export function getRuntimeProbe(id: string): RuntimeProbe | null {
  return currentInventory?.probes.find((probe) => probe.id === id) ?? null;
}

export function updateRuntimeRefreshAction(): void {
  refreshRuntimes.disabled = isRefreshingRuntimeInventory || isRunningStartupAiCliEnsure;
  refreshRuntimes.textContent = isRefreshingRuntimeInventory
    ? "Checking..."
    : isRunningStartupAiCliEnsure
    ? "Startup..."
    : "Refresh";
  refreshRuntimes.title = isRefreshingRuntimeInventory
    ? "Runtime status check is in progress."
    : isRunningStartupAiCliEnsure
    ? "Startup AI CLI install/update is in progress."
    : "Refresh WSL Ubuntu, Docker, apt package, Docker app, and AI CLI status.";
}

export function updateUbuntuInstallAction(): void {
  const ubuntuProbe = getRuntimeProbe("ubuntu");
  const isReady = ubuntuProbe?.status === "ready";
  installUbuntu.disabled = isInstallingUbuntu || isReady;
  installUbuntu.textContent = isInstallingUbuntu ? "Installing..." : "Install Ubuntu";
  installUbuntu.title = isReady
    ? ubuntuProbe?.detail ?? "Ubuntu is available."
    : "Install Ubuntu through WSL.";
}

export async function installUbuntuRuntime(): Promise<void> {
  if (isInstallingUbuntu) {
    return;
  }

  isInstallingUbuntu = true;
  updateUbuntuInstallAction();
  setStatus("starting WSL Ubuntu install");
  writePaneLog("starting WSL Ubuntu install");
  try {
    const message = await invoke<string>("wsl_ubuntu_install");
    setStatus(message);
    writePaneLog(message, "success");
  } catch (error) {
    setStatus(String(error));
    writePaneLog(`WSL Ubuntu install failed: ${String(error)}`, "error");
  } finally {
    isInstallingUbuntu = false;
    await loadRuntimeInventory();
  }
}

export function defaultAptPackageId(): string {
  return aptPackages[0]?.id ?? "git";
}

export function ensurePaneAptPackageId(pane: PaneModel): string {
  if (!pane.aptPackageId) {
    pane.aptPackageId = defaultAptPackageId();
  }
  if (aptPackages.length > 0 && !aptPackages.some((pkg) => pkg.id === pane.aptPackageId)) {
    pane.aptPackageId = defaultAptPackageId();
  }
  return pane.aptPackageId;
}

export function getSelectedAptPackage(pane = getActivePane()): AptPackageProbe | null {
  const packageId = ensurePaneAptPackageId(pane);
  return aptPackages.find((pkg) => pkg.id === packageId) ?? null;
}

export function renderAptPackages(packages: AptPackageProbe[]): void {
  aptPackages = packages;
  tabs.flatMap((tab) => tab.panes).forEach(ensurePaneAptPackageId);
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
  aptPackageSelect.value = ensurePaneAptPackageId(getActivePane());
  updateAptActions();
  saveWorkspaceState();
}

export function updateAptActions(): void {
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

export async function updateUbuntuApt(): Promise<void> {
  if (isUpdatingApt) {
    return;
  }

  isUpdatingApt = true;
  updateAptActions();
  setStatus("running apt update in managed Ubuntu");
  writePaneLog("running apt update in managed Ubuntu");
  try {
    const message = await invoke<string>("apt_update");
    setStatus(message);
    writePaneLog(message, "success");
  } catch (error) {
    setStatus(String(error));
    writePaneLog(`apt update failed: ${String(error)}`, "error");
  } finally {
    isUpdatingApt = false;
    await loadRuntimeInventory();
  }
}

export async function installSelectedAptPackage(): Promise<void> {
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
  writePaneLog(`installing apt package: ${pkg.label} (${pkg.packageName})`);
  try {
    const message = await invoke<string>("apt_package_install", {
      packageId: pkg.id
    });
    setStatus(message);
    writePaneLog(message, "success");
  } catch (error) {
    setStatus(String(error));
    writePaneLog(`apt package install failed: ${String(error)}`, "error");
  } finally {
    isInstallingAptPackage = false;
    await loadRuntimeInventory();
  }
}

export function updateDockerActions(): void {
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

export function defaultDockerAppId(): string {
  return dockerApps[0]?.id ?? "ubuntu-base";
}

export function ensurePaneDockerAppId(pane: PaneModel): string {
  if (!pane.dockerAppId) {
    pane.dockerAppId = defaultDockerAppId();
  }
  if (dockerApps.length > 0 && !dockerApps.some((app) => app.id === pane.dockerAppId)) {
    pane.dockerAppId = defaultDockerAppId();
  }
  return pane.dockerAppId;
}

export function getSelectedDockerApp(pane = getActivePane()): DockerAppProbe | null {
  const appId = ensurePaneDockerAppId(pane);
  return dockerApps.find((app) => app.id === appId) ?? null;
}

export function renderDockerApps(apps: DockerAppProbe[]): void {
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
  saveWorkspaceState();
}

export function updateDockerAppActions(): void {
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

export async function installDockerRuntime(): Promise<void> {
  if (isInstallingDocker) {
    return;
  }

  isInstallingDocker = true;
  updateDockerActions();
  setStatus("starting Docker Desktop install");
  writePaneLog("starting Docker Desktop install through winget");
  try {
    const message = await invoke<string>("docker_desktop_install");
    setStatus(message);
    writePaneLog(message, "success");
  } catch (error) {
    setStatus(String(error));
    writePaneLog(`Docker Desktop install failed: ${String(error)}`, "error");
  } finally {
    isInstallingDocker = false;
    await loadRuntimeInventory();
  }
}

export async function pullSelectedDockerApp(): Promise<void> {
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
  writePaneLog(`pulling Docker app image: ${app.label} (${app.image})`);
  try {
    const message = await invoke<string>("docker_app_pull", {
      appId: app.id
    });
    setStatus(message);
    writePaneLog(message, "success");
  } catch (error) {
    setStatus(String(error));
    writePaneLog(`Docker app image pull failed: ${String(error)}`, "error");
  } finally {
    isPullingDockerApp = false;
    await loadRuntimeInventory();
  }
}

export async function pullManagedDockerImage(): Promise<void> {
  if (isPullingDockerImage) {
    return;
  }

  isPullingDockerImage = true;
  updateDockerActions();
  setStatus("pulling managed Docker image");
  writePaneLog("pulling managed Docker image");
  try {
    const message = await invoke<string>("docker_image_pull");
    setStatus(message);
    writePaneLog(message, "success");
  } catch (error) {
    setStatus(String(error));
    writePaneLog(`managed Docker image pull failed: ${String(error)}`, "error");
  } finally {
    isPullingDockerImage = false;
    await loadRuntimeInventory();
  }
}

export function aiCliProbeIds(): RuntimeId[] {
  return ["codex", "claude", "gemini"];
}

export function todayLocalDateKey(): string {
  const now = new Date();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

export function readLocalStorage(key: string): string | null {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
}

export function writeLocalStorage(key: string, value: string): void {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    // Storage can be unavailable in restricted webviews; startup automation still proceeds.
  }
}

export function getAiCliProbes(inventory: RuntimeInventory): RuntimeProbe[] {
  return aiCliProbeIds()
    .map((id) => inventory.probes.find((probe) => probe.id === id))
    .filter((probe): probe is RuntimeProbe => probe !== undefined);
}

export function missingAiCliLabels(inventory: RuntimeInventory): string[] {
  return getAiCliProbes(inventory)
    .filter((probe) => probe.status !== "ready")
    .map((probe) => probe.label);
}

export async function ensureAiCliOnStartup(inventory: RuntimeInventory): Promise<void> {
  if (hasRunStartupAiCliEnsure) {
    return;
  }
  hasRunStartupAiCliEnsure = true;
  isRunningStartupAiCliEnsure = true;
  updateRuntimeRefreshAction();
  updateAiCliActions();
  writePaneLog("startup AI CLI check: Codex, Claude, and Gemini");

  try {
    const ubuntuReady = inventory.probes.some(
      (probe) => probe.id === "ubuntu" && probe.status === "ready"
    );
    if (!ubuntuReady) {
      setStatus("Ubuntu not ready; AI CLI startup ensure skipped");
      writePaneLog("startup AI CLI check skipped: Ubuntu is not ready");
      return;
    }

    const today = todayLocalDateKey();
    const missingLabels = missingAiCliLabels(inventory);
    if (missingLabels.length > 0) {
      if (readLocalStorage(aiCliAutoInstallDateKey) === today) {
        const message = `AI CLI startup install already attempted: ${missingLabels.join(", ")}`;
        setStatus(message);
        writePaneLog(message);
        return;
      }

      writeLocalStorage(aiCliAutoInstallDateKey, today);
      const message = `installing missing AI CLIs on startup: ${missingLabels.join(", ")}`;
      setStatus(message);
      writePaneLog(message);
      await installAiCliRuntime("startup");
      return;
    }

    if (readLocalStorage(aiCliAutoUpdateDateKey) === today) {
      setStatus("AI CLIs checked today");
      writePaneLog("startup AI CLI check complete: already checked today", "success");
      return;
    }

    writeLocalStorage(aiCliAutoUpdateDateKey, today);
    setStatus("updating AI CLIs on startup");
    writePaneLog("updating Codex, Claude, and Gemini CLIs on startup");
    await updateAiCliRuntime("startup");
  } catch (error) {
    setStatus(String(error));
    writePaneLog(`startup AI CLI check failed: ${String(error)}`, "error");
  } finally {
    isRunningStartupAiCliEnsure = false;
    updateRuntimeRefreshAction();
    updateAiCliActions();
  }
}

export function updateAiCliActions(): void {
  const ubuntuReady = getRuntimeProbe("ubuntu")?.status === "ready";
  const aiProbes = aiCliProbeIds()
    .map((id) => getRuntimeProbe(id))
    .filter((probe): probe is RuntimeProbe => probe !== null);
  const allReady = aiProbes.length === aiCliProbeIds().length &&
    aiProbes.every((probe) => probe.status === "ready");
  const isAiCliBusy = isInstallingAiCli || isUpdatingAiCli || isRunningStartupAiCliEnsure;
  installAiCli.disabled = isAiCliBusy || !ubuntuReady || allReady;
  updateAiCli.disabled = isAiCliBusy || !ubuntuReady;
  installAiCli.textContent = isInstallingAiCli
    ? "Installing..."
    : isRunningStartupAiCliEnsure
    ? "Startup..."
    : "Install AI CLIs";
  updateAiCli.textContent = isUpdatingAiCli
    ? "Updating..."
    : isRunningStartupAiCliEnsure
    ? "Startup..."
    : "Update AI CLIs";
  installAiCli.title = ubuntuReady
    ? isRunningStartupAiCliEnsure
      ? "Startup AI CLI install/update is in progress."
      : "Install Codex, Claude, and Gemini into the managed Ubuntu runtime."
    : "Install or enable Ubuntu before installing AI CLIs.";
  updateAiCli.title = ubuntuReady
    ? isRunningStartupAiCliEnsure
      ? "Startup AI CLI install/update is in progress."
      : "Update Codex, Claude, and Gemini inside the managed Ubuntu runtime."
    : "Install or enable Ubuntu before updating AI CLIs.";
}

export async function installAiCliRuntime(source: AiCliActionSource = "manual"): Promise<void> {
  if (isInstallingAiCli || isUpdatingAiCli || (source === "manual" && isRunningStartupAiCliEnsure)) {
    return;
  }

  isInstallingAiCli = true;
  updateAiCliActions();
  setStatus(source === "startup"
    ? "installing AI CLIs in managed Ubuntu on startup"
    : "installing AI CLIs in managed Ubuntu");
  writePaneLog(source === "startup"
    ? "startup install: Codex, Claude, and Gemini CLIs in managed Ubuntu"
    : "installing Codex, Claude, and Gemini CLIs in managed Ubuntu");
  try {
    const message = await invoke<string>("ai_cli_install");
    setStatus(message);
    writePaneLog(message, "success");
  } catch (error) {
    setStatus(String(error));
    writePaneLog(`AI CLI install failed: ${String(error)}`, "error");
  } finally {
    isInstallingAiCli = false;
    await loadRuntimeInventory();
  }
}

export async function updateAiCliRuntime(source: AiCliActionSource = "manual"): Promise<void> {
  if (isInstallingAiCli || isUpdatingAiCli || (source === "manual" && isRunningStartupAiCliEnsure)) {
    return;
  }

  isUpdatingAiCli = true;
  updateAiCliActions();
  setStatus(source === "startup"
    ? "updating AI CLIs in managed Ubuntu on startup"
    : "updating AI CLIs in managed Ubuntu");
  writePaneLog(source === "startup"
    ? "startup update: Codex, Claude, and Gemini CLIs in managed Ubuntu"
    : "updating Codex, Claude, and Gemini CLIs in managed Ubuntu");
  try {
    const message = await invoke<string>("ai_cli_update");
    setStatus(message);
    writePaneLog(message, "success");
  } catch (error) {
    setStatus(String(error));
    writePaneLog(`AI CLI update failed: ${String(error)}`, "error");
  } finally {
    isUpdatingAiCli = false;
    await loadRuntimeInventory();
  }
}
