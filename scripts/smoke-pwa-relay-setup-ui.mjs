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
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-setup-ui");
const evidencePath =
  process.env.RA_PWA_RELAY_SETUP_UI_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-setup-ui.json");
const screenshotPath = path.join(artifactRoot, "ra-pwa-relay-setup-ui.png");
const mobileScreenshotPath = path.join(artifactRoot, "ra-pwa-relay-setup-ui-mobile.png");

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

async function buildRelaySetupJson(page) {
  return page.evaluate(async () => {
    const app = await import(new URL("./app.mjs", window.location.href).href);
    const nowMs = Date.now();
    const companionIdentity = {
      deviceId: "web-setup01",
      noisePubkeyHex: "b".repeat(64),
      approvalPubkeyHex: "c".repeat(64),
    };
    const ticket = app.createRelaySessionTicket({
      sessionId: "relay-setup-ui-1",
      sessionToken: "token_relay_setup_ui_1234567890abcdef123456",
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
    return JSON.stringify(
      {
        relayProtocolVersion: 1,
        transportMode: "relay",
        deploymentMode: "self-hosted",
        relayEndpointUrl: "wss://relay.example.test/session",
        signedSessionTicket: { ...signedSessionTicket, key_id: "relay-active-1" },
        daemonConnect: app.relaySessionConnect(ticket, "daemon"),
        companionConnect: app.relaySessionConnect(ticket, "companion"),
        companionIdentity,
        operatorSetupText: "Self-hosted relay endpoint is ready for browser evidence.",
      },
      null,
      2,
    );
  });
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
  const page = await browser.newPage({ viewport: { width: 1280, height: 960 } });
  await page.goto(`${staticInfo.url}/index.html`, { waitUntil: "networkidle" });
  await page.click('[data-mode="relay"]');

  const setupJson = await buildRelaySetupJson(page);
  await page.fill("#relay-setup-input", setupJson);
  await page.click("#relay-setup-load-button");
  await page.waitForFunction(
    () => document.querySelector("#relay-state")?.textContent === "Ready",
    null,
    { timeout: 15000 },
  );
  await page.screenshot({ path: screenshotPath, fullPage: true });

  const result = await page.evaluate(() => ({
    title: document.title,
    pairStatus: document.querySelector("#pair-status")?.textContent || "",
    relayState: document.querySelector("#relay-state")?.textContent || "",
    productDefault: document.querySelector("#relay-default-mode")?.textContent || "",
    endpoint: document.querySelector("#relay-endpoint")?.textContent || "",
    deployment: document.querySelector("#relay-deployment")?.textContent || "",
    device: document.querySelector("#relay-device")?.textContent || "",
    session: document.querySelector("#relay-session")?.textContent || "",
    ticketKey: document.querySelector("#relay-ticket-key")?.textContent || "",
    blockers: Array.from(document.querySelectorAll("#relay-blocker-list li"), (item) => item.textContent || ""),
    companionConnect: document.querySelector("#relay-companion-connect")?.textContent || "",
    daemonConnect: document.querySelector("#relay-daemon-connect")?.textContent || "",
    bodyText: document.body.textContent || "",
  }));

  assert.equal(result.relayState, "Ready");
  assert.equal(result.productDefault, "live-loopback");
  assert.equal(result.endpoint, "wss://relay.example.test/session");
  assert.equal(result.deployment, "self-hosted");
  assert.equal(result.device, "web-setup01");
  assert.equal(result.session, "relay-setup-ui-1");
  assert.equal(result.ticketKey, "relay-active-1");
  assert.deepEqual(result.blockers, ["Relay setup ready"]);
  assert.match(result.companionConnect, /"peer":"companion"/);
  assert.match(result.daemonConnect, /"peer":"daemon"/);
  assert.equal(result.bodyText.includes("relay-ticket-secret"), false);
  assert.equal(/hmac_sha256_keys|hmacSha256Keys/.test(result.bodyText), false);

  const mobilePage = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await mobilePage.goto(`${staticInfo.url}/index.html`, { waitUntil: "networkidle" });
  await mobilePage.click('[data-mode="relay"]');
  await mobilePage.fill("#relay-setup-input", setupJson);
  await mobilePage.click("#relay-setup-load-button");
  await mobilePage.waitForFunction(
    () => document.querySelector("#relay-state")?.textContent === "Ready",
    null,
    { timeout: 15000 },
  );
  await mobilePage.screenshot({ path: mobileScreenshotPath, fullPage: true });
  const mobileLayout = await mobilePage.evaluate(() => ({
    relayState: document.querySelector("#relay-state")?.textContent || "",
    innerWidth: window.innerWidth,
    scrollWidth: document.documentElement.scrollWidth,
  }));
  assert.equal(mobileLayout.relayState, "Ready");
  assert.ok(
    mobileLayout.scrollWidth <= mobileLayout.innerWidth,
    `mobile layout overflowed: ${mobileLayout.scrollWidth} > ${mobileLayout.innerWidth}`,
  );

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective: "Verify visible self-hosted relay setup UI reaches ready state from runtime setup JSON",
    pwaUrl: staticInfo.url,
    browserExecutablePath: executablePath || "playwright-default",
    screenshots: {
      desktop: screenshotPath,
      mobile: mobileScreenshotPath,
    },
    mobileLayout,
    result: {
      ...result,
      bodyText: undefined,
    },
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_SETUP_UI_OK ${evidencePath}`);
}

try {
  await main();
} finally {
  if (browser) {
    await browser.close().catch(() => {});
  }
  await closeServer(staticServer);
}
