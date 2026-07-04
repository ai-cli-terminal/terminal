import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import {
  approvalResponseJson,
  approvalResponseForRequest,
  approvalSigningBytes,
  commandForApprovalVerify,
  commandForPairing,
  createRelaySessionTicket,
  createSignedRelaySessionTicket,
  relayDeploymentShapeDecision,
  relayManagedAbuseRetentionPolicy,
  relayManagedBillingQuotaPolicy,
  relayManagedControlPlaneContract,
  relayManagedOperationsPlan,
  relayManagedPayloadConfidentialityPlan,
  relayManagedRuntimeReadinessGate,
  relayManagedVerifierKeyOperationsPolicy,
  relayPrivateNetworkSetupContract,
  relayPrivateNetworkSetupPreflight,
  decodeRelaySetupPayloadFromUrl,
  deriveNoiseSharedSecretHex,
  decodeApprovalPayloadFromUrl,
  decodePairPayloadFromUrl,
  createRelayEndpoint,
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
  liveMonitorInitialState,
  liveMonitorNext,
  livePingMessage,
  livePongMessage,
  liveTransportJson,
  loadCompanionIdentity,
  postLiveTransportMessage,
  parseLiveTransportMessage,
  parseRelayFrame,
  parseApprovalInput,
  parsePairingInput,
  parseRelayPrivateNetworkRuntimeSetupInput,
  parseRelayRuntimeSetupInput,
  relayEndpointExchange,
  relayCompanionEndpointLoopFromSetup,
  relayEndpointLoopAcceptSocketMessage,
  relayEndpointLoopConnectJson,
  relayEndpointLoopInitialState,
  relayEndpointLoopNextFrame,
  relayPrivateNetworkCompanionEndpointLoopFromSetup,
  relayPrivateNetworkRuntimeSetupPreflight,
  relayRuntimeSetupPreflight,
  relaySessionConnect,
  relaySessionConnectJson,
  relaySessionExpiredAt,
  relaySessionTicketHmacSha256Hex,
  relaySessionTicketSigningPayload,
  relayTransportUxPreflight,
  relayWebSocketConnectUrl,
  relayFrameFromLiveMessage,
  relayFrameJson,
  relayFramePayloadMessage,
  relayFrameRouteEnvelope,
  relayFrameWithDefaultExpiry,
  relayEndpointAcceptFrame,
  relayEndpointNextFrame,
  validateRelaySessionConnect,
  validateRelaySessionTicket,
  validateSignedRelaySessionConnect,
  validateSignedRelaySessionTicket,
  validateSignedRelaySessionTicketMetadata,
  saveCompanionIdentity,
  signApprovalBytes,
  validateLiveTransportMessage,
  validateRelayEndpoint,
  validateRelayFrame,
  validateRelayPrivateNetworkRuntimeSetupMetadata,
  validateRelayRuntimeSetupMetadata,
  validRelayDeviceId,
  validRelaySender,
  validRelaySessionId,
  validRelaySessionToken,
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
const monitor0 = liveMonitorInitialState(1000);
assert.equal(monitor0.state, "Disconnected");
const monitor1 = liveMonitorNext(
  monitor0,
  { type: "connected", endpoint: "http://127.0.0.1:1", deviceId: "web-1", label: "hello" },
  1100,
);
assert.equal(monitor1.state, "Connected");
assert.equal(monitor1.endpoint, "http://127.0.0.1:1");
assert.equal(monitor1.deviceId, "web-1");
const monitor2 = liveMonitorNext(
  monitor1,
  { type: "approval_request", pendingCount: 1, label: "approval_request rm -rf build" },
  1200,
);
assert.equal(monitor2.pendingCount, 1);
assert.equal(monitor2.receivedCount, 1);
const monitor3 = liveMonitorNext(
  monitor2,
  { type: "approval_response", approve: false, pendingCount: 0, label: "approval_response reject" },
  1300,
);
assert.equal(monitor3.pendingCount, 0);
assert.equal(monitor3.sentCount, 1);
assert.equal(monitor3.rejectedCount, 1);
assert.equal(monitor3.lastResponseAtMs, 1300);
const monitor4 = liveMonitorNext(monitor3, { type: "ping", label: "ping p2" }, 1400);
assert.equal(monitor4.lastHeartbeatAtMs, 1400);
assert.equal(monitor4.history.length, 4);
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

assert.equal(validRelaySessionId("relay-session_1:daemon.web"), true);
assert.equal(validRelaySessionId("relay session"), false);
assert.equal(validRelaySessionId(""), false);
assert.equal(validRelaySender("daemon"), true);
assert.equal(validRelaySender("relay"), false);
assert.equal(validRelaySessionToken("token_1234567890abcdef1234567890abcdef"), true);
assert.equal(validRelaySessionToken("short"), false);
assert.equal(validRelayDeviceId("web-1234abcd"), true);
assert.equal(validRelayDeviceId("bad id"), false);
const relaySessionTicket = createRelaySessionTicket({
  sessionId: "relay-ws-session-1",
  sessionToken: "token_1234567890abcdef1234567890abcdef",
  issuedAtMs: 11000,
  daemonPubkeyHex: "a".repeat(64),
  companionDeviceId: generatedKeys.identity.deviceId,
  companionNoisePubkeyHex: generatedKeys.identity.noisePubkeyHex,
  companionApprovalPubkeyHex: generatedKeys.identity.approvalPubkeyHex,
});
assert.doesNotThrow(() => validateRelaySessionTicket(relaySessionTicket));
const relayDaemonConnect = relaySessionConnect(relaySessionTicket, "daemon");
const relayCompanionConnect = relaySessionConnect(relaySessionTicket, "companion");
assert.doesNotThrow(() => validateRelaySessionConnect(relaySessionTicket, relayDaemonConnect, 12000));
assert.doesNotThrow(() => validateRelaySessionConnect(relaySessionTicket, relayCompanionConnect, 12000));
assert.deepEqual(JSON.parse(relaySessionConnectJson(relayCompanionConnect)), relayCompanionConnect);
assert.equal(relaySessionExpiredAt(relaySessionTicket, 12000), false);
assert.equal(relaySessionExpiredAt(relaySessionTicket, relaySessionTicket.expires_at_ms), true);
assert.throws(() =>
  createRelaySessionTicket({
    ...relaySessionTicket,
    sessionId: "bad session",
    sessionToken: relaySessionTicket.session_token,
    issuedAtMs: relaySessionTicket.issued_at_ms,
    expiresAtMs: relaySessionTicket.expires_at_ms,
    daemonPubkeyHex: relaySessionTicket.daemon_pubkey_hex,
    companionDeviceId: relaySessionTicket.companion_device_id,
    companionNoisePubkeyHex: relaySessionTicket.companion_noise_pubkey_hex,
    companionApprovalPubkeyHex: relaySessionTicket.companion_approval_pubkey_hex,
  }),
);
assert.throws(() =>
  validateRelaySessionConnect(
    relaySessionTicket,
    { ...relayCompanionConnect, session_token: "wrong_1234567890abcdef1234567890abcdef" },
    12000,
  ),
);
assert.throws(() =>
  validateRelaySessionConnect(
    relaySessionTicket,
    { ...relayCompanionConnect, device_id: "web-other" },
    12000,
  ),
);
assert.throws(() =>
  validateRelaySessionConnect(
    relaySessionTicket,
    { ...relayDaemonConnect, daemon_pubkey_hex: "d".repeat(64) },
    12000,
  ),
);
assert.throws(() =>
  validateRelaySessionConnect(relaySessionTicket, relayCompanionConnect, relaySessionTicket.expires_at_ms),
);
const fixedRelaySessionTicket = createRelaySessionTicket({
  sessionId: "relay-ws-session-1",
  sessionToken: "token_1234567890abcdef1234567890abcdef",
  issuedAtMs: 1000,
  expiresAtMs: 2000,
  daemonPubkeyHex: "a".repeat(64),
  companionDeviceId: "web-1234abcd",
  companionNoisePubkeyHex: "b".repeat(64),
  companionApprovalPubkeyHex: "c".repeat(64),
});
assert.equal(
  relaySessionTicketSigningPayload(fixedRelaySessionTicket),
  [
    "ai-terminal-relay-ticket-v1",
    "relay_protocol_version=1",
    "transport=websocket",
    "session_id=relay-ws-session-1",
    "session_token=token_1234567890abcdef1234567890abcdef",
    "issued_at_ms=1000",
    "expires_at_ms=2000",
    `daemon_pubkey_hex=${"a".repeat(64)}`,
    "companion_device_id=web-1234abcd",
    `companion_noise_pubkey_hex=${"b".repeat(64)}`,
    `companion_approval_pubkey_hex=${"c".repeat(64)}`,
    "",
  ].join("\n"),
);
const relayTicketSecret = "relay-ticket-secret-1234567890abcdef";
const signedRelaySessionTicket = await createSignedRelaySessionTicket(
  fixedRelaySessionTicket,
  relayTicketSecret,
  webcrypto,
);
assert.equal(signedRelaySessionTicket.mac_alg, "hmac-sha256");
assert.match(signedRelaySessionTicket.mac_hex, /^[0-9a-f]{64}$/);
assert.equal(
  signedRelaySessionTicket.mac_hex,
  await relaySessionTicketHmacSha256Hex(fixedRelaySessionTicket, relayTicketSecret, webcrypto),
);
assert.doesNotThrow(() => validateSignedRelaySessionTicketMetadata(signedRelaySessionTicket));
assert.doesNotThrow(() =>
  validateSignedRelaySessionTicketMetadata({
    ...signedRelaySessionTicket,
    key_id: "relay-active-1",
  }),
);
assert.doesNotThrow(() =>
  validateSignedRelaySessionTicketMetadata({
    ticket: fixedRelaySessionTicket,
    mac_alg: "ed25519",
    mac_hex: "a".repeat(128),
    key_id: "relay-ed25519-1",
  }),
);
assert.throws(() =>
  validateSignedRelaySessionTicketMetadata({
    ticket: fixedRelaySessionTicket,
    mac_alg: "ed25519",
    mac_hex: "a".repeat(64),
    key_id: "relay-ed25519-1",
  }),
);
assert.deepEqual(
  await validateSignedRelaySessionTicket(signedRelaySessionTicket, relayTicketSecret, webcrypto),
  fixedRelaySessionTicket,
);
await validateSignedRelaySessionConnect(
  signedRelaySessionTicket,
  relaySessionConnect(fixedRelaySessionTicket, "companion"),
  1500,
  relayTicketSecret,
  webcrypto,
);
await assert.rejects(
  () => createSignedRelaySessionTicket(fixedRelaySessionTicket, "short", webcrypto),
  /too short/,
);
await assert.rejects(
  () =>
    validateSignedRelaySessionTicket(
      { ...signedRelaySessionTicket, mac_hex: "0".repeat(64) },
      relayTicketSecret,
      webcrypto,
    ),
  /mac mismatch/,
);
await assert.rejects(
  () =>
    validateSignedRelaySessionTicket(
      {
        ...signedRelaySessionTicket,
        ticket: {
          ...signedRelaySessionTicket.ticket,
          session_token: "tampered_1234567890abcdef1234567890abcdef",
        },
      },
      relayTicketSecret,
      webcrypto,
    ),
  /mac mismatch/,
);
assert.throws(() =>
  validateSignedRelaySessionTicketMetadata({
    ...signedRelaySessionTicket,
    mac_alg: "none",
  }),
);
assert.throws(() =>
  validateSignedRelaySessionTicketMetadata({
    ...signedRelaySessionTicket,
    key_id: "bad key id",
  }),
);
await assert.rejects(
  () =>
    validateSignedRelaySessionConnect(
      signedRelaySessionTicket,
      {
        ...relaySessionConnect(fixedRelaySessionTicket, "companion"),
        session_token: "wrong_1234567890abcdef1234567890abcdef",
      },
      1500,
      relayTicketSecret,
      webcrypto,
    ),
  /session_token mismatch/,
);
const relayUxTicket = createRelaySessionTicket({
  sessionId: "relay-ux-session-1",
  sessionToken: "token_relay_ux_1234567890abcdef1234567890",
  issuedAtMs: 1000,
  expiresAtMs: 2000,
  daemonPubkeyHex: "a".repeat(64),
  companionDeviceId: generatedKeys.identity.deviceId,
  companionNoisePubkeyHex: generatedKeys.identity.noisePubkeyHex,
  companionApprovalPubkeyHex: generatedKeys.identity.approvalPubkeyHex,
});
const signedRelayUxTicket = await createSignedRelaySessionTicket(
  relayUxTicket,
  relayTicketSecret,
  webcrypto,
);
const relayDeploymentDecision = relayDeploymentShapeDecision();
assert.equal(relayDeploymentDecision.selectedMode, "self-hosted");
assert.equal(relayDeploymentDecision.selectedSubstrate, "websocket");
assert.equal(relayDeploymentDecision.productDefault, "live-loopback");
assert.equal(relayDeploymentDecision.ticketVerifierMode, "ed25519-public-verifier-preferred");
assert.equal(relayDeploymentDecision.payloadConfidentiality, "explicit-self-hosted-operator-trust-decision");
assert.deepEqual(relayDeploymentDecision.deferredModes, ["private-network", "managed"]);
assert.ok(relayDeploymentDecision.guardrails.includes("relay_ui_requires_selected_self_hosted_mode"));
assert.ok(relayDeploymentDecision.guardrails.includes("hosted_relay_prefers_public_verifier_keys"));
assert.ok(relayDeploymentDecision.guardrails.includes("self_hosted_relay_operator_trust_required"));
const privateNetworkContract = relayPrivateNetworkSetupContract();
assert.equal(privateNetworkContract.deploymentMode, "private-network");
assert.equal(privateNetworkContract.productDefault, "live-loopback");
assert.equal(privateNetworkContract.selectedRuntime, "deferred");
assert.ok(privateNetworkContract.guardrails.includes("public_ws_blocked"));
const managedOperationsPlan = relayManagedOperationsPlan();
assert.equal(managedOperationsPlan.deploymentMode, "managed");
assert.equal(managedOperationsPlan.readiness, "planning");
assert.equal(managedOperationsPlan.productDefault, "live-loopback");
assert.equal(managedOperationsPlan.selectedRuntime, "deferred");
assert.equal(
  managedOperationsPlan.implementationStatus,
  "operations-contract-ready-runtime-still-deferred",
);
assert.ok(managedOperationsPlan.requiredBeforeImplementation.includes("control-plane-ownership"));
assert.ok(managedOperationsPlan.requiredBeforeImplementation.includes("tenant-isolation"));
assert.ok(managedOperationsPlan.requiredBeforeImplementation.includes("payload-confidentiality-plan"));
assert.ok(managedOperationsPlan.guardrails.includes("managed_relay_remains_deferred"));
assert.ok(managedOperationsPlan.operationAreas.includes("support-and-incident-response"));
assert.ok(managedOperationsPlan.completedOperationContracts.includes("abuse-handling"));
assert.ok(managedOperationsPlan.completedOperationContracts.includes("payload-confidentiality-plan"));
assert.ok(managedOperationsPlan.completedOperationContracts.includes("public-verifier-key-operations"));
assert.ok(managedOperationsPlan.completedOperationContracts.includes("billing-and-quota-policy"));
assert.deepEqual(managedOperationsPlan.remainingOperationContracts, []);
assert.deepEqual(managedOperationsPlan.blockers, []);
assert.equal(managedOperationsPlan.nextLocalSlice, "managed-relay-payload-blind-frame-encryption-spike");
const managedControlPlaneContract = relayManagedControlPlaneContract();
assert.equal(managedControlPlaneContract.deploymentMode, "managed");
assert.equal(managedControlPlaneContract.readiness, "contract");
assert.equal(managedControlPlaneContract.productDefault, "live-loopback");
assert.equal(managedControlPlaneContract.selectedRuntime, "deferred");
assert.equal(managedControlPlaneContract.tenantBoundary, "tenant-isolated-sessions-and-verifier-keys");
assert.ok(managedControlPlaneContract.requiredRoles.includes("service-operator"));
assert.ok(managedControlPlaneContract.requiredContracts.includes("session-registration"));
assert.ok(managedControlPlaneContract.prohibitedControlPlaneData.includes("payload_json"));
assert.ok(managedControlPlaneContract.guardrails.includes("operator_state_excludes_payload_json"));
assert.ok(managedControlPlaneContract.responsibilities.daemonOwner.includes("issue-session-tickets"));
assert.ok(managedControlPlaneContract.blockers.includes("support_audit_boundary_missing"));
assert.ok(managedControlPlaneContract.completedFollowupContracts.includes("billing-and-quota-policy"));
assert.equal(
  managedControlPlaneContract.nextLocalSlice,
  "managed-relay-payload-blind-frame-encryption-spike",
);
const managedAbuseRetentionPolicy = relayManagedAbuseRetentionPolicy();
assert.equal(managedAbuseRetentionPolicy.deploymentMode, "managed");
assert.equal(managedAbuseRetentionPolicy.readiness, "policy");
assert.equal(managedAbuseRetentionPolicy.productDefault, "live-loopback");
assert.equal(managedAbuseRetentionPolicy.selectedRuntime, "deferred");
assert.equal(
  managedAbuseRetentionPolicy.abuseHandling,
  "tenant-scoped-rate-limits-and-operator-escalation",
);
assert.ok(managedAbuseRetentionPolicy.rateLimitScopes.includes("tenant"));
assert.ok(managedAbuseRetentionPolicy.abuseSignals.includes("invalid-ticket-rate"));
assert.equal(managedAbuseRetentionPolicy.retentionWindows.payloadJson, "not-retained");
assert.equal(managedAbuseRetentionPolicy.retentionWindows.controlPlaneAuditDays, 90);
assert.ok(
  managedAbuseRetentionPolicy.deletionRequirements.includes(
    "tenant-deletion-removes-session-metadata",
  ),
);
assert.ok(
  managedAbuseRetentionPolicy.supportWorkflowConstraints.includes(
    "no-payload-json-or-secret-material",
  ),
);
assert.ok(managedAbuseRetentionPolicy.guardrails.includes("no_payload_or_secret_retention"));
assert.ok(managedAbuseRetentionPolicy.requiredBeforeRuntime.includes("payload-confidentiality-plan"));
assert.ok(managedAbuseRetentionPolicy.completedFollowupContracts.includes("payload-confidentiality-plan"));
assert.ok(managedAbuseRetentionPolicy.blockers.includes("support_access_review_missing"));
assert.equal(
  managedAbuseRetentionPolicy.nextLocalSlice,
  "managed-relay-payload-blind-frame-encryption-spike",
);
const managedPayloadConfidentialityPlan = relayManagedPayloadConfidentialityPlan();
assert.equal(managedPayloadConfidentialityPlan.deploymentMode, "managed");
assert.equal(managedPayloadConfidentialityPlan.readiness, "plan");
assert.equal(managedPayloadConfidentialityPlan.productDefault, "live-loopback");
assert.equal(managedPayloadConfidentialityPlan.selectedRuntime, "deferred");
assert.equal(
  managedPayloadConfidentialityPlan.payloadConfidentiality,
  "required-payload-blind-managed-relay",
);
assert.equal(
  managedPayloadConfidentialityPlan.operatorTrustBoundary,
  "relay-operator-cannot-read-payload-json-or-approval-content",
);
assert.ok(managedPayloadConfidentialityPlan.prohibitedManagedRelayData.includes("command_text"));
assert.ok(managedPayloadConfidentialityPlan.allowedRelayMetadata.includes("frame_sequence"));
assert.ok(
  managedPayloadConfidentialityPlan.requiredBeforeRuntime.includes(
    "frame-payload-e2e-encryption",
  ),
);
assert.ok(
  managedPayloadConfidentialityPlan.guardrails.includes(
    "operator_trust_not_sufficient_for_managed_relay",
  ),
);
assert.equal(
  managedPayloadConfidentialityPlan.designDecisions.managedRelay,
  "payload-blind-end-to-end-confidentiality-required",
);
assert.ok(
  managedPayloadConfidentialityPlan.confidentialityRequirements.includes(
    "relay-service-routes-opaque-ciphertext-only",
  ),
);
assert.ok(
  managedPayloadConfidentialityPlan.completedFollowupContracts.includes(
    "public-verifier-key-operations",
  ),
);
assert.ok(
  managedPayloadConfidentialityPlan.completedFollowupContracts.includes(
    "billing-and-quota-policy",
  ),
);
assert.equal(
  managedPayloadConfidentialityPlan.nextLocalSlice,
  "managed-relay-payload-blind-frame-encryption-spike",
);
const managedVerifierKeyOperationsPolicy = relayManagedVerifierKeyOperationsPolicy();
assert.equal(managedVerifierKeyOperationsPolicy.deploymentMode, "managed");
assert.equal(managedVerifierKeyOperationsPolicy.readiness, "policy");
assert.equal(managedVerifierKeyOperationsPolicy.productDefault, "live-loopback");
assert.equal(managedVerifierKeyOperationsPolicy.selectedRuntime, "deferred");
assert.equal(
  managedVerifierKeyOperationsPolicy.verifierKeyDistribution,
  "managed-relay-public-verifier-keys-only",
);
assert.equal(
  managedVerifierKeyOperationsPolicy.keyMaterialBoundary,
  "private-signing-keys-never-enter-managed-relay",
);
assert.ok(managedVerifierKeyOperationsPolicy.requiredKeyStates.includes("active"));
assert.ok(
  managedVerifierKeyOperationsPolicy.requiredKeyOperations.includes(
    "rotate-with-overlap-window",
  ),
);
assert.ok(
  managedVerifierKeyOperationsPolicy.prohibitedVerifierKeyData.includes(
    "private_signing_key",
  ),
);
assert.ok(
  managedVerifierKeyOperationsPolicy.guardrails.includes(
    "public_verifier_keys_only_in_relay_service",
  ),
);
assert.equal(managedVerifierKeyOperationsPolicy.keyRotationRequirements.overlapWindowHours, 24);
assert.equal(
  managedVerifierKeyOperationsPolicy.keyRotationRequirements.revokedKeyRegistration,
  "fail-closed",
);
assert.ok(
  managedVerifierKeyOperationsPolicy.distributionRequirements.includes(
    "propagate-revocation-before-runtime",
  ),
);
assert.equal(
  managedVerifierKeyOperationsPolicy.trustBoundaries.managedRelayService,
  "verifies-public-keys-only",
);
assert.ok(
  managedVerifierKeyOperationsPolicy.completedFollowupContracts.includes(
    "billing-and-quota-policy",
  ),
);
assert.equal(
  managedVerifierKeyOperationsPolicy.nextLocalSlice,
  "managed-relay-payload-blind-frame-encryption-spike",
);
const managedBillingQuotaPolicy = relayManagedBillingQuotaPolicy();
assert.equal(managedBillingQuotaPolicy.deploymentMode, "managed");
assert.equal(managedBillingQuotaPolicy.readiness, "policy");
assert.equal(managedBillingQuotaPolicy.productDefault, "live-loopback");
assert.equal(managedBillingQuotaPolicy.selectedRuntime, "deferred");
assert.equal(
  managedBillingQuotaPolicy.billingModel,
  "tenant-scoped-metered-usage-before-managed-runtime",
);
assert.equal(
  managedBillingQuotaPolicy.quotaEnforcement,
  "tenant-and-session-quota-fail-closed-before-runtime",
);
assert.equal(
  managedBillingQuotaPolicy.usageVisibility,
  "tenant-aggregate-usage-no-payload-or-secret-data",
);
assert.ok(managedBillingQuotaPolicy.requiredQuotaScopes.includes("tenant"));
assert.ok(managedBillingQuotaPolicy.requiredQuotaScopes.includes("verifier-key"));
assert.ok(
  managedBillingQuotaPolicy.meteredUsageDimensions.includes(
    "relay-byte-count",
  ),
);
assert.ok(managedBillingQuotaPolicy.prohibitedBillingData.includes("payload_json"));
assert.ok(
  managedBillingQuotaPolicy.guardrails.includes(
    "billing_records_exclude_payloads_and_secrets",
  ),
);
assert.equal(managedBillingQuotaPolicy.quotaDefaults.activeSessionsPerTenant, 100);
assert.equal(
  managedBillingQuotaPolicy.quotaDefaults.quotaExceededBehavior,
  "reject-new-session-or-frame",
);
assert.ok(
  managedBillingQuotaPolicy.enforcementRequirements.includes(
    "separate-abuse-rate-limits-from-billing-meters",
  ),
);
assert.ok(
  managedBillingQuotaPolicy.retentionRequirements.includes(
    "payload-and-secret-data-not-retained",
  ),
);
assert.equal(
  managedBillingQuotaPolicy.trustBoundaries.managedRelayService,
  "meters-routing-events-and-quota-denials-only",
);
assert.ok(
  managedBillingQuotaPolicy.implementationBlockers.includes(
    "quota_enforcement_smoke_missing",
  ),
);
assert.equal(
  managedBillingQuotaPolicy.nextLocalSlice,
  "managed-relay-payload-blind-frame-encryption-spike",
);
const managedRuntimeReadinessGate = relayManagedRuntimeReadinessGate();
assert.equal(managedRuntimeReadinessGate.deploymentMode, "managed");
assert.equal(managedRuntimeReadinessGate.readiness, "gate");
assert.equal(managedRuntimeReadinessGate.productDefault, "live-loopback");
assert.equal(managedRuntimeReadinessGate.selectedRuntime, "deferred");
assert.equal(managedRuntimeReadinessGate.gateStatus, "blocked-until-runtime-evidence");
assert.equal(
  managedRuntimeReadinessGate.implementationDecision,
  "managed-runtime-implementation-not-started",
);
assert.equal(managedRuntimeReadinessGate.implementationCanStart, false);
assert.ok(
  managedRuntimeReadinessGate.guardrails.includes(
    "no_managed_runtime_until_readiness_gate_green",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.completedPlanningInputs.includes(
    "billing-and-quota-policy",
  ),
);
assert.deepEqual(managedRuntimeReadinessGate.missingPlanningInputs, []);
assert.ok(
  managedRuntimeReadinessGate.requiredRuntimeEvidence.includes(
    "payload-blind-frame-encryption-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.requiredRuntimeEvidence.includes(
    "public-verifier-key-registry-runtime-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.requiredRuntimeEvidence.includes(
    "tenant-session-registration-quota-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.requiredRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "e2e_payload_encryption_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "managed_key_registry_runtime_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "quota_enforcement_smoke_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "support_access_review_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.payloadConfidentiality.evidence.includes(
    "client-key-agreement-runtime-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.quotaAndUsage.evidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
);
assert.equal(
  managedRuntimeReadinessGate.nextLocalSlice,
  "managed-relay-payload-blind-frame-encryption-spike",
);
const privateNetworkReady = relayPrivateNetworkSetupPreflight(
  {
    transportMode: "relay",
    deploymentMode: "private-network",
    relayEndpointUrl: "wss://relay.tailnet.example/relay",
    privateNetworkName: "tailnet-dev",
    signedSessionTicket: signedRelayUxTicket,
    companionIdentity: generatedKeys.identity,
    operatorSetupText: "Private-network relay setup is ready.",
  },
  1500,
);
assert.equal(privateNetworkReady.status, "ready");
assert.equal(privateNetworkReady.relayVisible, false);
assert.equal(privateNetworkReady.contractReady, true);
assert.deepEqual(privateNetworkReady.blockers, []);
const privateNetworkPublicWs = relayPrivateNetworkSetupPreflight(
  {
    transportMode: "relay",
    deploymentMode: "private-network",
    relayEndpointUrl: "ws://relay.example.test/relay",
    privateNetworkName: "tailnet-dev",
    signedSessionTicket: signedRelayUxTicket,
    companionIdentity: generatedKeys.identity,
    operatorSetupText: "Private-network relay setup is ready.",
  },
  1500,
);
assert.equal(privateNetworkPublicWs.status, "hidden");
assert.ok(privateNetworkPublicWs.blockers.includes("relay_endpoint_url_invalid"));
const defaultRelayUxPreflight = relayTransportUxPreflight({}, 1500);
assert.equal(defaultRelayUxPreflight.status, "hidden");
assert.equal(defaultRelayUxPreflight.relayVisible, false);
assert.ok(defaultRelayUxPreflight.blockers.includes("transport_mode_not_relay"));
assert.ok(defaultRelayUxPreflight.blockers.includes("relay_endpoint_url_missing"));
assert.ok(defaultRelayUxPreflight.blockers.includes("relay_signed_ticket_missing"));
assert.ok(defaultRelayUxPreflight.blockers.includes("companion_identity_missing"));

const readyRelayUxPreflight = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "wss://relay.example.test/session",
    signedSessionTicket: signedRelayUxTicket,
    companionIdentity: generatedKeys.identity,
    deploymentMode: "self-hosted",
    operatorSetupText: "Self-hosted relay setup is ready.",
  },
  1500,
);
assert.deepEqual(readyRelayUxPreflight.blockers, []);
assert.equal(readyRelayUxPreflight.status, "ready");
assert.equal(readyRelayUxPreflight.relayVisible, true);
assert.equal(readyRelayUxPreflight.relayEnabled, true);

const localRelayUxPreflight = relayTransportUxPreflight(
  {
    ...readyRelayUxPreflight,
    transportMode: "relay",
    relayEndpointUrl: "ws://127.0.0.1:49152/relay",
    signedSessionTicket: signedRelayUxTicket,
    companionIdentity: generatedKeys.identity,
    deploymentMode: "self-hosted",
    operatorSetupText: "Local relay setup is ready.",
  },
  1500,
);
assert.equal(localRelayUxPreflight.status, "ready");

const relayRuntimeSetup = {
  relayProtocolVersion: 1,
  transportMode: "relay",
  deploymentMode: "self-hosted",
  relayEndpointUrl: "wss://relay.example.test/session",
  signedSessionTicket: { ...signedRelayUxTicket, key_id: "relay-active-1" },
  daemonConnect: relaySessionConnect(relayUxTicket, "daemon"),
  companionConnect: relaySessionConnect(relayUxTicket, "companion"),
  companionIdentity: generatedKeys.identity,
  operatorSetupText: "Self-hosted relay endpoint is ready for this companion.",
};
assert.doesNotThrow(() => validateRelayRuntimeSetupMetadata(relayRuntimeSetup));
assert.throws(
  () => validateRelayRuntimeSetupMetadata({ ...relayRuntimeSetup, privateNetworkName: "tailnet-dev" }),
  /privateNetworkName/,
);
const relayRuntimeSetupJson = JSON.stringify(relayRuntimeSetup);
assert.deepEqual(parseRelayRuntimeSetupInput(relayRuntimeSetupJson), relayRuntimeSetup);
const relaySetupEncoded = encodeURIComponent(relayRuntimeSetupJson);
assert.equal(
  decodeRelaySetupPayloadFromUrl(`aiterminal://relay?relaySetup=${relaySetupEncoded}`),
  relayRuntimeSetupJson,
);
assert.deepEqual(parseRelayRuntimeSetupInput("", `?relaySetup=${relaySetupEncoded}`), relayRuntimeSetup);
const runtimeSetupPreflight = relayRuntimeSetupPreflight(relayRuntimeSetup, 1500);
assert.equal(runtimeSetupPreflight.status, "ready");
assert.deepEqual(runtimeSetupPreflight.blockers, []);
const expiredRuntimeSetupPreflight = relayRuntimeSetupPreflight(relayRuntimeSetup, 2000);
assert.equal(expiredRuntimeSetupPreflight.status, "hidden");
assert.ok(expiredRuntimeSetupPreflight.blockers.includes("relay_signed_ticket_expired"));
const privateNetworkRuntimeSetup = {
  ...relayRuntimeSetup,
  deploymentMode: "private-network",
  privateNetworkName: "tailnet-dev",
  relayEndpointUrl: "wss://relay.tailnet.example/relay",
  operatorSetupText: "Private-network relay tailnet-dev endpoint is ready.",
};
assert.doesNotThrow(() => validateRelayPrivateNetworkRuntimeSetupMetadata(privateNetworkRuntimeSetup));
const privateNetworkRuntimePreflight = relayPrivateNetworkRuntimeSetupPreflight(
  privateNetworkRuntimeSetup,
  1500,
);
assert.equal(privateNetworkRuntimePreflight.status, "ready");
assert.equal(privateNetworkRuntimePreflight.contractReady, true);
assert.deepEqual(privateNetworkRuntimePreflight.blockers, []);
const privateNetworkRuntimeSetupJson = JSON.stringify(privateNetworkRuntimeSetup);
assert.deepEqual(
  parseRelayPrivateNetworkRuntimeSetupInput(privateNetworkRuntimeSetupJson),
  privateNetworkRuntimeSetup,
);
const privateNetworkLoop = relayPrivateNetworkCompanionEndpointLoopFromSetup(
  {
    ...privateNetworkRuntimeSetup,
    relayEndpointUrl: "ws://127.0.0.1:49153/relay",
  },
  30000,
  1500,
);
assert.equal(
  privateNetworkLoop.webSocketUrl,
  "ws://127.0.0.1:49153/relay?session_id=relay-ux-session-1&role=companion",
);
assert.deepEqual(
  JSON.parse(relayEndpointLoopConnectJson(privateNetworkLoop)),
  privateNetworkRuntimeSetup.companionConnect,
);
assert.throws(
  () => parseRelayPrivateNetworkRuntimeSetupInput(relayRuntimeSetupJson),
  /deploymentMode/,
);
assert.throws(
  () => validateRelayRuntimeSetupMetadata(privateNetworkRuntimeSetup),
  /deploymentMode/,
);
assert.throws(
  () =>
    validateRelayPrivateNetworkRuntimeSetupMetadata({
      ...privateNetworkRuntimeSetup,
      privateNetworkName: "bad name",
    }),
  /privateNetworkName/,
);
assert.throws(
  () =>
    relayPrivateNetworkRuntimeSetupPreflight({
      ...privateNetworkRuntimeSetup,
      relayEndpointUrl: "ws://relay.example.test/relay",
    }),
  /endpoint URL/,
);
assert.throws(
  () => parseRelayRuntimeSetupInput(JSON.stringify({ ...relayRuntimeSetup, hmac_sha256_keys: [] })),
  /secret field/,
);
assert.throws(
  () =>
    parseRelayRuntimeSetupInput(
      JSON.stringify({
        ...relayRuntimeSetup,
        companionConnect: {
          ...relayRuntimeSetup.companionConnect,
          session_token: "wrong_1234567890abcdef1234567890abcdef",
        },
      }),
    ),
  /session_token mismatch/,
);

const relayLoopSetup = {
  ...relayRuntimeSetup,
  relayEndpointUrl: "ws://127.0.0.1:49152/relay",
};
const companionLoop = relayCompanionEndpointLoopFromSetup(relayLoopSetup, 30000, 1500);
assert.equal(
  companionLoop.webSocketUrl,
  "ws://127.0.0.1:49152/relay?session_id=relay-ux-session-1&role=companion",
);
assert.deepEqual(JSON.parse(relayEndpointLoopConnectJson(companionLoop)), relayLoopSetup.companionConnect);
assert.equal(
  relayWebSocketConnectUrl(relayLoopSetup.relayEndpointUrl, relayLoopSetup.daemonConnect),
  "ws://127.0.0.1:49152/relay?session_id=relay-ux-session-1&role=daemon",
);
assert.deepEqual(
  relayEndpointLoopAcceptSocketMessage(
    companionLoop,
    JSON.stringify({
      kind: "connected",
      session_id: relayLoopSetup.companionConnect.session_id,
      peer: "companion",
    }),
    1500,
  ).kind,
  "connected",
);
assert.equal(companionLoop.connected, true);

const daemonLoop = relayEndpointLoopInitialState(relayLoopSetup.daemonConnect);
relayEndpointLoopAcceptSocketMessage(
  daemonLoop,
  {
    kind: "connected",
    session_id: relayLoopSetup.daemonConnect.session_id,
    peer: "daemon",
  },
  1500,
);
const relayLoopRequest = relayEndpointLoopNextFrame(
  daemonLoop,
  liveApprovalRequestMessage(approvalRequest),
  1501,
);
assert.equal("payload_json" in relayLoopRequest.route, false);
assert.equal(daemonLoop.sentCount, 1);
const relayLoopCompanionDelivery = relayEndpointLoopAcceptSocketMessage(
  companionLoop,
  {
    kind: "frame",
    route: relayLoopRequest.route,
    frame_json: relayLoopRequest.frameJson,
  },
  1502,
);
assert.equal(relayLoopCompanionDelivery.kind, "live_message");
assert.deepEqual(relayLoopCompanionDelivery.liveMessage, liveApprovalRequestMessage(approvalRequest));
assert.equal(companionLoop.receivedCount, 1);
const relayLoopResponse = relayEndpointLoopNextFrame(
  companionLoop,
  liveApprovalResponseMessage(signedApprove),
  1503,
);
const relayLoopCompanionAck = relayEndpointLoopAcceptSocketMessage(
  companionLoop,
  { kind: "queued", route: relayLoopResponse.route },
  1504,
);
assert.equal(relayLoopCompanionAck.kind, "queued");
assert.equal(companionLoop.sentCount, 1);
assert.equal(companionLoop.queuedCount, 1);
const relayLoopDaemonDelivery = relayEndpointLoopAcceptSocketMessage(
  daemonLoop,
  {
    kind: "frame",
    route: relayLoopResponse.route,
    frame_json: relayLoopResponse.frameJson,
  },
  1505,
);
assert.deepEqual(relayLoopDaemonDelivery.liveMessage, liveApprovalResponseMessage(signedApprove));
assert.throws(() =>
  relayEndpointLoopAcceptSocketMessage(
    companionLoop,
    {
      kind: "connected",
      session_id: relayLoopSetup.companionConnect.session_id,
      peer: "daemon",
    },
    1506,
  ),
);
assert.throws(() => relayWebSocketConnectUrl("https://relay.example.test/session", relayLoopSetup.companionConnect));

const expiredRelayUxPreflight = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "wss://relay.example.test/session",
    signedSessionTicket: signedRelayUxTicket,
    companionIdentity: generatedKeys.identity,
    deploymentMode: "self-hosted",
    operatorSetupText: "Self-hosted relay setup is ready.",
  },
  2000,
);
assert.equal(expiredRelayUxPreflight.relayVisible, false);
assert.ok(expiredRelayUxPreflight.blockers.includes("relay_signed_ticket_expired"));

