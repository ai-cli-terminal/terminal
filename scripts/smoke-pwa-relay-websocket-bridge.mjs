import assert from "node:assert/strict";
import { createHash, createHmac, timingSafeEqual } from "node:crypto";
import { existsSync } from "node:fs";
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright";

const RELAY_PROTOCOL_VERSION = 1;
const DEFAULT_RELAY_SESSION_TTL_MS = 5 * 60 * 1000;
const RELAY_TICKET_MAC_ALG_HMAC_SHA256 = "hmac-sha256";
const MAX_RELAY_SESSION_ID_LENGTH = 96;
const MIN_RELAY_SESSION_TOKEN_LENGTH = 32;
const MAX_RELAY_SESSION_TOKEN_LENGTH = 128;
const MAX_RELAY_DEVICE_ID_LENGTH = 96;
const MAX_RELAY_PAYLOAD_JSON_BYTES = 1 << 20;
const MIN_RELAY_TICKET_HMAC_KEY_BYTES = 32;
const MAX_WS_MESSAGE_BYTES = MAX_RELAY_PAYLOAD_JSON_BYTES + 4096;
const WEBSOCKET_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const RELAY_TICKET_HMAC_SECRET = "relay-ticket-secret-1234567890abcdef";

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

function validRelaySessionToken(value) {
  return (
    typeof value === "string" &&
    value.length >= MIN_RELAY_SESSION_TOKEN_LENGTH &&
    value.length <= MAX_RELAY_SESSION_TOKEN_LENGTH &&
    /^[A-Za-z0-9._:~-]+$/.test(value)
  );
}

function validRelayDeviceId(value) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_RELAY_DEVICE_ID_LENGTH &&
    /^[A-Za-z0-9._:-]+$/.test(value)
  );
}

function validRelayPubkeyHex(value) {
  return typeof value === "string" && /^[0-9a-f]{64}$/i.test(value);
}

function validateRelaySessionTicket(ticket) {
  if (ticket?.relay_protocol_version !== RELAY_PROTOCOL_VERSION) {
    throw new Error("unsupported relay session protocol version");
  }
  if (ticket.transport !== "websocket") {
    throw new Error("relay session transport format error");
  }
  if (!validRelaySessionId(ticket.session_id)) {
    throw new Error("relay session_id format error");
  }
  if (!validRelaySessionToken(ticket.session_token)) {
    throw new Error("relay session_token format error");
  }
  if (
    !Number.isSafeInteger(ticket.issued_at_ms) ||
    ticket.issued_at_ms <= 0 ||
    !Number.isSafeInteger(ticket.expires_at_ms) ||
    ticket.expires_at_ms <= ticket.issued_at_ms ||
    ticket.expires_at_ms - ticket.issued_at_ms > DEFAULT_RELAY_SESSION_TTL_MS
  ) {
    throw new Error("relay session expiry format error");
  }
  if (!validRelayPubkeyHex(ticket.daemon_pubkey_hex)) {
    throw new Error("relay daemon_pubkey_hex format error");
  }
  if (!validRelayDeviceId(ticket.companion_device_id)) {
    throw new Error("relay companion device_id format error");
  }
  if (!validRelayPubkeyHex(ticket.companion_noise_pubkey_hex)) {
    throw new Error("relay companion noise_pubkey_hex format error");
  }
  if (!validRelayPubkeyHex(ticket.companion_approval_pubkey_hex)) {
    throw new Error("relay companion approval_pubkey_hex format error");
  }
}

function relaySessionTicketSigningPayload(ticket) {
  validateRelaySessionTicket(ticket);
  return [
    "ai-terminal-relay-ticket-v1",
    `relay_protocol_version=${ticket.relay_protocol_version}`,
    `transport=${ticket.transport}`,
    `session_id=${ticket.session_id}`,
    `session_token=${ticket.session_token}`,
    `issued_at_ms=${ticket.issued_at_ms}`,
    `expires_at_ms=${ticket.expires_at_ms}`,
    `daemon_pubkey_hex=${ticket.daemon_pubkey_hex}`,
    `companion_device_id=${ticket.companion_device_id}`,
    `companion_noise_pubkey_hex=${ticket.companion_noise_pubkey_hex}`,
    `companion_approval_pubkey_hex=${ticket.companion_approval_pubkey_hex}`,
    "",
  ].join("\n");
}

