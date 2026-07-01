import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import {
  approvalResponseJson,
  approvalResponseForRequest,
  approvalSigningBytes,
  commandForApprovalVerify,
  commandForPairing,
  deriveNoiseSharedSecretHex,
  decodeApprovalPayloadFromUrl,
  decodePairPayloadFromUrl,
  generateCompanionIdentity,
  generateCompanionKeyMaterial,
  liveApprovalRequestMessage,
  liveApprovalResponseMessage,
  liveApprovalQueueNext,
  liveApprovalRequestKey,
  liveErrorMessage,
  liveEndpointUrls,
  liveEventSourceUrl,
  liveHelloMessage,
  liveMessageRequest,
  livePingMessage,
  livePongMessage,
  liveTransportJson,
  loadCompanionIdentity,
  postLiveTransportMessage,
  parseLiveTransportMessage,
  parseApprovalInput,
  parsePairingInput,
  saveCompanionIdentity,
  signApprovalBytes,
  validateLiveTransportMessage,
  validateApprovalResponse,
  validateApprovalRequest,
  validatePairingPayload,
  verifyApprovalBytes,
} from "./app.mjs";

const payload = {
  protocol_version: 1,
  pairing_code: "123456",
  daemon_pubkey_hex: "a".repeat(64),
  transport_addr: "unix:///tmp/ai-terminal/device.sock",
  expires_at_ms: 1782804241456,
};

const encoded = encodeURIComponent(JSON.stringify(payload));
assert.equal(
  decodePairPayloadFromUrl(`aiterminal://pair?payload=${encoded}`),
  JSON.stringify(payload),
);
assert.deepEqual(parsePairingInput("", `?payload=${encoded}`), payload);
assert.deepEqual(parsePairingInput(`aiterminal://pair?payload=${encoded}`), payload);
assert.doesNotThrow(() => validatePairingPayload(payload));
assert.throws(() => validatePairingPayload({ ...payload, pairing_code: "12345" }));

assert.equal(
  commandForPairing(payload, {
    deviceId: "phone-1",
    noisePubkeyHex: "b".repeat(64),
    approvalPubkeyHex: "c".repeat(64),
  }),
  `ai remote pair --device-id phone-1 --code 123456 --noise-pubkey-hex ${"b".repeat(
    64,
  )} --approval-pubkey-hex ${"c".repeat(64)}`,
);
assert.equal(
  commandForPairing(payload, {
    deviceId: "phone 1",
    noisePubkeyHex: "b".repeat(64),
    approvalPubkeyHex: "bad",
  }),
  "-",
);

const generated = await generateCompanionIdentity(webcrypto);
assert.match(generated.deviceId, /^web-[0-9a-f]{8}$/);
assert.match(generated.noisePubkeyHex, /^[0-9a-f]{64}$/);
assert.match(generated.approvalPubkeyHex, /^[0-9a-f]{64}$/);
assert.notEqual(
  commandForPairing(payload, generated),
  "-",
);

const memoryStorage = new Map();
const storage = {
  getItem: (key) => memoryStorage.get(key) || null,
  setItem: (key, value) => memoryStorage.set(key, value),
};
saveCompanionIdentity(storage, generated);
assert.deepEqual(loadCompanionIdentity(storage), generated);
storage.setItem("ai-terminal-companion-identity-v1", "{\"deviceId\":\"bad space\"}");
assert.equal(loadCompanionIdentity(storage), null);

const generatedKeys = await generateCompanionKeyMaterial(webcrypto);
assert.equal(generatedKeys.keyMaterial.noise.privateKey.extractable, false);
assert.equal(generatedKeys.keyMaterial.approval.privateKey.extractable, false);
assert.deepEqual(await generateCompanionIdentity(webcrypto).then(Object.keys), [
  "deviceId",
  "noisePubkeyHex",
  "approvalPubkeyHex",
]);

const approvalMessage = new TextEncoder().encode("approve:nonce:context");
const signatureHex = await signApprovalBytes(approvalMessage, generatedKeys.keyMaterial, webcrypto);
assert.match(signatureHex, /^[0-9a-f]{128}$/);
assert.equal(
  await verifyApprovalBytes(approvalMessage, signatureHex, generatedKeys.keyMaterial, webcrypto),
  true,
);
assert.equal(
  await verifyApprovalBytes(new TextEncoder().encode("tampered"), signatureHex, generatedKeys.keyMaterial, webcrypto),
  false,
);

const peerKeys = await generateCompanionKeyMaterial(webcrypto);
const localSecret = await deriveNoiseSharedSecretHex(
  peerKeys.identity.noisePubkeyHex,
  generatedKeys.keyMaterial,
  webcrypto,
);
const peerSecret = await deriveNoiseSharedSecretHex(
  generatedKeys.identity.noisePubkeyHex,
  peerKeys.keyMaterial,
  webcrypto,
);
assert.match(localSecret, /^[0-9a-f]{64}$/);
assert.equal(localSecret, peerSecret);

const approvalRequest = {
  approval_id: [97, 112, 112, 114, 45, 49],
  nonce: Array.from({ length: 32 }, (_, i) => i),
  command_masked: "rm -rf build",
  context_hash: "ctx-A",
  expires_at: 1782804241456,
  device_epoch: 1,
};
const approvalEncoded = encodeURIComponent(JSON.stringify(approvalRequest));
assert.equal(
  decodeApprovalPayloadFromUrl(`aiterminal://approve?approval=${approvalEncoded}`),
  JSON.stringify(approvalRequest),
);
assert.deepEqual(parseApprovalInput("", `?approval=${approvalEncoded}`), approvalRequest);
assert.deepEqual(parseApprovalInput(`aiterminal://approve?approval=${approvalEncoded}`), approvalRequest);
assert.deepEqual(parseApprovalInput(JSON.stringify(approvalRequest)), approvalRequest);
assert.doesNotThrow(() => validateApprovalRequest(approvalRequest));
assert.throws(() => validateApprovalRequest({ ...approvalRequest, nonce: [1, 2, 3] }));
assert.deepEqual(Array.from(approvalSigningBytes(approvalRequest, true)), [
  ...approvalRequest.approval_id,
  ...approvalRequest.nonce,
  1,
]);
assert.deepEqual(Array.from(approvalSigningBytes(approvalRequest, false)).slice(-1), [0]);