const deferredRelayUxPreflight = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "wss://relay.example.test/session",
    signedSessionTicket: signedRelayUxTicket,
    companionIdentity: generatedKeys.identity,
    deploymentMode: "managed",
    operatorSetupText: "Managed relay setup is documented but deferred.",
  },
  1500,
);
assert.equal(deferredRelayUxPreflight.status, "hidden");
assert.ok(deferredRelayUxPreflight.blockers.includes("relay_deployment_mode_not_selected"));

const mismatchRelayUxPreflight = relayTransportUxPreflight(
  {
    transportMode: "relay",
    relayEndpointUrl: "https://relay.example.test/session",
    signedSessionTicket: signedRelayUxTicket,
    companionIdentity: { ...generatedKeys.identity, deviceId: "web-other" },
    deploymentMode: "unknown",
    operatorSetupText: "short",
  },
  1500,
);
assert.equal(mismatchRelayUxPreflight.status, "hidden");
assert.ok(mismatchRelayUxPreflight.blockers.includes("relay_endpoint_url_invalid"));
assert.ok(mismatchRelayUxPreflight.blockers.includes("relay_deployment_mode_invalid"));
assert.ok(mismatchRelayUxPreflight.blockers.includes("relay_operator_setup_text_missing"));
assert.ok(mismatchRelayUxPreflight.blockers.includes("relay_ticket_identity_mismatch"));
const relayPingFrame = relayFrameFromLiveMessage(
  "relay-session-1",
  "companion",
  1,
  100,
  200,
  livePingMessage("relay-ping-1"),
);
assert.deepEqual(relayFramePayloadMessage(relayPingFrame), livePingMessage("relay-ping-1"));
assert.deepEqual(parseRelayFrame(relayFrameJson(relayPingFrame)), relayPingFrame);
assert.equal(
  relayFrameWithDefaultExpiry("relay-session-1", "daemon", 2, 1000, livePongMessage("relay-pong-1"))
    .expires_at_ms,
  31000,
);
assert.doesNotThrow(() => validateRelayFrame({ ...relayPingFrame }));
assert.throws(() => validateRelayFrame({ ...relayPingFrame, relay_protocol_version: 2 }));
assert.throws(() => validateRelayFrame({ ...relayPingFrame, session_id: "bad session" }));
assert.throws(() => validateRelayFrame({ ...relayPingFrame, sender: "relay" }));
assert.throws(() => validateRelayFrame({ ...relayPingFrame, sequence: 0 }));
assert.throws(() => validateRelayFrame({ ...relayPingFrame, sent_at_ms: 0 }));
assert.throws(() => validateRelayFrame({ ...relayPingFrame, expires_at_ms: 100 }));
assert.throws(() => validateRelayFrame({ ...relayPingFrame, payload_json: "" }));
const relayOpaqueBadPayload = { ...relayPingFrame, payload_json: JSON.stringify({ type: "ping", nonce: "" }) };
assert.doesNotThrow(() => validateRelayFrame(relayOpaqueBadPayload));
const relayRouteEnvelope = relayFrameRouteEnvelope(relayOpaqueBadPayload);
assert.equal(relayRouteEnvelope.session_id, "relay-session-1");
assert.equal(relayRouteEnvelope.sender, "companion");
assert.equal(relayRouteEnvelope.sequence, 1);
assert.equal(
  relayRouteEnvelope.payload_json_bytes,
  new TextEncoder().encode(relayOpaqueBadPayload.payload_json).length,
);
assert.equal("payload_json" in relayRouteEnvelope, false);
assert.deepEqual(relayFrameRouteEnvelope(relayFrameJson(relayPingFrame)), {
  relay_protocol_version: relayPingFrame.relay_protocol_version,
  session_id: relayPingFrame.session_id,
  sender: relayPingFrame.sender,
  sequence: relayPingFrame.sequence,
  sent_at_ms: relayPingFrame.sent_at_ms,
  expires_at_ms: relayPingFrame.expires_at_ms,
  payload_json_bytes: new TextEncoder().encode(relayPingFrame.payload_json).length,
});
assert.throws(() => relayFramePayloadMessage(relayOpaqueBadPayload));

