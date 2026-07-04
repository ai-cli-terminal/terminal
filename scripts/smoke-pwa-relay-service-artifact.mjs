import assert from "node:assert/strict";
import { createPrivateKey, createPublicKey, randomBytes, sign } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createRelayEndpoint,
  createRelaySessionTicket,
  livePingMessage,
  livePongMessage,
  relayEndpointAcceptFrame,
  relayEndpointNextFrame,
  relaySessionTicketSigningPayload,
  relaySessionConnect,
  relayWebSocketConnectUrl,
} from "../pwa/app.mjs";
import { createRelayService } from "./relay-self-hosted-service.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-service-artifact");
const evidencePath =
  process.env.RA_PWA_RELAY_SERVICE_ARTIFACT_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-service-artifact.json");

const relayTicketSigningSeedHex = "11".repeat(32);
const relayTicketKeyId = "relay-smoke-ed25519";

async function main() {
  await mkdir(artifactRoot, { recursive: true });
  const service = createRelayService({
    host: "127.0.0.1",
    port: 0,
    publicKeys: [{ keyId: relayTicketKeyId, publicKeyHex: ed25519PublicKeyHex(relayTicketSigningSeedHex) }],
  });
  const urls = await service.start();
  try {
    const initialHealth = await getJson(urls.healthUrl);
    assert.equal(initialHealth.status, "ok");
    assert.equal(initialHealth.logging.payloadJson, "disabled");

    const nowMs = Date.now();
    const ticket = createRelaySessionTicket({
      sessionId: "relay-service-artifact",
      sessionToken: "token_relay_service_artifact_1234567890",
      issuedAtMs: nowMs,
      expiresAtMs: nowMs + 60_000,
      daemonPubkeyHex: "a".repeat(64),
      companionDeviceId: "web-service1",
      companionNoisePubkeyHex: "b".repeat(64),
      companionApprovalPubkeyHex: "c".repeat(64),
    });
    const signedTicket = ed25519SignedTicket(ticket, relayTicketSigningSeedHex, relayTicketKeyId);

    const unsignedRejected = await postJson(urls.sessionUrl, ticket);
    assert.equal(unsignedRejected.status, 400);
    const badMacRejected = await postJson(urls.sessionUrl, {
      ...signedTicket,
      mac_hex: "0".repeat(128),
    });
    assert.equal(badMacRejected.status, 400);
    const expiredTicket = createRelaySessionTicket({
      sessionId: "relay-service-expired",
      sessionToken: "token_relay_service_expired_1234567890",
      issuedAtMs: nowMs - 120_000,
      expiresAtMs: nowMs - 60_000,
      daemonPubkeyHex: "a".repeat(64),
      companionDeviceId: "web-service1",
      companionNoisePubkeyHex: "b".repeat(64),
      companionApprovalPubkeyHex: "c".repeat(64),
    });
    const expiredTicketRejected = await postJson(
      urls.sessionUrl,
      ed25519SignedTicket(expiredTicket, relayTicketSigningSeedHex, relayTicketKeyId),
    );
    assert.equal(expiredTicketRejected.status, 400);

    const missingTicket = createRelaySessionTicket({
      sessionId: "relay-service-missing",
      sessionToken: "token_relay_service_missing_1234567890",
      issuedAtMs: nowMs,
      expiresAtMs: nowMs + 60_000,
      daemonPubkeyHex: "a".repeat(64),
      companionDeviceId: "web-service1",
      companionNoisePubkeyHex: "b".repeat(64),
      companionApprovalPubkeyHex: "c".repeat(64),
    });
    const missingTicketError = await rejectedConnect(
      urls.websocketUrl,
      relaySessionConnect(missingTicket, "daemon"),
    );
    assert.match(missingTicketError, /missing/);

    const registered = await postJson(urls.sessionUrl, signedTicket);
    assert.equal(registered.status, 201);
    assert.equal(registered.body.session_id, ticket.session_id);

    const daemonConnect = relaySessionConnect(ticket, "daemon");
    const companionConnect = relaySessionConnect(ticket, "companion");
    const badTokenError = await rejectedConnect(urls.websocketUrl, {
      ...daemonConnect,
      session_token: "wrong_relay_service_token_1234567890",
    });
    assert.match(badTokenError, /token mismatch/);
    const wrongRoleError = await rejectedConnect(urls.websocketUrl, companionConnect, "daemon");
    assert.match(wrongRoleError, /peer mismatch/);

    const daemonWs = await RawWebSocketClient.connect(
      relayWebSocketConnectUrl(urls.websocketUrl, daemonConnect),
    );
    const companionWs = await RawWebSocketClient.connect(
      relayWebSocketConnectUrl(urls.websocketUrl, companionConnect),
    );
    try {
      daemonWs.sendJson(daemonConnect);
      companionWs.sendJson(companionConnect);
      assert.equal((await daemonWs.readJson()).kind, "connected");
      assert.equal((await companionWs.readJson()).kind, "connected");

      const daemonEndpoint = createRelayEndpoint(ticket.session_id, "daemon");
      const companionEndpoint = createRelayEndpoint(ticket.session_id, "companion");
      const ping = "service-smoke-ping";
      const daemonFrame = relayEndpointNextFrame(daemonEndpoint, livePingMessage(ping), Date.now());
      daemonWs.sendJson(daemonFrame);
      const daemonAck = await daemonWs.readJson();
      assert.equal(daemonAck.kind, "queued");
      assert.equal("payload_json" in daemonAck.route, false);

      const companionDelivery = await companionWs.readJson();
      assert.equal(companionDelivery.kind, "frame");
      const companionMessage = relayEndpointAcceptFrame(
        companionEndpoint,
        companionDelivery.frame_json,
        Date.now(),
      );
      assert.deepEqual(companionMessage, livePingMessage(ping));

      const pong = "service-smoke-pong";
      const companionFrame = relayEndpointNextFrame(
        companionEndpoint,
        livePongMessage(pong),
        Date.now(),
      );
      companionWs.sendJson(companionFrame);
      const companionAck = await companionWs.readJson();
      assert.equal(companionAck.kind, "queued");
      assert.equal("payload_json" in companionAck.route, false);

      const daemonDelivery = await daemonWs.readJson();
      assert.equal(daemonDelivery.kind, "frame");
      const daemonMessage = relayEndpointAcceptFrame(daemonEndpoint, daemonDelivery.frame_json, Date.now());
      assert.deepEqual(daemonMessage, livePongMessage(pong));

      const wrongSenderFrame = relayEndpointNextFrame(companionEndpoint, livePingMessage("wrong-sender"), Date.now());
      daemonWs.sendJson(wrongSenderFrame);
      const wrongSenderError = await daemonWs.readJson();
      assert.equal(wrongSenderError.kind, "error");
      assert.match(wrongSenderError.message, /sender mismatch/);

      daemonWs.sendJson(daemonFrame);
      const duplicateSequenceError = await daemonWs.readJson();
      assert.equal(duplicateSequenceError.kind, "error");
      assert.match(duplicateSequenceError.message, /sequence/);

      const expiredFrame = relayEndpointNextFrame(daemonEndpoint, livePingMessage("expired-frame"), Date.now() - 60_000);
      daemonWs.sendJson(expiredFrame);
      const expiredFrameAck = await daemonWs.readJson();
      assert.equal(expiredFrameAck.kind, "queued");

      const finalHealth = await getJson(urls.healthUrl);
      const healthText = JSON.stringify(finalHealth);
      assert.equal(finalHealth.queuedFrames, 0);
      assert.equal(finalHealth.stats.registeredTickets, 1);
      assert.equal(finalHealth.stats.rejectedTickets, 3);
      assert.equal(finalHealth.stats.acceptedConnects, 2);
      assert.equal(finalHealth.stats.rejectedConnects, 3);
      assert.equal(finalHealth.stats.acceptedFrames, 3);
      assert.equal(finalHealth.stats.deliveredFrames, 2);
      assert.equal(finalHealth.stats.rejectedFrames, 2);
      assert.equal(finalHealth.stats.expiredFrames, 1);
      assert.equal(finalHealth.verifierKeys.ed25519, 1);
      assert.equal(finalHealth.verifierKeys.hmacSha256, 0);
      assert.equal(finalHealth.observability.metricScope, "aggregate-only");
      assert.equal(finalHealth.observability.retentionPolicy.payloadJson, "not-retained");
      assert.equal(finalHealth.observability.retentionPolicy.sessionTokens, "not-retained");
      assert.equal(finalHealth.observability.retentionPolicy.setupJson, "not-retained");
      assert.equal(finalHealth.observability.retentionPolicy.privateKeyMaterial, "not-retained");
      assert.ok(finalHealth.observability.errorClasses.includes("bad_ticket"));
      assert.ok(finalHealth.observability.errorClasses.includes("bad_frame"));
      assert.equal(healthText.includes(ping), false);
      assert.equal(healthText.includes(pong), false);
      assert.equal(healthText.includes(relayTicketSigningSeedHex), false);

      const evidence = {
        status: "ok",
        generatedAt: new Date().toISOString(),
        objective:
          "Verify the production-oriented self-hosted relay service artifact registers signed tickets and routes daemon/companion WebSocket frames without exposing payloads or secrets in health evidence",
        urls,
        wssCompatibleDeployShape: "run behind TLS reverse proxy; service preserves /sessions, /health, and /relay contract",
        result: {
          initialHealthStatus: initialHealth.status,
          unsignedTicketRejected: unsignedRejected.status === 400,
          badMacRejected: badMacRejected.status === 400,
          expiredTicketRejected: expiredTicketRejected.status === 400,
          missingTicketConnectRejected: /missing/.test(missingTicketError),
          badTokenConnectRejected: /token mismatch/.test(badTokenError),
          wrongRoleConnectRejected: /peer mismatch/.test(wrongRoleError),
          signedTicketRegistered: registered.status === 201,
          daemonConnected: true,
          companionConnected: true,
          daemonAckRouteHasNoPayload: !("payload_json" in daemonAck.route),
          companionAckRouteHasNoPayload: !("payload_json" in companionAck.route),
          companionMessageType: companionMessage.type,
          daemonMessageType: daemonMessage.type,
          wrongSenderFrameRejected: /sender mismatch/.test(wrongSenderError.message),
          duplicateSequenceRejected: /sequence/.test(duplicateSequenceError.message),
          expiredFrameDropped: finalHealth.stats.expiredFrames === 1,
          finalHealth,
          healthPayloadLeak: healthText.includes(ping) || healthText.includes(pong),
          verifierKeyMode: "ed25519-public-key",
          observabilityMetricScope: finalHealth.observability.metricScope,
          retentionPolicy: finalHealth.observability.retentionPolicy,
          healthPrivateKeyLeak: healthText.includes(relayTicketSigningSeedHex),
        },
      };
      await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
      console.log(`RA_PWA_RELAY_SERVICE_ARTIFACT_OK ${evidencePath}`);
    } finally {
      daemonWs.close();
      companionWs.close();
    }
  } finally {
    await service.close();
  }
}

