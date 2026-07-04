import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import {
  createRelayEndpoint,
  createRelaySessionTicket,
  createSignedRelaySessionTicket,
  liveApprovalRequestMessage,
  relayEndpointAcceptFrame,
  relayEndpointNextFrame,
  relayFrameJson,
  relaySessionConnect,
  relaySessionConnectJson,
  relayWebSocketConnectUrl,
} from "../pwa/app.mjs";
import { createRelayService } from "./relay-self-hosted-service.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-private-network-approval-flow-evidence");
const evidencePath =
  process.env.RA_PWA_RELAY_PRIVATE_NETWORK_APPROVAL_FLOW_EVIDENCE_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-private-network-approval-flow-evidence.json");
const pendingScreenshotPath = path.join(artifactRoot, "private-network-approval-pending.png");
const responseScreenshotPath = path.join(artifactRoot, "private-network-approval-responses.png");
const mobileScreenshotPath = path.join(artifactRoot, "private-network-approval-mobile.png");

const relayTicketKeyId = "relay-private-approval-hmac";
const relayTicketSecret = "relay-private-approval-secret-1234567890";

let staticServer = null;
let relayService = null;
let browser = null;
let daemonWs = null;

function browserExecutablePath() {
  const candidates = [
    process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE,
    "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
    "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
    "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
    path.join(os.homedir(), "AppData", "Local", "Google", "Chrome", "Application", "chrome.exe"),
    path.join(os.homedir(), "AppData", "Local", "Microsoft", "Edge", "Application", "msedge.exe"),
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
      const relative = url.pathname === "/" ? "index.html" : decodeURIComponent(url.pathname.slice(1));
      const filePath = path.resolve(pwaDir, relative);
      if (!filePath.startsWith(pwaDir + path.sep)) {
        res.writeHead(403);
        res.end("forbidden");
        return;
      }
      const body = await readFile(filePath);
      res.writeHead(200, {
        "content-type": contentTypes.get(path.extname(filePath)) || "application/octet-stream",
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

async function postJson(url, body) {
  const response = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  return {
    status: response.status,
    body: await response.json(),
  };
}

async function websocketDataText(data) {
  if (typeof data === "string") {
    return data;
  }
  if (data instanceof ArrayBuffer) {
    return new TextDecoder().decode(data);
  }
  if (ArrayBuffer.isView(data)) {
    return new TextDecoder().decode(data);
  }
  if (data && typeof data.text === "function") {
    return data.text();
  }
  return String(data);
}

function openWebSocket(url, timeoutMs = 15000) {
  if (typeof WebSocket !== "function") {
    throw new Error("Node WebSocket is not available");
  }
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    const timer = setTimeout(() => reject(new Error(`websocket open timed out: ${url}`)), timeoutMs);
    ws.addEventListener(
      "open",
      () => {
        clearTimeout(timer);
        resolve(ws);
      },
      { once: true },
    );
    ws.addEventListener(
      "error",
      () => {
        clearTimeout(timer);
        reject(new Error(`websocket open failed: ${url}`));
      },
      { once: true },
    );
  });
}

function waitForWsJson(ws, predicate, timeoutMs = 15000) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      cleanup();
      reject(new Error("websocket message timed out"));
    }, timeoutMs);
    const cleanup = () => {
      clearTimeout(timer);
      ws.removeEventListener("message", onMessage);
      ws.removeEventListener("error", onError);
      ws.removeEventListener("close", onClose);
    };
    const onError = () => {
      cleanup();
      reject(new Error("websocket error"));
    };
    const onClose = () => {
      cleanup();
      reject(new Error("websocket closed"));
    };
    const onMessage = (event) => {
      websocketDataText(event.data)
        .then((text) => {
          const message = JSON.parse(text);
          if (predicate(message)) {
            cleanup();
            resolve(message);
          }
        })
        .catch((error) => {
          cleanup();
          reject(error);
        });
    };
    ws.addEventListener("message", onMessage);
    ws.addEventListener("error", onError);
    ws.addEventListener("close", onClose);
  });
}

async function createPrivateNetworkSetup(identity, relayWebSocketUrl) {
  const nowMs = Date.now();
  const ticket = createRelaySessionTicket({
    sessionId: "relay-private-approval-1",
    sessionToken: "token_relay_private_approval_1234567890abcdef",
    issuedAtMs: nowMs,
    expiresAtMs: nowMs + 300000,
    daemonPubkeyHex: "a".repeat(64),
    companionDeviceId: identity.deviceId,
    companionNoisePubkeyHex: identity.noisePubkeyHex,
    companionApprovalPubkeyHex: identity.approvalPubkeyHex,
  });
  const signedSessionTicket = await createSignedRelaySessionTicket(ticket, relayTicketSecret, globalThis.crypto);
  return {
    relayProtocolVersion: 1,
    transportMode: "relay",
    deploymentMode: "private-network",
    privateNetworkName: "tailnet-dev",
    relayEndpointUrl: relayWebSocketUrl,
    signedSessionTicket: { ...signedSessionTicket, key_id: relayTicketKeyId },
    daemonConnect: relaySessionConnect(ticket, "daemon"),
    companionConnect: relaySessionConnect(ticket, "companion"),
    companionIdentity: identity,
    operatorSetupText: "Private-network relay tailnet-dev endpoint is ready for approval flow evidence.",
  };
}