const relayApprovalFrame = relayFrameFromLiveMessage(
  "approval-session",
  "daemon",
  3,
  2000,
  4000,
  liveApprovalRequestMessage(approvalRequest),
);
assert.deepEqual(relayFramePayloadMessage(relayApprovalFrame), liveApprovalRequestMessage(approvalRequest));

const relayDaemonEndpoint = createRelayEndpoint("relay-session-2", "daemon");
const relayCompanionEndpoint = createRelayEndpoint("relay-session-2", "companion");
assert.doesNotThrow(() => validateRelayEndpoint(relayDaemonEndpoint));
const relayOutbound = relayEndpointNextFrame(relayDaemonEndpoint, livePingMessage("endpoint-ping"), 5000);
assert.equal(relayOutbound.sequence, 1);
assert.equal(relayDaemonEndpoint.nextSequence, 2);
assert.equal(relayOutbound.expires_at_ms, 35000);
assert.deepEqual(
  relayEndpointAcceptFrame(relayCompanionEndpoint, relayFrameJson(relayOutbound), 5001),
  livePingMessage("endpoint-ping"),
);
assert.throws(() => relayEndpointAcceptFrame(relayDaemonEndpoint, relayOutbound, 5001));
assert.throws(() =>
  relayEndpointAcceptFrame(createRelayEndpoint("other-session", "companion"), relayOutbound, 5001),
);
assert.equal(relayEndpointAcceptFrame(relayCompanionEndpoint, relayOutbound, 35000), null);

