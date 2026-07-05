import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import {
  managedRelayRuntimeOperatorSetupImportPreflight,
  relayManagedRuntimeOperatorSetupBrowserEvidence,
  relayManagedRuntimeOperatorSetupImportPreflight,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-operator-setup-browser-evidence",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_BROWSER_EVIDENCE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-operator-setup-browser-evidence.json",
  );
const desktopScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-browser-evidence.png",
);
const mobileScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-browser-evidence-mobile.png",
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
      "Managed relay browser evidence setup text stays hidden after import.",
    rollback_transport: "live-loopback",
    setup_label: "managed-browser-evidence",
    support_contact: "support-managed-relay",
    not_before_ms: issuedAtMs,
  };
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
    () => document.querySelector("#relay-managed-import-state")?.textContent === "Ready",
    null,
    { timeout: 15000 },
  );
}

async function managedRelaySetupVisibleState(page) {
  return page.evaluate(() => ({
    state: document.querySelector("#relay-managed-state")?.textContent || "",
    importState:
      document.querySelector("#relay-managed-import-state")?.textContent || "",
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
    tenant: document.querySelector("#relay-managed-tenant")?.textContent || "",
    sessionHash:
      document.querySelector("#relay-managed-session-hash")?.textContent || "",
    daemonHash:
      document.querySelector("#relay-managed-daemon-hash")?.textContent || "",
    companionHash:
      document.querySelector("#relay-managed-companion-hash")?.textContent || "",
    verifierKey:
      document.querySelector("#relay-managed-verifier-key")?.textContent || "",
    activation:
      document.querySelector("#relay-managed-activation")?.textContent || "",
    blockers: Array.from(
      document.querySelectorAll("#relay-managed-setup-blocker-list li"),
      (item) => item.textContent || "",
    ),
    summary:
      document.querySelector("#relay-managed-setup-summary")?.textContent || "",
    evidence: Array.from(
      document.querySelectorAll("#relay-managed-evidence-list li"),
      (item) => item.textContent || "",
    ),
    setupInputValue:
      document.querySelector("#relay-managed-setup-input")?.value || "",
    managedConnectButtonMissing:
      document.querySelector("#relay-managed-connect-button") === null,
    relayConnectButtonDisabled:
      document.querySelector("#relay-connect-button")?.disabled === true,
    bodyText: document.body.textContent || "",
    innerWidth: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
}

function assertVisibleState(state, summary, setupPayload) {
  const expected = summary.browserEvidence.expectedVisibleText;
  assert.equal(state.state, expected.state);
  assert.equal(state.importState, expected.importState);
  assert.equal(state.productDefault, expected.productDefault);
  assert.equal(state.endpointMode, summary.endpointMode);
  assert.equal(state.publicBind, "off");
  assert.equal(state.autoStart, "off");
  assert.equal(state.rollback, summary.rollbackDefault);
  assert.equal(state.endpoint, expected.endpoint);
  assert.equal(state.tenant, expected.tenant);
  assert.equal(state.sessionHash, expected.sessionHash);
  assert.equal(state.daemonHash, expected.daemonHash);
  assert.equal(state.companionHash, expected.companionHash);
  assert.equal(state.verifierKey, expected.verifierKey);
  assert.equal(state.activation, expected.activation);
  assert.equal(state.setupInputValue, expected.setupInputAfterLoad);
  assert.deepEqual(state.blockers, [expected.blocker]);
  assert.deepEqual(state.evidence, ["Managed relay explicit opt-in exposure ready"]);
  assert.equal(state.managedConnectButtonMissing, true);
  assert.equal(state.relayConnectButtonDisabled, true);
  for (const line of expected.summaryLines) {
    assert.ok(state.summary.includes(line), `managed summary missing ${line}`);
  }
  for (const prohibited of summary.prohibitedVisibleTokens) {
    assert.equal(
      state.bodyText.includes(prohibited),
      false,
      `managed setup visible body leaked ${prohibited}`,
    );
  }
  assert.equal(
    state.bodyText.includes(setupPayload.operator_setup_text),
    false,
    "managed setup visible body leaked operator setup text",
  );
  assert.equal(
    state.bodyText.includes(setupPayload.support_contact),
    false,
    "managed setup visible body leaked support contact",
  );
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  const importSummary = relayManagedRuntimeOperatorSetupImportPreflight();
  const summary = relayManagedRuntimeOperatorSetupBrowserEvidence();
  const setupPayload = managedSetupPayload();
  const importPreflight = managedRelayRuntimeOperatorSetupImportPreflight(
    setupPayload,
    Date.now(),
  );
  assert.equal(importSummary.nextLocalSlice, summary.importPreflight.nextLocalSlice);
  assert.equal(summary.readiness, "operator-setup-browser-evidence");
  assert.equal(summary.productDefault, "live-loopback");
  assert.equal(summary.endpointAutoStart, false);
  assert.equal(summary.publicBind, false);
  assert.equal(importPreflight.status, "ready");
  assert.equal(importPreflight.importReady, true);
  assert.equal(importPreflight.connectEnabled, false);

  const staticInfo = await startStaticServer();
  staticServer = staticInfo.server;
  const executablePath = browserExecutablePath();
  browser = await chromium.launch({
    headless: true,
    executablePath,
  });

  const desktopViewport = summary.browserEvidence.requiredViewports.find(
    ({ name }) => name === "desktop",
  );
  const page = await browser.newPage({
    viewport: { width: desktopViewport.width, height: desktopViewport.height },
  });
  await loadManagedRelaySetup(page, staticInfo.url, setupPayload);
  await page.screenshot({ path: desktopScreenshotPath, fullPage: true });
  const desktop = await managedRelaySetupVisibleState(page);
  assertVisibleState(desktop, summary, setupPayload);

  const mobileViewport = summary.browserEvidence.requiredViewports.find(
    ({ name }) => name === "mobile",
  );
  const mobilePage = await browser.newPage({
    viewport: { width: mobileViewport.width, height: mobileViewport.height },
  });
  await loadManagedRelaySetup(mobilePage, staticInfo.url, setupPayload);
  await mobilePage.screenshot({ path: mobileScreenshotPath, fullPage: true });
  const mobile = await managedRelaySetupVisibleState(mobilePage);
  assertVisibleState(mobile, summary, setupPayload);
  assert.ok(
    mobile.scrollWidth <= mobile.innerWidth,
    `mobile layout overflowed: ${mobile.scrollWidth} > ${mobile.innerWidth}`,
  );

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective:
      "Capture managed relay operator setup import browser evidence before connection controls",
    pwaUrl: staticInfo.url,
    browserExecutablePath: executablePath || "playwright-default",
    screenshots: {
      desktop: desktopScreenshotPath,
      mobile: mobileScreenshotPath,
    },
    importSummary: {
      readiness: importSummary.readiness,
      implementationStatus: importSummary.implementationStatus,
      setupRendering: importSummary.setupRendering,
      endpointAutoStart: importSummary.endpointAutoStart,
      publicBind: importSummary.publicBind,
      nextLocalSlice: importSummary.nextLocalSlice,
    },
    summary: {
      readiness: summary.readiness,
      implementationStatus: summary.implementationStatus,
      selectedRuntime: summary.selectedRuntime,
      pwaExposureDecision: summary.pwaExposureDecision,
      endpointAutoStart: summary.endpointAutoStart,
      publicBind: summary.publicBind,
      nextLocalSlice: summary.nextLocalSlice,
    },
    setupPayload: {
      ...setupPayload,
      operator_setup_text: "redacted-from-evidence-summary",
      support_contact: "redacted-from-evidence-summary",
    },
    desktop: {
      ...desktop,
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
    `RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_BROWSER_EVIDENCE_OK ${evidencePath}`,
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
