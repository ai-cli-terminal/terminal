import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import {
  relayManagedRuntimeOperatorSetupApprovalResponseEndpointBrowserEvidence,
  relayManagedRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-browser-evidence",
);
const evidencePath =
  process.env
    .RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_APPROVAL_RESPONSE_ENDPOINT_BROWSER_EVIDENCE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-browser-evidence.json",
  );
const pendingScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-approval-response-endpoint-browser-evidence.png",
);
const deliveredScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-approval-response-endpoint-browser-evidence-delivered.png",
);
const mobileScreenshotPath = path.join(
  artifactRoot,
  "managed-relay-operator-setup-approval-response-endpoint-browser-evidence-mobile.png",
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
      "Managed relay endpoint browser evidence setup text stays hidden after import.",
    rollback_transport: "live-loopback",
    setup_label: "managed-endpoint-browser-evidence",
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
      document.querySelector("#approval-source")?.textContent === "Managed Relay",
    null,
    { timeout: 15000 },
  );
}

async function signManagedResponse(page, approve) {
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
          document.querySelector("#relay-managed-endpoint-delivery-state")?.textContent ===
            "Ready to deliver" &&
          document.querySelector("#relay-managed-deliver-approval-button")?.disabled === false
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

async function deliverManagedResponse(page) {
  await page.click("#relay-managed-deliver-approval-button");
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-managed-endpoint-delivery-state")?.textContent ===
        "Endpoint delivery ready" &&
      document.querySelector("#relay-managed-endpoint-delivery-route")?.textContent ===
        "encrypted frame routed" &&
      document.querySelector("#relay-managed-endpoint-delivery-daemon")?.textContent ===
        "response received",
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
    handshakeState:
      document.querySelector("#relay-managed-handshake-state")?.textContent || "",
    approvalState:
      document.querySelector("#relay-managed-approval-state")?.textContent || "",
    endpointDeliveryState:
      document.querySelector("#relay-managed-endpoint-delivery-state")?.textContent || "",
    endpointRoute:
      document.querySelector("#relay-managed-endpoint-delivery-route")?.textContent || "",
    endpointFallback:
      document.querySelector("#relay-managed-endpoint-delivery-fallback")?.textContent || "",
    endpointDaemon:
      document.querySelector("#relay-managed-endpoint-delivery-daemon")?.textContent || "",
    managedApprovalSource:
      document.querySelector("#relay-managed-approval-source")?.textContent || "",
    managedApprovalContext:
      document.querySelector("#relay-managed-approval-context")?.textContent || "",
    approvalSource: document.querySelector("#approval-source")?.textContent || "",
    approvalCommand: document.querySelector("#approval-command")?.textContent || "",
    responseText: document.querySelector("#approval-response")?.textContent || "",
    verifyCommand:
      document.querySelector("#approval-verify-command")?.textContent || "",
    lastEvent: document.querySelector("#relay-managed-last-event")?.textContent || "",
    publicBind:
      document.querySelector("#relay-managed-public-bind")?.textContent || "",
    autoStart:
      document.querySelector("#relay-managed-auto-start")?.textContent || "",
    deliverButtonDisabled:
      document.querySelector("#relay-managed-deliver-approval-button")?.disabled ?? true,
    copyResponseButtonVisible:
      document.querySelector("#copy-response-button") !== null,
    managedSetupSummary:
      document.querySelector("#relay-managed-setup-summary")?.textContent || "",
    managedSetupCopy:
      document.querySelector("#relay-managed-copy")?.textContent || "",
    bodyText: document.body.textContent || "",
    webSocketAttempts: window.__managedRelayWebSocketAttempts || 0,
    innerWidth: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
}

function assertPendingState(state, summary, setupPayload) {
  assert.equal(state.importState, "Ready");
  assert.equal(state.connectionState, "Manual connect requested");
  assert.equal(state.handshakeState, "Handshake ready");
  assert.equal(
    state.approvalState,
    summary.browserEvidence.expectedVisibleText.approvalState,
  );
  assert.equal(state.endpointDeliveryState, "Ready to deliver");
  assert.equal(state.endpointRoute, "-");
  assert.equal(state.endpointFallback, "Manual copy fallback available");
  assert.equal(state.endpointDaemon, "-");
  assert.equal(state.approvalSource, "Managed Relay");
  assert.equal(state.managedApprovalSource, "Managed Relay");
  assert.match(state.managedApprovalContext, /^sha256:[0-9a-f]{64}$/);
  assert.equal(state.publicBind, "off");
  assert.equal(state.autoStart, "off");
  assert.equal(state.deliverButtonDisabled, false);
  assert.equal(state.copyResponseButtonVisible, true);
  assert.ok(state.verifyCommand.includes("ai remote approval-verify"));
  assert.equal(state.webSocketAttempts, summary.browserEvidence.browserDirectWebSocketAttempts);
  assert.equal(state.bodyText.includes(setupPayload.operator_setup_text), false);
  assert.equal(state.bodyText.includes(setupPayload.support_contact), false);
  for (const prohibited of summary.prohibitedVisibleTokens) {
    assert.equal(
      state.bodyText.includes(prohibited),
      false,
      `managed endpoint pending browser body leaked ${prohibited}`,
    );
  }
}

function assertDeliveredState(state, summary, setupPayload) {
  const expected = summary.browserEvidence.expectedVisibleText;
  assert.equal(state.importState, expected.importState);
  assert.equal(state.connectionState, expected.connectionState);
  assert.equal(state.handshakeState, expected.handshakeState);
  assert.equal(state.approvalState, expected.approvalState);
  assert.equal(state.endpointDeliveryState, expected.endpointDeliveryReadyState);
  assert.equal(state.endpointRoute, expected.endpointRoute);
  assert.equal(state.endpointFallback, expected.endpointFallback);
  assert.equal(state.endpointDaemon, expected.endpointDaemon);
  assert.equal(state.lastEvent, expected.lastEvent);
  assert.equal(state.approvalSource, expected.approvalSource);
  assert.equal(state.managedApprovalSource, expected.approvalSource);
  assert.equal(state.publicBind, "off");
  assert.equal(state.autoStart, "off");
  assert.equal(state.deliverButtonDisabled, true);
  assert.equal(state.copyResponseButtonVisible, true);
  assert.ok(state.verifyCommand.includes("ai remote approval-verify"));
  assert.equal(state.webSocketAttempts, summary.browserEvidence.browserDirectWebSocketAttempts);
  assert.equal(state.managedSetupSummary.includes("approval_response"), false);
  assert.equal(state.managedSetupSummary.includes("payload_key"), false);
  assert.equal(state.managedSetupCopy.includes("approval_response"), false);
  assert.equal(state.bodyText.includes(setupPayload.operator_setup_text), false);
  assert.equal(state.bodyText.includes(setupPayload.support_contact), false);
  for (const prohibited of summary.prohibitedVisibleTokens) {
    assert.equal(
      state.bodyText.includes(prohibited),
      false,
      `managed endpoint delivered browser body leaked ${prohibited}`,
    );
  }
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  const endpointDeliverySummary =
    relayManagedRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence();
  const summary =
    relayManagedRuntimeOperatorSetupApprovalResponseEndpointBrowserEvidence();
  const setupPayload = managedSetupPayload();
  assert.equal(
    endpointDeliverySummary.nextLocalSlice,
    "managed-relay-runtime-operator-setup-approval-response-endpoint-browser-evidence",
  );
  assert.equal(summary.readiness, "operator-setup-approval-response-endpoint-browser-evidence");
  assert.equal(summary.endpointAutoStart, false);
  assert.equal(summary.publicBind, false);
  assert.equal(summary.browserEvidence.browserDirectWebSocketAttempts, 0);

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
  await installWebSocketCounter(page);
  await loadManagedApproval(page, staticInfo.url, setupPayload);
  const approveResponse = await signManagedResponse(page, true);
  assert.equal(approveResponse.approve, true);
  await page.screenshot({ path: pendingScreenshotPath, fullPage: true });
  const pending = await visibleState(page);
  assertPendingState(pending, summary, setupPayload);
  await deliverManagedResponse(page);
  await page.screenshot({ path: deliveredScreenshotPath, fullPage: true });
  const delivered = await visibleState(page);
  assertDeliveredState(delivered, summary, setupPayload);

  const mobileViewport = summary.browserEvidence.requiredViewports.find(
    ({ name }) => name === "mobile",
  );
  const mobilePage = await browser.newPage({
    viewport: { width: mobileViewport.width, height: mobileViewport.height },
  });
  await installWebSocketCounter(mobilePage);
  await loadManagedApproval(mobilePage, staticInfo.url, setupPayload);
  const rejectResponse = await signManagedResponse(mobilePage, false);
  assert.equal(rejectResponse.approve, false);
  await deliverManagedResponse(mobilePage);
  await mobilePage.screenshot({ path: mobileScreenshotPath, fullPage: true });
  const mobile = await visibleState(mobilePage);
  assertDeliveredState(mobile, summary, setupPayload);
  assert.ok(
    mobile.scrollWidth <= mobile.innerWidth,
    `mobile layout overflowed: ${mobile.scrollWidth} > ${mobile.innerWidth}`,
  );

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective:
      "Capture browser/operator evidence for managed approval response endpoint delivery while keeping route payloads hidden",
    pwaUrl: staticInfo.url,
    browserExecutablePath: executablePath || "playwright-default",
    screenshots: {
      pending: pendingScreenshotPath,
      delivered: deliveredScreenshotPath,
      mobile: mobileScreenshotPath,
    },
    endpointDeliverySummary: {
      readiness: endpointDeliverySummary.readiness,
      implementationStatus: endpointDeliverySummary.implementationStatus,
      deliveryMode: endpointDeliverySummary.deliveryMode,
      networkDeliveryStatus: endpointDeliverySummary.networkDeliveryStatus,
      nextLocalSlice: endpointDeliverySummary.nextLocalSlice,
    },
    summary: {
      readiness: summary.readiness,
      implementationStatus: summary.implementationStatus,
      selectedRuntime: summary.selectedRuntime,
      deliveryMode: summary.deliveryMode,
      approvalResponseDelivery: summary.approvalResponseDelivery,
      manualCopyFallback: summary.manualCopyFallback,
      networkDeliveryStatus: summary.networkDeliveryStatus,
      endpointAutoStart: summary.endpointAutoStart,
      publicBind: summary.publicBind,
      nextLocalSlice: summary.nextLocalSlice,
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
    delivered: {
      ...delivered,
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
    `RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_APPROVAL_RESPONSE_ENDPOINT_BROWSER_EVIDENCE_OK ${evidencePath}`,
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