function approvalRequest(id, commandMasked, contextHash, nonceOffset) {
  return {
    approval_id: Array.from(new TextEncoder().encode(id)),
    nonce: Array.from({ length: 32 }, (_, index) => (nonceOffset + index) % 256),
    command_masked: commandMasked,
    context_hash: contextHash,
    expires_at: Date.now() + 60000,
    device_epoch: 1,
  };
}

async function loadAndConnectPrivateSetup(page, setup) {
  await page.click('[data-mode="relay"]');
  await page.fill("#relay-private-setup-input", JSON.stringify(setup, null, 2));
  await page.click("#relay-private-load-button");
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-private-state")?.textContent === "Ready" &&
      document.querySelector("#relay-private-connect-button")?.disabled === false,
    null,
    { timeout: 15000 },
  );
  await page.click("#relay-private-connect-button");
  await page.waitForFunction(
    () => document.querySelector("#relay-private-connection-state")?.textContent === "Connected",
    null,
    { timeout: 15000 },
  );
}

async function connectDaemon(setup) {
  const url = relayWebSocketConnectUrl(setup.relayEndpointUrl, setup.daemonConnect);
  const ws = await openWebSocket(url);
  const connected = waitForWsJson(
    ws,
    (message) => message.kind === "connected" && message.peer === "daemon",
  );
  ws.send(relaySessionConnectJson(setup.daemonConnect));
  await connected;
  return ws;
}

async function sendApprovalRequest(ws, daemonEndpoint, request) {
  const frame = relayEndpointNextFrame(daemonEndpoint, liveApprovalRequestMessage(request), Date.now());
  const queued = waitForWsJson(
    ws,
    (message) =>
      message.kind === "queued" &&
      message.route?.sender === "daemon" &&
      message.route?.sequence === frame.sequence,
  );
  ws.send(relayFrameJson(frame));
  await queued;
  return frame;
}

async function waitForPrivateApproval(page, request, expectedPending) {
  await page.waitForFunction(
    ({ commandMasked, contextHash, pending }) =>
      document.querySelector("#approval-source")?.textContent === "Private Relay" &&
      document.querySelector("#approval-command")?.textContent === commandMasked &&
      document.querySelector("#approval-context")?.textContent === contextHash &&
      document.querySelector("#relay-private-pending-count")?.textContent === String(pending),
    {
      commandMasked: request.command_masked,
      contextHash: request.context_hash,
      pending: expectedPending,
    },
    { timeout: 15000 },
  );
}

async function clickDecisionAndWaitForDaemonResponse(page, ws, daemonEndpoint, approve) {
  const delivery = waitForWsJson(
    ws,
    (message) => message.kind === "frame" && message.route?.sender === "companion",
  );
  await page.click(approve ? "#approve-button" : "#reject-button");
  const message = relayEndpointAcceptFrame(daemonEndpoint, (await delivery).frame_json, Date.now());
  assert.equal(message.type, "approval_response");
  assert.equal(message.response.approve, approve);
  return message.response;
}