const relayCompanionResponse = relayEndpointNextFrame(
  relayCompanionEndpoint,
  liveApprovalResponseMessage(signedApprove),
  6000,
);
assert.deepEqual(
  relayEndpointAcceptFrame(relayDaemonEndpoint, relayCompanionResponse, 6001),
  liveApprovalResponseMessage(signedApprove),
);
assert.throws(() => createRelayEndpoint("bad session", "daemon"));
assert.throws(() => createRelayEndpoint("relay-session-3", "relay"));
assert.throws(() => createRelayEndpoint("relay-session-3", "daemon", 0));
assert.throws(() => validateRelayEndpoint({ ...relayDaemonEndpoint, nextSequence: 0 }));

const relayExchangeDaemon = createRelayEndpoint("relay-exchange-1", "daemon");
const relayExchangeCompanion = createRelayEndpoint("relay-exchange-1", "companion");
const relayExchange = relayEndpointExchange(
  relayExchangeDaemon,
  relayExchangeCompanion,
  liveApprovalRequestMessage(approvalRequest),
  liveApprovalResponseMessage(signedApprove),
  7000,
);
assert.equal(relayExchange.daemonFrame.sequence, 1);
assert.equal(relayExchange.daemonFrame.sent_at_ms, 7000);
assert.equal(relayExchange.companionFrame.sequence, 1);
assert.equal(relayExchange.companionFrame.sent_at_ms, 7002);
assert.deepEqual(parseRelayFrame(relayExchange.daemonFrameJson), relayExchange.daemonFrame);
assert.deepEqual(parseRelayFrame(relayExchange.companionFrameJson), relayExchange.companionFrame);
assert.deepEqual(relayExchange.companionMessage, liveApprovalRequestMessage(approvalRequest));
assert.deepEqual(relayExchange.daemonReply, liveApprovalResponseMessage(signedApprove));
assert.equal(relayExchangeDaemon.nextSequence, 2);
assert.equal(relayExchangeCompanion.nextSequence, 2);
const relayWrongSessionDaemon = createRelayEndpoint("relay-exchange-a", "daemon");
const relayWrongSessionCompanion = createRelayEndpoint("relay-exchange-b", "companion");
assert.throws(() =>
  relayEndpointExchange(
    relayWrongSessionDaemon,
    relayWrongSessionCompanion,
    livePingMessage("relay-exchange-wrong-session"),
    livePongMessage("relay-exchange-wrong-session"),
    8000,
  ),
);
assert.equal(relayWrongSessionDaemon.nextSequence, 1);
assert.equal(relayWrongSessionCompanion.nextSequence, 1);
assert.throws(() =>
  relayEndpointExchange(
    createRelayEndpoint("relay-exchange-same-sender", "daemon"),
    createRelayEndpoint("relay-exchange-same-sender", "daemon"),
    livePingMessage("relay-exchange-same-sender"),
    livePongMessage("relay-exchange-same-sender"),
    9000,
  ),
);
assert.throws(() =>
  relayEndpointExchange(
    createRelayEndpoint("relay-exchange-short-ttl", "daemon", 1),
    createRelayEndpoint("relay-exchange-short-ttl", "companion", 1),
    livePingMessage("relay-exchange-short-ttl"),
    livePongMessage("relay-exchange-short-ttl"),
    10000,
  ),
);

console.log("PWA_COMPANION_TEST_OK");

function bytesToHexForTest(bytes) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
