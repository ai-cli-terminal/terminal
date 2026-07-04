import { createHash, createHmac, createPublicKey, timingSafeEqual, verify } from "node:crypto";
import { createServer } from "node:http";
import { pathToFileURL } from "node:url";

const RELAY_PROTOCOL_VERSION = 1;
const DEFAULT_RELAY_SESSION_TTL_MS = 5 * 60 * 1000;
const RELAY_TICKET_MAC_ALG_HMAC_SHA256 = "hmac-sha256";
const RELAY_TICKET_MAC_ALG_ED25519 = "ed25519";
const MAX_RELAY_SESSION_ID_LENGTH = 96;
const MIN_RELAY_SESSION_TOKEN_LENGTH = 32;
const MAX_RELAY_SESSION_TOKEN_LENGTH = 128;
const MAX_RELAY_DEVICE_ID_LENGTH = 96;
const MAX_RELAY_PAYLOAD_JSON_BYTES = 1 << 20;
const MAX_HTTP_BODY_BYTES = MAX_RELAY_PAYLOAD_JSON_BYTES + 4096;
const MAX_WS_MESSAGE_BYTES = MAX_RELAY_PAYLOAD_JSON_BYTES + 4096;
const MIN_RELAY_TICKET_HMAC_KEY_BYTES = 32;
const WEBSOCKET_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

export function hmacKeysFromEnv(env = process.env) {
  if (env.AI_TERMINAL_RELAY_HMAC_KEYS_JSON) {
    const parsed = JSON.parse(env.AI_TERMINAL_RELAY_HMAC_KEYS_JSON);
    if (!Array.isArray(parsed)) {
      throw new Error("AI_TERMINAL_RELAY_HMAC_KEYS_JSON must be a JSON array");
    }
    return parsed.map((entry) => ({
      keyId: entry.key_id || entry.keyId,
      secret: entry.secret,
    }));
  }
  if (env.AI_TERMINAL_RELAY_HMAC_SECRET) {
    return [
      {
        keyId: env.AI_TERMINAL_RELAY_HMAC_KEY_ID || undefined,
        secret: env.AI_TERMINAL_RELAY_HMAC_SECRET,
      },
    ];
  }
  throw new Error(
    "relay verifier keys missing; set AI_TERMINAL_RELAY_ED25519_PUBLIC_KEY_HEX, AI_TERMINAL_RELAY_ED25519_PUBLIC_KEYS_JSON, AI_TERMINAL_RELAY_HMAC_SECRET, or AI_TERMINAL_RELAY_HMAC_KEYS_JSON",
  );
}

export function publicKeysFromEnv(env = process.env) {
  if (env.AI_TERMINAL_RELAY_ED25519_PUBLIC_KEYS_JSON) {
    const parsed = JSON.parse(env.AI_TERMINAL_RELAY_ED25519_PUBLIC_KEYS_JSON);
    if (!Array.isArray(parsed)) {
      throw new Error("AI_TERMINAL_RELAY_ED25519_PUBLIC_KEYS_JSON must be a JSON array");
    }
    return parsed.map((entry) => ({
      keyId: entry.key_id || entry.keyId,
      publicKeyHex: entry.public_key_hex || entry.publicKeyHex,
    }));
  }
  if (env.AI_TERMINAL_RELAY_ED25519_PUBLIC_KEY_HEX) {
    return [
      {
        keyId: env.AI_TERMINAL_RELAY_ED25519_KEY_ID || undefined,
        publicKeyHex: env.AI_TERMINAL_RELAY_ED25519_PUBLIC_KEY_HEX,
      },
    ];
  }
  return [];
}

export function verifierKeysFromEnv(env = process.env) {
  const publicKeys = publicKeysFromEnv(env);
  if (publicKeys.length > 0) {
    return { publicKeys, hmacKeys: env.AI_TERMINAL_RELAY_ALLOW_HMAC_KEYS === "1" ? hmacKeysFromEnv(env) : [] };
  }
  return { publicKeys, hmacKeys: hmacKeysFromEnv(env) };
}

