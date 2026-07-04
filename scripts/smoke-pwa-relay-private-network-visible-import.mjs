import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-private-network-visible-import");
const evidencePath =
  process.env.RA_PWA_RELAY_PRIVATE_NETWORK_VISIBLE_IMPORT_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-private-network-visible-import.json");
const desktopScreenshotPath = path.join(artifactRoot, "private-network-visible-import.png");
const mobileScreenshotPath = path.join(artifactRoot, "private-network-visible-import-mobile.png");

let staticServer = null;
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

async function buildRelaySetupJson(page, deploymentMode) {
  return page.evaluate(async (mode) => {
    const app = await import(new URL("./app.mjs", window.location.href).href);
    const nowMs = Date.now();
    const companionIdentity = {
      deviceId: mode === "private-network" ? "web-private-visible" : "web-setup01",
      noisePubkeyHex: "b".repeat(64),
      approvalPubkeyHex: "c".repeat(64),
    };
    const ticket = app.createRelaySessionTicket({
      sessionId: mode === "private-network" ? "relay-private-visible-1" : "relay-setup-ui-1",
      sessionToken:
        mode === "private-network"
          ? "token_relay_private_visible_1234567890abcdef"
          : "token_relay_setup_ui_1234567890abcdef123456",
      issuedAtMs: nowMs,
      expiresAtMs: nowMs + 300000,
      daemonPubkeyHex: "a".repeat(64),
      companionDeviceId: companionIdentity.deviceId,
      companionNoisePubkeyHex: companionIdentity.noisePubkeyHex,
      companionApprovalPubkeyHex: companionIdentity.approvalPubkeyHex,
    });
    const signedSessionTicket = await app.createSignedRelaySessionTicket(
      ticket,
      "relay-ticket-secret-1234567890abcdef",
      window.crypto,
    );
    const setup = {
      relayProtocolVersion: 1,
      transportMode: "relay",
      deploymentMode: mode,
      relayEndpointUrl:
        mode === "private-network"
          ? "wss://relay.tailnet.example/relay"
          : "wss://relay.example.test/session",
      signedSessionTicket: { ...signedSessionTicket, key_id: "relay-active-1" },
      daemonConnect: app.relaySessionConnect(ticket, "daemon"),
      companionConnect: app.relaySessionConnect(ticket, "companion"),
      companionIdentity,
      operatorSetupText:
        mode === "private-network"
          ? "Private-network relay tailnet-dev endpoint is ready for browser evidence."
          : "Self-hosted relay endpoint is ready for browser evidence.",
    };
    if (mode === "private-network") {
      setup.privateNetworkName = "tailnet-dev";
    }
    return JSON.stringify(setup, null, 2);
  }, deploymentMode);
}

async function loadBothSetups(page, selfHostedSetupJson, privateNetworkSetupJson) {
  await page.click('[data-mode="relay"]');
  await page.fill("#relay-setup-input", selfHostedSetupJson);
  await page.click("#relay-setup-load-button");
  await page.waitForFunction(
    () => document.querySelector("#relay-state")?.textContent === "Ready",
    null,
    { timeout: 15000 },
  );
  await page.fill("#relay-private-setup-input", privateNetworkSetupJson);
  await page.click("#relay-private-load-button");
  await page.waitForFunction(
    () => document.querySelector("#relay-private-state")?.textContent === "Ready",
    null,
    { timeout: 15000 },
  );
}

async function visibleState(page) {
  return page.evaluate(() => ({
    relayState: document.querySelector("#relay-state")?.textContent || "",
    relayConnectDisabled: document.querySelector("#relay-connect-button")?.disabled ?? true,
    relayDeployment: document.querySelector("#relay-deployment")?.textContent || "",
    relayEndpoint: document.querySelector("#relay-endpoint")?.textContent || "",
    privateState: document.querySelector("#relay-private-state")?.textContent || "",
    privateDefault: document.querySelector("#relay-private-default-mode")?.textContent || "",
    privateNetwork: document.querySelector("#relay-private-network")?.textContent || "",
    privateDeployment: document.querySelector("#relay-private-deployment")?.textContent || "",
    privateEndpoint: document.querySelector("#relay-private-endpoint")?.textContent || "",
    privateDevice: document.querySelector("#relay-private-device")?.textContent || "",
    privateBlockers: Array.from(
      document.querySelectorAll("#relay-private-blocker-list li"),
      (item) => item.textContent || "",
    ),
    bodyText: document.body.textContent || "",
    innerWidth: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  const staticInfo = await startStaticServer();
  staticServer = staticInfo.server;
  const executablePath = browserExecutablePath();
  browser = await chromium.launch({
    headless: true,
    executablePath,
  });
  const page = await browser.newPage({ viewport: { width: 1280, height: 1040 } });
  await page.goto(`${staticInfo.url}/index.html`, { waitUntil: "networkidle" });

  const selfHostedSetupJson = await buildRelaySetupJson(page, "self-hosted");
  const privateNetworkSetupJson = await buildRelaySetupJson(page, "private-network");
  await loadBothSetups(page, selfHostedSetupJson, privateNetworkSetupJson);
  await page.screenshot({ path: desktopScreenshotPath, fullPage: true });
  const desktop = await visibleState(page);

  assert.equal(desktop.relayState, "Ready");
  assert.equal(desktop.relayConnectDisabled, false);
  assert.equal(desktop.relayDeployment, "self-hosted");
  assert.equal(desktop.relayEndpoint, "wss://relay.example.test/session");
  assert.equal(desktop.privateState, "Ready");
  assert.equal(desktop.privateDefault, "live-loopback");
  assert.equal(desktop.privateNetwork, "tailnet-dev");
  assert.equal(desktop.privateDeployment, "private-network");
  assert.equal(desktop.privateEndpoint, "wss://relay.tailnet.example/relay");
  assert.equal(desktop.privateDevice, "web-private-visible");
  assert.deepEqual(desktop.privateBlockers, ["Private-network relay setup ready"]);
  assert.equal(desktop.bodyText.includes("relay-ticket-secret"), false);
  assert.equal(/hmac_sha256_keys|hmacSha256Keys/.test(desktop.bodyText), false);

  const mobilePage = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await mobilePage.goto(`${staticInfo.url}/index.html`, { waitUntil: "networkidle" });
  await loadBothSetups(mobilePage, selfHostedSetupJson, privateNetworkSetupJson);
  await mobilePage.screenshot({ path: mobileScreenshotPath, fullPage: true });
  const mobile = await visibleState(mobilePage);

  assert.equal(mobile.privateState, "Ready");
  assert.ok(
    mobile.scrollWidth <= mobile.innerWidth,
    `mobile layout overflowed: ${mobile.scrollWidth} > ${mobile.innerWidth}`,
  );

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective: "Verify explicit private-network relay import status path without changing self-hosted relay setup",
    pwaUrl: staticInfo.url,
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
    nextLocalSlice: "managed-relay-billing-abuse-boundary-review",
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_PRIVATE_NETWORK_VISIBLE_IMPORT_OK ${evidencePath}`);
}

try {
  await main();
} finally {
  if (browser) {
    await browser.close().catch(() => {});
  }
  await closeServer(staticServer);
}
