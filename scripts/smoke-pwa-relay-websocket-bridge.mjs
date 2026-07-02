import assert from "node:assert/strict";
import { createHash } from "node:crypto";
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
const MAX_WS_MESSAGE_BYTES = MAX_RELAY_PAYLOAD_JSON_BYTES + 4096;
const WEBSOCKET_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const pwaDir = path.join(repoRoot, "pwa");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-websocket-bridge");
const evidencePath =
  process.env.RA_PWA_RELAY_WEBSOCKET_BRIDGE_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-websocket-bridge.json");
const screenshotPath = path.join(artifactRoot, "ra-pwa-relay-websocket-bridge.png");

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
    "access-control-allow-methods": "GET, OPTIONS",
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

function recipientFromSender(sender) {
  return sender === "daemon" ? "companion" : "daemon";
}

function recipientQueue(session, recipient) {
  return recipient === "companion" ? session.daemonToCompanion : session.companionToDaemon;
}

function senderQueue(session, sender) {
  return sender === "daemon" ? session.daemonToCompanion : session.companionToDaemon;
}

function connectionSet(session, role) {
  return role === "daemon" ? session.daemonConnections : session.companionConnections;
}

function lastSequenceKey(sender) {
  return sender === "daemon" ? "lastDaemonSequence" : "lastCompanionSequence";
}

function newRelaySession() {
  return {
    daemonToCompanion: [],
    companionToDaemon: [],
    lastDaemonSequence: 0,
    lastCompanionSequence: 0,
    daemonConnections: new Set(),
    companionConnections: new Set(),
  };
}

function getRelaySession(sessions, sessionId) {
  const existing = sessions.get(sessionId);
  if (existing) {
    return existing;
  }
  const session = newRelaySession();
  sessions.set(sessionId, session);
  return session;
}

