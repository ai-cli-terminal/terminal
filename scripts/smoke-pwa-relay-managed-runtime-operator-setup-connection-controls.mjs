import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import {
  managedRelayRuntimeOperatorSetupConnectionControls,
  relayManagedRuntimeOperatorSetupConnectionControls,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-operator-setup-connection-controls",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_CONNECTION_CONTROLS_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-operator-setup-connection-controls.json",
  );
const desktopScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-connection-controls.png",
);
const mobileScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-connection-controls-mobile.png",
);

let staticServer = null;
let browser = null;

function browserExecutablePath() {
  const candidates = [
    process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE,
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
    "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
    path.join(
      os.homedir(),
      "AppData",
      "Local",
      "Google",
      "Chrome",
      "Application",
      "chrome.exe",
    ),
    path.join(
      os.homedir(),
      "AppData",
      "Local",
      "Microsoft",
      "Edge",
      "Application",
      "msedge.exe",
    ),
  ].filter(Boolean);
  for (const candidate of candidates) {
    if (os.platform() === "win32" && existsSync(candidate)) {
      return candidate;
    }
  }
  return undefined;
}

async function startStaticServer() {
  const contentTypes = new Map([
    [".html", "text/html; charset=utf-8"],
    [".mjs", "text/javascript; charset=utf-8"],
    [".js", "text/javascript; charset=utf-8"],
    [".css", "text/css; charset=utf-8"],
    [".svg", "image/svg+xml"],
    [".webmanifest", "application/manifest+json; charset=utf-8"],
  ]);
  const server = createServer(async (req, res) => {
    try {
      const url = new URL(req.url || "/", "http://127.0.0.1/");
      const relative =
        url.pathname === "/" ? "index.html" : decodeURIComponent(url.pathname.slice(1));
      const filePath = path.resolve(pwaDir, relative);
      if (!filePath.startsWith(`${pwaDir}${path.sep}`)) {
        res.writeHead(403);
        res.end("forbidden");
        return;
      }
      const body = await readFile(filePath);
      res.writeHead(200, {
        "content-type":
          contentTypes.get(path.extname(filePath)) || "application/octet-stream",
      });
      res.end(body);
    } catch {
      res.writeHead(404);
      res.end("not found");
    }
  });
  await new Promise((resolve, reject) => {
    server.on("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  return { server, url: `http://127.0.0.1:${address.port}` };
}

function closeServer(server) {
  return new Promise((resolve) => {
    if (!server) {
      resolve();
      return;
    }
    server.close(() => resolve());
  });
}

function managedSetupPayload() {
  const issuedAtMs = Date.now() - 1000;
  const expiresAtMs = Date.now() + 300000;
  return {
    setup_version: 1,
    deployment_mode: "managed",
    relay_endpoint_url: "wss://managed-relay.example/relay",
    tenant_id: "tenant-managed-relay",
    session_id_hash: "sha256:1111111111111111",
    daemon_device_id_hash: "sha256:2222222222222222",
    companion_device_id_hash: "sha256:3333333333333333",
    verifier_key_id: "managed-relay-key-a",
    verifier_key_version: 1,
    issued_at_ms: issuedAtMs,
    expires_at_ms: expiresAtMs,
    operator_setup_text:
      "Managed relay connection controls setup text stays hidden after import.",
    rollback_transport: "live-loopback",
    setup_label: "managed-connection-controls",
    support_contact: "support-managed-relay",
    not_before_ms: issuedAtMs,
  };
}

async function installWebSocketCounter(page) {
  await page.addInitScript(() => {
    window.__managedRelayWebSocketAttempts = 0;
    const NativeWebSocket = window.WebSocket;
    window.WebSocket = new Proxy(NativeWebSocket, {
      construct(target, args) {
        window.__managedRelayWebSocketAttempts += 1;
        return Reflect.construct(target, args);
      },
    });
  });
}

async function loadManagedRelaySetup(page, url, setupPayload) {
  await page.goto(`${url}/index.html`, { waitUntil: "networkidle" });
  await page.click('[data-mode="relay"]');
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-managed-state")?.textContent ===
      "Explicit opt-in ready",
    null,
    { timeout: 15000 },
  );
  await page.fill("#relay-managed-setup-input", JSON.stringify(setupPayload));
  await page.click("#relay-managed-load-button");
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-managed-import-state")?.textContent === "Ready" &&
      document.querySelector("#relay-managed-connection-state")?.textContent === "Ready" &&
      document.querySelector("#relay-managed-request-connect-button")?.disabled === false,
    null,
    { timeout: 15000 },
  );
}

async function visibleState(page) {
  return page.evaluate(() => ({
    importState:
      document.querySelector("#relay-managed-import-state")?.textContent || "",
    connectionState:
      document.querySelector("#relay-managed-connection-state")?.textContent || "",
    lastEvent: document.querySelector("#relay-managed-last-event")?.textContent || "",
    productDefault:
      document.querySelector("#relay-managed-default-mode")?.textContent || "",
    endpointMode:
      document.querySelector("#relay-managed-endpoint-mode")?.textContent || "",
    publicBind:
      document.querySelector("#relay-managed-public-bind")?.textContent || "",
    autoStart:
      document.querySelector("#relay-managed-auto-start")?.textContent || "",
    rollback: document.querySelector("#relay-managed-rollback")?.textContent || "",
    endpoint:
      document.querySelector("#relay-managed-setup-endpoint")?.textContent || "",
    setupInputValue:
      document.querySelector("#relay-managed-setup-input")?.value || "",
    requestDisabled:
      document.querySelector("#relay-managed-request-connect-button")?.disabled ?? true,
    cancelDisabled:
      document.querySelector("#relay-managed-cancel-connect-button")?.disabled ?? true,
    requestText:
      document.querySelector("#relay-managed-request-connect-button")?.textContent || "",
    cancelText:
      document.querySelector("#relay-managed-cancel-connect-button")?.textContent || "",
    summary:
      document.querySelector("#relay-managed-setup-summary")?.textContent || "",
    bodyText: document.body.textContent || "",
    webSocketAttempts: window.__managedRelayWebSocketAttempts || 0,
    innerWidth: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
}

function assertReadyState(state, summary) {
  assert.equal(state.importState, "Ready");
  assert.equal(state.connectionState, "Ready");
  assert.equal(state.lastEvent, "setup-ready");
  assert.equal(state.productDefault, summary.productDefault);
  assert.equal(state.endpointMode, summary.endpointMode);
  assert.equal(state.publicBind, "off");
  assert.equal(state.autoStart, "off");
  assert.equal(state.rollback, summary.rollbackDefault);
  assert.equal(state.endpoint, "wss://managed-relay.example/relay");
  assert.equal(state.setupInputValue, "Managed setup imported (metadata hidden)");
  assert.equal(state.requestDisabled, false);
  assert.equal(state.cancelDisabled, true);
  assert.equal(state.requestText, summary.connectionControls.requestButtonText);
  assert.equal(state.cancelText, summary.connectionControls.cancelButtonText);
  assert.equal(state.webSocketAttempts, 0);
}

function assertRequestedState(state, summary, setupPayload) {
  assert.equal(
    state.connectionState,
    summary.connectionControls.requestedConnectionState,
  );
  assert.equal(state.lastEvent, summary.connectionControls.requestedLastEvent);
  assert.equal(state.requestDisabled, true);
  assert.equal(state.cancelDisabled, false);
  assert.equal(state.webSocketAttempts, 0);
  assert.equal(state.bodyText.includes(setupPayload.operator_setup_text), false);
  assert.equal(state.bodyText.includes(setupPayload.support_contact), false);
  for (const prohibited of summary.prohibitedVisibleTokens) {
    assert.equal(
      state.bodyText.includes(prohibited),
      false,
      `managed connection controls visible body leaked ${prohibited}`,
    );
  }
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  const summary = relayManagedRuntimeOperatorSetupConnectionControls();
  const setupPayload = managedSetupPayload();
  const readyControls = managedRelayRuntimeOperatorSetupConnectionControls(
    setupPayload,
    { manualConnectRequested: false },
    Date.now(),
  );
  const requestedControls = managedRelayRuntimeOperatorSetupConnectionControls(
    setupPayload,
    { manualConnectRequested: true },
    Date.now(),
  );
  assert.equal(summary.readiness, "operator-setup-connection-controls");
  assert.equal(summary.networkConnectionStartedOnRequest, false);
  assert.equal(summary.endpointAutoStart, false);
  assert.equal(summary.publicBind, false);
  assert.equal(readyControls.connectControlEnabled, true);
  assert.equal(readyControls.disconnectControlEnabled, false);
  assert.equal(requestedControls.connectControlEnabled, false);
  assert.equal(requestedControls.disconnectControlEnabled, true);
  assert.equal(requestedControls.networkConnectionStarted, false);
  assert.equal(requestedControls.webSocketCreated, false);

  const staticInfo = await startStaticServer();
  staticServer = staticInfo.server;
  const executablePath = browserExecutablePath();
  browser = await chromium.launch({
    headless: true,
    executablePath,
  });

  const desktopViewport = summary.connectionControls.requiredViewports.find(
    ({ name }) => name === "desktop",
  );
  const page = await browser.newPage({
    viewport: { width: desktopViewport.width, height: desktopViewport.height },
  });
  await installWebSocketCounter(page);
  await loadManagedRelaySetup(page, staticInfo.url, setupPayload);
  const ready = await visibleState(page);
  assertReadyState(ready, summary);
  await page.click("#relay-managed-request-connect-button");
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-managed-connection-state")?.textContent ===
        "Manual connect requested" &&
      document.querySelector("#relay-managed-cancel-connect-button")?.disabled === false,
    null,
    { timeout: 15000 },
  );
  await page.screenshot({ path: desktopScreenshotPath, fullPage: true });
  const requested = await visibleState(page);
  assertRequestedState(requested, summary, setupPayload);

  await page.click("#relay-managed-cancel-connect-button");
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-managed-connection-state")?.textContent === "Ready" &&
      document.querySelector("#relay-managed-last-event")?.textContent ===
        "manual-connect-cancelled" &&
      document.querySelector("#relay-managed-request-connect-button")?.disabled === false,
    null,
    { timeout: 15000 },
  );
  const cancelled = await visibleState(page);
  assert.equal(cancelled.connectionState, "Ready");
  assert.equal(cancelled.lastEvent, summary.connectionControls.cancelledLastEvent);
  assert.equal(cancelled.requestDisabled, false);
  assert.equal(cancelled.cancelDisabled, true);
  assert.equal(cancelled.webSocketAttempts, 0);

  const mobileViewport = summary.connectionControls.requiredViewports.find(
    ({ name }) => name === "mobile",
  );
  const mobilePage = await browser.newPage({
    viewport: { width: mobileViewport.width, height: mobileViewport.height },
  });
  await installWebSocketCounter(mobilePage);
  await loadManagedRelaySetup(mobilePage, staticInfo.url, setupPayload);
  await mobilePage.click("#relay-managed-request-connect-button");
  await mobilePage.waitForFunction(
    () =>
      document.querySelector("#relay-managed-connection-state")?.textContent ===
      "Manual connect requested",
    null,
    { timeout: 15000 },
  );
  await mobilePage.screenshot({ path: mobileScreenshotPath, fullPage: true });
  const mobile = await visibleState(mobilePage);
  assertRequestedState(mobile, summary, setupPayload);
  assert.ok(
    mobile.scrollWidth <= mobile.innerWidth,
    `mobile layout overflowed: ${mobile.scrollWidth} > ${mobile.innerWidth}`,
  );

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective:
      "Verify managed relay operator setup request/cancel connection controls remain manual and status-only",
    pwaUrl: staticInfo.url,
    browserExecutablePath: executablePath || "playwright-default",
    screenshots: {
      desktop: desktopScreenshotPath,
      mobile: mobileScreenshotPath,
    },
    summary: {
      readiness: summary.readiness,
      implementationStatus: summary.implementationStatus,
      selectedRuntime: summary.selectedRuntime,
      connectionControlMode: summary.connectionControlMode,
      networkConnectionStartedOnRequest:
        summary.networkConnectionStartedOnRequest,
      endpointAutoStart: summary.endpointAutoStart,
      publicBind: summary.publicBind,
      nextLocalSlice: summary.nextLocalSlice,
    },
    setupPayload: {
      ...setupPayload,
      operator_setup_text: "redacted-from-evidence-summary",
      support_contact: "redacted-from-evidence-summary",
    },
    ready: {
      ...ready,
      bodyText: undefined,
    },
    requested: {
      ...requested,
      bodyText: undefined,
    },
    cancelled: {
      ...cancelled,
      bodyText: undefined,
    },
    mobile: {
      ...mobile,
      bodyText: undefined,
    },
    evidenceChecks: summary.evidenceChecks,
    guardrails: summary.guardrails,
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(
    `RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_CONNECTION_CONTROLS_OK ${evidencePath}`,
  );
}

try {
  await main();
} finally {
  if (browser) {
    await browser.close().catch(() => {});
  }
  await closeServer(staticServer);
}