function relaySessionTicketHmacSha256Hex(ticket, secret) {
  const secretBytes = Buffer.from(secret, "utf8");
  if (secretBytes.length < MIN_RELAY_TICKET_HMAC_KEY_BYTES) {
    throw new Error("relay ticket hmac key too short");
  }
  return createHmac("sha256", secretBytes)
    .update(relaySessionTicketSigningPayload(ticket), "utf8")
    .digest("hex");
}

function validateSignedRelaySessionTicket(signed, secret) {
  validateRelaySessionTicket(signed?.ticket);
  if (signed.mac_alg !== RELAY_TICKET_MAC_ALG_HMAC_SHA256) {
    throw new Error("relay ticket mac_alg format error");
  }
  if (typeof signed.mac_hex !== "string" || !/^[0-9a-f]{64}$/i.test(signed.mac_hex)) {
    throw new Error("relay ticket mac_hex format error");
  }
  const expected = Buffer.from(relaySessionTicketHmacSha256Hex(signed.ticket, secret), "hex");
  const actual = Buffer.from(signed.mac_hex, "hex");
  if (actual.length !== expected.length || !timingSafeEqual(actual, expected)) {
    throw new Error("relay ticket mac mismatch");
  }
  return signed.ticket;
}

function validateRelaySessionConnectMetadata(connect) {
  if (connect?.relay_protocol_version !== RELAY_PROTOCOL_VERSION) {
    throw new Error("unsupported relay session protocol version");
  }
  if (!validRelaySessionId(connect.session_id)) {
    throw new Error("relay session_id format error");
  }
  if (connect.peer !== "daemon" && connect.peer !== "companion") {
    throw new Error("relay peer format error");
  }
  if (!validRelaySessionToken(connect.session_token)) {
    throw new Error("relay session_token format error");
  }
  if (connect.peer === "daemon") {
    if (!validRelayPubkeyHex(connect.daemon_pubkey_hex)) {
      throw new Error("relay daemon_pubkey_hex format error");
    }
    if (
      connect.device_id !== undefined ||
      connect.noise_pubkey_hex !== undefined ||
      connect.approval_pubkey_hex !== undefined
    ) {
      throw new Error("relay daemon connect companion field error");
    }
    return;
  }
  if (!validRelayDeviceId(connect.device_id)) {
    throw new Error("relay companion device_id format error");
  }
  if (!validRelayPubkeyHex(connect.noise_pubkey_hex)) {
    throw new Error("relay companion noise_pubkey_hex format error");
  }
  if (!validRelayPubkeyHex(connect.approval_pubkey_hex)) {
    throw new Error("relay companion approval_pubkey_hex format error");
  }
  if (connect.daemon_pubkey_hex !== undefined) {
    throw new Error("relay companion connect daemon field error");
  }
}

function validateRelaySessionConnect(ticket, connect, nowMs) {
  validateRelaySessionTicket(ticket);
  validateRelaySessionConnectMetadata(connect);
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("relay session now_ms format error");
  }
  if (nowMs >= ticket.expires_at_ms) {
    throw new Error("relay session expired");
  }
  if (connect.session_id !== ticket.session_id) {
    throw new Error("relay session_id mismatch");
  }
  if (connect.session_token !== ticket.session_token) {
    throw new Error("relay session_token mismatch");
  }
  if (connect.peer === "daemon") {
    if (connect.daemon_pubkey_hex !== ticket.daemon_pubkey_hex) {
      throw new Error("relay daemon pubkey mismatch");
    }
    return;
  }
  if (connect.device_id !== ticket.companion_device_id) {
    throw new Error("relay companion device_id mismatch");
  }
  if (connect.noise_pubkey_hex !== ticket.companion_noise_pubkey_hex) {
    throw new Error("relay companion noise pubkey mismatch");
  }
  if (connect.approval_pubkey_hex !== ticket.companion_approval_pubkey_hex) {
    throw new Error("relay companion approval pubkey mismatch");
  }
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

