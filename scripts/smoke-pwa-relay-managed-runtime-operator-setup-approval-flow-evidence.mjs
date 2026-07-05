import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import {
  managedRelayRuntimeOperatorSetupApprovalFlowEvidence,
  relayManagedRuntimeOperatorSetupApprovalFlowEvidence,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-operator-setup-approval-flow-evidence",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_APPROVAL_FLOW_EVIDENCE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-operator-setup-approval-flow-evidence.json",
  );
const pendingScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-approval-flow-evidence.png",
);
const responseScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-approval-flow-evidence-responses.png",
);
const mobileScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-approval-flow-evidence-mobile.png",
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
      "Managed relay approval flow setup text stays hidden after import.",
    rollback_transport: "live-loopback",
    setup_label: "managed-approval-flow",
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

async function generateIdentity(page) {
  await page.click("#identity-button");
  await page.waitForFunction(
    () =>
      /^web-[0-9a-f]{8}$/.test(document.querySelector("#device-id")?.value || "") &&
      /^[0-9a-f]{64}$/.test(document.querySelector("#noise-pubkey")?.value || "") &&
      /^[0-9a-f]{64}$/.test(document.querySelector("#approval-pubkey")?.value || ""),
    null,
    { timeout: 15000 },
  );
  return page.evaluate(() => ({
    deviceId: document.querySelector("#device-id").value,
    noisePubkeyHex: document.querySelector("#noise-pubkey").value,
    approvalPubkeyHex: document.querySelector("#approval-pubkey").value,
  }));
}

async function loadManagedApproval(page, url, setupPayload) {
  await page.goto(`${url}/index.html`, { waitUntil: "networkidle" });
  await generateIdentity(page);
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
      document.querySelector("#relay-managed-request-connect-button")?.disabled === false,
    null,
    { timeout: 15000 },
  );
  await page.click("#relay-managed-request-connect-button");
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-managed-connection-state")?.textContent ===
        "Manual connect requested" &&
      document.querySelector("#relay-managed-start-handshake-button")?.disabled === false,
    null,
    { timeout: 15000 },
  );
  await page.click("#relay-managed-start-handshake-button");
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-managed-handshake-state")?.textContent ===
        "Handshake ready" &&
      document.querySelector("#relay-managed-load-approval-button")?.disabled === false,
    null,
    { timeout: 15000 },
  );
  await page.click("#relay-managed-load-approval-button");
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-managed-approval-state")?.textContent ===
        "Approval request ready" &&
      document.querySelector("#approval-source")?.textContent === "Managed Relay" &&
      document.querySelector("#approval-command")?.textContent ===
        "managed relay approval evidence command",
    null,
    { timeout: 15000 },
  );
}

async function clickDecision(page, approve) {
  await page.click(approve ? "#approve-button" : "#reject-button");
  await page.waitForFunction(
    (expectedApprove) => {
      const raw = document.querySelector("#approval-response")?.textContent || "";
      try {
        const response = JSON.parse(raw);
        return (
          response.approve === expectedApprove &&
          Array.isArray(response.sig) &&
          response.sig.length === 64 &&
          document.querySelector("#approval-verify-command")?.textContent.includes(
            "ai remote approval-verify",
          )
        );
      } catch {
        return false;
      }
    },
    approve,
    { timeout: 15000 },
  );
  return page.evaluate(() =>
    JSON.parse(document.querySelector("#approval-response")?.textContent || "{}"),
  );
}

