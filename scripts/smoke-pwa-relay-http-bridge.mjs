import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const RELAY_PROTOCOL_VERSION = 1;
const MAX_RELAY_SESSION_ID_LENGTH = 96;
const MAX_RELAY_PAYLOAD_JSON_BYTES = 1 << 20;
const MAX_REQUEST_BYTES = MAX_RELAY_PAYLOAD_JSON_BYTES + 4096;

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-http-bridge");
const evidencePath =
  process.env.RA_PWA_RELAY_HTTP_BRIDGE_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-http-bridge.json");
const screenshotPath = path.join(artifactRoot, "ra-pwa-relay-http-bridge.png");

let pwaServer = null;
let bridgeServer = null;
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

function corsHeaders(extra = {}) {
  return {
    "access-control-allow-origin": "*",
    "access-control-allow-methods": "GET, POST, OPTIONS",
    "access-control-allow-headers": "content-type",
    ...extra,
  };
}

function validRelaySessionId(value) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_RELAY_SESSION_ID_LENGTH &&
    /^[A-Za-z0-9._:-]+$/.test(value)
  );
}

function validateRelayFrameMetadata(frame) {
  if (frame?.relay_protocol_version !== RELAY_PROTOCOL_VERSION) {
    throw new Error("unsupported relay protocol version");
  }
  if (!validRelaySessionId(frame.session_id)) {
    throw new Error("relay session_id format error");
  }
  if (frame.sender !== "daemon" && frame.sender !== "companion") {
    throw new Error("relay sender format error");
  }
  if (!Number.isSafeInteger(frame.sequence) || frame.sequence <= 0) {
    throw new Error("relay sequence format error");
  }
  if (!Number.isSafeInteger(frame.sent_at_ms) || frame.sent_at_ms <= 0) {
    throw new Error("relay sent_at_ms format error");
  }
  if (!Number.isSafeInteger(frame.expires_at_ms) || frame.expires_at_ms <= frame.sent_at_ms) {
    throw new Error("relay expires_at_ms format error");
  }
  if (
    typeof frame.payload_json !== "string" ||
    frame.payload_json.length === 0 ||
    new TextEncoder().encode(frame.payload_json).length > MAX_RELAY_PAYLOAD_JSON_BYTES
  ) {
    throw new Error("relay payload_json size error");
  }
}

function routeEnvelope(frame) {
  validateRelayFrameMetadata(frame);
  return {
    relay_protocol_version: frame.relay_protocol_version,
    session_id: frame.session_id,
    sender: frame.sender,
    sequence: frame.sequence,
    sent_at_ms: frame.sent_at_ms,
    expires_at_ms: frame.expires_at_ms,
    payload_json_bytes: new TextEncoder().encode(frame.payload_json).length,
  };
}

function recipientQueue(session, recipient) {
  return recipient === "companion" ? session.daemonToCompanion : session.companionToDaemon;
}

function senderQueue(session, sender) {
  return sender === "daemon" ? session.daemonToCompanion : session.companionToDaemon;
}

function lastSequenceKey(sender) {
  return sender === "daemon" ? "lastDaemonSequence" : "lastCompanionSequence";
}

function writeJson(res, status, body) {
  res.writeHead(status, corsHeaders({ "content-type": "application/json; charset=utf-8" }));
  res.end(`${JSON.stringify(body, null, 2)}\n`);
}

function writeError(res, status, message) {
  writeJson(res, status, { status: "error", message });
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let body = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      body += chunk;
      if (Buffer.byteLength(body, "utf8") > MAX_REQUEST_BYTES) {
        reject(new Error("request too large"));
        req.destroy();
      }
    });
    req.on("end", () => resolve(body));
    req.on("error", reject);
  });
}

async function startPwaServer() {
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
  await listenLocal(server);
  return { server, url: serverUrl(server) };
}