export function createRelayService({
  host = "127.0.0.1",
  port = 8080,
  hmacKeys,
  publicKeys,
  now = () => Date.now(),
} = {}) {
  const verifierKeys = normalizeVerifierKeys({ hmacKeys, publicKeys });
  const sessions = new Map();
  const tickets = new Map();
  const openSockets = new Set();
  const startedAtMs = now();
  const stats = {
    registeredTickets: 0,
    rejectedTickets: 0,
    acceptedConnects: 0,
    rejectedConnects: 0,
    openedConnections: 0,
    closedConnections: 0,
    acceptedFrames: 0,
    deliveredFrames: 0,
    rejectedFrames: 0,
    expiredFrames: 0,
    badRequests: 0,
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
        pruneExpiredTickets(tickets, now());
        writeJson(res, 200, healthBody({ sessions, tickets, stats, verifierKeys, startedAtMs, now }));
        return;
      }
      if (req.method === "POST" && url.pathname === "/sessions") {
        const signedTicket = JSON.parse(await readBody(req, MAX_HTTP_BODY_BYTES));
        const ticket = validateSignedRelaySessionTicket(signedTicket, verifierKeys);
        if (now() >= ticket.expires_at_ms) {
          throw new Error("relay session ticket expired");
        }
        tickets.set(ticket.session_id, ticket);
        stats.registeredTickets += 1;
        writeJson(res, 201, {
          status: "registered",
          session_id: ticket.session_id,
          expires_at_ms: ticket.expires_at_ms,
        });
        return;
      }
      stats.badRequests += 1;
      writeJson(res, 404, { status: "error", message: "not found" });
    } catch (error) {
      if (req.method === "POST") {
        stats.rejectedTickets += 1;
      } else {
        stats.badRequests += 1;
      }
      writeJson(res, 400, {
        status: "error",
        message: error.message || "bad relay request",
      });
    }
  });

  server.on("connection", (socket) => {
    openSockets.add(socket);
    socket.on("close", () => openSockets.delete(socket));
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
      stats.badRequests += 1;
      rejectUpgrade(socket);
      return;
    }

    const accept = createHash("sha1").update(`${key}${WEBSOCKET_GUID}`, "binary").digest("base64");
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
        handleSocketData(conn, chunk, sessions, tickets, stats, now);
      } catch (error) {
        stats.rejectedFrames += 1;
        sendJson(conn, {
          kind: "error",
          message: error.message || "bad websocket frame",
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

  return {
    server,
    host,
    port,
    stats,
    sessions,
    tickets,
    async start() {
      await listen(server, host, port);
      const address = server.address();
      const actualHost = address.address === "::" ? "127.0.0.1" : address.address;
      const httpUrl = `http://${actualHost}:${address.port}`;
      return {
        httpUrl,
        healthUrl: `${httpUrl}/health`,
        sessionUrl: `${httpUrl}/sessions`,
        websocketUrl: `${httpUrl.replace(/^http:/, "ws:")}/relay`,
      };
    },
    async close() {
      for (const socket of openSockets) {
        socket.destroy();
      }
      await closeServer(server);
    },
  };
}

function normalizeHmacKeys(hmacKeys) {
  if (hmacKeys === undefined) {
    return [];
  }
  if (!Array.isArray(hmacKeys)) {
    throw new Error("relay hmac verifier keys must be an array");
  }
  return hmacKeys.map((entry) => {
    const keyId = entry.keyId || entry.key_id || undefined;
    if (keyId !== undefined && !validRelayTicketKeyId(keyId)) {
      throw new Error("relay ticket key_id format error");
    }
    const secret = entry.secret;
    if (typeof secret !== "string" || Buffer.byteLength(secret, "utf8") < MIN_RELAY_TICKET_HMAC_KEY_BYTES) {
      throw new Error("relay ticket hmac key too short");
    }
    return { alg: RELAY_TICKET_MAC_ALG_HMAC_SHA256, keyId, secret };
  });
}

function normalizePublicKeys(publicKeys) {
  if (publicKeys === undefined) {
    return [];
  }
  if (!Array.isArray(publicKeys)) {
    throw new Error("relay public verifier keys must be an array");
  }
  return publicKeys.map((entry) => {
    const keyId = entry.keyId || entry.key_id || undefined;
    if (keyId !== undefined && !validRelayTicketKeyId(keyId)) {
      throw new Error("relay ticket key_id format error");
    }
    const publicKeyHex = entry.publicKeyHex || entry.public_key_hex;
    if (typeof publicKeyHex !== "string" || !/^[0-9a-f]{64}$/i.test(publicKeyHex)) {
      throw new Error("relay ed25519 public key format error");
    }
    return {
      alg: RELAY_TICKET_MAC_ALG_ED25519,
      keyId,
      publicKeyHex: publicKeyHex.toLowerCase(),
      publicKey: ed25519PublicKeyFromRawHex(publicKeyHex),
    };
  });
}

function normalizeVerifierKeys({ hmacKeys, publicKeys }) {
  const keys = [...normalizePublicKeys(publicKeys), ...normalizeHmacKeys(hmacKeys)];
  if (keys.length === 0) {
    throw new Error("relay verifier keys missing");
  }
  return keys;
}

function healthBody({ sessions, tickets, stats, verifierKeys, startedAtMs, now }) {
  return {
    status: "ok",
    relay_protocol_version: RELAY_PROTOCOL_VERSION,
    uptime_ms: Math.max(0, now() - startedAtMs),
    sessions: sessions.size,
    tickets: tickets.size,
    queuedFrames: queuedFrames(sessions),
    stats,
    limits: {
      maxHttpBodyBytes: MAX_HTTP_BODY_BYTES,
      maxWebSocketMessageBytes: MAX_WS_MESSAGE_BYTES,
      maxPayloadJsonBytes: MAX_RELAY_PAYLOAD_JSON_BYTES,
    },
    logging: {
      payloadJson: "disabled",
      sessionTokens: "disabled",
      setupJson: "disabled",
    },
    verifierKeys: verifierKeyCounts(verifierKeys),
  };
}

function verifierKeyCounts(verifierKeys) {
  return {
    ed25519: verifierKeys.filter((key) => key.alg === RELAY_TICKET_MAC_ALG_ED25519).length,
    hmacSha256: verifierKeys.filter((key) => key.alg === RELAY_TICKET_MAC_ALG_HMAC_SHA256).length,
  };
}

function handleSocketData(conn, chunk, sessions, tickets, stats, now) {
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
      authenticateSocketMessage(conn, parsed.payload.toString("utf8"), sessions, tickets, stats, now);
      continue;
    }
    routeSocketMessage(conn, parsed.payload.toString("utf8"), sessions, stats, now);
  }
}