function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on("data", (chunk) => {
      size += chunk.length;
      if (size > MAX_WS_MESSAGE_BYTES) {
        reject(new Error("request body too large"));
        req.destroy();
        return;
      }
      chunks.push(chunk);
    });
    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
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

async function startRelayWebSocketBridge() {
  const sessions = new Map();
  const tickets = new Map();
  const stats = {
    acceptedFrames: 0,
    deliveredFrames: 0,
    expiredFrames: 0,
    rejectedFrames: 0,
    acceptedConnects: 0,
    rejectedConnects: 0,
    registeredTickets: 0,
    rejectedTickets: 0,
    openedConnections: 0,
    closedConnections: 0,
  };
  const server = createServer(async (req, res) => {
    try {
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
          tickets: tickets.size,
          queuedFrames: queuedFrames(sessions),
          stats,
        });
        return;
      }
      if (req.method === "POST" && url.pathname === "/sessions") {
        const signedTicket = JSON.parse(await readBody(req));
        const ticket = validateSignedRelaySessionTicket(signedTicket, RELAY_TICKET_HMAC_SECRET);
        tickets.set(ticket.session_id, ticket);
        stats.registeredTickets += 1;
        writeJson(res, 201, {
          status: "registered",
          session_id: ticket.session_id,
        });
        return;
      }
      writeJson(res, 404, { status: "error", message: "not found" });
    } catch (err) {
      if (req.method === "POST") {
        stats.rejectedTickets += 1;
      }
      writeJson(res, 400, {
        status: "error",
        message: err.message || "bad relay session request",
      });
    }
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
      authenticated: false,
      closed: false,
    };
    stats.openedConnections += 1;

    socket.on("data", (chunk) => {
      try {
        handleSocketData(conn, chunk, sessions, tickets, stats);
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
    sessionUrl: `${httpUrl}/sessions`,
    websocketUrl: `${httpUrl.replace(/^http:/, "ws:")}/relay`,
  };
}

function handleSocketData(conn, chunk, sessions, tickets, stats) {
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
    if (!conn.authenticated) {
      authenticateSocketMessage(conn, parsed.payload.toString("utf8"), sessions, tickets, stats);
      continue;
    }
    routeSocketMessage(conn, parsed.payload.toString("utf8"), sessions, stats);
  }
}

function authenticateSocketMessage(conn, text, sessions, tickets, stats) {
  try {
    const connect = JSON.parse(text);
    validateRelaySessionConnectMetadata(connect);
    if (connect.session_id !== conn.sessionId) {
      throw new Error("relay websocket session mismatch");
    }
    if (connect.peer !== conn.role) {
      throw new Error("relay websocket peer mismatch");
    }
    const ticket = tickets.get(connect.session_id);
    if (!ticket) {
      throw new Error("relay session ticket missing");
    }
    validateRelaySessionConnect(ticket, connect, Date.now());
    conn.authenticated = true;
    const session = getRelaySession(sessions, conn.sessionId);
    connectionSet(session, conn.role).add(conn);
    stats.acceptedConnects += 1;
    sendJson(conn, {
      kind: "connected",
      session_id: conn.sessionId,
      peer: conn.role,
    });
    flushQueuedToRecipient(sessions, conn.sessionId, conn.role, stats);
  } catch (err) {
    stats.rejectedConnects += 1;
    sendJson(conn, {
      kind: "error",
      message: err.message || "relay websocket auth failed",
    });
    closeWebSocket(conn);
  }
}

