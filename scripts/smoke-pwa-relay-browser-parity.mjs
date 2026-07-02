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
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-browser-parity");
const evidencePath =
  process.env.RA_PWA_RELAY_BROWSER_PARITY_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-browser-parity.json");
const screenshotPath = path.join(artifactRoot, "ra-pwa-relay-browser-parity.png");

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

async function runBrowserRelayParity(page) {
  return page.evaluate(async () => {
    const app = await import(new URL("./app.mjs", window.location.href).href);
    const expect = (condition, message) => {
      if (!condition) {
        throw new Error(message);
      }
    };
    const sameJson = (left, right) => JSON.stringify(left) === JSON.stringify(right);
    const encodedBytes = (value) => new TextEncoder().encode(value).length;

    const approvalRequest = {
      approval_id: [97, 112, 112, 114, 45, 49],
      nonce: Array.from({ length: 32 }, (_, i) => i),
      command_masked: "rm -rf build",
      context_hash: "ctx-A",
      expires_at: 1782804241456,
      device_epoch: 1,
    };
    const approvalResponse = {
      approval_id: approvalRequest.approval_id,
      nonce: approvalRequest.nonce,
      approve: true,
      sig: Array.from({ length: 64 }, (_, i) => i + 1),
    };

    app.validateApprovalRequest(approvalRequest);
    app.validateApprovalResponse(approvalResponse);
    const daemonEndpoint = app.createRelayEndpoint("browser-relay-parity-1", "daemon");
    const companionEndpoint = app.createRelayEndpoint("browser-relay-parity-1", "companion");
    const requestMessage = app.liveApprovalRequestMessage(approvalRequest);
    const responseMessage = app.liveApprovalResponseMessage(approvalResponse);
    const exchange = app.relayEndpointExchange(
      daemonEndpoint,
      companionEndpoint,
      requestMessage,
      responseMessage,
      12000,
    );

    const parsedDaemonFrame = app.parseRelayFrame(exchange.daemonFrameJson);
    const parsedCompanionFrame = app.parseRelayFrame(exchange.companionFrameJson);
    const daemonRoute = app.relayFrameRouteEnvelope(exchange.daemonFrameJson);
    const companionRoute = app.relayFrameRouteEnvelope(exchange.companionFrameJson);
    expect(sameJson(parsedDaemonFrame, exchange.daemonFrame), "daemon frame JSON mismatch");
    expect(sameJson(parsedCompanionFrame, exchange.companionFrame), "companion frame JSON mismatch");
    expect(!("payload_json" in daemonRoute), "daemon route envelope leaked payload_json");
    expect(!("payload_json" in companionRoute), "companion route envelope leaked payload_json");
    expect(
      daemonRoute.payload_json_bytes === encodedBytes(exchange.daemonFrame.payload_json),
      "daemon route byte length mismatch",
    );
    expect(
      companionRoute.payload_json_bytes === encodedBytes(exchange.companionFrame.payload_json),
      "companion route byte length mismatch",
    );
    expect(sameJson(exchange.companionMessage, requestMessage), "companion decoded request mismatch");
    expect(sameJson(exchange.daemonReply, responseMessage), "daemon decoded response mismatch");
    expect(daemonEndpoint.nextSequence === 2, "daemon endpoint sequence mismatch");
    expect(companionEndpoint.nextSequence === 2, "companion endpoint sequence mismatch");

    const badPayloadFrame = {
      ...exchange.daemonFrame,
      payload_json: JSON.stringify({ type: "ping", nonce: "" }),
    };
    app.validateRelayFrame(badPayloadFrame);
    const badPayloadRoute = app.relayFrameRouteEnvelope(badPayloadFrame);
    let payloadDecodeFailed = false;
    try {
      app.relayFramePayloadMessage(badPayloadFrame);
    } catch {
      payloadDecodeFailed = true;
    }
    expect(payloadDecodeFailed, "invalid payload decoded at endpoint boundary");

    let wrongSessionFailed = false;
    try {
      app.relayEndpointExchange(
        app.createRelayEndpoint("browser-relay-a", "daemon"),
        app.createRelayEndpoint("browser-relay-b", "companion"),
        app.livePingMessage("wrong-session"),
        app.livePongMessage("wrong-session"),
        13000,
      );
    } catch {
      wrongSessionFailed = true;
    }
    expect(wrongSessionFailed, "wrong session exchange was accepted");

    return {
      status: "ok",
      title: document.title,
      pairStatus: document.querySelector("#pair-status")?.textContent || "",
      sessionId: daemonRoute.session_id,
      daemonRoute,
      companionRoute,
      badPayloadRoute,
      payloadDecodeFailed,
      wrongSessionFailed,
      companionMessageType: exchange.companionMessage.type,
      daemonReplyType: exchange.daemonReply.type,
      daemonNextSequence: daemonEndpoint.nextSequence,
      companionNextSequence: companionEndpoint.nextSequence,
    };
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
  const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
  await page.goto(`${staticInfo.url}/index.html`, { waitUntil: "networkidle" });
  await page.screenshot({ path: screenshotPath, fullPage: true });
  const result = await runBrowserRelayParity(page);
  assert.equal(result.status, "ok");
  assert.equal(result.companionMessageType, "approval_request");
  assert.equal(result.daemonReplyType, "approval_response");
  assert.equal(result.payloadDecodeFailed, true);
  assert.equal(result.wrongSessionFailed, true);

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective: "Verify PWA relay helpers in a real browser module runtime",
    pwaUrl: staticInfo.url,
    browserExecutablePath: executablePath || "playwright-default",
    screenshotPath,
    result,
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_BROWSER_PARITY_OK ${evidencePath}`);
}

try {
  await main();
} finally {
  if (browser) {
    await browser.close().catch(() => {});
  }
  await closeServer(staticServer);
}