const signedApprove = await approvalResponseForRequest(
  approvalRequest,
  true,
  generatedKeys.keyMaterial,
  webcrypto,
);
assert.deepEqual(signedApprove.approval_id, approvalRequest.approval_id);
assert.deepEqual(signedApprove.nonce, approvalRequest.nonce);
assert.equal(signedApprove.approve, true);
assert.equal(signedApprove.sig.length, 64);
assert.equal(JSON.parse(approvalResponseJson(signedApprove)).approve, true);
assert.doesNotThrow(() => validateApprovalResponse(signedApprove));
const verifyCommand = commandForApprovalVerify(approvalRequest, signedApprove, generatedKeys.identity.deviceId);
assert.match(verifyCommand, /^ai remote approval-verify --device-id web-[0-9a-f]{8} /);
assert.match(verifyCommand, /--request-json '/);
assert.match(verifyCommand, /--response-json '/);
assert.equal(
  await verifyApprovalBytes(
    approvalSigningBytes(approvalRequest, true),
    bytesToHexForTest(signedApprove.sig),
    generatedKeys.keyMaterial,
    webcrypto,
  ),
  true,
);
assert.equal(
  await verifyApprovalBytes(
    approvalSigningBytes(approvalRequest, false),
    bytesToHexForTest(signedApprove.sig),
    generatedKeys.keyMaterial,
    webcrypto,
  ),
  false,
);

const liveHello = liveHelloMessage(generatedKeys.identity);
assert.deepEqual(liveHello, {
  type: "hello",
  protocol_version: 1,
  device_id: generatedKeys.identity.deviceId,
  noise_pubkey_hex: generatedKeys.identity.noisePubkeyHex,
  approval_pubkey_hex: generatedKeys.identity.approvalPubkeyHex,
});
assert.deepEqual(parseLiveTransportMessage(liveTransportJson(liveHello)), liveHello);
assert.throws(() => liveHelloMessage({ ...generatedKeys.identity, deviceId: "bad id" }));
assert.throws(() =>
  parseLiveTransportMessage(JSON.stringify({ ...liveHello, protocol_version: 2 })),
);

const liveRequest = liveApprovalRequestMessage(approvalRequest);
assert.deepEqual(parseLiveTransportMessage(liveTransportJson(liveRequest)), liveRequest);
const liveResponse = liveApprovalResponseMessage(signedApprove);
assert.deepEqual(parseLiveTransportMessage(liveTransportJson(liveResponse)), liveResponse);
assert.deepEqual(parseLiveTransportMessage(liveTransportJson(livePingMessage("p1"))), {
  type: "ping",
  nonce: "p1",
});
assert.deepEqual(parseLiveTransportMessage(liveTransportJson(livePongMessage("p1"))), {
  type: "pong",
  nonce: "p1",
});
assert.deepEqual(parseLiveTransportMessage(liveTransportJson(liveErrorMessage("boom"))), {
  type: "error",
  message: "boom",
});
assert.deepEqual(liveEndpointUrls("http://127.0.0.1:49152/live"), {
  baseUrl: "http://127.0.0.1:49152",
  healthUrl: "http://127.0.0.1:49152/health",
  eventsUrl: "http://127.0.0.1:49152/events",
  messageUrl: "http://127.0.0.1:49152/message",
});
assert.equal(liveEventSourceUrl("http://127.0.0.1:49152/live"), "http://127.0.0.1:49152/events");
assert.equal(
  liveApprovalRequestKey(approvalRequest),
  `${approvalRequest.approval_id.join(".")}:${approvalRequest.nonce.join(".")}`,
);
const queuedApproval = liveApprovalQueueNext([], liveRequest);
assert.equal(queuedApproval.length, 1);
assert.deepEqual(queuedApproval[0].request, approvalRequest);
assert.equal(liveApprovalQueueNext(queuedApproval, livePingMessage("p2")).length, 1);
assert.equal(liveApprovalQueueNext(queuedApproval, liveRequest).length, 1);
assert.deepEqual(liveMessageRequest(livePingMessage("p2")), {
  method: "POST",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ type: "ping", nonce: "p2" }),
});
assert.deepEqual(
  await postLiveTransportMessage(
    "http://127.0.0.1:49152",
    livePingMessage("p3"),
    async (url, request) => {
      assert.equal(url, "http://127.0.0.1:49152/message");
      assert.equal(request.method, "POST");
      return {
        ok: true,
        status: 200,
        text: async () => JSON.stringify({ type: "pong", nonce: "p3" }),
      };
    },
  ),
  { type: "pong", nonce: "p3" },
);
await assert.rejects(
  () =>
    postLiveTransportMessage("http://127.0.0.1:49152", livePingMessage("p4"), async () => ({
      ok: false,
      status: 400,
      text: async () => JSON.stringify({ type: "error", message: "bad request" }),
    })),
  /bad request/,
);
assert.throws(() => validateLiveTransportMessage({ type: "ping", nonce: "" }));
assert.throws(() => parseLiveTransportMessage("{"));
assert.throws(() => validateLiveTransportMessage({ type: "unknown" }));

console.log("PWA_COMPANION_TEST_OK");

function bytesToHexForTest(bytes) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