async function postJson(url, body) {
  const response = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  return {
    status: response.status,
    body: await response.json().catch(() => ({})),
  };
}

async function getJson(url) {
  const response = await fetch(url);
  assert.equal(response.ok, true);
  return response.json();
}

async function rejectedConnect(websocketUrl, connect, urlRole = connect.peer) {
  const url = new URL(websocketUrl);
  url.searchParams.set("session_id", connect.session_id);
  url.searchParams.set("role", urlRole);
  const ws = await RawWebSocketClient.connect(url.toString());
  try {
    ws.sendJson(connect);
    const error = await ws.readJson();
    assert.equal(error.kind, "error");
    return error.message || "";
  } finally {
    ws.close();
  }
}

function ed25519SignedTicket(ticket, privateSeedHex, keyId) {
  const privateKey = ed25519PrivateKeyFromSeedHex(privateSeedHex);
  const signature = sign(
    null,
    Buffer.from(relaySessionTicketSigningPayload(ticket), "utf8"),
    privateKey,
  );
  return {
    ticket,
    mac_alg: "ed25519",
    mac_hex: signature.toString("hex"),
    key_id: keyId,
  };
}

function ed25519PrivateKeyFromSeedHex(privateSeedHex) {
  const pkcs8 = Buffer.concat([
    Buffer.from("302e020100300506032b657004220420", "hex"),
    Buffer.from(privateSeedHex, "hex"),
  ]);
  return createPrivateKey({ key: pkcs8, format: "der", type: "pkcs8" });
}