async function visibleState(page) {
  return page.evaluate(() => ({
    privateState: document.querySelector("#relay-private-state")?.textContent || "",
    privateConnection: document.querySelector("#relay-private-connection-state")?.textContent || "",
    privatePending: document.querySelector("#relay-private-pending-count")?.textContent || "",
    privateReceived: document.querySelector("#relay-private-received-count")?.textContent || "",
    privateSent: document.querySelector("#relay-private-sent-count")?.textContent || "",
    privateApproved: document.querySelector("#relay-private-approved-count")?.textContent || "",
    privateRejected: document.querySelector("#relay-private-rejected-count")?.textContent || "",
    approvalSource: document.querySelector("#approval-source")?.textContent || "",
    approvalCommand: document.querySelector("#approval-command")?.textContent || "",
    responseText: document.querySelector("#approval-response")?.textContent || "",
    bodyText: document.body.textContent || "",
    innerWidth: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  relayService = createRelayService({
    host: "127.0.0.1",
    port: 0,
    hmacKeys: [{ keyId: relayTicketKeyId, secret: relayTicketSecret }],
  });
  const relayUrls = await relayService.start();
  const staticInfo = await startStaticServer();
  staticServer = staticInfo.server;
  const executablePath = browserExecutablePath();
  browser = await chromium.launch({
    headless: true,
    executablePath,
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 1120 } });
  await page.goto(`${staticInfo.url}/index.html`, { waitUntil: "networkidle" });
  await page.click("#identity-button");
  await page.waitForFunction(
    () =>
      /^web-[0-9a-f]{8}$/.test(document.querySelector("#device-id")?.value || "") &&
      /^[0-9a-f]{64}$/.test(document.querySelector("#noise-pubkey")?.value || "") &&
      /^[0-9a-f]{64}$/.test(document.querySelector("#approval-pubkey")?.value || ""),
    null,
    { timeout: 15000 },
  );
  const identity = await page.evaluate(() => ({
    deviceId: document.querySelector("#device-id").value,
    noisePubkeyHex: document.querySelector("#noise-pubkey").value,
    approvalPubkeyHex: document.querySelector("#approval-pubkey").value,
  }));

  const privateSetup = await createPrivateNetworkSetup(identity, relayUrls.websocketUrl);
  const registered = await postJson(relayUrls.sessionUrl, privateSetup.signedSessionTicket);
  assert.equal(registered.status, 201);

  await loadAndConnectPrivateSetup(page, privateSetup);
  daemonWs = await connectDaemon(privateSetup);
  const daemonEndpoint = createRelayEndpoint(
    privateSetup.signedSessionTicket.ticket.session_id,
    "daemon",
  );

  const approveRequest = approvalRequest(
    "private-approve-1",
    "rm -rf build",
    "ctx-private-approve",
    11,
  );
  await sendApprovalRequest(daemonWs, daemonEndpoint, approveRequest);
  await waitForPrivateApproval(page, approveRequest, 1);
  await page.screenshot({ path: pendingScreenshotPath, fullPage: true });
  const approveResponse = await clickDecisionAndWaitForDaemonResponse(page, daemonWs, daemonEndpoint, true);
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-private-pending-count")?.textContent === "0" &&
      document.querySelector("#relay-private-approved-count")?.textContent === "1",
    null,
    { timeout: 15000 },
  );

  const rejectRequest = approvalRequest(
    "private-reject-1",
    "cat ~/.ssh/id_rsa",
    "ctx-private-reject",
    73,
  );
  await sendApprovalRequest(daemonWs, daemonEndpoint, rejectRequest);
  await waitForPrivateApproval(page, rejectRequest, 1);
  const rejectResponse = await clickDecisionAndWaitForDaemonResponse(page, daemonWs, daemonEndpoint, false);
  await page.waitForFunction(
    () =>
      document.querySelector("#relay-private-pending-count")?.textContent === "0" &&
      document.querySelector("#relay-private-sent-count")?.textContent === "2" &&
      document.querySelector("#relay-private-approved-count")?.textContent === "1" &&
      document.querySelector("#relay-private-rejected-count")?.textContent === "1",
    null,
    { timeout: 15000 },
  );
  await page.screenshot({ path: responseScreenshotPath, fullPage: true });
  const desktop = await visibleState(page);

  assert.equal(desktop.privateState, "Ready");
  assert.equal(desktop.privateConnection, "Connected");
  assert.equal(desktop.privatePending, "0");
  assert.equal(desktop.privateReceived, "2");
  assert.equal(desktop.privateSent, "2");
  assert.equal(desktop.privateApproved, "1");
  assert.equal(desktop.privateRejected, "1");
  assert.equal(desktop.approvalSource, "Private Relay");
  assert.equal(desktop.approvalCommand, rejectRequest.command_masked);
  assert.equal(JSON.parse(desktop.responseText).approve, false);
  assert.equal(approveResponse.approve, true);
  assert.equal(rejectResponse.approve, false);
  assert.equal(desktop.bodyText.includes(relayTicketSecret), false);

  await page.setViewportSize({ width: 390, height: 844 });
  await page.screenshot({ path: mobileScreenshotPath, fullPage: true });
  const mobile = await visibleState(page);
  assert.equal(mobile.privateConnection, "Connected");
  assert.ok(
    mobile.scrollWidth <= mobile.innerWidth,
    `mobile layout overflowed: ${mobile.scrollWidth} > ${mobile.innerWidth}`,
  );

  const health = await fetch(relayUrls.healthUrl).then((response) => response.json());
  assert.equal(health.status, "ok");
  assert.ok(health.stats.acceptedFrames >= 4);
  assert.ok(health.stats.deliveredFrames >= 4);

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective: "Verify private-network relay approval request and response evidence in the browser",
    pwaUrl: staticInfo.url,
    relay: {
      httpUrl: relayUrls.httpUrl,
      websocketUrl: relayUrls.websocketUrl,
      acceptedFrames: health.stats.acceptedFrames,
      deliveredFrames: health.stats.deliveredFrames,
    },
    browserExecutablePath: executablePath || "playwright-default",
    screenshots: {
      pending: pendingScreenshotPath,
      responses: responseScreenshotPath,
      mobile: mobileScreenshotPath,
    },
    approvals: {
      approved: approveResponse.approve,
      rejected: rejectResponse.approve === false,
    },
    desktop: {
      ...desktop,
      bodyText: undefined,
    },
    mobile: {
      ...mobile,
      bodyText: undefined,
    },
    nextLocalSlice: "managed-relay-billing-abuse-boundary-review",
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_PRIVATE_NETWORK_APPROVAL_FLOW_EVIDENCE_OK ${evidencePath}`);
}

try {
  await main();
} finally {
  if (daemonWs) {
    daemonWs.close();
  }
  if (browser) {
    await browser.close().catch(() => {});
  }
  await closeServer(staticServer);
  if (relayService) {
    await relayService.close().catch(() => {});
  }
}