async function startRelayBridge() {
  const sessions = new Map();
  const stats = {
    acceptedFrames: 0,
    deliveredFrames: 0,
    expiredFrames: 0,
    rejectedFrames: 0,
  };
  const server = createServer(async (req, res) => {
    if (req.method === "OPTIONS") {
      res.writeHead(204, corsHeaders());
      res.end();
      return;
    }
    const url = new URL(req.url || "/", "http://127.0.0.1/");
    try {
      if (req.method === "GET" && url.pathname === "/health") {
        writeJson(res, 200, {
          status: "ok",
          sessions: sessions.size,
          queuedFrames: queuedFrames(sessions),
          stats,
        });
        return;
      }
      if (req.method === "POST" && url.pathname === "/frames") {
        const body = await readBody(req);
        const frame = JSON.parse(body);
        const route = routeEnvelope(frame);
        const session = sessions.get(route.session_id) || {
          daemonToCompanion: [],
          companionToDaemon: [],
          lastDaemonSequence: 0,
          lastCompanionSequence: 0,
        };
        const key = lastSequenceKey(route.sender);
        if (route.sequence <= session[key]) {
          stats.rejectedFrames += 1;
          writeError(res, 409, "relay sequence must increase for sender");
          return;
        }
        session[key] = route.sequence;
        senderQueue(session, route.sender).push({ route, frameJson: body });
        sessions.set(route.session_id, session);
        stats.acceptedFrames += 1;
        writeJson(res, 202, { status: "queued", route });
        return;
      }
      if (req.method === "GET" && url.pathname === "/frames") {
        const sessionId = url.searchParams.get("session_id") || "";
        const recipient = url.searchParams.get("recipient") || "";
        const nowMs = Number(url.searchParams.get("now_ms") || "");
        if (!validRelaySessionId(sessionId)) {
          writeError(res, 400, "relay session_id format error");
          return;
        }
        if (recipient !== "daemon" && recipient !== "companion") {
          writeError(res, 400, "relay recipient format error");
          return;
        }
        if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
          writeError(res, 400, "relay now_ms format error");
          return;
        }
        const session = sessions.get(sessionId);
        if (!session) {
          res.writeHead(204, corsHeaders());
          res.end();
          return;
        }
        const queue = recipientQueue(session, recipient);
        while (queue.length > 0) {
          const item = queue.shift();
          if (nowMs < item.route.expires_at_ms) {
            stats.deliveredFrames += 1;
            if (session.daemonToCompanion.length === 0 && session.companionToDaemon.length === 0) {
              sessions.delete(sessionId);
            }
            writeJson(res, 200, {
              status: "delivered",
              route: item.route,
              frame_json: item.frameJson,
            });
            return;
          }
          stats.expiredFrames += 1;
        }
        if (session.daemonToCompanion.length === 0 && session.companionToDaemon.length === 0) {
          sessions.delete(sessionId);
        }
        res.writeHead(204, corsHeaders());
        res.end();
        return;
      }
      writeError(res, 404, "not found");
    } catch (err) {
      stats.rejectedFrames += 1;
      writeError(res, 400, err.message || "bad request");
    }
  });
  await listenLocal(server);
  return { server, url: serverUrl(server) };
}

function queuedFrames(sessions) {
  let count = 0;
  for (const session of sessions.values()) {
    count += session.daemonToCompanion.length + session.companionToDaemon.length;
  }
  return count;
}