async function visibleState(page) {
  return page.evaluate(() => ({
    importState:
      document.querySelector("#relay-managed-import-state")?.textContent || "",
    connectionState:
      document.querySelector("#relay-managed-connection-state")?.textContent || "",
    handshakeState:
      document.querySelector("#relay-managed-handshake-state")?.textContent || "",
    approvalState:
      document.querySelector("#relay-managed-approval-state")?.textContent || "",
    managedApprovalSource:
      document.querySelector("#relay-managed-approval-source")?.textContent || "",
    managedApprovalContext:
      document.querySelector("#relay-managed-approval-context")?.textContent || "",
    approvalSource: document.querySelector("#approval-source")?.textContent || "",
    approvalCommand: document.querySelector("#approval-command")?.textContent || "",
    approvalContext: document.querySelector("#approval-context")?.textContent || "",
    responseText: document.querySelector("#approval-response")?.textContent || "",
    verifyCommand:
      document.querySelector("#approval-verify-command")?.textContent || "",
    lastEvent: document.querySelector("#relay-managed-last-event")?.textContent || "",
    capabilityHandle:
      document.querySelector("#relay-managed-capability-handle")?.textContent || "",
    transcript:
      document.querySelector("#relay-managed-handshake-transcript")?.textContent || "",
    publicBind:
      document.querySelector("#relay-managed-public-bind")?.textContent || "",
    autoStart:
      document.querySelector("#relay-managed-auto-start")?.textContent || "",
    loadApprovalDisabled:
      document.querySelector("#relay-managed-load-approval-button")?.disabled ?? true,
    bodyText: document.body.textContent || "",
    webSocketAttempts: window.__managedRelayWebSocketAttempts || 0,
    innerWidth: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
}

function assertApprovalState(state, summary, setupPayload) {
  assert.equal(state.importState, "Ready");
  assert.equal(state.connectionState, "Manual connect requested");
  assert.equal(state.handshakeState, "Handshake ready");
  assert.equal(
    state.approvalState,
    summary.approvalFlow.expectedVisibleText.managedApprovalState,
  );
  assert.equal(state.managedApprovalSource, "Managed Relay");
  assert.equal(state.approvalSource, "Managed Relay");
  assert.equal(state.approvalCommand, "managed relay approval evidence command");
  assert.match(state.managedApprovalContext, /^sha256:[0-9a-f]{64}$/);
  assert.equal(state.approvalContext, state.managedApprovalContext);
  assert.equal(state.lastEvent, "approval-request-ready");
  assert.match(state.capabilityHandle, /^managed-cap:[0-9a-f]{24}$/);
  assert.match(state.transcript, /^sha256:[0-9a-f]{64}$/);
  assert.equal(state.publicBind, "off");
  assert.equal(state.autoStart, "off");
  assert.equal(state.loadApprovalDisabled, true);
  assert.equal(state.webSocketAttempts, 0);
  assert.equal(state.bodyText.includes(setupPayload.operator_setup_text), false);
  assert.equal(state.bodyText.includes(setupPayload.support_contact), false);
  for (const prohibited of summary.prohibitedVisibleTokens) {
    assert.equal(
      state.bodyText.includes(prohibited),
      false,
      `managed approval flow visible body leaked ${prohibited}`,
    );
  }
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  const summary = relayManagedRuntimeOperatorSetupApprovalFlowEvidence();
  const setupPayload = managedSetupPayload();
  const expectedApproval =
    await managedRelayRuntimeOperatorSetupApprovalFlowEvidence(
      setupPayload,
      { manualConnectRequested: true },
      Date.now(),
      webcrypto,
    );
  assert.equal(summary.readiness, "operator-setup-approval-flow-evidence");
  assert.equal(summary.approvalFlowMode, "manual-approval-request-via-session-capability");
  assert.equal(expectedApproval.status, "approval-flow-ready");
  assert.equal(expectedApproval.approvalFlowReady, true);
  assert.equal(expectedApproval.networkConnectionStarted, false);
  assert.equal(expectedApproval.webSocketCreated, false);

  const staticInfo = await startStaticServer();
  staticServer = staticInfo.server;
  const executablePath = browserExecutablePath();
  browser = await chromium.launch({
    headless: true,
    executablePath,
  });

  const desktopViewport = summary.approvalFlow.requiredViewports.find(
    ({ name }) => name === "desktop",
  );
  const page = await browser.newPage({
    viewport: { width: desktopViewport.width, height: desktopViewport.height },
  });
  await installWebSocketCounter(page);
  await loadManagedApproval(page, staticInfo.url, setupPayload);
  await page.screenshot({ path: pendingScreenshotPath, fullPage: true });
  const pending = await visibleState(page);
  assertApprovalState(pending, summary, setupPayload);

  const approveResponse = await clickDecision(page, true);
  assert.equal(approveResponse.approve, true);
  const rejectResponse = await clickDecision(page, false);
  assert.equal(rejectResponse.approve, false);
  await page.screenshot({ path: responseScreenshotPath, fullPage: true });
  const responses = await visibleState(page);
  assertApprovalState(responses, summary, setupPayload);
  assert.equal(JSON.parse(responses.responseText).approve, false);
  assert.ok(responses.verifyCommand.includes("ai remote approval-verify"));

  const mobileViewport = summary.approvalFlow.requiredViewports.find(
    ({ name }) => name === "mobile",
  );
  const mobilePage = await browser.newPage({
    viewport: { width: mobileViewport.width, height: mobileViewport.height },
  });
  await installWebSocketCounter(mobilePage);
  await loadManagedApproval(mobilePage, staticInfo.url, setupPayload);
  await mobilePage.screenshot({ path: mobileScreenshotPath, fullPage: true });
  const mobile = await visibleState(mobilePage);
  assertApprovalState(mobile, summary, setupPayload);
  assert.ok(
    mobile.scrollWidth <= mobile.innerWidth,
    `mobile layout overflowed: ${mobile.scrollWidth} > ${mobile.innerWidth}`,
  );

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective:
      "Verify managed relay operator setup approval flow uses the session capability boundary without network start",
    pwaUrl: staticInfo.url,
    browserExecutablePath: executablePath || "playwright-default",
    screenshots: {
      pending: pendingScreenshotPath,
      responses: responseScreenshotPath,
      mobile: mobileScreenshotPath,
    },
    summary: {
      readiness: summary.readiness,
      implementationStatus: summary.implementationStatus,
      selectedRuntime: summary.selectedRuntime,
      approvalFlowMode: summary.approvalFlowMode,
      approvalResponseDelivery: summary.approvalResponseDelivery,
      networkConnectionStartedOnApproval:
        summary.networkConnectionStartedOnApproval,
      webSocketCreatedOnApproval: summary.webSocketCreatedOnApproval,
      endpointAutoStart: summary.endpointAutoStart,
      publicBind: summary.publicBind,
      nextLocalSlice: summary.nextLocalSlice,
    },
    expectedApproval: {
      status: expectedApproval.status,
      approvalFlowReady: expectedApproval.approvalFlowReady,
      approvalSourceText: expectedApproval.approvalSourceText,
      approvalResponseDelivery: expectedApproval.approvalResponseDelivery,
      capabilityEnvelopeVisible: expectedApproval.capabilityEnvelopeVisible,
      signedTicketVisible: expectedApproval.signedTicketVisible,
      rawTokenVisible: expectedApproval.rawTokenVisible,
      payloadVisible: expectedApproval.payloadVisible,
      privateKeyMaterialVisible: expectedApproval.privateKeyMaterialVisible,
      networkConnectionStarted: expectedApproval.networkConnectionStarted,
      webSocketCreated: expectedApproval.webSocketCreated,
    },
    setupPayload: {
      ...setupPayload,
      operator_setup_text: "redacted-from-evidence-summary",
      support_contact: "redacted-from-evidence-summary",
    },
    pending: {
      ...pending,
      bodyText: undefined,
    },
    responses: {
      ...responses,
      bodyText: undefined,
    },
    mobile: {
      ...mobile,
      bodyText: undefined,
    },
    approveResponse: {
      approve: approveResponse.approve,
      sigLength: approveResponse.sig.length,
    },
    rejectResponse: {
      approve: rejectResponse.approve,
      sigLength: rejectResponse.sig.length,
    },
    evidenceChecks: summary.evidenceChecks,
    guardrails: summary.guardrails,
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(
    `RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_APPROVAL_FLOW_EVIDENCE_OK ${evidencePath}`,
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