function authenticateSocketMessage(conn, text, sessions, tickets, stats, now) {
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
    validateRelaySessionConnect(ticket, connect, now());
    conn.authenticated = true;
    const session = getRelaySession(sessions, conn.sessionId);
    connectionSet(session, conn.role).add(conn);
    stats.acceptedConnects += 1;
    sendJson(conn, {
      kind: "connected",
      session_id: conn.sessionId,
      peer: conn.role,
    });
    flushQueuedToRecipient(sessions, conn.sessionId, conn.role, stats, now);
  } catch (error) {
    stats.rejectedConnects += 1;
    sendJson(conn, {
      kind: "error",
      message: error.message || "relay websocket auth failed",
    });
    closeWebSocket(conn);
  }
}

function routeSocketMessage(conn, text, sessions, stats, now) {
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
    flushQueuedToRecipient(sessions, route.session_id, recipientFromSender(route.sender), stats, now);
  } catch (error) {
    stats.rejectedFrames += 1;
    sendJson(conn, {
      kind: "error",
      message: error.message || "bad relay frame",
    });
  }
}

function flushQueuedToRecipient(sessions, sessionId, recipient, stats, now) {
  const session = sessions.get(sessionId);
  if (!session) {
    return;
  }
  const connections = connectionSet(session, recipient);
  if (connections.size === 0) {
    return;
  }
  const queue = recipientQueue(session, recipient);
  const nowMs = now();
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

function validateSignedRelaySessionTicket(signed, verifierKeys) {
  validateRelaySessionTicket(signed?.ticket);
  if (![RELAY_TICKET_MAC_ALG_HMAC_SHA256, RELAY_TICKET_MAC_ALG_ED25519].includes(signed.mac_alg)) {
    throw new Error("relay ticket mac_alg format error");
  }
  const expectedMacHexLength = signed.mac_alg === RELAY_TICKET_MAC_ALG_ED25519 ? 128 : 64;
  if (
    typeof signed.mac_hex !== "string" ||
    !new RegExp(`^[0-9a-f]{${expectedMacHexLength}}$`, "i").test(signed.mac_hex)
  ) {
    throw new Error("relay ticket mac_hex format error");
  }
  if (signed.key_id !== undefined && !validRelayTicketKeyId(signed.key_id)) {
    throw new Error("relay ticket key_id format error");
  }
  const candidateKeys = verifierKeys.filter(
    (entry) =>
      entry.alg === signed.mac_alg &&
      (signed.key_id === undefined || entry.keyId === signed.key_id),
  );
  if (candidateKeys.length === 0) {
    throw new Error("relay ticket verifier key missing");
  }
  const actual = Buffer.from(signed.mac_hex, "hex");
  for (const key of candidateKeys) {
    if (key.alg === RELAY_TICKET_MAC_ALG_HMAC_SHA256 && verifyHmacTicket(signed.ticket, key, actual)) {
      return signed.ticket;
    }
    if (key.alg === RELAY_TICKET_MAC_ALG_ED25519 && verifyEd25519Ticket(signed.ticket, key, actual)) {
      return signed.ticket;
    }
  }
  throw new Error("relay ticket mac mismatch");
}

function verifyHmacTicket(ticket, key, actual) {
  const expected = Buffer.from(relaySessionTicketHmacSha256Hex(ticket, key.secret), "hex");
  return actual.length === expected.length && timingSafeEqual(actual, expected);
}

function verifyEd25519Ticket(ticket, key, actual) {
  return verify(null, Buffer.from(relaySessionTicketSigningPayload(ticket), "utf8"), key.publicKey, actual);
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

function relaySessionTicketHmacSha256Hex(ticket, secret) {
  return createHmac("sha256", Buffer.from(secret, "utf8"))
    .update(relaySessionTicketSigningPayload(ticket), "utf8")
    .digest("hex");
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

function ed25519PublicKeyFromRawHex(publicKeyHex) {
  const spki = Buffer.concat([
    Buffer.from("302a300506032b6570032100", "hex"),
    Buffer.from(publicKeyHex, "hex"),
  ]);
  return createPublicKey({ key: spki, format: "der", type: "spki" });
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

function routeEnvelope(frame) {
  validateRelayFrameMetadata(frame);
  return {
    relay_protocol_version: frame.relay_protocol_version,
    session_id: frame.session_id,
    sender: frame.sender,
    sequence: frame.sequence,
    sent_at_ms: frame.sent_at_ms,
    expires_at_ms: frame.expires_at_ms,
    payload_json_bytes: Buffer.byteLength(frame.payload_json, "utf8"),
  };
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
    Buffer.byteLength(frame.payload_json, "utf8") > MAX_RELAY_PAYLOAD_JSON_BYTES
  ) {
    throw new Error("relay payload_json format error");
  }
}

function getRelaySession(sessions, sessionId) {
  let session = sessions.get(sessionId);
  if (!session) {
    session = {
      daemonConnections: new Set(),
      companionConnections: new Set(),
      daemonToCompanion: [],
      companionToDaemon: [],
      daemonLastSequence: 0,
      companionLastSequence: 0,
    };
    sessions.set(sessionId, session);
  }
  return session;
}

function connectionSet(session, role) {
  return role === "daemon" ? session.daemonConnections : session.companionConnections;
}

function senderQueue(session, sender) {
  return sender === "daemon" ? session.daemonToCompanion : session.companionToDaemon;
}

function recipientQueue(session, recipient) {
  return recipient === "daemon" ? session.companionToDaemon : session.daemonToCompanion;
}

function recipientFromSender(sender) {
  return sender === "daemon" ? "companion" : "daemon";
}

function lastSequenceKey(sender) {
  return sender === "daemon" ? "daemonLastSequence" : "companionLastSequence";
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

function pruneExpiredTickets(tickets, nowMs) {
  for (const [sessionId, ticket] of tickets.entries()) {
    if (nowMs >= ticket.expires_at_ms) {
      tickets.delete(sessionId);
    }
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

function readBody(req, maxBytes) {
  return new Promise((resolve, reject) => {
    let size = 0;
    const chunks = [];
    req.on("data", (chunk) => {
      size += chunk.length;
      if (size > maxBytes) {
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

function writeJson(res, status, body) {
  const text = `${JSON.stringify(body)}\n`;
  res.writeHead(status, corsHeaders({ "content-type": "application/json", "content-length": Buffer.byteLength(text) }));
  res.end(text);
}

function corsHeaders(extra = {}) {
  return {
    "access-control-allow-origin": "*",
    "access-control-allow-methods": "GET, POST, OPTIONS",
    "access-control-allow-headers": "content-type",
    ...extra,
  };
}

function listen(server, host, port) {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, host, () => {
      server.off("error", reject);
      resolve();
    });
  });
}

function closeServer(server) {
  return new Promise((resolve) => {
    server.close(() => resolve());
  });
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

function validRelayTicketKeyId(value) {
  return typeof value === "string" && /^[A-Za-z0-9._:-]{1,64}$/.test(value);
}

async function runCli() {
  const verifierKeys = verifierKeysFromEnv(process.env);
  const service = createRelayService({
    host: process.env.AI_TERMINAL_RELAY_HOST || "127.0.0.1",
    port: Number.parseInt(process.env.AI_TERMINAL_RELAY_PORT || "8080", 10),
    ...verifierKeys,
  });
  const urls = await service.start();
  console.log(
    JSON.stringify({
      status: "listening",
      service: "ai-terminal-self-hosted-relay",
      httpUrl: urls.httpUrl,
      healthUrl: urls.healthUrl,
      websocketUrl: urls.websocketUrl,
      verifierKeysConfigured: true,
      payloadLogging: "disabled",
    }),
  );
  for (const signal of ["SIGINT", "SIGTERM"]) {
    process.on(signal, async () => {
      await service.close();
      process.exit(0);
    });
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  runCli().catch((error) => {
    console.error(`relay self-hosted service failed: ${error.message || error}`);
    process.exit(1);
  });
}
