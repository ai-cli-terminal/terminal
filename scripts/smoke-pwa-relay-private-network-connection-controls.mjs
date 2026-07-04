import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

import {
  createRelaySessionTicket,
  createSignedRelaySessionTicket,
  relaySessionConnect,
  relaySessionConnectJson,
} from "../pwa/app.mjs";
import { createRelayService } from "./relay-self-hosted-service.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-private-network-connection-controls");
const evidencePath =
  process.env.RA_PWA_RELAY_PRIVATE_NETWORK_CONNECTION_CONTROLS_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-private-network-connection-controls.json");
const desktopScreenshotPath = path.join(artifactRoot, "private-network-connection-controls.png");
const mobileScreenshotPath = path.join(artifactRoot, "private-network-connection-controls-mobile.png");

const relayTicketKeyId = "relay-private-controls-hmac";
const relayTicketSecret = "relay-private-controls-secret-1234567890";

let staticServer = null;
let relayService = null;
let browser = null;

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

async function createPrivateNetworkSetup(identity, relayWebSocketUrl) {
  const nowMs = Date.now();
  const ticket = createRelaySessionTicket({
    sessionId: "relay-private-controls-1",
    sessionToken: "token_relay_private_controls_1234567890abcdef",
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
    operatorSetupText: "Private-network relay tailnet-dev endpoint is ready for browser connect controls.",
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
    () =>
      document.querySelector("#relay-private-connection-state")?.textContent === "Connected" &&
      document.querySelector("#relay-private-disconnect-button")?.disabled === false,
    null,
    { timeout: 15000 },
  );
}

async function visibleState(page) {
  return page.evaluate(() => ({
    privateState: document.querySelector("#relay-private-state")?.textContent || "",
    privateConnection: document.querySelector("#relay-private-connection-state")?.textContent || "",
    privateLastEvent: document.querySelector("#relay-private-last-event")?.textContent || "",
    privateDefault: document.querySelector("#relay-private-default-mode")?.textContent || "",
    privateNetwork: document.querySelector("#relay-private-network")?.textContent || "",
    privateDeployment: document.querySelector("#relay-private-deployment")?.textContent || "",
    privateEndpoint: document.querySelector("#relay-private-endpoint")?.textContent || "",
    privateCompanionConnect: document.querySelector("#relay-private-companion-connect")?.textContent || "",
    privateDaemonConnect: document.querySelector("#relay-private-daemon-connect")?.textContent || "",
    privateConnectDisabled: document.querySelector("#relay-private-connect-button")?.disabled ?? true,
    privateDisconnectDisabled: document.querySelector("#relay-private-disconnect-button")?.disabled ?? true,
    relayConnection: document.querySelector("#relay-connection-state")?.textContent || "",
    relayConnectDisabled: document.querySelector("#relay-connect-button")?.disabled ?? true,
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
  assert.equal(registered.body.session_id, privateSetup.signedSessionTicket.ticket.session_id);

  await loadAndConnectPrivateSetup(page, privateSetup);
  await page.screenshot({ path: desktopScreenshotPath, fullPage: true });
  const desktop = await visibleState(page);

  assert.equal(desktop.privateState, "Ready");
  assert.equal(desktop.privateConnection, "Connected");
  assert.equal(desktop.privateLastEvent, "connected");
  assert.equal(desktop.privateDefault, "live-loopback");
  assert.equal(desktop.privateNetwork, "tailnet-dev");
  assert.equal(desktop.privateDeployment, "private-network");
  assert.equal(desktop.privateEndpoint, relayUrls.websocketUrl);
  assert.equal(desktop.privateCompanionConnect, relaySessionConnectJson(privateSetup.companionConnect));
  assert.equal(desktop.privateDaemonConnect, relaySessionConnectJson(privateSetup.daemonConnect));
  assert.equal(desktop.privateConnectDisabled, true);
  assert.equal(desktop.privateDisconnectDisabled, false);
  assert.equal(desktop.relayConnection, "Disconnected");
  assert.equal(desktop.relayConnectDisabled, true);
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
  assert.ok(health.stats.acceptedConnects >= 1);

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective: "Verify explicit private-network relay connect controls use setup-derived browser endpoint loop",
    pwaUrl: staticInfo.url,
    relay: {
      httpUrl: relayUrls.httpUrl,
      websocketUrl: relayUrls.websocketUrl,
      acceptedConnects: health.stats.acceptedConnects,
    },
    browserExecutablePath: executablePath || "playwright-default",
    screenshots: {
      desktop: desktopScreenshotPath,
      mobile: mobileScreenshotPath,
    },
    desktop: {
      ...desktop,
      bodyText: undefined,
    },
    mobile: {
      ...mobile,
      bodyText: undefined,
    },
    nextLocalSlice: "managed-relay-runtime-service-scaffold",
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_PRIVATE_NETWORK_CONNECTION_CONTROLS_OK ${evidencePath}`);
}

try {
  await main();
} finally {
  if (browser) {
    await browser.close().catch(() => {});
  }
  await closeServer(staticServer);
  if (relayService) {
    await relayService.close().catch(() => {});
  }
}