function routeSocketMessage(conn, text, sessions, stats) {
  try {
    if (!conn.authenticated) {
      throw new Error("relay websocket unauthenticated");
    }
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
  return page.evaluate(async ({ websocketUrl, healthUrl, sessionUrl, relayTicketSecret }) => {
    const app = await import(new URL("./app.mjs", window.location.href).href);
    const expect = (condition, message) => {
      if (!condition) {
        throw new Error(message);
      }
    };
    const sameJson = (left, right) => JSON.stringify(left) === JSON.stringify(right);
    const relaySessionToken = "token_1234567890abcdef1234567890abcdef";
    const relayKeys = {
      daemonPubkeyHex: "a".repeat(64),
      companionDeviceId: "web-1234abcd",
      companionNoisePubkeyHex: "b".repeat(64),
      companionApprovalPubkeyHex: "c".repeat(64),
    };

    const ticketFor = (sessionId, overrides = {}) =>
      app.createRelaySessionTicket({
        sessionId,
        sessionToken: relaySessionToken,
        issuedAtMs: Date.now(),
        ...relayKeys,
        ...overrides,
      });

    const signTicket = (ticket) =>
      app.createSignedRelaySessionTicket(ticket, relayTicketSecret, window.crypto);

    const registerTicket = async (ticket) => {
      const signedTicket = await signTicket(ticket);
      const response = await fetch(sessionUrl, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(signedTicket),
      });
      const body = await response.json();
      expect(
        response.ok && body.status === "registered",
        `ticket registration failed: ${body.message}`,
      );
      return body;
    };

    const rejectTicketRegistration = async (body, pattern) => {
      const response = await fetch(sessionUrl, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(body),
      });
      const result = await response.json();
      expect(
        !response.ok && pattern.test(result.message || ""),
        "ticket registration was not rejected",
      );
      return result;
    };

    const connectEndpoint = (sessionId, role) =>
      new Promise((resolve, reject) => {
        const url = new URL(websocketUrl);
        url.searchParams.set("session_id", sessionId);
        url.searchParams.set("role", role);
        const ws = new WebSocket(url);
        let closeResolve = () => {};
        let failTimer = null;
        const state = {
          ws,
          messages: [],
          waiters: [],
          isClosed: false,
          closed: new Promise((resolve) => {
            closeResolve = resolve;
          }),
        };
        failTimer = setTimeout(() => {
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
        ws.addEventListener("close", () => {
          clearTimeout(failTimer);
          state.isClosed = true;
          closeResolve();
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

    const authenticateEndpoint = (state, connect) => {
      state.ws.send(app.relaySessionConnectJson(connect));
      return waitFor(
        state,
        (message) => message.kind === "connected" && message.peer === connect.peer,
        `${connect.peer} connect ack`,
      );
    };

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

    const waitForClosed = (state, label, timeoutMs = 2000) => {
      if (state.isClosed) {
        return Promise.resolve();
      }
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          reject(new Error(`timeout waiting for ${label} close`));
        }, timeoutMs);
        state.closed.then(() => {
          clearTimeout(timer);
          resolve();
        });
      });
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

    const unsignedTicketRejected = await rejectTicketRegistration(
      ticketFor("relay-websocket-bridge-unsigned"),
      /session|ticket|mac/,
    );
    const badMacTicket = await signTicket(ticketFor("relay-websocket-bridge-bad-mac"));
    badMacTicket.mac_hex = "0".repeat(64);
    const badMacRejected = await rejectTicketRegistration(badMacTicket, /mac/);

    const sessionId = "relay-websocket-bridge-1";
    const ticket = ticketFor(sessionId);
    await registerTicket(ticket);
    const daemon = app.createRelayEndpoint(sessionId, "daemon");
    const companion = app.createRelayEndpoint(sessionId, "companion");
    const daemonSocket = await connectEndpoint(sessionId, "daemon");
    const companionSocket = await connectEndpoint(sessionId, "companion");
    const daemonConnected = await authenticateEndpoint(
      daemonSocket,
      app.relaySessionConnect(ticket, "daemon"),
    );
    const companionConnected = await authenticateEndpoint(
      companionSocket,
      app.relaySessionConnect(ticket, "companion"),
    );
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
    const expiredTicket = ticketFor(expiredSession);
    await registerTicket(expiredTicket);
    const expiredDaemon = app.createRelayEndpoint(expiredSession, "daemon", 1);
    const expiredDaemonSocket = await connectEndpoint(expiredSession, "daemon");
    const expiredCompanionSocket = await connectEndpoint(expiredSession, "companion");
    await authenticateEndpoint(expiredDaemonSocket, app.relaySessionConnect(expiredTicket, "daemon"));
    await authenticateEndpoint(
      expiredCompanionSocket,
      app.relaySessionConnect(expiredTicket, "companion"),
    );
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

    const expiredTicketNowMs = Date.now();
    const expiredTicketSession = "relay-websocket-bridge-expired-ticket";
    const expiredConnectTicket = ticketFor(expiredTicketSession, {
      issuedAtMs: expiredTicketNowMs - 2000,
      expiresAtMs: expiredTicketNowMs - 1000,
    });
    await registerTicket(expiredConnectTicket);
    const expiredTicketSocket = await connectEndpoint(expiredTicketSession, "companion");
    expiredTicketSocket.ws.send(
      app.relaySessionConnectJson(app.relaySessionConnect(expiredConnectTicket, "companion")),
    );
    const expiredTicketError = await waitFor(
      expiredTicketSocket,
      (message) => message.kind === "error" && /expired/.test(message.message || ""),
      "expired ticket connect rejection",
    );

    const rotationOldSession = "relay-websocket-bridge-rotate-old";
    const rotationNewSession = "relay-websocket-bridge-rotate-new";
    const rotationOldTicket = ticketFor(rotationOldSession, {
      sessionToken: "token_rotate_old_1234567890abcdef1234567890",
    });
    const rotationNewTicket = ticketFor(rotationNewSession, {
      sessionToken: "token_rotate_new_1234567890abcdef1234567890",
    });
    expect(
      rotationOldTicket.session_token !== rotationNewTicket.session_token,
      "rotation did not change session token",
    );

    await registerTicket(rotationOldTicket);
    const rotationOldDaemon = app.createRelayEndpoint(rotationOldSession, "daemon");
    const rotationOldCompanion = app.createRelayEndpoint(rotationOldSession, "companion");
    const rotationOldDaemonSocket = await connectEndpoint(rotationOldSession, "daemon");
    const rotationOldCompanionSocket = await connectEndpoint(rotationOldSession, "companion");
    await authenticateEndpoint(
      rotationOldDaemonSocket,
      app.relaySessionConnect(rotationOldTicket, "daemon"),
    );
    await authenticateEndpoint(
      rotationOldCompanionSocket,
      app.relaySessionConnect(rotationOldTicket, "companion"),
    );
    const rotationOldPing = app.livePingMessage("relay-websocket-rotate-old");
    const rotationOldFrame = app.relayEndpointNextFrame(
      rotationOldDaemon,
      rotationOldPing,
      Date.now(),
    );
    sendFrame(rotationOldDaemonSocket, rotationOldFrame);
    const rotationOldAck = await waitFor(
      rotationOldDaemonSocket,
      (message) => message.kind === "queued" && message.route?.sender === "daemon",
      "rotation old frame ack",
    );
    const rotationOldDelivery = await waitFor(
      rotationOldCompanionSocket,
      (message) => message.kind === "frame" && message.route?.sender === "daemon",
      "rotation old delivery",
    );
    const rotationOldMessage = app.relayEndpointAcceptFrame(
      rotationOldCompanion,
      rotationOldDelivery.frame_json,
      Date.now(),
    );
    expect(sameJson(rotationOldMessage, rotationOldPing), "rotation old delivery mismatch");

    closeState(rotationOldDaemonSocket);
    closeState(rotationOldCompanionSocket);
    await Promise.all([
      waitForClosed(rotationOldDaemonSocket, "rotation old daemon"),
      waitForClosed(rotationOldCompanionSocket, "rotation old companion"),
    ]);

    await registerTicket(rotationNewTicket);
    const rotationNewDaemon = app.createRelayEndpoint(rotationNewSession, "daemon");
    const rotationNewCompanion = app.createRelayEndpoint(rotationNewSession, "companion");
    const rotationNewDaemonSocket = await connectEndpoint(rotationNewSession, "daemon");
    const rotationNewCompanionSocket = await connectEndpoint(rotationNewSession, "companion");
    const rotationNewDaemonConnected = await authenticateEndpoint(
      rotationNewDaemonSocket,
      app.relaySessionConnect(rotationNewTicket, "daemon"),
    );
    const rotationNewCompanionConnected = await authenticateEndpoint(
      rotationNewCompanionSocket,
      app.relaySessionConnect(rotationNewTicket, "companion"),
    );

    const rotationOldTokenSocket = await connectEndpoint(rotationNewSession, "companion");
    const rotationOldTokenConnect = {
      ...app.relaySessionConnect(rotationNewTicket, "companion"),
      session_token: rotationOldTicket.session_token,
    };
    rotationOldTokenSocket.ws.send(app.relaySessionConnectJson(rotationOldTokenConnect));
    const rotationOldTokenError = await waitFor(
      rotationOldTokenSocket,
      (message) => message.kind === "error" && /token/.test(message.message || ""),
      "rotation old token rejection",
    );

    const rotationNewPing = app.livePingMessage("relay-websocket-rotate-new");
    const rotationNewFrame = app.relayEndpointNextFrame(
      rotationNewCompanion,
      rotationNewPing,
      Date.now(),
    );
    sendFrame(rotationNewCompanionSocket, rotationNewFrame);
    const rotationNewAck = await waitFor(
      rotationNewCompanionSocket,
      (message) => message.kind === "queued" && message.route?.sender === "companion",
      "rotation new frame ack",
    );
    const rotationNewDelivery = await waitFor(
      rotationNewDaemonSocket,
      (message) => message.kind === "frame" && message.route?.sender === "companion",
      "rotation new delivery",
    );
    const rotationReconnectMessage = app.relayEndpointAcceptFrame(
      rotationNewDaemon,
      rotationNewDelivery.frame_json,
      Date.now(),
    );
    expect(sameJson(rotationReconnectMessage, rotationNewPing), "rotation reconnect mismatch");

    let rotationOldFrameIsolated = false;
    try {
      app.relayEndpointAcceptFrame(rotationNewDaemon, rotationOldDelivery.frame_json, Date.now());
    } catch (err) {
      rotationOldFrameIsolated = /session_id mismatch/.test(err.message || "");
    }
    expect(rotationOldFrameIsolated, "rotated endpoint accepted old session frame");

    const unauthenticatedSession = "relay-websocket-bridge-unauth";
    const unauthenticatedTicket = ticketFor(unauthenticatedSession);
    await registerTicket(unauthenticatedTicket);
    const unauthenticatedDaemon = app.createRelayEndpoint(unauthenticatedSession, "daemon");
    const unauthenticatedSocket = await connectEndpoint(unauthenticatedSession, "daemon");
    sendFrame(
      unauthenticatedSocket,
      app.relayEndpointNextFrame(
        unauthenticatedDaemon,
        app.livePingMessage("relay-websocket-unauth"),
        Date.now(),
      ),
    );
    const unauthenticatedError = await waitFor(
      unauthenticatedSocket,
      (message) =>
        message.kind === "error" && /session|peer|token|connect/.test(message.message || ""),
      "unauthenticated frame rejection",
    );

    const badTokenSession = "relay-websocket-bridge-bad-token";
    const badTokenTicket = ticketFor(badTokenSession);
    await registerTicket(badTokenTicket);
    const badTokenSocket = await connectEndpoint(badTokenSession, "companion");
    const badTokenConnect = {
      ...app.relaySessionConnect(badTokenTicket, "companion"),
      session_token: "wrong_1234567890abcdef1234567890abcdef",
    };
    badTokenSocket.ws.send(app.relaySessionConnectJson(badTokenConnect));
    const badTokenError = await waitFor(
      badTokenSocket,
      (message) => message.kind === "error" && /token/.test(message.message || ""),
      "bad token rejection",
    );

    closeState(daemonSocket);
    closeState(companionSocket);
    closeState(expiredDaemonSocket);
    closeState(expiredCompanionSocket);
    closeState(expiredTicketSocket);
    closeState(rotationOldDaemonSocket);
    closeState(rotationOldCompanionSocket);
    closeState(rotationNewDaemonSocket);
    closeState(rotationNewCompanionSocket);
    closeState(rotationOldTokenSocket);
    closeState(unauthenticatedSocket);
    closeState(badTokenSocket);

    await Promise.all([
      waitForClosed(daemonSocket, "daemon"),
      waitForClosed(companionSocket, "companion"),
      waitForClosed(expiredDaemonSocket, "expired daemon"),
      waitForClosed(expiredCompanionSocket, "expired companion"),
      waitForClosed(expiredTicketSocket, "expired ticket"),
      waitForClosed(rotationOldDaemonSocket, "rotation old daemon"),
      waitForClosed(rotationOldCompanionSocket, "rotation old companion"),
      waitForClosed(rotationNewDaemonSocket, "rotation new daemon"),
      waitForClosed(rotationNewCompanionSocket, "rotation new companion"),
      waitForClosed(rotationOldTokenSocket, "rotation old token"),
      waitForClosed(unauthenticatedSocket, "unauthenticated"),
      waitForClosed(badTokenSocket, "bad token"),
    ]);

    await new Promise((resolve) => setTimeout(resolve, 200));
    const finalHealth = await fetch(healthUrl).then((response) => response.json());
    return {
      status: "ok",
      title: document.title,
      sessionId,
      daemonConnected: daemonConnected.kind === "connected",
      companionConnected: companionConnected.kind === "connected",
      daemonRoute: daemonAck.route,
      companionRoute: companionAck.route,
      duplicateRejected: duplicateError.kind === "error",
      expiredRoute: expiredAck.route,
      expiredDropped,
      expiredTicketConnectRejected: expiredTicketError.kind === "error",
      rotationReconnected:
        rotationNewDaemonConnected.kind === "connected" &&
        rotationNewCompanionConnected.kind === "connected",
      rotationOldRoute: rotationOldAck.route,
      rotationNewRoute: rotationNewAck.route,
      rotationReconnectDelivered: rotationReconnectMessage.type === "ping",
      rotationOldTokenRejected: rotationOldTokenError.kind === "error",
      rotationOldFrameIsolated,
      unsignedTicketRejected: unsignedTicketRejected.status === "error",
      badMacTicketRejected: badMacRejected.status === "error",
      unauthenticatedFrameRejected: unauthenticatedError.kind === "error",
      badTokenRejected: badTokenError.kind === "error",
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
    sessionUrl: bridgeInfo.sessionUrl,
    relayTicketSecret: RELAY_TICKET_HMAC_SECRET,
  });
  assert.equal(result.status, "ok");
  assert.equal(result.daemonConnected, true);
  assert.equal(result.companionConnected, true);
  assert.equal(result.companionMessageType, "approval_request");
  assert.equal(result.daemonReplyType, "approval_response");
  assert.equal(result.duplicateRejected, true);
  assert.equal(result.expiredDropped, true);
  assert.equal(result.expiredTicketConnectRejected, true);
  assert.equal(result.rotationReconnected, true);
  assert.equal(result.rotationReconnectDelivered, true);
  assert.equal(result.rotationOldTokenRejected, true);
  assert.equal(result.rotationOldFrameIsolated, true);
  assert.equal(result.unsignedTicketRejected, true);
  assert.equal(result.badMacTicketRejected, true);
  assert.equal(result.unauthenticatedFrameRejected, true);
  assert.equal(result.badTokenRejected, true);
  assert.equal(result.finalHealth.queuedFrames, 0);
  assert.equal(result.finalHealth.stats.acceptedFrames, 5);
  assert.equal(result.finalHealth.stats.deliveredFrames, 4);
  assert.equal(result.finalHealth.stats.expiredFrames, 1);
  assert.equal(result.finalHealth.stats.rejectedFrames, 1);
  assert.equal(result.finalHealth.stats.acceptedConnects, 8);
  assert.equal(result.finalHealth.stats.rejectedConnects, 4);
  assert.equal(result.finalHealth.stats.registeredTickets, 7);
  assert.equal(result.finalHealth.stats.rejectedTickets, 2);
  assert.equal(result.finalHealth.stats.openedConnections, 12);
  assert.equal(result.finalHealth.stats.closedConnections, 12);

  const evidence = {
    status: "ok",
    generatedAt: new Date().toISOString(),
    objective:
      "Verify signed-ticket relay frame JSON plus rotation and reconnect across a browser-native WebSocket bridge candidate",
    pwaUrl: pwaInfo.url,
    websocketUrl: bridgeInfo.websocketUrl,
    healthUrl: bridgeInfo.healthUrl,
    sessionUrl: bridgeInfo.sessionUrl,
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