function ed25519PublicKeyHex(privateSeedHex) {
  const publicKey = createPublicKey(ed25519PrivateKeyFromSeedHex(privateSeedHex));
  const spki = publicKey.export({ format: "der", type: "spki" });
  return Buffer.from(spki).subarray(-32).toString("hex");
}

class RawWebSocketClient {
  static async connect(urlText) {
    const url = new URL(urlText);
    const socket = net.createConnection({
      host: url.hostname,
      port: Number(url.port || 80),
    });
    const client = new RawWebSocketClient(socket);
    await client.connected();
    const key = randomBytes(16).toString("base64");
    const pathAndQuery = `${url.pathname}${url.search}`;
    socket.write(
      [
        `GET ${pathAndQuery} HTTP/1.1`,
        `Host: ${url.host}`,
        "Upgrade: websocket",
        "Connection: Upgrade",
        `Sec-WebSocket-Key: ${key}`,
        "Sec-WebSocket-Version: 13",
        "\r\n",
      ].join("\r\n"),
    );
    const header = await client.readHttpHeader();
    assert.match(header, /^HTTP\/1\.1 101 /);
    return client;
  }

  constructor(socket) {
    this.socket = socket;
    this.buffer = Buffer.alloc(0);
    this.waiters = [];
    socket.on("data", (chunk) => {
      this.buffer = Buffer.concat([this.buffer, chunk]);
      this.drain();
    });
    socket.on("error", (error) => {
      for (const waiter of this.waiters.splice(0)) {
        waiter.reject(error);
      }
    });
  }