function writeJson(res, status, body) {
  res.writeHead(status, corsHeaders({ "content-type": "application/json; charset=utf-8" }));
  res.end(`${JSON.stringify(body, null, 2)}\n`);
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

async function startRelayWebSocketBridge() {
  const sessions = new Map();
  const stats = {
    acceptedFrames: 0,
    deliveredFrames: 0,
    expiredFrames: 0,
    rejectedFrames: 0,
    openedConnections: 0,
    closedConnections: 0,
  };
  const server = createServer((req, res) => {
    if (req.method === "OPTIONS") {
      res.writeHead(204, corsHeaders());
      res.end();
      return;
    }
    const url = new URL(req.url || "/", "http://127.0.0.1/");
    if (req.method === "GET" && url.pathname === "/health") {
      writeJson(res, 200, {
        status: "ok",
        sessions: sessions.size,
        queuedFrames: queuedFrames(sessions),
        stats,
      });
      return;
    }
    writeJson(res, 404, { status: "error", message: "not found" });
  });

  server.on("upgrade", (req, socket) => {
    const url = new URL(req.url || "/", "http://127.0.0.1/");
    const sessionId = url.searchParams.get("session_id") || "";
    const role = url.searchParams.get("role") || "";
    const key = req.headers["sec-websocket-key"];
    if (
      url.pathname !== "/relay" ||
      !validRelaySessionId(sessionId) ||
      (role !== "daemon" && role !== "companion") ||
      typeof key !== "string" ||
      !key
    ) {
      rejectUpgrade(socket);
      return;
    }

    const accept = createHash("sha1")
      .update(`${key}${WEBSOCKET_GUID}`, "binary")
      .digest("base64");
    socket.write(
      [
        "HTTP/1.1 101 Switching Protocols",
        "Upgrade: websocket",
        "Connection: Upgrade",
        `Sec-WebSocket-Accept: ${accept}`,
        "\r\n",
      ].join("\r\n"),
    );

    const conn = {
      socket,
      sessionId,
      role,
      buffer: Buffer.alloc(0),
      closed: false,
    };
    const session = getRelaySession(sessions, sessionId);
    connectionSet(session, role).add(conn);
    stats.openedConnections += 1;
    flushQueuedToRecipient(sessions, sessionId, role, stats);

    socket.on("data", (chunk) => {
      try {
        handleSocketData(conn, chunk, sessions, stats);
      } catch (err) {
        stats.rejectedFrames += 1;
        sendJson(conn, {
          kind: "error",
          message: err.message || "bad websocket frame",
        });
        closeWebSocket(conn);
      }
    });
    socket.on("close", () => {
      removeConnection(sessions, conn);
      stats.closedConnections += 1;
    });
    socket.on("error", () => {
      removeConnection(sessions, conn);
    });
  });

  await listenLocal(server);
  const httpUrl = serverUrl(server);
  return {
    server,
    healthUrl: `${httpUrl}/health`,
    websocketUrl: `${httpUrl.replace(/^http:/, "ws:")}/relay`,
  };
}

function handleSocketData(conn, chunk, sessions, stats) {
  conn.buffer = Buffer.concat([conn.buffer, chunk]);
  while (conn.buffer.length > 0) {
    const parsed = readWebSocketFrame(conn.buffer);
    if (!parsed) {
      return;
    }
    conn.buffer = parsed.remaining;
    if (parsed.opcode === 0x8) {
      closeWebSocket(conn);
      return;
    }
    if (parsed.opcode === 0x9) {
      sendWebSocketFrame(conn, parsed.payload, 0xA);
      continue;
    }
    if (parsed.opcode !== 0x1) {
      throw new Error("unsupported websocket opcode");
    }
    if (parsed.payload.length > MAX_WS_MESSAGE_BYTES) {
      throw new Error("websocket message too large");
    }
    routeSocketMessage(conn, parsed.payload.toString("utf8"), sessions, stats);
  }
}

function routeSocketMessage(conn, text, sessions, stats) {
  try {
    const frame = JSON.parse(text);
    const route = routeEnvelope(frame);
    if (route.session_id !== conn.sessionId) {
      throw new Error("relay websocket session mismatch");
    }
    if (route.sender !== conn.role) {
      throw new Error("relay websocket sender mismatch");
    }
    const session = getRelaySession(sessions, route.session_id);
    const key = lastSequenceKey(route.sender);
    if (route.sequence <= session[key]) {
      throw new Error("relay sequence must increase for sender");
    }
    session[key] = route.sequence;
    senderQueue(session, route.sender).push({ route, frameJson: text });
    stats.acceptedFrames += 1;
    sendJson(conn, { kind: "queued", route });
    flushQueuedToRecipient(sessions, route.session_id, recipientFromSender(route.sender), stats);
  } catch (err) {
    stats.rejectedFrames += 1;
    sendJson(conn, {
      kind: "error",
      message: err.message || "bad relay frame",
    });
  }
}

function flushQueuedToRecipient(sessions, sessionId, recipient, stats) {
  const session = sessions.get(sessionId);
  if (!session) {
    return;
  }
  const connections = connectionSet(session, recipient);
  if (connections.size === 0) {
    return;
  }
  const queue = recipientQueue(session, recipient);
  const nowMs = Date.now();
  while (queue.length > 0) {
    const item = queue.shift();
    if (nowMs >= item.route.expires_at_ms) {
      stats.expiredFrames += 1;
      continue;
    }
    for (const conn of connections) {
      sendJson(conn, {
        kind: "frame",
        route: item.route,
        frame_json: item.frameJson,
      });
    }
    stats.deliveredFrames += 1;
  }
  cleanupSession(sessions, sessionId);
}

function queuedFrames(sessions) {
  let count = 0;
  for (const session of sessions.values()) {
    count += session.daemonToCompanion.length + session.companionToDaemon.length;
  }
  return count;
}

function removeConnection(sessions, conn) {
  if (conn.closed) {
    return;
  }
  conn.closed = true;
  const session = sessions.get(conn.sessionId);
  if (!session) {
    return;
  }
  connectionSet(session, conn.role).delete(conn);
  cleanupSession(sessions, conn.sessionId);
}

function cleanupSession(sessions, sessionId) {
  const session = sessions.get(sessionId);
  if (!session) {
    return;
  }
  if (
    session.daemonToCompanion.length === 0 &&
    session.companionToDaemon.length === 0 &&
    session.daemonConnections.size === 0 &&
    session.companionConnections.size === 0
  ) {
    sessions.delete(sessionId);
  }
}

function readWebSocketFrame(buffer) {
  if (buffer.length < 2) {
    return null;
  }
  const first = buffer[0];
  const second = buffer[1];
  const fin = (first & 0x80) !== 0;
  const opcode = first & 0x0f;
  const masked = (second & 0x80) !== 0;
  let payloadLength = second & 0x7f;
  let offset = 2;

  if (!fin) {
    throw new Error("fragmented websocket messages are not supported");
  }
  if (payloadLength === 126) {
    if (buffer.length < offset + 2) {
      return null;
    }
    payloadLength = buffer.readUInt16BE(offset);
    offset += 2;
  } else if (payloadLength === 127) {
    if (buffer.length < offset + 8) {
      return null;
    }
    const bigLength = buffer.readBigUInt64BE(offset);
    if (bigLength > BigInt(Number.MAX_SAFE_INTEGER)) {
      throw new Error("websocket message too large");
    }
    payloadLength = Number(bigLength);
    offset += 8;
  }

  if (payloadLength > MAX_WS_MESSAGE_BYTES) {
    throw new Error("websocket message too large");
  }
  if (!masked) {
    throw new Error("client websocket frames must be masked");
  }
  if (buffer.length < offset + 4 + payloadLength) {
    return null;
  }
  const mask = buffer.subarray(offset, offset + 4);
  offset += 4;
  const rawPayload = buffer.subarray(offset, offset + payloadLength);
  const payload = Buffer.alloc(payloadLength);
  for (let i = 0; i < payloadLength; i += 1) {
    payload[i] = rawPayload[i] ^ mask[i % 4];
  }
  return {
    opcode,
    payload,
    remaining: buffer.subarray(offset + payloadLength),
  };
}

function sendJson(conn, body) {
  sendWebSocketFrame(conn, Buffer.from(JSON.stringify(body), "utf8"), 0x1);
}

function sendWebSocketFrame(conn, payload, opcode) {
  if (conn.closed || conn.socket.destroyed) {
    return;
  }
  conn.socket.write(encodeWebSocketFrame(payload, opcode));
}

function closeWebSocket(conn) {
  if (conn.closed || conn.socket.destroyed) {
    return;
  }
  conn.socket.end(encodeWebSocketFrame(Buffer.alloc(0), 0x8));
}

function encodeWebSocketFrame(payload, opcode) {
  if (payload.length < 126) {
    return Buffer.concat([Buffer.from([0x80 | opcode, payload.length]), payload]);
  }
  if (payload.length <= 0xffff) {
    const header = Buffer.alloc(4);
    header[0] = 0x80 | opcode;
    header[1] = 126;
    header.writeUInt16BE(payload.length, 2);
    return Buffer.concat([header, payload]);
  }
  const header = Buffer.alloc(10);
  header[0] = 0x80 | opcode;
  header[1] = 127;
  header.writeBigUInt64BE(BigInt(payload.length), 2);
  return Buffer.concat([header, payload]);
}

function rejectUpgrade(socket) {
  socket.write("HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n");
  socket.destroy();
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

async function runBridgeSmoke(page, bridge) {
  return page.evaluate(async ({ websocketUrl, healthUrl }) => {
    const app = await import(new URL("./app.mjs", window.location.href).href);
    const expect = (condition, message) => {
      if (!condition) {
        throw new Error(message);
      }
    };
    const sameJson = (left, right) => JSON.stringify(left) === JSON.stringify(right);

    const connectEndpoint = (sessionId, role) =>
      new Promise((resolve, reject) => {
        const url = new URL(websocketUrl);
        url.searchParams.set("session_id", sessionId);
        url.searchParams.set("role", role);
        const ws = new WebSocket(url);
        const state = {
          ws,
          messages: [],
          waiters: [],
        };
        const failTimer = setTimeout(() => {
          reject(new Error(`websocket ${role} open timeout`));
        }, 2000);
        ws.addEventListener("open", () => {
          clearTimeout(failTimer);
          resolve(state);
        });
        ws.addEventListener("error", () => {
          clearTimeout(failTimer);
          reject(new Error(`websocket ${role} failed`));
        });
        ws.addEventListener("message", (event) => {
          const message = JSON.parse(event.data);
          const waiterIndex = state.waiters.findIndex((waiter) => waiter.predicate(message));
          if (waiterIndex >= 0) {
            const [waiter] = state.waiters.splice(waiterIndex, 1);
            clearTimeout(waiter.timer);
            waiter.resolve(message);
            return;
          }
          state.messages.push(message);
        });
      });

    const waitFor = (state, predicate, label, timeoutMs = 2000) => {
      const existingIndex = state.messages.findIndex(predicate);
      if (existingIndex >= 0) {
        const [message] = state.messages.splice(existingIndex, 1);
        return Promise.resolve(message);
      }
      return new Promise((resolve, reject) => {
        const waiter = {
          predicate,
          resolve,
          timer: setTimeout(() => {
            const index = state.waiters.indexOf(waiter);
            if (index >= 0) {
              state.waiters.splice(index, 1);
            }
            reject(new Error(`timeout waiting for ${label}`));
          }, timeoutMs),
        };
        state.waiters.push(waiter);
      });
    };

    const expectNoFrame = async (state, predicate, timeoutMs) => {
      try {
        await waitFor(state, predicate, "unexpected frame", timeoutMs);
        return false;
      } catch (err) {
        if (err.message && err.message.startsWith("timeout waiting")) {
          return true;
        }
        throw err;
      }
    };

    const sendFrame = (state, frame) => {
      state.ws.send(app.relayFrameJson(frame));
    };

    const closeState = (state) => {
      if (state.ws.readyState === WebSocket.OPEN) {
        state.ws.close();
      }
    };

    const approvalRequest = {
      approval_id: [119, 115, 45, 97, 112, 112, 114],
      nonce: Array.from({ length: 32 }, (_, i) => i + 3),
      command_masked: "git push --force",
      context_hash: "ctx-websocket-A",
      expires_at: 1782804241456,
      device_epoch: 1,
    };
    const approvalResponse = {
      approval_id: approvalRequest.approval_id,
      nonce: approvalRequest.nonce,
      approve: false,
      sig: Array.from({ length: 64 }, (_, i) => 64 - i),
    };
    const sessionId = "relay-websocket-bridge-1";
    const daemon = app.createRelayEndpoint(sessionId, "daemon");
    const companion = app.createRelayEndpoint(sessionId, "companion");
    const daemonSocket = await connectEndpoint(sessionId, "daemon");
    const companionSocket = await connectEndpoint(sessionId, "companion");
    const requestMessage = app.liveApprovalRequestMessage(approvalRequest);
    const responseMessage = app.liveApprovalResponseMessage(approvalResponse);

    const daemonFrame = app.relayEndpointNextFrame(daemon, requestMessage, Date.now());
    sendFrame(daemonSocket, daemonFrame);
    const daemonAck = await waitFor(
      daemonSocket,
      (message) => message.kind === "queued" && message.route?.sender === "daemon",
      "daemon frame ack",
    );
    expect(!("payload_json" in daemonAck.route), "bridge route leaked daemon payload");

    const companionDelivery = await waitFor(
      companionSocket,
      (message) => message.kind === "frame" && message.route?.sender === "daemon",
      "companion delivery",
    );
    const companionMessage = app.relayEndpointAcceptFrame(
      companion,
      companionDelivery.frame_json,
      Date.now(),
    );
    expect(sameJson(companionMessage, requestMessage), "companion decoded request mismatch");

    sendFrame(daemonSocket, daemonFrame);
    const duplicateError = await waitFor(
      daemonSocket,
      (message) => message.kind === "error" && /sequence/.test(message.message || ""),
      "duplicate sequence error",
    );

    const companionFrame = app.relayEndpointNextFrame(companion, responseMessage, Date.now());
    sendFrame(companionSocket, companionFrame);
    const companionAck = await waitFor(
      companionSocket,
      (message) => message.kind === "queued" && message.route?.sender === "companion",
      "companion frame ack",
    );
    expect(!("payload_json" in companionAck.route), "bridge route leaked companion payload");

    const daemonDelivery = await waitFor(
      daemonSocket,
      (message) => message.kind === "frame" && message.route?.sender === "companion",
      "daemon delivery",
    );
    const daemonReply = app.relayEndpointAcceptFrame(daemon, daemonDelivery.frame_json, Date.now());
    expect(sameJson(daemonReply, responseMessage), "daemon decoded response mismatch");

    const expiredSession = "relay-websocket-bridge-expired";
    const expiredDaemon = app.createRelayEndpoint(expiredSession, "daemon", 1);
    const expiredDaemonSocket = await connectEndpoint(expiredSession, "daemon");
    const expiredCompanionSocket = await connectEndpoint(expiredSession, "companion");
    const expiredFrame = app.relayEndpointNextFrame(
      expiredDaemon,
      app.livePingMessage("relay-websocket-expired"),
      Date.now() - 10000,
    );
    sendFrame(expiredDaemonSocket, expiredFrame);
    const expiredAck = await waitFor(
      expiredDaemonSocket,
      (message) => message.kind === "queued" && message.route?.sender === "daemon",
      "expired frame ack",
    );
    const expiredDropped = await expectNoFrame(
      expiredCompanionSocket,
      (message) => message.kind === "frame",
      300,
    );

    closeState(daemonSocket);
    closeState(companionSocket);
    closeState(expiredDaemonSocket);
    closeState(expiredCompanionSocket);

    await new Promise((resolve) => setTimeout(resolve, 100));
    const finalHealth = await fetch(healthUrl).then((response) => response.json());
    return {
      status: "ok",
      title: document.title,
      sessionId,
      daemonRoute: daemonAck.route,
      companionRoute: companionAck.route,
      duplicateRejected: duplicateError.kind === "error",
      expiredRoute: expiredAck.route,
      expiredDropped,
      companionMessageType: companionMessage.type,
      daemonReplyType: daemonReply.type,
      daemonNextSequence: daemon.nextSequence,
      companionNextSequence: companion.nextSequence,
      finalHealth,
    };
  }, bridge);
}

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  const pwaInfo = await startPwaServer();
  pwaServer = pwaInfo.server;
  const bridgeInfo = await startRelayWebSocketBridge();
  bridgeServer = bridgeInfo.server;
  const executablePath = browserExecutablePath();
  browser = await chromium.launch({
    headless: true,
    executablePath,
  });
  const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
  await page.goto(`${pwaInfo.url}/index.html`, { waitUntil: "networkidle" });
  await page.screenshot({ path: screenshotPath, fullPage: true });
  const result = await runBridgeSmoke(page, {
    websocketUrl: bridgeInfo.websocketUrl,
    healthUrl: bridgeInfo.healthUrl,
  });
  assert.equal(result.status, "ok");
  assert.equal(result.companionMessageType, "approval_request");
  assert.equal(result.daemonReplyType, "approval_response");
  assert.equal(result.duplicateRejected, true);
  assert.equal(result.expiredDropped, true);
  assert.equal(result.finalHealth.queuedFrames, 0);
  assert.equal(result.finalHealth.stats.acceptedFrames, 3);
  assert.equal(result.finalHealth.stats.deliveredFrames, 2);
  assert.equal(result.finalHealth.stats.expiredFrames, 1);
  assert.equal(result.finalHealth.stats.rejectedFrames, 1);

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective: "Verify relay frame JSON across a browser-native WebSocket bridge candidate",
    pwaUrl: pwaInfo.url,
    websocketUrl: bridgeInfo.websocketUrl,
    healthUrl: bridgeInfo.healthUrl,
    browserExecutablePath: executablePath || "playwright-default",
    screenshotPath,
    result,
  };
  await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  console.log(`RA_PWA_RELAY_WEBSOCKET_BRIDGE_OK ${evidencePath}`);
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