function listenLocal(server) {
  return new Promise((resolve, reject) => {
    server.on("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
}

function serverUrl(server) {
  const address = server.address();
  return `http://127.0.0.1:${address.port}`;
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

async function runBridgeSmoke(page, bridgeUrl) {
  return page.evaluate(async (relayBaseUrl) => {
    const app = await import(new URL("./app.mjs", window.location.href).href);
    const expect = (condition, message) => {
      if (!condition) {
        throw new Error(message);
      }
    };
    const sameJson = (left, right) => JSON.stringify(left) === JSON.stringify(right);
    const postFrame = async (frame) => {
      const response = await fetch(`${relayBaseUrl}/frames`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: app.relayFrameJson(frame),
      });
      const text = await response.text();
      const body = text ? JSON.parse(text) : null;
      return { status: response.status, body };
    };
    const getFrame = async (sessionId, recipient, nowMs) => {
      const url = new URL(`${relayBaseUrl}/frames`);
      url.searchParams.set("session_id", sessionId);
      url.searchParams.set("recipient", recipient);
      url.searchParams.set("now_ms", String(nowMs));
      const response = await fetch(url);
      const text = await response.text();
      return {
        status: response.status,
        body: text ? JSON.parse(text) : null,
      };
    };
    const health = async () => {
      const response = await fetch(`${relayBaseUrl}/health`);
      return response.json();
    };

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
    const sessionId = "relay-http-bridge-1";
    const daemon = app.createRelayEndpoint(sessionId, "daemon");
    const companion = app.createRelayEndpoint(sessionId, "companion");
    const requestMessage = app.liveApprovalRequestMessage(approvalRequest);
    const responseMessage = app.liveApprovalResponseMessage(approvalResponse);

    const daemonFrame = app.relayEndpointNextFrame(daemon, requestMessage, 21000);
    const daemonPost = await postFrame(daemonFrame);
    expect(daemonPost.status === 202, "daemon frame was not accepted");
    expect(!("payload_json" in daemonPost.body.route), "bridge route leaked daemon payload");
    const duplicatePost = await postFrame(daemonFrame);
    expect(duplicatePost.status === 409, "duplicate daemon sequence was not rejected");

    const companionGet = await getFrame(sessionId, "companion", 21001);
    expect(companionGet.status === 200, "companion did not receive daemon frame");
    const companionMessage = app.relayEndpointAcceptFrame(
      companion,
      companionGet.body.frame_json,
      21001,
    );
    expect(sameJson(companionMessage, requestMessage), "companion decoded request mismatch");

    const companionFrame = app.relayEndpointNextFrame(companion, responseMessage, 21002);
    const companionPost = await postFrame(companionFrame);
    expect(companionPost.status === 202, "companion frame was not accepted");
    expect(!("payload_json" in companionPost.body.route), "bridge route leaked companion payload");
    const daemonGet = await getFrame(sessionId, "daemon", 21003);
    expect(daemonGet.status === 200, "daemon did not receive companion frame");
    const daemonReply = app.relayEndpointAcceptFrame(daemon, daemonGet.body.frame_json, 21003);
    expect(sameJson(daemonReply, responseMessage), "daemon decoded response mismatch");

    const expiredSession = "relay-http-bridge-expired";
    const expiredDaemon = app.createRelayEndpoint(expiredSession, "daemon", 1);
    const expiredFrame = app.relayEndpointNextFrame(expiredDaemon, app.livePingMessage("expired"), 22000);
    const expiredPost = await postFrame(expiredFrame);
    expect(expiredPost.status === 202, "expired test frame was not accepted");
    const expiredGet = await getFrame(expiredSession, "companion", 22001);
    expect(expiredGet.status === 204, "expired frame was delivered");

    const finalHealth = await health();
    return {
      status: "ok",
      title: document.title,
      sessionId,
      daemonRoute: daemonPost.body.route,
      companionRoute: companionPost.body.route,
      duplicateRejected: duplicatePost.status === 409,
      expiredDropped: expiredGet.status === 204,
      companionMessageType: companionMessage.type,
      daemonReplyType: daemonReply.type,
      daemonNextSequence: daemon.nextSequence,
      companionNextSequence: companion.nextSequence,
      finalHealth,
    };
  }, bridgeUrl);
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  const pwaInfo = await startPwaServer();
  pwaServer = pwaInfo.server;
  const bridgeInfo = await startRelayBridge();
  bridgeServer = bridgeInfo.server;
  const executablePath = browserExecutablePath();
  browser = await chromium.launch({
    headless: true,
    executablePath,
  });
  const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
  await page.goto(`${pwaInfo.url}/index.html`, { waitUntil: "networkidle" });
  await page.screenshot({ path: screenshotPath, fullPage: true });
  const result = await runBridgeSmoke(page, bridgeInfo.url);
  assert.equal(result.status, "ok");
  assert.equal(result.companionMessageType, "approval_request");
  assert.equal(result.daemonReplyType, "approval_response");
  assert.equal(result.duplicateRejected, true);
  assert.equal(result.expiredDropped, true);
  assert.equal(result.finalHealth.queuedFrames, 0);

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective: "Verify relay frame JSON across a local process-like HTTP bridge",
    pwaUrl: pwaInfo.url,
    bridgeUrl: bridgeInfo.url,
    browserExecutablePath: executablePath || "playwright-default",
    screenshotPath,
    result,
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_HTTP_BRIDGE_OK ${evidencePath}`);
}

try {
  await main();
} finally {
  if (browser) {
    await browser.close().catch(() => {});
  }
  await closeServer(bridgeServer);
  await closeServer(pwaServer);
}