  connected() {
    if (!this.socket.pending) {
      return Promise.resolve();
    }
    return new Promise((resolve, reject) => {
      this.socket.once("connect", resolve);
      this.socket.once("error", reject);
    });
  }

  readHttpHeader() {
    return this.take((buffer) => {
      const index = buffer.indexOf("\r\n\r\n");
      if (index === -1) {
        return null;
      }
      return {
        value: buffer.subarray(0, index + 4).toString("utf8"),
        remaining: buffer.subarray(index + 4),
      };
    });
  }

  readJson() {
    return this.take((buffer) => {
      const parsed = readServerWebSocketFrame(buffer);
      if (!parsed) {
        return null;
      }
      return {
        value: JSON.parse(parsed.payload.toString("utf8")),
        remaining: parsed.remaining,
      };
    });
  }

  sendJson(body) {
    this.socket.write(encodeClientWebSocketFrame(Buffer.from(JSON.stringify(body), "utf8")));
  }

  close() {
    this.socket.end();
  }

  take(parser) {
    const parsed = parser(this.buffer);
    if (parsed) {
      this.buffer = parsed.remaining;
      return Promise.resolve(parsed.value);
    }
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error("websocket smoke timeout"));
      }, 5000);
      this.waiters.push({ parser, resolve, reject, timeout });
    });
  }

  drain() {
    for (let index = 0; index < this.waiters.length; index += 1) {
      const waiter = this.waiters[index];
      const parsed = waiter.parser(this.buffer);
      if (!parsed) {
        continue;
      }
      this.waiters.splice(index, 1);
      clearTimeout(waiter.timeout);
      this.buffer = parsed.remaining;
      waiter.resolve(parsed.value);
      index -= 1;
    }
  }
}

function readServerWebSocketFrame(buffer) {
  if (buffer.length < 2) {
    return null;
  }
  const opcode = buffer[0] & 0x0f;
  let payloadLength = buffer[1] & 0x7f;
  let offset = 2;
  if (opcode === 0x8) {
    throw new Error("websocket closed");
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
    payloadLength = Number(buffer.readBigUInt64BE(offset));
    offset += 8;
  }
  if (buffer.length < offset + payloadLength) {
    return null;
  }
  return {
    opcode,
    payload: buffer.subarray(offset, offset + payloadLength),
    remaining: buffer.subarray(offset + payloadLength),
  };
}

function encodeClientWebSocketFrame(payload) {
  const mask = randomBytes(4);
  const header = [];
  header.push(0x81);
  if (payload.length <= 125) {
    header.push(0x80 | payload.length);
  } else if (payload.length <= 0xffff) {
    header.push(0x80 | 126, (payload.length >> 8) & 0xff, payload.length & 0xff);
  } else {
    throw new Error("websocket smoke payload too large");
  }
  const masked = Buffer.alloc(payload.length);
  for (let index = 0; index < payload.length; index += 1) {
    masked[index] = payload[index] ^ mask[index % 4];
  }
  return Buffer.concat([Buffer.from(header), mask, masked]);
}

await main();
