import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import {
  approvalResponseJson,
  approvalResponseForRequest,
  approvalSigningBytes,
  commandForApprovalVerify,
  commandForPairing,
  createEd25519SignedRelaySessionTicket,
  createManagedRelayBillingAbuseBoundaryReview,
  createManagedRelayRuntimeEncryptedFrameRouting,
  createManagedRelayRuntimeOperatorSetupContract,
  createManagedRelayRuntimePwaExposureGate,
  createManagedRelayRuntimeQuotaAndMeteringIntegration,
  createManagedRelayRuntimeSupportAndAbuseOperationsIntegration,
  createManagedRelayRuntimeControlPlaneContractWiring,
  createManagedRelayRuntimeServiceScaffold,
  createManagedRelayPublicVerifierKeyRegistry,
  createManagedRelayPublicVerifierKeyRegistrySnapshot,
  createManagedRelayActiveSessionAndByteQuotaState,
  createManagedRelaySupportRedactionAccessReview,
  createManagedRelayTenantAggregateUsageExport,
  createManagedRelayTenantSessionRegistrationQuotaState,
  createRelaySessionTicket,
  createSignedRelaySessionTicket,
  evaluateManagedRelayActiveSessionAndByteQuota,
  evaluateManagedRelayTenantSessionRegistrationQuota,
  lookupManagedRelayPublicVerifierKeyFromRegistrySnapshot,
  lookupManagedRelayPublicVerifierKey,
  managedRelayEncryptedFrameFromLiveMessage,
  managedRelayEncryptedFrameJson,
  managedRelayEncryptedFramePayloadMessage,
  managedRelayEncryptedFrameRouteEnvelope,
  managedRelayDeriveSessionPayloadKeyHex,
  parseManagedRelayEncryptedFrame,
  routeManagedRelayRuntimeEncryptedFrame,
  routeManagedRelayRuntimeQuotaMeteredFrame,
  relayDeploymentShapeDecision,
  relayManagedAbuseRetentionPolicy,
  relayManagedActiveSessionAndByteQuotaSmoke,
  relayManagedBillingAbuseBoundaryReview,
  relayManagedBillingQuotaPolicy,
  relayManagedClientKeyAgreementRuntimeSmoke,
  relayManagedControlPlaneContract,
  relayManagedMetadataMinimizationReview,
  relayManagedOperationsPlan,
  relayManagedPayloadBlindFrameEncryptionSpike,
  relayManagedPayloadConfidentialityPlan,
  relayManagedPublicVerifierKeyRegistryRuntimeSmoke,
  relayManagedRevocationAndRotationPropagationSmoke,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimeBrowserOperatorEvidence,
  relayManagedRuntimeEncryptedFrameRouting,
  relayManagedRuntimeOperatorSetupBrowserEvidence,
  relayManagedRuntimeOperatorSetupConnectionControls,
  relayManagedRuntimeOperatorSetupApprovalFlowEvidence,
  relayManagedRuntimeOperatorSetupImportPreflight,
  relayManagedRuntimeOperatorSetupRunbookCloseout,
  relayManagedRuntimeOperatorSetupSessionHandshake,
  relayManagedRuntimeOperatorSetupContract,
  relayManagedRuntimePwaExposureGate,
  relayManagedRuntimeQuotaAndMeteringIntegration,
  relayManagedRuntimeSupportAndAbuseOperationsIntegration,
  relayManagedRuntimeControlPlaneContractWiring,
  relayManagedRuntimeServiceScaffold,
  relayManagedRuntimeReadinessGate,
  relayManagedSupportRedactionAndAccessReviewEvidence,
  relayManagedTenantAggregateUsageExportSmoke,
  relayManagedTenantSessionRegistrationQuotaSmoke,
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
  managedRelayRuntimeOperatorSetupConnectionControls,
  managedRelayRuntimeOperatorSetupApprovalFlowEvidence,
  managedRelayRuntimeOperatorSetupApprovalFlowEvidenceFromHandshake,
  managedRelayRuntimeOperatorSetupImportPreflight,
  managedRelayRuntimeOperatorSetupApprovalRequest,
  managedRelayRuntimeOperatorSetupSessionHandshake,
  managedRelayRuntimeOperatorSetupSessionHandshakePayload,
  postLiveTransportMessage,
  parseManagedRelayRuntimeOperatorSetupInput,
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
  relaySessionTicketEd25519SignatureHex,
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
  validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry,
  validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot,
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
  validateManagedRelayRuntimeOperatorSetupMetadata,
  validRelayDeviceId,
  validRelaySender,
  validRelaySessionId,
  validRelaySessionToken,
  validateApprovalResponse,
  validateApprovalRequest,
  validatePairingPayload,
  validateManagedRelayEncryptedFrame,
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
const managedSessionPayloadKey = await managedRelayDeriveSessionPayloadKeyHex(
  "managed-session-alpha",
  peerKeys.identity.noisePubkeyHex,
  generatedKeys.keyMaterial,
  webcrypto,
);
const managedPeerSessionPayloadKey = await managedRelayDeriveSessionPayloadKeyHex(
  "managed-session-alpha",
  generatedKeys.identity.noisePubkeyHex,
  peerKeys.keyMaterial,
  webcrypto,
);
const managedOtherSessionPayloadKey = await managedRelayDeriveSessionPayloadKeyHex(
  "managed-session-beta",
  peerKeys.identity.noisePubkeyHex,
  generatedKeys.keyMaterial,
  webcrypto,
);
assert.match(managedSessionPayloadKey, /^[0-9a-f]{64}$/);
assert.equal(managedSessionPayloadKey, managedPeerSessionPayloadKey);
assert.notEqual(managedSessionPayloadKey, managedOtherSessionPayloadKey);
await assert.rejects(
  () =>
    managedRelayDeriveSessionPayloadKeyHex(
      "managed-session-alpha",
      peerKeys.identity.noisePubkeyHex,
      { noise: { publicKey: generatedKeys.keyMaterial.noise.publicKey } },
      webcrypto,
    ),
  /private key/,
);

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
    key_version: 4,
  }),
);
assert.throws(() =>
  validateSignedRelaySessionTicketMetadata({
    ticket: fixedRelaySessionTicket,
    mac_alg: "ed25519",
    mac_hex: "a".repeat(64),
    key_id: "relay-ed25519-1",
    key_version: 4,
  }),
);
const managedTicketSigningKeys = await generateCompanionKeyMaterial(webcrypto);
const managedEd25519SignedTicket = await createEd25519SignedRelaySessionTicket(
  fixedRelaySessionTicket,
  managedTicketSigningKeys.keyMaterial,
  { keyId: "managed-key-1", keyVersion: 4 },
  webcrypto,
);
assert.equal(managedEd25519SignedTicket.mac_alg, "ed25519");
assert.equal(managedEd25519SignedTicket.key_id, "managed-key-1");
assert.equal(managedEd25519SignedTicket.key_version, 4);
assert.equal(
  managedEd25519SignedTicket.mac_hex,
  await relaySessionTicketEd25519SignatureHex(
    fixedRelaySessionTicket,
    managedTicketSigningKeys.keyMaterial,
    webcrypto,
  ),
);
const managedPublicVerifierRegistry = createManagedRelayPublicVerifierKeyRegistry([
  {
    tenant_id: "tenant-demo",
    key_id: "managed-key-1",
    key_version: 4,
    public_key_alg: "ed25519",
    public_key_hex: managedTicketSigningKeys.identity.approvalPubkeyHex,
    state: "active",
    not_before_ms: 1000,
    expires_at_ms: 2000,
  },
]);
assert.deepEqual(
  lookupManagedRelayPublicVerifierKey(
    managedPublicVerifierRegistry,
    { tenantId: "tenant-demo", keyId: "managed-key-1", keyVersion: 4 },
    1500,
  ),
  managedPublicVerifierRegistry.entries[0],
);
assert.deepEqual(
  await validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
    managedEd25519SignedTicket,
    managedPublicVerifierRegistry,
    { tenantId: "tenant-demo", nowMs: 1500 },
    webcrypto,
  ),
  fixedRelaySessionTicket,
);
assert.throws(() =>
  lookupManagedRelayPublicVerifierKey(
    managedPublicVerifierRegistry,
    { tenantId: "tenant-demo", keyId: "missing-key", keyVersion: 4 },
    1500,
  ),
);
const managedRevokedVerifierRegistry = createManagedRelayPublicVerifierKeyRegistry([
  {
    ...managedPublicVerifierRegistry.entries[0],
    state: "revoked",
  },
]);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
      managedEd25519SignedTicket,
      managedRevokedVerifierRegistry,
      { tenantId: "tenant-demo", nowMs: 1500 },
      webcrypto,
    ),
  /inactive/,
);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
      {
        ...managedEd25519SignedTicket,
        ticket: {
          ...managedEd25519SignedTicket.ticket,
          session_id: "tampered-managed-session",
        },
      },
      managedPublicVerifierRegistry,
      { tenantId: "tenant-demo", nowMs: 1500 },
      webcrypto,
    ),
  /signature mismatch/,
);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
      signedRelaySessionTicket,
      managedPublicVerifierRegistry,
      { tenantId: "tenant-demo", nowMs: 1500 },
      webcrypto,
    ),
  /requires ed25519/,
);
assert.throws(() =>
  createManagedRelayPublicVerifierKeyRegistry([
    {
      ...managedPublicVerifierRegistry.entries[0],
      hmac_secret: "not-allowed",
    },
  ]),
);
const managedRotatingTicketSigningKeys = await generateCompanionKeyMaterial(webcrypto);
const managedRotatingEd25519SignedTicket = await createEd25519SignedRelaySessionTicket(
  fixedRelaySessionTicket,
  managedRotatingTicketSigningKeys.keyMaterial,
  { keyId: "managed-key-1", keyVersion: 5 },
  webcrypto,
);
const managedRotationOverlapRegistry = createManagedRelayPublicVerifierKeyRegistry([
  {
    ...managedPublicVerifierRegistry.entries[0],
    state: "active",
  },
  {
    tenant_id: "tenant-demo",
    key_id: "managed-key-1",
    key_version: 5,
    public_key_alg: "ed25519",
    public_key_hex: managedRotatingTicketSigningKeys.identity.approvalPubkeyHex,
    state: "rotating",
    not_before_ms: 1000,
    expires_at_ms: 2000,
  },
]);
const managedRotationOverlapSnapshot = createManagedRelayPublicVerifierKeyRegistrySnapshot(
  managedRotationOverlapRegistry,
  {
    snapshotId: "managed-rotation-snapshot-1",
    effectiveAtMs: 1400,
    reason: "rotation-overlap",
  },
);
assert.equal(
  lookupManagedRelayPublicVerifierKeyFromRegistrySnapshot(
    managedRotationOverlapSnapshot,
    { tenantId: "tenant-demo", keyId: "managed-key-1", keyVersion: 5 },
    1500,
  ).registry_snapshot_id,
  "managed-rotation-snapshot-1",
);
assert.deepEqual(
  (
    await validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
      managedEd25519SignedTicket,
      managedRotationOverlapSnapshot,
      { tenantId: "tenant-demo", nowMs: 1500 },
      webcrypto,
    )
  ).ticket,
  fixedRelaySessionTicket,
);
const managedRotatingValidation =
  await validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
    managedRotatingEd25519SignedTicket,
    managedRotationOverlapSnapshot,
    { tenantId: "tenant-demo", nowMs: 1500 },
    webcrypto,
  );
assert.deepEqual(managedRotatingValidation.ticket, fixedRelaySessionTicket);
assert.deepEqual(
  {
    tenant_id: managedRotatingValidation.auditEvent.tenant_id,
    key_id: managedRotatingValidation.auditEvent.key_id,
    key_version: managedRotatingValidation.auditEvent.key_version,
    key_state: managedRotatingValidation.auditEvent.key_state,
    registry_snapshot_id: managedRotatingValidation.auditEvent.registry_snapshot_id,
  },
  {
    tenant_id: "tenant-demo",
    key_id: "managed-key-1",
    key_version: 5,
    key_state: "rotating",
    registry_snapshot_id: "managed-rotation-snapshot-1",
  },
);
const managedRetiringSnapshot = createManagedRelayPublicVerifierKeyRegistrySnapshot(
  createManagedRelayPublicVerifierKeyRegistry([
    {
      ...managedPublicVerifierRegistry.entries[0],
      state: "retiring",
    },
    {
      ...managedRotationOverlapRegistry.entries[1],
      state: "active",
    },
  ]),
  {
    snapshotId: "managed-rotation-snapshot-2",
    effectiveAtMs: 1600,
    previousSnapshotId: "managed-rotation-snapshot-1",
    reason: "old-key-retiring",
  },
);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
      managedEd25519SignedTicket,
      managedRetiringSnapshot,
      { tenantId: "tenant-demo", nowMs: 1700 },
      webcrypto,
    ),
  /inactive/,
);
const managedRevokedSnapshot = createManagedRelayPublicVerifierKeyRegistrySnapshot(
  createManagedRelayPublicVerifierKeyRegistry([
    {
      ...managedPublicVerifierRegistry.entries[0],
      state: "revoked",
    },
    {
      ...managedRotationOverlapRegistry.entries[1],
      state: "active",
    },
  ]),
  {
    snapshotId: "managed-rotation-snapshot-3",
    effectiveAtMs: 1800,
    previousSnapshotId: "managed-rotation-snapshot-2",
    reason: "old-key-revoked",
  },
);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
      managedEd25519SignedTicket,
      managedRevokedSnapshot,
      { tenantId: "tenant-demo", nowMs: 1900 },
      webcrypto,
    ),
  /inactive/,
);
assert.equal(
  (
    await validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
      managedRotatingEd25519SignedTicket,
      managedRevokedSnapshot,
      { tenantId: "tenant-demo", nowMs: 1900 },
      webcrypto,
    )
  ).auditEvent.key_state,
  "active",
);
const managedRegistrationQuotaState = createManagedRelayTenantSessionRegistrationQuotaState({
  tenantId: "tenant-demo",
  windowStartMs: 1000,
  windowEndMs: 2000,
  registrationLimit: 3,
  registrationsUsed: 2,
  billingMeter: {
    session_registration_count: 2,
    quota_denial_count: 0,
  },
  abuseSignals: {
    rate_limit_denial_count: 0,
    invalid_ticket_count: 0,
  },
});
const managedQuotaRegistration = {
  tenant_id: "tenant-demo",
  session_id: "managed-quota-session-1",
  daemon_device_id: generatedKeys.identity.deviceId,
  verifier_key_id: "managed-key-1",
  verifier_key_version: 5,
  source_ip_hash: "a".repeat(64),
};
const managedQuotaAccepted = evaluateManagedRelayTenantSessionRegistrationQuota(
  managedRegistrationQuotaState,
  managedQuotaRegistration,
  1500,
);
assert.equal(managedQuotaAccepted.decision, "accept");
assert.equal(managedQuotaAccepted.registrationAllowed, true);
assert.equal(managedQuotaAccepted.reason, "within-tenant-session-registration-quota");
assert.equal(managedQuotaAccepted.billingMeterDelta.session_registration_count, 1);
assert.equal(managedQuotaAccepted.billingMeterDelta.quota_denial_count, 0);
assert.equal(managedQuotaAccepted.abuseSignalDelta.rate_limit_denial_count, 0);
assert.equal(managedQuotaAccepted.auditEvent.quota_remaining_after_decision, 0);
const managedQuotaExceeded = evaluateManagedRelayTenantSessionRegistrationQuota(
  createManagedRelayTenantSessionRegistrationQuotaState({
    tenantId: "tenant-demo",
    windowStartMs: 1000,
    windowEndMs: 2000,
    registrationLimit: 3,
    registrationsUsed: 3,
  }),
  {
    ...managedQuotaRegistration,
    session_id: "managed-quota-session-2",
  },
  1500,
);
assert.equal(managedQuotaExceeded.decision, "reject");
assert.equal(managedQuotaExceeded.registrationAllowed, false);
assert.equal(managedQuotaExceeded.reason, "tenant-session-registration-quota-exceeded");
assert.equal(managedQuotaExceeded.billingMeterDelta.session_registration_count, 0);
assert.equal(managedQuotaExceeded.billingMeterDelta.quota_denial_count, 1);
assert.equal(managedQuotaExceeded.abuseSignalDelta.rate_limit_denial_count, 0);
assert.deepEqual(
  {
    tenant_id: managedQuotaExceeded.auditEvent.tenant_id,
    session_id: managedQuotaExceeded.auditEvent.session_id,
    verifier_key_id: managedQuotaExceeded.auditEvent.verifier_key_id,
    verifier_key_version: managedQuotaExceeded.auditEvent.verifier_key_version,
    decision: managedQuotaExceeded.auditEvent.decision,
  },
  {
    tenant_id: "tenant-demo",
    session_id: "managed-quota-session-2",
    verifier_key_id: "managed-key-1",
    verifier_key_version: 5,
    decision: "reject",
  },
);
const managedQuotaAuditJson = JSON.stringify(managedQuotaExceeded.auditEvent);
for (const prohibited of ["payload_json", "session_token", "secret", "mac_hex"]) {
  assert.equal(managedQuotaAuditJson.includes(prohibited), false);
}
assert.equal(
  evaluateManagedRelayTenantSessionRegistrationQuota(
    managedRegistrationQuotaState,
    {
      ...managedQuotaRegistration,
      session_id: "managed-quota-session-3",
    },
    900,
  ).reason,
  "quota-window-not-effective",
);
assert.throws(() =>
  evaluateManagedRelayTenantSessionRegistrationQuota(
    managedRegistrationQuotaState,
    {
      ...managedQuotaRegistration,
      payload_json: { command: "not allowed" },
    },
    1500,
  ),
);
const managedActiveQuotaState = createManagedRelayActiveSessionAndByteQuotaState({
  tenantId: "tenant-demo",
  daemonDeviceId: generatedKeys.identity.deviceId,
  windowStartMs: 1000,
  windowEndMs: 2000,
  tenantActiveSessionLimit: 3,
  tenantActiveSessions: 2,
  daemonDeviceActiveSessionLimit: 2,
  daemonDeviceActiveSessions: 1,
  relayFrameLimit: 5,
  relayFramesUsed: 4,
  relayByteLimit: 1000,
  relayBytesUsed: 800,
  billingMeter: {
    active_session_count: 2,
    relay_frame_count: 4,
    relay_byte_count: 800,
    quota_denial_count: 0,
  },
  abuseSignals: {
    rate_limit_denial_count: 0,
    invalid_ticket_count: 0,
  },
});
const managedActiveQuotaRoute = {
  tenant_id: "tenant-demo",
  session_id: "managed-active-quota-session-1",
  daemon_device_id: generatedKeys.identity.deviceId,
  verifier_key_id: "managed-key-1",
  verifier_key_version: 5,
  frame_sequence: 5,
  payload_ciphertext_bytes: 100,
};
const managedActiveQuotaAccepted = evaluateManagedRelayActiveSessionAndByteQuota(
  managedActiveQuotaState,
  managedActiveQuotaRoute,
  1500,
);
assert.equal(managedActiveQuotaAccepted.decision, "accept");
assert.equal(managedActiveQuotaAccepted.relayAllowed, true);
assert.equal(managedActiveQuotaAccepted.reason, "within-active-session-and-byte-quota");
assert.equal(managedActiveQuotaAccepted.billingMeterDelta.active_session_count, 1);
assert.equal(managedActiveQuotaAccepted.billingMeterDelta.relay_frame_count, 1);
assert.equal(managedActiveQuotaAccepted.billingMeterDelta.relay_byte_count, 100);
assert.equal(managedActiveQuotaAccepted.billingMeterDelta.quota_denial_count, 0);
assert.equal(managedActiveQuotaAccepted.abuseSignalDelta.rate_limit_denial_count, 0);
assert.equal(managedActiveQuotaAccepted.auditEvent.relay_bytes_after_decision, 900);
const managedTenantActiveExceeded = evaluateManagedRelayActiveSessionAndByteQuota(
  createManagedRelayActiveSessionAndByteQuotaState({
    ...managedActiveQuotaState,
    tenantId: managedActiveQuotaState.tenant_id,
    daemonDeviceId: managedActiveQuotaState.daemon_device_id,
    windowStartMs: managedActiveQuotaState.window_start_ms,
    windowEndMs: managedActiveQuotaState.window_end_ms,
    tenantActiveSessionLimit: 3,
    tenantActiveSessions: 3,
    daemonDeviceActiveSessionLimit: 2,
    daemonDeviceActiveSessions: 1,
    relayFrameLimit: 5,
    relayFramesUsed: 4,
    relayByteLimit: 1000,
    relayBytesUsed: 800,
  }),
  {
    ...managedActiveQuotaRoute,
    session_id: "managed-active-quota-session-2",
  },
  1500,
);
assert.equal(managedTenantActiveExceeded.decision, "reject");
assert.equal(managedTenantActiveExceeded.reason, "tenant-active-session-quota-exceeded");
assert.equal(managedTenantActiveExceeded.billingMeterDelta.relay_frame_count, 0);
assert.equal(managedTenantActiveExceeded.billingMeterDelta.relay_byte_count, 0);
assert.equal(managedTenantActiveExceeded.billingMeterDelta.quota_denial_count, 1);
assert.equal(managedTenantActiveExceeded.abuseSignalDelta.rate_limit_denial_count, 0);
assert.equal(
  evaluateManagedRelayActiveSessionAndByteQuota(
    createManagedRelayActiveSessionAndByteQuotaState({
      tenantId: "tenant-demo",
      daemonDeviceId: generatedKeys.identity.deviceId,
      windowStartMs: 1000,
      windowEndMs: 2000,
      tenantActiveSessionLimit: 3,
      tenantActiveSessions: 2,
      daemonDeviceActiveSessionLimit: 1,
      daemonDeviceActiveSessions: 1,
      relayFrameLimit: 5,
      relayFramesUsed: 4,
      relayByteLimit: 1000,
      relayBytesUsed: 800,
    }),
    managedActiveQuotaRoute,
    1500,
  ).reason,
  "daemon-device-active-session-quota-exceeded",
);
assert.equal(
  evaluateManagedRelayActiveSessionAndByteQuota(
    createManagedRelayActiveSessionAndByteQuotaState({
      tenantId: "tenant-demo",
      daemonDeviceId: generatedKeys.identity.deviceId,
      windowStartMs: 1000,
      windowEndMs: 2000,
      tenantActiveSessionLimit: 3,
      tenantActiveSessions: 2,
      daemonDeviceActiveSessionLimit: 2,
      daemonDeviceActiveSessions: 1,
      relayFrameLimit: 5,
      relayFramesUsed: 5,
      relayByteLimit: 1000,
      relayBytesUsed: 800,
    }),
    managedActiveQuotaRoute,
    1500,
  ).reason,
  "relay-frame-quota-exceeded",
);
assert.equal(
  evaluateManagedRelayActiveSessionAndByteQuota(
    createManagedRelayActiveSessionAndByteQuotaState({
      tenantId: "tenant-demo",
      daemonDeviceId: generatedKeys.identity.deviceId,
      windowStartMs: 1000,
      windowEndMs: 2000,
      tenantActiveSessionLimit: 3,
      tenantActiveSessions: 2,
      daemonDeviceActiveSessionLimit: 2,
      daemonDeviceActiveSessions: 1,
      relayFrameLimit: 5,
      relayFramesUsed: 4,
      relayByteLimit: 850,
      relayBytesUsed: 800,
    }),
    managedActiveQuotaRoute,
    1500,
  ).reason,
  "relay-byte-quota-exceeded",
);
assert.equal(
  evaluateManagedRelayActiveSessionAndByteQuota(
    managedActiveQuotaState,
    managedActiveQuotaRoute,
    900,
  ).reason,
  "quota-window-not-effective",
);
assert.throws(() =>
  evaluateManagedRelayActiveSessionAndByteQuota(
    managedActiveQuotaState,
    {
      ...managedActiveQuotaRoute,
      payload_json: { command: "not allowed" },
    },
    1500,
  ),
);
const managedActiveQuotaAuditJson = JSON.stringify(managedTenantActiveExceeded.auditEvent);
for (const prohibited of ["payload_json", "command_text", "session_token", "secret", "mac_hex"]) {
  assert.equal(managedActiveQuotaAuditJson.includes(prohibited), false);
}
const managedTenantUsageExport = createManagedRelayTenantAggregateUsageExport({
  tenantId: "tenant-demo",
  windowStartMs: 1000,
  windowEndMs: 2000,
  generatedAtMs: 2100,
  planId: "managed-relay-plan-a",
  billingMeter: {
    session_registration_count: 3,
    active_session_count: 2,
    relay_frame_count: 5,
    relay_byte_count: 900,
    invalid_ticket_count: 1,
    quota_denial_count: 2,
  },
  abuseSignals: {
    rate_limit_denial_count: 1,
    invalid_ticket_count: 1,
    abuse_case_count: 0,
  },
});
assert.equal(managedTenantUsageExport.export_scope, "tenant-aggregate-usage");
assert.equal(managedTenantUsageExport.payload_visibility, "payload-free");
assert.equal(managedTenantUsageExport.support_visibility, "aggregate-only");
assert.equal(managedTenantUsageExport.billing_usage.session_registration_count, 3);
assert.equal(managedTenantUsageExport.billing_usage.active_session_count, 2);
assert.equal(managedTenantUsageExport.billing_usage.relay_frame_count, 5);
assert.equal(managedTenantUsageExport.billing_usage.relay_byte_count, 900);
assert.equal(managedTenantUsageExport.billing_usage.quota_denial_count, 2);
assert.equal(managedTenantUsageExport.abuse_signal_summary.rate_limit_denial_count, 1);
assert.equal(
  managedTenantUsageExport.billing_abuse_boundary.abuse_signals_are_not_billing_meters,
  true,
);
assert.ok(
  managedTenantUsageExport.billing_abuse_boundary.billing_usage_fields.includes(
    "relay_byte_count",
  ),
);
assert.ok(
  managedTenantUsageExport.billing_abuse_boundary.abuse_signal_fields.includes(
    "rate_limit_denial_count",
  ),
);
const managedTenantUsageExportJson = JSON.stringify(managedTenantUsageExport);
for (const prohibited of ["payload_json", "command_text", "session_token", "secret", "mac_hex"]) {
  assert.equal(managedTenantUsageExportJson.includes(prohibited), false);
}
assert.throws(() =>
  createManagedRelayTenantAggregateUsageExport({
    tenantId: "tenant-demo",
    windowStartMs: 1000,
    windowEndMs: 2000,
    generatedAtMs: 2100,
    planId: "managed-relay-plan-a",
    billingMeter: {
      session_registration_count: 1,
    },
    payload_json: { command: "not allowed" },
  }),
);
const managedSupportRedactionView = createManagedRelaySupportRedactionAccessReview({
  tenantId: "tenant-demo",
  supportCaseId: "support-case-2026-07",
  supportActorIdHash: "sha256:aaaaaaaaaaaaaaaa",
  tenantAdminApprovalId: "approval-2026-07",
  accessApprovedAtMs: 1000,
  accessExpiresAtMs: 2000,
  generatedAtMs: 1500,
  sessionIdHash: "sha256:bbbbbbbbbbbbbbbb",
  daemonDeviceIdHash: "sha256:cccccccccccccccc",
  companionDeviceIdHash: "sha256:dddddddddddddddd",
  aggregateErrorClass: "quota-denial",
  quotaState: "within-limit",
  keyId: "managed-relay-key-a",
  keyVersion: 2,
  billingUsage: {
    session_registration_count: 3,
    active_session_count: 2,
    relay_frame_count: 5,
    relay_byte_count: 900,
    invalid_ticket_count: 1,
    quota_denial_count: 2,
  },
  abuseSignals: {
    rate_limit_denial_count: 1,
    invalid_ticket_count: 1,
    abuse_case_count: 0,
  },
});
assert.equal(managedSupportRedactionView.support_visibility, "aggregate-only");
assert.equal(managedSupportRedactionView.redaction_state, "redacted");
assert.equal(managedSupportRedactionView.payload_visibility, "payload-free");
assert.equal(managedSupportRedactionView.session_id_hash, "sha256:bbbbbbbbbbbbbbbb");
assert.equal(managedSupportRedactionView.daemon_device_id_hash, "sha256:cccccccccccccccc");
assert.equal(managedSupportRedactionView.companion_device_id_hash, "sha256:dddddddddddddddd");
assert.equal(managedSupportRedactionView.billing_usage_summary.relay_byte_count, 900);
assert.equal(managedSupportRedactionView.abuse_signal_summary.rate_limit_denial_count, 1);
assert.equal(managedSupportRedactionView.access_review_audit.decision, "approved");
assert.equal(
  managedSupportRedactionView.access_review_audit.tenant_admin_approval_id,
  "approval-2026-07",
);
assert.equal("session_id" in managedSupportRedactionView, false);
assert.equal("daemon_device_id" in managedSupportRedactionView, false);
assert.equal("companion_device_id" in managedSupportRedactionView, false);
assert.equal("support_actor_id" in managedSupportRedactionView, false);
const managedSupportRedactionJson = JSON.stringify(managedSupportRedactionView);
for (const prohibited of ["payload_json", "command_text", "session_token", "secret", "mac_hex"]) {
  assert.equal(managedSupportRedactionJson.includes(prohibited), false);
}
assert.throws(
  () =>
    createManagedRelaySupportRedactionAccessReview({
      tenantId: "tenant-demo",
      supportCaseId: "support-case-2026-07",
      supportActorIdHash: "sha256:aaaaaaaaaaaaaaaa",
      tenantAdminApprovalId: "approval-2026-07",
      accessApprovedAtMs: 1000,
      accessExpiresAtMs: 2000,
      generatedAtMs: 1500,
      sessionIdHash: "sha256:bbbbbbbbbbbbbbbb",
      daemonDeviceIdHash: "sha256:cccccccccccccccc",
      companionDeviceIdHash: "sha256:dddddddddddddddd",
      aggregateErrorClass: "quota-denial",
      quotaState: "within-limit",
      keyId: "managed-relay-key-a",
      keyVersion: 2,
      session_id: "raw-session-not-allowed",
    }),
  /raw support identifier/,
);
assert.throws(
  () =>
    createManagedRelaySupportRedactionAccessReview({
      tenantId: "tenant-demo",
      supportCaseId: "support-case-2026-07",
      supportActorIdHash: "sha256:aaaaaaaaaaaaaaaa",
      tenantAdminApprovalId: "approval-2026-07",
      accessApprovedAtMs: 1000,
      accessExpiresAtMs: 2000,
      generatedAtMs: 1500,
      sessionIdHash: "sha256:bbbbbbbbbbbbbbbb",
      daemonDeviceIdHash: "sha256:cccccccccccccccc",
      companionDeviceIdHash: "sha256:dddddddddddddddd",
      aggregateErrorClass: "quota-denial",
      quotaState: "within-limit",
      keyId: "managed-relay-key-a",
      keyVersion: 2,
      payload_json: { command: "not allowed" },
    }),
  /prohibited payload or secret data/,
);
const managedBillingAbuseBoundaryReview = createManagedRelayBillingAbuseBoundaryReview({
  tenantId: "tenant-demo",
  windowStartMs: 1000,
  windowEndMs: 2000,
  generatedAtMs: 2100,
  planId: "managed-relay-plan-a",
  billingUsage: {
    session_registration_count: 3,
    active_session_count: 2,
    relay_frame_count: 5,
    relay_byte_count: 900,
    invalid_ticket_count: 1,
    quota_denial_count: 2,
  },
  abuseSignals: {
    rate_limit_denial_count: 1,
    invalid_ticket_count: 1,
    abuse_case_count: 0,
  },
  tenantUsageExport: managedTenantUsageExport,
  supportView: {
    ...managedSupportRedactionView,
    billing_usage_summary: {
      ...managedSupportRedactionView.billing_usage_summary,
      relay_byte_count: 999999,
    },
  },
});
assert.equal(
  managedBillingAbuseBoundaryReview.review_scope,
  "managed-relay-billing-abuse-boundary",
);
assert.equal(
  managedBillingAbuseBoundaryReview.billing_usage_summary.relay_byte_count,
  900,
);
assert.equal(
  managedBillingAbuseBoundaryReview.abuse_signal_summary.rate_limit_denial_count,
  1,
);
assert.equal(
  managedBillingAbuseBoundaryReview.tenant_usage_export_boundary.abuse_signals_are_not_billing_meters,
  true,
);
assert.equal(
  managedBillingAbuseBoundaryReview.support_evidence_boundary.support_evidence_is_not_billing_source,
  true,
);
assert.equal(
  managedBillingAbuseBoundaryReview.boundary_decisions.tenant_deletion_workflow_reviewed,
  true,
);
assert.equal(
  managedBillingAbuseBoundaryReview.boundary_decisions.abuse_escalation_runbook_reviewed,
  true,
);
assert.ok(
  managedBillingAbuseBoundaryReview.billing_abuse_boundary.billing_usage_fields.includes(
    "relay_byte_count",
  ),
);
assert.ok(
  managedBillingAbuseBoundaryReview.billing_abuse_boundary.abuse_signal_fields.includes(
    "rate_limit_denial_count",
  ),
);
const managedBillingAbuseBoundaryJson = JSON.stringify(managedBillingAbuseBoundaryReview);
for (const prohibited of ["payload_json", "command_text", "session_token", "secret", "mac_hex"]) {
  assert.equal(managedBillingAbuseBoundaryJson.includes(prohibited), false);
}
assert.throws(
  () =>
    createManagedRelayBillingAbuseBoundaryReview({
      tenantId: "tenant-demo",
      windowStartMs: 1000,
      windowEndMs: 2000,
      generatedAtMs: 2100,
      planId: "managed-relay-plan-a",
      billingUsage: {
        relay_byte_count: 900,
        support_case_id: "support-case-2026-07",
      },
      abuseSignals: {
        rate_limit_denial_count: 1,
      },
      tenantUsageExport: managedTenantUsageExport,
      supportView: managedSupportRedactionView,
    }),
  /billing usage contains abuse or support data/,
);
assert.throws(
  () =>
    createManagedRelayBillingAbuseBoundaryReview({
      tenantId: "tenant-demo",
      windowStartMs: 1000,
      windowEndMs: 2000,
      generatedAtMs: 2100,
      planId: "managed-relay-plan-a",
      billingUsage: {
        relay_byte_count: 900,
      },
      abuseSignals: {
        relay_byte_count: 900,
      },
      tenantUsageExport: managedTenantUsageExport,
      supportView: managedSupportRedactionView,
    }),
  /abuse signal contains billing data/,
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
assert.equal(managedOperationsPlan.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");
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
  "managed-relay-runtime-pwa-exposure-gate",
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
  "managed-relay-runtime-pwa-exposure-gate",
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
  "managed-relay-runtime-pwa-exposure-gate",
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
  "managed-relay-runtime-pwa-exposure-gate",
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
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedRuntimeReadinessGate = relayManagedRuntimeReadinessGate();
assert.equal(managedRuntimeReadinessGate.deploymentMode, "managed");
assert.equal(managedRuntimeReadinessGate.readiness, "gate");
assert.equal(managedRuntimeReadinessGate.productDefault, "live-loopback");
assert.equal(managedRuntimeReadinessGate.selectedRuntime, "deferred");
assert.equal(managedRuntimeReadinessGate.gateStatus, "runtime-evidence-green");
assert.equal(
  managedRuntimeReadinessGate.implementationDecision,
  "managed-runtime-implementation-can-start",
);
assert.equal(managedRuntimeReadinessGate.implementationCanStart, true);
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
  managedRuntimeReadinessGate.completedRuntimeEvidence.includes(
    "payload-blind-frame-encryption-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.completedRuntimeEvidence.includes(
    "client-key-agreement-runtime-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.completedRuntimeEvidence.includes(
    "metadata-minimization-review",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.completedRuntimeEvidence.includes(
    "public-verifier-key-registry-runtime-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.completedRuntimeEvidence.includes(
    "revocation-and-rotation-propagation-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.completedRuntimeEvidence.includes(
    "tenant-session-registration-quota-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.completedRuntimeEvidence.includes(
    "active-session-and-byte-quota-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.completedRuntimeEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.completedRuntimeEvidence.includes(
    "billing-abuse-boundary-review",
  ),
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "payload-blind-frame-encryption-smoke",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "client-key-agreement-runtime-smoke",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "metadata-minimization-review",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "public-verifier-key-registry-runtime-smoke",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "revocation-and-rotation-propagation-smoke",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "tenant-session-registration-quota-smoke",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "active-session-and-byte-quota-smoke",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeEvidence.includes(
    "billing-abuse-boundary-review",
  ),
  false,
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "e2e_payload_encryption_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "client_key_agreement_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "metadata_minimization_review_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "managed_key_registry_runtime_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "key_revocation_propagation_smoke_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "rotation_overlap_smoke_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "quota_enforcement_smoke_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "managed_usage_meter_runtime_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "tenant_usage_export_smoke_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "support_audit_boundary_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "support_access_review_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "support_redaction_evidence_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "billing_abuse_boundary_review_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "runtime_rate_limit_enforcement_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "abuse_escalation_runbook_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.resolvedRuntimeBlockers.includes(
    "tenant_deletion_workflow_missing",
  ),
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "e2e_payload_encryption_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "client_key_agreement_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "metadata_minimization_review_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "managed_key_registry_runtime_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "key_revocation_propagation_smoke_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "rotation_overlap_smoke_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "quota_enforcement_smoke_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "managed_usage_meter_runtime_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "tenant_usage_export_smoke_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "support_audit_boundary_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "support_access_review_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "support_redaction_evidence_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "billing_abuse_boundary_review_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "runtime_rate_limit_enforcement_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "abuse_escalation_runbook_missing",
  ),
  false,
);
assert.equal(
  managedRuntimeReadinessGate.remainingRuntimeBlockers.includes(
    "tenant_deletion_workflow_missing",
  ),
  false,
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
    "active-session-and-byte-quota-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.requiredRuntimeEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.requiredRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.requiredRuntimeEvidence.includes(
    "billing-abuse-boundary-review",
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
    "managed_usage_meter_runtime_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "tenant_usage_export_smoke_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "support_access_review_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "support_redaction_evidence_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "billing_abuse_boundary_review_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "runtime_rate_limit_enforcement_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "abuse_escalation_runbook_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.auditedRuntimeBlockers.includes(
    "tenant_deletion_workflow_missing",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.payloadConfidentiality.evidence.includes(
    "client-key-agreement-runtime-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.payloadConfidentiality.completedEvidence.includes(
    "payload-blind-frame-encryption-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.payloadConfidentiality.completedEvidence.includes(
    "client-key-agreement-runtime-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.payloadConfidentiality.completedEvidence.includes(
    "metadata-minimization-review",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.verifierKeys.completedEvidence.includes(
    "public-verifier-key-registry-runtime-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.verifierKeys.completedEvidence.includes(
    "revocation-and-rotation-propagation-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.quotaAndUsage.evidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.quotaAndUsage.completedEvidence.includes(
    "tenant-session-registration-quota-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.quotaAndUsage.completedEvidence.includes(
    "active-session-and-byte-quota-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.quotaAndUsage.completedEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.abuseRetentionAndSupport.completedEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
);
assert.ok(
  managedRuntimeReadinessGate.runtimeReadinessDomains.abuseRetentionAndSupport.completedEvidence.includes(
    "billing-abuse-boundary-review",
  ),
);
assert.equal(
  managedRuntimeReadinessGate.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedPayloadBlindFrameEncryptionSpike = relayManagedPayloadBlindFrameEncryptionSpike();
assert.equal(managedPayloadBlindFrameEncryptionSpike.deploymentMode, "managed");
assert.equal(managedPayloadBlindFrameEncryptionSpike.readiness, "spike");
assert.equal(managedPayloadBlindFrameEncryptionSpike.selectedRuntime, "deferred");
assert.equal(
  managedPayloadBlindFrameEncryptionSpike.payloadCiphertextAlg,
  "aes-256-gcm",
);
assert.ok(
  managedPayloadBlindFrameEncryptionSpike.completedRuntimeEvidence.includes(
    "payload-blind-frame-encryption-smoke",
  ),
);
assert.ok(
  managedPayloadBlindFrameEncryptionSpike.closedReadinessBlockers.includes(
    "confidentiality_smoke_missing",
  ),
);
assert.equal(
  managedPayloadBlindFrameEncryptionSpike.remainingRuntimeEvidence.includes(
    "client-key-agreement-runtime-smoke",
  ),
  false,
);
assert.ok(
  managedPayloadBlindFrameEncryptionSpike.frameEnvelope.routeVisibleFields.includes(
    "payload_ciphertext_bytes",
  ),
);
assert.ok(
  managedPayloadBlindFrameEncryptionSpike.frameEnvelope.prohibitedManagedFrameFields.includes(
    "payload_json",
  ),
);
assert.equal(
  managedPayloadBlindFrameEncryptionSpike.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedClientKeyAgreementRuntimeSmoke = relayManagedClientKeyAgreementRuntimeSmoke();
assert.equal(managedClientKeyAgreementRuntimeSmoke.deploymentMode, "managed");
assert.equal(managedClientKeyAgreementRuntimeSmoke.readiness, "smoke");
assert.equal(managedClientKeyAgreementRuntimeSmoke.selectedRuntime, "deferred");
assert.equal(
  managedClientKeyAgreementRuntimeSmoke.keyAgreementAlg,
  "x25519-hkdf-sha256",
);
assert.ok(
  managedClientKeyAgreementRuntimeSmoke.completedRuntimeEvidence.includes(
    "client-key-agreement-runtime-smoke",
  ),
);
assert.ok(
  managedClientKeyAgreementRuntimeSmoke.closedReadinessBlockers.includes(
    "client_key_agreement_missing",
  ),
);
assert.equal(
  managedClientKeyAgreementRuntimeSmoke.remainingRuntimeEvidence.includes(
    "client-key-agreement-runtime-smoke",
  ),
  false,
);
assert.equal(
  managedClientKeyAgreementRuntimeSmoke.remainingRuntimeEvidence.includes(
    "metadata-minimization-review",
  ),
  false,
);
assert.equal(
  managedClientKeyAgreementRuntimeSmoke.remainingRuntimeEvidence.includes(
    "public-verifier-key-registry-runtime-smoke",
  ),
  false,
);
assert.ok(
  managedClientKeyAgreementRuntimeSmoke.keyAgreement.prohibitedRouteKeyMaterial.includes(
    "payload_key_hex",
  ),
);
assert.equal(
  managedClientKeyAgreementRuntimeSmoke.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedMetadataMinimizationReview = relayManagedMetadataMinimizationReview();
assert.equal(managedMetadataMinimizationReview.deploymentMode, "managed");
assert.equal(managedMetadataMinimizationReview.readiness, "review");
assert.equal(managedMetadataMinimizationReview.selectedRuntime, "deferred");
assert.equal(
  managedMetadataMinimizationReview.implementationStatus,
  "metadata-minimization-review-complete-runtime-still-deferred",
);
assert.ok(
  managedMetadataMinimizationReview.completedRuntimeEvidence.includes(
    "metadata-minimization-review",
  ),
);
assert.ok(
  managedMetadataMinimizationReview.closedReadinessBlockers.includes(
    "metadata_minimization_review_missing",
  ),
);
assert.equal(
  managedMetadataMinimizationReview.remainingRuntimeEvidence.includes(
    "metadata-minimization-review",
  ),
  false,
);
assert.equal(
  managedMetadataMinimizationReview.remainingRuntimeEvidence.includes(
    "public-verifier-key-registry-runtime-smoke",
  ),
  false,
);
assert.ok(
  managedMetadataMinimizationReview.metadataSurfaces.routeEnvelope.includes(
    "payload_ciphertext_bytes",
  ),
);
assert.equal(
  managedMetadataMinimizationReview.metadataSurfaces.routeEnvelope.includes(
    "payload_ciphertext_hex",
  ),
  false,
);
assert.ok(
  managedMetadataMinimizationReview.prohibitedMetadataFields.includes(
    "payload_ciphertext_hex",
  ),
);
assert.ok(
  managedMetadataMinimizationReview.minimizationEvidence.includes(
    "support-view-redacts-session-and-device-identifiers",
  ),
);
assert.equal(
  managedMetadataMinimizationReview.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedPublicVerifierKeyRegistryRuntimeSmoke =
  relayManagedPublicVerifierKeyRegistryRuntimeSmoke();
assert.equal(managedPublicVerifierKeyRegistryRuntimeSmoke.deploymentMode, "managed");
assert.equal(managedPublicVerifierKeyRegistryRuntimeSmoke.readiness, "smoke");
assert.equal(managedPublicVerifierKeyRegistryRuntimeSmoke.selectedRuntime, "deferred");
assert.equal(
  managedPublicVerifierKeyRegistryRuntimeSmoke.implementationStatus,
  "public-verifier-key-registry-smoke-ready-runtime-still-deferred",
);
assert.equal(managedPublicVerifierKeyRegistryRuntimeSmoke.verifierKeyAlg, "ed25519");
assert.equal(
  managedPublicVerifierKeyRegistryRuntimeSmoke.registryBoundary,
  "tenant-key-id-version-public-verifiers-only",
);
assert.ok(
  managedPublicVerifierKeyRegistryRuntimeSmoke.completedRuntimeEvidence.includes(
    "public-verifier-key-registry-runtime-smoke",
  ),
);
assert.ok(
  managedPublicVerifierKeyRegistryRuntimeSmoke.closedReadinessBlockers.includes(
    "managed_key_registry_runtime_missing",
  ),
);
assert.ok(
  managedPublicVerifierKeyRegistryRuntimeSmoke.registryContract.lookupFields.includes(
    "key_version",
  ),
);
assert.ok(
  managedPublicVerifierKeyRegistryRuntimeSmoke.registryContract.prohibitedRegistryFields.includes(
    "private_signing_key",
  ),
);
assert.ok(
  managedPublicVerifierKeyRegistryRuntimeSmoke.smokeEvidence.includes(
    "ed25519-session-ticket-verified-with-public-key-only",
  ),
);
assert.equal(
  managedPublicVerifierKeyRegistryRuntimeSmoke.remainingRuntimeEvidence.includes(
    "public-verifier-key-registry-runtime-smoke",
  ),
  false,
);
assert.equal(
  managedPublicVerifierKeyRegistryRuntimeSmoke.remainingRuntimeEvidence.includes(
    "revocation-and-rotation-propagation-smoke",
  ),
  false,
);
assert.equal(managedPublicVerifierKeyRegistryRuntimeSmoke.implementationCanStart, true);
assert.equal(
  managedPublicVerifierKeyRegistryRuntimeSmoke.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedRevocationAndRotationPropagationSmoke =
  relayManagedRevocationAndRotationPropagationSmoke();
assert.equal(managedRevocationAndRotationPropagationSmoke.deploymentMode, "managed");
assert.equal(managedRevocationAndRotationPropagationSmoke.readiness, "smoke");
assert.equal(managedRevocationAndRotationPropagationSmoke.selectedRuntime, "deferred");
assert.equal(
  managedRevocationAndRotationPropagationSmoke.implementationStatus,
  "revocation-and-rotation-propagation-smoke-ready-runtime-still-deferred",
);
assert.equal(
  managedRevocationAndRotationPropagationSmoke.propagationBoundary,
  "snapshot-based-tenant-key-version-state",
);
assert.ok(
  managedRevocationAndRotationPropagationSmoke.completedRuntimeEvidence.includes(
    "revocation-and-rotation-propagation-smoke",
  ),
);
assert.ok(
  managedRevocationAndRotationPropagationSmoke.closedReadinessBlockers.includes(
    "key_revocation_propagation_smoke_missing",
  ),
);
assert.ok(
  managedRevocationAndRotationPropagationSmoke.closedReadinessBlockers.includes(
    "rotation_overlap_smoke_missing",
  ),
);
assert.ok(
  managedRevocationAndRotationPropagationSmoke.propagationContract.acceptedDuringOverlap.includes(
    "rotating",
  ),
);
assert.ok(
  managedRevocationAndRotationPropagationSmoke.propagationContract.failClosedForNewSessions.includes(
    "revoked",
  ),
);
assert.ok(
  managedRevocationAndRotationPropagationSmoke.propagationContract.auditFields.includes(
    "registry_snapshot_id",
  ),
);
assert.ok(
  managedRevocationAndRotationPropagationSmoke.smokeEvidence.includes(
    "revoked-key-version-fails-closed-after-snapshot-propagation",
  ),
);
assert.equal(
  managedRevocationAndRotationPropagationSmoke.remainingRuntimeEvidence.includes(
    "revocation-and-rotation-propagation-smoke",
  ),
  false,
);
assert.equal(
  managedRevocationAndRotationPropagationSmoke.remainingRuntimeEvidence.includes(
    "tenant-session-registration-quota-smoke",
  ),
  false,
);
assert.equal(
  managedRevocationAndRotationPropagationSmoke.remainingRuntimeEvidence.includes(
    "active-session-and-byte-quota-smoke",
  ),
  false,
);
assert.equal(
  managedRevocationAndRotationPropagationSmoke.remainingRuntimeEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
  false,
);
assert.equal(
  managedRevocationAndRotationPropagationSmoke.remainingRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
  false,
);
assert.equal(
  managedRevocationAndRotationPropagationSmoke.remainingRuntimeEvidence.includes(
    "billing-abuse-boundary-review",
  ),
  false,
);
assert.equal(managedRevocationAndRotationPropagationSmoke.implementationCanStart, true);
assert.equal(
  managedRevocationAndRotationPropagationSmoke.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedTenantSessionRegistrationQuotaSmoke =
  relayManagedTenantSessionRegistrationQuotaSmoke();
assert.equal(managedTenantSessionRegistrationQuotaSmoke.deploymentMode, "managed");
assert.equal(managedTenantSessionRegistrationQuotaSmoke.readiness, "smoke");
assert.equal(managedTenantSessionRegistrationQuotaSmoke.selectedRuntime, "deferred");
assert.equal(
  managedTenantSessionRegistrationQuotaSmoke.implementationStatus,
  "tenant-session-registration-quota-smoke-ready-runtime-still-deferred",
);
assert.equal(
  managedTenantSessionRegistrationQuotaSmoke.quotaBoundary,
  "tenant-scoped-session-registration-preflight",
);
assert.ok(
  managedTenantSessionRegistrationQuotaSmoke.completedRuntimeEvidence.includes(
    "tenant-session-registration-quota-smoke",
  ),
);
assert.ok(
  managedTenantSessionRegistrationQuotaSmoke.closedReadinessBlockers.includes(
    "quota_enforcement_smoke_missing",
  ),
);
assert.ok(
  managedTenantSessionRegistrationQuotaSmoke.quotaContract.registrationRequestFields.includes(
    "source_ip_hash",
  ),
);
assert.ok(
  managedTenantSessionRegistrationQuotaSmoke.quotaContract.failClosedReasons.includes(
    "tenant-session-registration-quota-exceeded",
  ),
);
assert.ok(
  managedTenantSessionRegistrationQuotaSmoke.quotaContract.auditFields.includes(
    "billing_meter_delta",
  ),
);
assert.ok(
  managedTenantSessionRegistrationQuotaSmoke.smokeEvidence.includes(
    "quota-denials-increment-billing-meter-not-abuse-rate-limit",
  ),
);
assert.equal(
  managedTenantSessionRegistrationQuotaSmoke.remainingRuntimeEvidence.includes(
    "tenant-session-registration-quota-smoke",
  ),
  false,
);
assert.equal(
  managedTenantSessionRegistrationQuotaSmoke.remainingRuntimeEvidence.includes(
    "active-session-and-byte-quota-smoke",
  ),
  false,
);
assert.equal(
  managedTenantSessionRegistrationQuotaSmoke.remainingRuntimeEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
  false,
);
assert.equal(
  managedTenantSessionRegistrationQuotaSmoke.remainingRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
  false,
);
assert.equal(
  managedTenantSessionRegistrationQuotaSmoke.remainingRuntimeEvidence.includes(
    "billing-abuse-boundary-review",
  ),
  false,
);
assert.equal(managedTenantSessionRegistrationQuotaSmoke.implementationCanStart, true);
assert.equal(
  managedTenantSessionRegistrationQuotaSmoke.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedActiveSessionAndByteQuotaSmoke =
  relayManagedActiveSessionAndByteQuotaSmoke();
assert.equal(managedActiveSessionAndByteQuotaSmoke.deploymentMode, "managed");
assert.equal(managedActiveSessionAndByteQuotaSmoke.readiness, "smoke");
assert.equal(managedActiveSessionAndByteQuotaSmoke.selectedRuntime, "deferred");
assert.equal(
  managedActiveSessionAndByteQuotaSmoke.implementationStatus,
  "active-session-and-byte-quota-smoke-ready-runtime-still-deferred",
);
assert.equal(
  managedActiveSessionAndByteQuotaSmoke.quotaBoundary,
  "tenant-and-daemon-active-session-plus-frame-byte-preflight",
);
assert.ok(
  managedActiveSessionAndByteQuotaSmoke.completedRuntimeEvidence.includes(
    "active-session-and-byte-quota-smoke",
  ),
);
assert.ok(
  managedActiveSessionAndByteQuotaSmoke.closedReadinessBlockers.includes(
    "managed_usage_meter_runtime_missing",
  ),
);
assert.ok(
  managedActiveSessionAndByteQuotaSmoke.quotaContract.routeRequestFields.includes(
    "payload_ciphertext_bytes",
  ),
);
assert.ok(
  managedActiveSessionAndByteQuotaSmoke.quotaContract.failClosedReasons.includes(
    "tenant-active-session-quota-exceeded",
  ),
);
assert.ok(
  managedActiveSessionAndByteQuotaSmoke.quotaContract.failClosedReasons.includes(
    "relay-byte-quota-exceeded",
  ),
);
assert.ok(
  managedActiveSessionAndByteQuotaSmoke.quotaContract.auditFields.includes(
    "billing_meter_delta",
  ),
);
assert.ok(
  managedActiveSessionAndByteQuotaSmoke.smokeEvidence.includes(
    "relay-byte-limit-rejects-before-routing",
  ),
);
assert.ok(
  managedActiveSessionAndByteQuotaSmoke.smokeEvidence.includes(
    "accepted-routes-increment-active-frame-byte-billing-meters",
  ),
);
assert.equal(
  managedActiveSessionAndByteQuotaSmoke.remainingRuntimeEvidence.includes(
    "active-session-and-byte-quota-smoke",
  ),
  false,
);
assert.equal(
  managedActiveSessionAndByteQuotaSmoke.remainingRuntimeEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
  false,
);
assert.equal(
  managedActiveSessionAndByteQuotaSmoke.remainingRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
  false,
);
assert.equal(
  managedActiveSessionAndByteQuotaSmoke.remainingRuntimeEvidence.includes(
    "billing-abuse-boundary-review",
  ),
  false,
);
assert.equal(managedActiveSessionAndByteQuotaSmoke.implementationCanStart, true);
assert.equal(
  managedActiveSessionAndByteQuotaSmoke.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedTenantAggregateUsageExportSmoke =
  relayManagedTenantAggregateUsageExportSmoke();
assert.equal(managedTenantAggregateUsageExportSmoke.deploymentMode, "managed");
assert.equal(managedTenantAggregateUsageExportSmoke.readiness, "smoke");
assert.equal(managedTenantAggregateUsageExportSmoke.selectedRuntime, "deferred");
assert.equal(
  managedTenantAggregateUsageExportSmoke.implementationStatus,
  "tenant-aggregate-usage-export-smoke-ready-runtime-still-deferred",
);
assert.equal(
  managedTenantAggregateUsageExportSmoke.exportBoundary,
  "tenant-aggregate-usage-counters-without-payloads-or-secrets",
);
assert.ok(
  managedTenantAggregateUsageExportSmoke.completedRuntimeEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
);
assert.ok(
  managedTenantAggregateUsageExportSmoke.closedReadinessBlockers.includes(
    "tenant_usage_export_smoke_missing",
  ),
);
assert.ok(
  managedTenantAggregateUsageExportSmoke.exportContract.billingUsageFields.includes(
    "relay_byte_count",
  ),
);
assert.ok(
  managedTenantAggregateUsageExportSmoke.exportContract.abuseSignalFields.includes(
    "rate_limit_denial_count",
  ),
);
assert.ok(
  managedTenantAggregateUsageExportSmoke.exportContract.outputFields.includes(
    "billing_abuse_boundary",
  ),
);
assert.ok(
  managedTenantAggregateUsageExportSmoke.smokeEvidence.includes(
    "billing-usage-and-abuse-signals-exported-in-separate-sections",
  ),
);
assert.equal(
  managedTenantAggregateUsageExportSmoke.remainingRuntimeEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
  false,
);
assert.equal(
  managedTenantAggregateUsageExportSmoke.remainingRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
  false,
);
assert.equal(
  managedTenantAggregateUsageExportSmoke.remainingRuntimeEvidence.includes(
    "billing-abuse-boundary-review",
  ),
  false,
);
assert.equal(managedTenantAggregateUsageExportSmoke.implementationCanStart, true);
assert.equal(
  managedTenantAggregateUsageExportSmoke.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedSupportRedactionAndAccessReviewEvidence =
  relayManagedSupportRedactionAndAccessReviewEvidence();
assert.equal(managedSupportRedactionAndAccessReviewEvidence.deploymentMode, "managed");
assert.equal(managedSupportRedactionAndAccessReviewEvidence.readiness, "evidence");
assert.equal(managedSupportRedactionAndAccessReviewEvidence.selectedRuntime, "deferred");
assert.equal(
  managedSupportRedactionAndAccessReviewEvidence.implementationStatus,
  "support-redaction-access-review-ready-runtime-still-deferred",
);
assert.equal(
  managedSupportRedactionAndAccessReviewEvidence.supportBoundary,
  "aggregate-redacted-support-view-with-audited-access",
);
assert.ok(
  managedSupportRedactionAndAccessReviewEvidence.completedRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
);
assert.ok(
  managedSupportRedactionAndAccessReviewEvidence.closedReadinessBlockers.includes(
    "support_access_review_missing",
  ),
);
assert.ok(
  managedSupportRedactionAndAccessReviewEvidence.closedReadinessBlockers.includes(
    "support_redaction_evidence_missing",
  ),
);
assert.ok(
  managedSupportRedactionAndAccessReviewEvidence.supportAccessContract.hashedIdentifierFields.includes(
    "session_id_hash",
  ),
);
assert.ok(
  managedSupportRedactionAndAccessReviewEvidence.supportAccessContract.auditFields.includes(
    "tenant_admin_approval_id",
  ),
);
assert.ok(
  managedSupportRedactionAndAccessReviewEvidence.supportAccessContract.prohibitedRawFields.includes(
    "session_id",
  ),
);
assert.ok(
  managedSupportRedactionAndAccessReviewEvidence.evidenceChecks.includes(
    "support-identifiers-are-hashed",
  ),
);
assert.equal(
  managedSupportRedactionAndAccessReviewEvidence.remainingRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
  false,
);
assert.equal(
  managedSupportRedactionAndAccessReviewEvidence.remainingRuntimeEvidence.includes(
    "billing-abuse-boundary-review",
  ),
  false,
);
assert.equal(managedSupportRedactionAndAccessReviewEvidence.implementationCanStart, true);
assert.equal(
  managedSupportRedactionAndAccessReviewEvidence.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedBillingAbuseBoundaryReviewSummary =
  relayManagedBillingAbuseBoundaryReview();
assert.equal(managedBillingAbuseBoundaryReviewSummary.deploymentMode, "managed");
assert.equal(managedBillingAbuseBoundaryReviewSummary.readiness, "review");
assert.equal(managedBillingAbuseBoundaryReviewSummary.selectedRuntime, "deferred");
assert.equal(
  managedBillingAbuseBoundaryReviewSummary.implementationStatus,
  "billing-abuse-boundary-review-ready-runtime-still-deferred",
);
assert.equal(
  managedBillingAbuseBoundaryReviewSummary.billingAbuseBoundary,
  "billing-usage-and-abuse-signals-separate-non-reclassifiable",
);
assert.ok(
  managedBillingAbuseBoundaryReviewSummary.completedRuntimeEvidence.includes(
    "billing-abuse-boundary-review",
  ),
);
assert.ok(
  managedBillingAbuseBoundaryReviewSummary.closedReadinessBlockers.includes(
    "billing_abuse_boundary_review_missing",
  ),
);
assert.ok(
  managedBillingAbuseBoundaryReviewSummary.closedReadinessBlockers.includes(
    "runtime_rate_limit_enforcement_missing",
  ),
);
assert.ok(
  managedBillingAbuseBoundaryReviewSummary.closedReadinessBlockers.includes(
    "abuse_escalation_runbook_missing",
  ),
);
assert.ok(
  managedBillingAbuseBoundaryReviewSummary.closedReadinessBlockers.includes(
    "tenant_deletion_workflow_missing",
  ),
);
assert.ok(
  managedBillingAbuseBoundaryReviewSummary.boundaryContract.billingUsageFields.includes(
    "relay_byte_count",
  ),
);
assert.ok(
  managedBillingAbuseBoundaryReviewSummary.boundaryContract.abuseSignalFields.includes(
    "rate_limit_denial_count",
  ),
);
assert.ok(
  managedBillingAbuseBoundaryReviewSummary.boundaryContract.prohibitedBillingFields.includes(
    "support_case_id",
  ),
);
assert.ok(
  managedBillingAbuseBoundaryReviewSummary.evidenceChecks.includes(
    "support-evidence-is-not-a-billing-source",
  ),
);
assert.deepEqual(managedBillingAbuseBoundaryReviewSummary.remainingRuntimeEvidence, []);
assert.deepEqual(managedBillingAbuseBoundaryReviewSummary.remainingRuntimeBlockers, []);
assert.equal(managedBillingAbuseBoundaryReviewSummary.implementationCanStart, true);
assert.equal(
  managedBillingAbuseBoundaryReviewSummary.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedRuntimeImplementationPlan =
  relayManagedRuntimeImplementationPlan();
assert.equal(managedRuntimeImplementationPlan.deploymentMode, "managed");
assert.equal(managedRuntimeImplementationPlan.readiness, "plan");
assert.equal(managedRuntimeImplementationPlan.productDefault, "live-loopback");
assert.equal(managedRuntimeImplementationPlan.selectedRuntime, "deferred");
assert.equal(managedRuntimeImplementationPlan.runtimeDefault, "not-selected");
assert.equal(
  managedRuntimeImplementationPlan.implementationStatus,
  "managed-runtime-implementation-plan-ready-runtime-still-deferred",
);
assert.equal(
  managedRuntimeImplementationPlan.implementationBoundary,
  "managed-service-plan-ready-with-pwa-exposure-deferred",
);
assert.equal(
  managedRuntimeImplementationPlan.pwaExposureDecision,
  "deferred-until-runtime-scaffold-and-exposure-gate",
);
assert.ok(
  managedRuntimeImplementationPlan.completedPlanningEvidence.includes(
    "managed-runtime-implementation-plan",
  ),
);
assert.equal(
  managedRuntimeImplementationPlan.readinessGate.gateStatus,
  "runtime-evidence-green",
);
assert.equal(
  managedRuntimeImplementationPlan.readinessGate.implementationCanStart,
  true,
);
assert.deepEqual(
  managedRuntimeImplementationPlan.readinessGate.remainingRuntimeEvidence,
  [],
);
assert.deepEqual(
  managedRuntimeImplementationPlan.readinessGate.remainingRuntimeBlockers,
  [],
);
assert.ok(
  managedRuntimeImplementationPlan.serviceBoundary.allowedManagedVisibleFields.includes(
    "payload_ciphertext_bytes",
  ),
);
assert.ok(
  managedRuntimeImplementationPlan.serviceBoundary.prohibitedManagedVisibleFields.includes(
    "payload_json",
  ),
);
assert.ok(
  managedRuntimeImplementationPlan.serviceBoundary.prohibitedManagedVisibleFields.includes(
    "payload_key_hex",
  ),
);
assert.deepEqual(
  managedRuntimeImplementationPlan.implementationPhases.map(({ phase }) => phase),
  [
    "managed-runtime-service-scaffold",
    "managed-runtime-control-plane-contract-wiring",
    "managed-runtime-encrypted-frame-routing",
    "managed-runtime-quota-and-metering-integration",
    "managed-runtime-support-and-abuse-operations-integration",
    "managed-runtime-pwa-exposure-gate",
  ],
);
assert.ok(
  managedRuntimeImplementationPlan.exposureGates.includes(
    "payload_blind_frame_routing_smoke_passed",
  ),
);
assert.ok(
  managedRuntimeImplementationPlan.exposureGates.includes(
    "live_loopback_rollback_documented",
  ),
);
assert.ok(
  managedRuntimeImplementationPlan.regressionChecks.includes(
    "check:pwa-relay-managed-runtime-implementation-plan",
  ),
);
assert.ok(
  managedRuntimeImplementationPlan.regressionChecks.includes(
    "check:pwa-relay-managed-runtime-service-scaffold",
  ),
);
assert.ok(
  managedRuntimeImplementationPlan.regressionChecks.includes(
    "check:pwa-relay-managed-runtime-control-plane-contract-wiring",
  ),
);
assert.ok(
  managedRuntimeImplementationPlan.guardrails.includes(
    "managed_runtime_not_exposed_until_plan_gate",
  ),
);
assert.equal(managedRuntimeImplementationPlan.implementationCanStart, true);
assert.equal(managedRuntimeImplementationPlan.selectedRuntimeCanChange, false);
assert.equal(
  managedRuntimeImplementationPlan.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedRuntimeServiceScaffold = relayManagedRuntimeServiceScaffold();
assert.equal(managedRuntimeServiceScaffold.deploymentMode, "managed");
assert.equal(managedRuntimeServiceScaffold.readiness, "scaffold");
assert.equal(managedRuntimeServiceScaffold.productDefault, "live-loopback");
assert.equal(managedRuntimeServiceScaffold.selectedRuntime, "deferred");
assert.equal(managedRuntimeServiceScaffold.runtimeDefault, "not-selected");
assert.equal(
  managedRuntimeServiceScaffold.implementationStatus,
  "managed-runtime-service-scaffold-ready-no-pwa-exposure",
);
assert.equal(managedRuntimeServiceScaffold.serviceState, "scaffold-ready");
assert.equal(
  managedRuntimeServiceScaffold.pwaExposureDecision,
  "disabled-until-managed-runtime-exposure-gate",
);
assert.equal(managedRuntimeServiceScaffold.rollbackDefault, "live-loopback");
assert.ok(
  managedRuntimeServiceScaffold.completedImplementationEvidence.includes(
    "managed-runtime-service-scaffold",
  ),
);
assert.equal(
  managedRuntimeServiceScaffold.implementationPlan.implementationCanStart,
  true,
);
assert.equal(managedRuntimeServiceScaffold.startupContract.publicBind, false);
assert.equal(managedRuntimeServiceScaffold.startupContract.endpointMode, "disabled");
assert.equal(managedRuntimeServiceScaffold.startupContract.pwaExposure, "disabled");
assert.equal(
  managedRuntimeServiceScaffold.startupContract.sessionRegistrationHandler,
  "disabled-until-control-plane-contract-wiring",
);
assert.equal(
  managedRuntimeServiceScaffold.serviceScaffold.lifecycle.starts_without_public_listener,
  true,
);
assert.equal(
  managedRuntimeServiceScaffold.serviceScaffold.lifecycle.starts_without_pwa_exposure,
  true,
);
assert.equal(
  managedRuntimeServiceScaffold.serviceScaffold.health_surface.payload_visibility,
  "payload-free",
);
assert.equal(
  managedRuntimeServiceScaffold.serviceScaffold.health_surface.support_visibility,
  "aggregate-only",
);
assert.ok(
  managedRuntimeServiceScaffold.healthSurface.allowedFields.includes(
    "relay_byte_count",
  ),
);
assert.ok(
  managedRuntimeServiceScaffold.healthSurface.prohibitedFields.includes(
    "payload_json",
  ),
);
assert.equal(
  managedRuntimeServiceScaffold.healthSurface.allowedFields.includes(
    "payload_json",
  ),
  false,
);
assert.deepEqual(
  managedRuntimeServiceScaffold.remainingImplementationPhases,
  [
    "managed-runtime-control-plane-contract-wiring",
    "managed-runtime-encrypted-frame-routing",
    "managed-runtime-quota-and-metering-integration",
    "managed-runtime-support-and-abuse-operations-integration",
    "managed-runtime-pwa-exposure-gate",
  ],
);
assert.ok(
  managedRuntimeServiceScaffold.evidenceChecks.includes(
    "managed-service-scaffold-has-no-public-bind",
  ),
);
assert.equal(managedRuntimeServiceScaffold.implementationCanContinue, true);
assert.equal(managedRuntimeServiceScaffold.selectedRuntimeCanChange, false);
assert.equal(
  managedRuntimeServiceScaffold.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedRuntimeServiceScaffoldConfig =
  createManagedRelayRuntimeServiceScaffold({
    serviceId: "managed-relay-runtime-scaffold-test",
    generatedAtMs: 2000,
  });
assert.equal(managedRuntimeServiceScaffoldConfig.endpoint_mode, "disabled");
assert.equal(managedRuntimeServiceScaffoldConfig.public_bind_enabled, false);
assert.equal(managedRuntimeServiceScaffoldConfig.pwa_exposure, "disabled");
assert.throws(
  () =>
    createManagedRelayRuntimeServiceScaffold({
      serviceId: "managed-relay-runtime-scaffold-test",
      generatedAtMs: 2000,
      publicBind: true,
    }),
  /public bind must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeServiceScaffold({
      serviceId: "managed-relay-runtime-scaffold-test",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
const managedRuntimeControlPlaneWiring =
  relayManagedRuntimeControlPlaneContractWiring();
assert.equal(managedRuntimeControlPlaneWiring.deploymentMode, "managed");
assert.equal(managedRuntimeControlPlaneWiring.readiness, "wiring");
assert.equal(managedRuntimeControlPlaneWiring.productDefault, "live-loopback");
assert.equal(managedRuntimeControlPlaneWiring.selectedRuntime, "deferred");
assert.equal(managedRuntimeControlPlaneWiring.runtimeDefault, "not-selected");
assert.equal(
  managedRuntimeControlPlaneWiring.implementationStatus,
  "managed-runtime-control-plane-contract-wired-no-pwa-exposure",
);
assert.equal(
  managedRuntimeControlPlaneWiring.controlPlaneRuntime,
  "tenant-session-registration-contract-wired",
);
assert.equal(managedRuntimeControlPlaneWiring.routeRuntime, "not-wired");
assert.equal(
  managedRuntimeControlPlaneWiring.pwaExposureDecision,
  "disabled-until-managed-runtime-exposure-gate",
);
assert.ok(
  managedRuntimeControlPlaneWiring.completedImplementationEvidence.includes(
    "managed-runtime-control-plane-contract-wiring",
  ),
);
assert.equal(
  managedRuntimeControlPlaneWiring.startupContract.sessionRegistrationHandler,
  "tenant-session-registration-contract-wired",
);
assert.equal(
  managedRuntimeControlPlaneWiring.startupContract.routeFrameHandler,
  "disabled-until-encrypted-frame-routing",
);
assert.equal(managedRuntimeControlPlaneWiring.startupContract.publicBind, false);
assert.equal(managedRuntimeControlPlaneWiring.startupContract.endpointMode, "disabled");
assert.equal(managedRuntimeControlPlaneWiring.startupContract.pwaExposure, "disabled");
assert.ok(
  managedRuntimeControlPlaneWiring.controlPlaneContract.wiredContracts.includes(
    "session-registration",
  ),
);
assert.ok(
  managedRuntimeControlPlaneWiring.controlPlaneContract.wiredContracts.includes(
    "public-verifier-key-lookup",
  ),
);
assert.ok(
  managedRuntimeControlPlaneWiring.controlPlaneContract.wiredContracts.includes(
    "quota-preflight",
  ),
);
assert.ok(
  managedRuntimeControlPlaneWiring.controlPlaneContract.wiredContracts.includes(
    "audit-event",
  ),
);
assert.equal(
  managedRuntimeControlPlaneWiring.controlPlaneContract.sessionRegistration
    .disabled_public_endpoint,
  true,
);
assert.equal(
  managedRuntimeControlPlaneWiring.controlPlaneContract.sessionRegistration
    .route_frame_handler,
  "disabled-until-encrypted-frame-routing",
);
assert.equal(
  managedRuntimeControlPlaneWiring.controlPlaneContract.publicVerifierKeyLookup
    .private_signing_material_allowed,
  false,
);
assert.equal(
  managedRuntimeControlPlaneWiring.controlPlaneContract.publicVerifierKeyLookup
    .hmac_material_allowed,
  false,
);
assert.equal(
  managedRuntimeControlPlaneWiring.controlPlaneContract.quotaPreflight
    .decision_point,
  "before-session-registration",
);
assert.equal(
  managedRuntimeControlPlaneWiring.controlPlaneContract.audit.payload_visibility,
  "payload-free",
);
assert.equal(
  managedRuntimeControlPlaneWiring.healthSurface.allowedFields.includes(
    "payload_json",
  ),
  false,
);
assert.ok(
  managedRuntimeControlPlaneWiring.healthSurface.prohibitedFields.includes(
    "payload_json",
  ),
);
assert.equal(
  JSON.stringify(managedRuntimeControlPlaneWiring.controlPlaneWiring.control_plane_health).includes(
    "payload_json",
  ),
  false,
);
assert.deepEqual(
  managedRuntimeControlPlaneWiring.remainingImplementationPhases,
  [
    "managed-runtime-encrypted-frame-routing",
    "managed-runtime-quota-and-metering-integration",
    "managed-runtime-support-and-abuse-operations-integration",
    "managed-runtime-pwa-exposure-gate",
  ],
);
assert.ok(
  managedRuntimeControlPlaneWiring.evidenceChecks.includes(
    "next-encrypted-frame-routing-slice-selected",
  ),
);
assert.equal(managedRuntimeControlPlaneWiring.implementationCanContinue, true);
assert.equal(managedRuntimeControlPlaneWiring.selectedRuntimeCanChange, false);
assert.equal(
  managedRuntimeControlPlaneWiring.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedRuntimeControlPlaneWiringConfig =
  createManagedRelayRuntimeControlPlaneContractWiring({
    serviceId: "managed-relay-runtime-control-plane-test",
    generatedAtMs: 2000,
  });
assert.equal(
  managedRuntimeControlPlaneWiringConfig.control_plane_runtime,
  "tenant-session-registration-contract-wired",
);
assert.equal(managedRuntimeControlPlaneWiringConfig.route_runtime, "not-wired");
assert.equal(managedRuntimeControlPlaneWiringConfig.endpoint_mode, "disabled");
assert.equal(managedRuntimeControlPlaneWiringConfig.public_bind_enabled, false);
assert.equal(managedRuntimeControlPlaneWiringConfig.pwa_exposure, "disabled");
assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-test",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-test",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-test",
      generatedAtMs: 2000,
      routeRuntime: "wired",
    }),
  /route runtime must stay not-wired/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-test",
      generatedAtMs: 2000,
      selectedRuntime: "managed",
    }),
  /selected runtime must stay deferred/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-test",
      generatedAtMs: 2000,
      productDefault: "relay",
    }),
  /product default must stay live-loopback/,
);
const managedRuntimeEncryptedFrameRouting =
  relayManagedRuntimeEncryptedFrameRouting();
assert.equal(managedRuntimeEncryptedFrameRouting.deploymentMode, "managed");
assert.equal(managedRuntimeEncryptedFrameRouting.readiness, "routing");
assert.equal(managedRuntimeEncryptedFrameRouting.productDefault, "live-loopback");
assert.equal(managedRuntimeEncryptedFrameRouting.selectedRuntime, "deferred");
assert.equal(managedRuntimeEncryptedFrameRouting.runtimeDefault, "not-selected");
assert.equal(
  managedRuntimeEncryptedFrameRouting.implementationStatus,
  "managed-runtime-encrypted-frame-routing-wired-no-pwa-exposure",
);
assert.equal(
  managedRuntimeEncryptedFrameRouting.controlPlaneRuntime,
  "tenant-session-registration-contract-wired",
);
assert.equal(
  managedRuntimeEncryptedFrameRouting.routeRuntime,
  "encrypted-frame-routing-wired",
);
assert.equal(
  managedRuntimeEncryptedFrameRouting.pwaExposureDecision,
  "disabled-until-managed-runtime-exposure-gate",
);
assert.ok(
  managedRuntimeEncryptedFrameRouting.completedImplementationEvidence.includes(
    "managed-runtime-encrypted-frame-routing",
  ),
);
assert.equal(
  managedRuntimeEncryptedFrameRouting.startupContract.routeFrameHandler,
  "encrypted-frame-routing-wired",
);
assert.equal(
  managedRuntimeEncryptedFrameRouting.startupContract.sessionRegistrationHandler,
  "tenant-session-registration-contract-wired",
);
assert.equal(managedRuntimeEncryptedFrameRouting.startupContract.publicBind, false);
assert.equal(managedRuntimeEncryptedFrameRouting.startupContract.endpointMode, "disabled");
assert.equal(managedRuntimeEncryptedFrameRouting.startupContract.pwaExposure, "disabled");
assert.ok(
  managedRuntimeEncryptedFrameRouting.routeContract.routeVisibleFields.includes(
    "payload_ciphertext_bytes",
  ),
);
assert.equal(
  managedRuntimeEncryptedFrameRouting.routeContract.routeVisibleFields.includes(
    "payload_ciphertext_hex",
  ),
  false,
);
assert.ok(
  managedRuntimeEncryptedFrameRouting.routeContract.prohibitedRouteVisibleFields.includes(
    "payload_ciphertext_hex",
  ),
);
assert.equal(
  managedRuntimeEncryptedFrameRouting.routeContract.deliveryBoundary
    .plaintext_payload_visible,
  false,
);
assert.equal(
  managedRuntimeEncryptedFrameRouting.routeContract.deliveryBoundary
    .operator_visible_ciphertext,
  false,
);
assert.deepEqual(
  managedRuntimeEncryptedFrameRouting.remainingImplementationPhases,
  [
    "managed-runtime-quota-and-metering-integration",
    "managed-runtime-support-and-abuse-operations-integration",
    "managed-runtime-pwa-exposure-gate",
  ],
);
assert.ok(
  managedRuntimeEncryptedFrameRouting.evidenceChecks.includes(
    "next-quota-and-metering-integration-slice-selected",
  ),
);
assert.equal(managedRuntimeEncryptedFrameRouting.implementationCanContinue, true);
assert.equal(managedRuntimeEncryptedFrameRouting.selectedRuntimeCanChange, false);
assert.equal(
  managedRuntimeEncryptedFrameRouting.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedRuntimeEncryptedFrameRoutingConfig =
  createManagedRelayRuntimeEncryptedFrameRouting({
    serviceId: "managed-relay-runtime-encrypted-routing-test",
    generatedAtMs: 2000,
  });
assert.equal(
  managedRuntimeEncryptedFrameRoutingConfig.route_runtime,
  "encrypted-frame-routing-wired",
);
assert.equal(
  managedRuntimeEncryptedFrameRoutingConfig.control_plane_runtime,
  "tenant-session-registration-contract-wired",
);
assert.equal(managedRuntimeEncryptedFrameRoutingConfig.endpoint_mode, "disabled");
assert.equal(managedRuntimeEncryptedFrameRoutingConfig.public_bind_enabled, false);
assert.equal(managedRuntimeEncryptedFrameRoutingConfig.pwa_exposure, "disabled");
const managedRuntimeRouteFrame = await managedRelayEncryptedFrameFromLiveMessage(
  "managed-runtime-route-session",
  "daemon",
  1,
  3000,
  33000,
  liveApprovalRequestMessage({
    approval_id: [114, 111, 117, 116, 101],
    nonce: Array.from({ length: 32 }, (_, i) => i),
    command_masked: "deploy production --tenant alpha",
    context_hash: "ctx-route-alpha",
    expires_at: 33000,
    device_epoch: 2,
  }),
  "44".repeat(32),
  { nonceHex: "45".repeat(12), webCrypto: webcrypto },
);
const managedRuntimeRoute =
  routeManagedRelayRuntimeEncryptedFrame(managedRuntimeRouteFrame, {
    nowMs: 3100,
  });
assert.equal(managedRuntimeRoute.route_decision, "accepted");
assert.equal(managedRuntimeRoute.route_state, "encrypted-frame-routed");
assert.deepEqual(
  managedRuntimeRoute.route_envelope,
  managedRelayEncryptedFrameRouteEnvelope(managedRuntimeRouteFrame),
);
assert.deepEqual(
  managedRuntimeRoute.route_visible_fields,
  [
    "relay_protocol_version",
    "session_id",
    "sender",
    "sequence",
    "sent_at_ms",
    "expires_at_ms",
    "payload_ciphertext_alg",
    "payload_key_scope",
    "payload_ciphertext_bytes",
  ],
);
assert.equal(
  JSON.stringify(managedRuntimeRoute).includes("payload_ciphertext_hex"),
  false,
);
assert.equal(
  JSON.stringify(managedRuntimeRoute).includes("payload_nonce_hex"),
  false,
);
assert.equal(
  JSON.stringify(managedRuntimeRoute).includes("deploy production"),
  false,
);
assert.equal(JSON.stringify(managedRuntimeRoute).includes("ctx-route-alpha"), false);
assert.equal(managedRuntimeRoute.route_delivery.plaintext_payload_visible, false);
assert.equal(managedRuntimeRoute.route_delivery.operator_visible_ciphertext, false);
assert.equal(
  managedRuntimeRoute.audit_event.payload_ciphertext_bytes,
  managedRuntimeRoute.route_envelope.payload_ciphertext_bytes,
);
assert.throws(
  () =>
    routeManagedRelayRuntimeEncryptedFrame(managedRuntimeRouteFrame, {
      nowMs: 33000,
    }),
  /expired before route/,
);
assert.throws(
  () =>
    routeManagedRelayRuntimeEncryptedFrame({
      ...managedRuntimeRouteFrame,
      payload_json: "{}",
    }),
  /plaintext field not allowed: payload_json/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-test",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-test",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-test",
      generatedAtMs: 2000,
      routeRuntime: "not-wired",
    }),
  /route runtime must wire encrypted frame routing/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-test",
      generatedAtMs: 2000,
      controlPlaneRuntime: "not-wired",
    }),
  /requires wired control plane/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-test",
      generatedAtMs: 2000,
      selectedRuntime: "managed",
    }),
  /selected runtime must stay deferred/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-test",
      generatedAtMs: 2000,
      productDefault: "relay",
    }),
  /product default must stay live-loopback/,
);
const managedRuntimeQuotaAndMeteringIntegration =
  relayManagedRuntimeQuotaAndMeteringIntegration();
assert.equal(managedRuntimeQuotaAndMeteringIntegration.deploymentMode, "managed");
assert.equal(managedRuntimeQuotaAndMeteringIntegration.readiness, "integration");
assert.equal(managedRuntimeQuotaAndMeteringIntegration.productDefault, "live-loopback");
assert.equal(managedRuntimeQuotaAndMeteringIntegration.selectedRuntime, "deferred");
assert.equal(managedRuntimeQuotaAndMeteringIntegration.runtimeDefault, "not-selected");
assert.equal(
  managedRuntimeQuotaAndMeteringIntegration.implementationStatus,
  "managed-runtime-quota-and-metering-integrated-no-pwa-exposure",
);
assert.equal(
  managedRuntimeQuotaAndMeteringIntegration.controlPlaneRuntime,
  "tenant-session-registration-contract-wired",
);
assert.equal(
  managedRuntimeQuotaAndMeteringIntegration.routeRuntime,
  "encrypted-frame-routing-wired",
);
assert.equal(
  managedRuntimeQuotaAndMeteringIntegration.quotaRuntime,
  "active-session-frame-byte-metering-wired",
);
assert.equal(
  managedRuntimeQuotaAndMeteringIntegration.pwaExposureDecision,
  "disabled-until-managed-runtime-exposure-gate",
);
assert.ok(
  managedRuntimeQuotaAndMeteringIntegration.completedImplementationEvidence.includes(
    "managed-runtime-quota-and-metering-integration",
  ),
);
assert.equal(
  managedRuntimeQuotaAndMeteringIntegration.startupContract.quotaMeteringHandler,
  "active-session-frame-byte-metering-wired",
);
assert.equal(managedRuntimeQuotaAndMeteringIntegration.startupContract.publicBind, false);
assert.equal(managedRuntimeQuotaAndMeteringIntegration.startupContract.endpointMode, "disabled");
assert.equal(managedRuntimeQuotaAndMeteringIntegration.startupContract.pwaExposure, "disabled");
assert.equal(
  managedRuntimeQuotaAndMeteringIntegration.quotaMeteringContract.decisionPoint,
  "before-encrypted-frame-delivery",
);
assert.ok(
  managedRuntimeQuotaAndMeteringIntegration.quotaMeteringContract.meteredUsageDimensions.includes(
    "relay_byte_count",
  ),
);
assert.ok(
  managedRuntimeQuotaAndMeteringIntegration.quotaMeteringContract.prohibitedMeteringFields.includes(
    "payload_ciphertext_hex",
  ),
);
assert.deepEqual(
  managedRuntimeQuotaAndMeteringIntegration.remainingImplementationPhases,
  [
    "managed-runtime-support-and-abuse-operations-integration",
    "managed-runtime-pwa-exposure-gate",
  ],
);
assert.ok(
  managedRuntimeQuotaAndMeteringIntegration.evidenceChecks.includes(
    "support-and-abuse-operations-integration-complete",
  ),
);
assert.equal(managedRuntimeQuotaAndMeteringIntegration.implementationCanContinue, true);
assert.equal(managedRuntimeQuotaAndMeteringIntegration.selectedRuntimeCanChange, false);
assert.equal(
  managedRuntimeQuotaAndMeteringIntegration.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedRuntimeQuotaAndMeteringConfig =
  createManagedRelayRuntimeQuotaAndMeteringIntegration({
    serviceId: "managed-relay-runtime-quota-metering-test",
    generatedAtMs: 2000,
  });
assert.equal(
  managedRuntimeQuotaAndMeteringConfig.quota_runtime,
  "active-session-frame-byte-metering-wired",
);
assert.equal(managedRuntimeQuotaAndMeteringConfig.endpoint_mode, "disabled");
assert.equal(managedRuntimeQuotaAndMeteringConfig.public_bind_enabled, false);
assert.equal(managedRuntimeQuotaAndMeteringConfig.pwa_exposure, "disabled");
const managedRuntimeMeteringQuotaState = createManagedRelayActiveSessionAndByteQuotaState({
  tenantId: "tenant-runtime-metering",
  daemonDeviceId: generatedKeys.identity.deviceId,
  windowStartMs: 3000,
  windowEndMs: 40000,
  tenantActiveSessionLimit: 3,
  tenantActiveSessions: 2,
  daemonDeviceActiveSessionLimit: 2,
  daemonDeviceActiveSessions: 1,
  relayFrameLimit: 6,
  relayFramesUsed: 4,
  relayByteLimit: 4096,
  relayBytesUsed: 256,
});
const managedRuntimeMeteredRoute = routeManagedRelayRuntimeQuotaMeteredFrame(
  managedRuntimeRouteFrame,
  managedRuntimeMeteringQuotaState,
  {
    tenantId: "tenant-runtime-metering",
    daemonDeviceId: generatedKeys.identity.deviceId,
    verifierKeyId: "managed-meter-key-1",
    verifierKeyVersion: 3,
  },
  { nowMs: 3200 },
);
assert.equal(managedRuntimeMeteredRoute.route_decision, "accepted");
assert.equal(
  managedRuntimeMeteredRoute.route_state,
  "quota-metered-encrypted-frame-routed",
);
assert.equal(
  managedRuntimeMeteredRoute.route_delivery.frame_delivery,
  "encrypted-frame-forwarded-after-quota",
);
assert.equal(managedRuntimeMeteredRoute.quota_decision.decision, "accept");
assert.equal(managedRuntimeMeteredRoute.quota_decision.relay_allowed, true);
assert.equal(
  managedRuntimeMeteredRoute.quota_decision.billing_meter_delta.relay_frame_count,
  1,
);
assert.equal(
  managedRuntimeMeteredRoute.quota_decision.billing_meter_delta.relay_byte_count,
  managedRuntimeMeteredRoute.route_envelope.payload_ciphertext_bytes,
);
assert.equal(
  managedRuntimeMeteredRoute.quota_decision.billing_meter_delta.quota_denial_count,
  0,
);
assert.equal(
  managedRuntimeMeteredRoute.quota_decision.abuse_signal_delta.rate_limit_denial_count,
  0,
);
assert.equal(
  JSON.stringify(managedRuntimeMeteredRoute).includes("payload_ciphertext_hex"),
  false,
);
assert.equal(
  JSON.stringify(managedRuntimeMeteredRoute).includes("payload_nonce_hex"),
  false,
);
assert.equal(
  JSON.stringify(managedRuntimeMeteredRoute).includes("deploy production"),
  false,
);
const managedRuntimeQuotaRejectedRoute = routeManagedRelayRuntimeQuotaMeteredFrame(
  managedRuntimeRouteFrame,
  createManagedRelayActiveSessionAndByteQuotaState({
    tenantId: "tenant-runtime-metering",
    daemonDeviceId: generatedKeys.identity.deviceId,
    windowStartMs: 3000,
    windowEndMs: 40000,
    tenantActiveSessionLimit: 3,
    tenantActiveSessions: 2,
    daemonDeviceActiveSessionLimit: 2,
    daemonDeviceActiveSessions: 1,
    relayFrameLimit: 6,
    relayFramesUsed: 4,
    relayByteLimit: 300,
    relayBytesUsed: 256,
  }),
  {
    tenantId: "tenant-runtime-metering",
    daemonDeviceId: generatedKeys.identity.deviceId,
    verifierKeyId: "managed-meter-key-1",
    verifierKeyVersion: 3,
  },
  { nowMs: 3200 },
);
assert.equal(managedRuntimeQuotaRejectedRoute.route_decision, "rejected");
assert.equal(
  managedRuntimeQuotaRejectedRoute.route_state,
  "quota-rejected-before-frame-delivery",
);
assert.equal(
  managedRuntimeQuotaRejectedRoute.route_delivery.frame_delivery,
  "not-delivered-quota-fail-closed",
);
assert.equal(managedRuntimeQuotaRejectedRoute.quota_decision.decision, "reject");
assert.equal(managedRuntimeQuotaRejectedRoute.quota_decision.relay_allowed, false);
assert.equal(managedRuntimeQuotaRejectedRoute.quota_decision.reason, "relay-byte-quota-exceeded");
assert.equal(
  managedRuntimeQuotaRejectedRoute.quota_decision.billing_meter_delta.relay_frame_count,
  0,
);
assert.equal(
  managedRuntimeQuotaRejectedRoute.quota_decision.billing_meter_delta.relay_byte_count,
  0,
);
assert.equal(
  managedRuntimeQuotaRejectedRoute.quota_decision.billing_meter_delta.quota_denial_count,
  1,
);
assert.equal(
  managedRuntimeQuotaRejectedRoute.quota_decision.abuse_signal_delta.rate_limit_denial_count,
  0,
);
assert.throws(
  () =>
    routeManagedRelayRuntimeQuotaMeteredFrame(
      managedRuntimeRouteFrame,
      managedRuntimeMeteringQuotaState,
      {
        tenantId: "tenant-runtime-metering",
        daemonDeviceId: generatedKeys.identity.deviceId,
        verifierKeyId: "managed-meter-key-1",
        verifierKeyVersion: 3,
      },
      { nowMs: 33000 },
    ),
  /expired before route/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeQuotaAndMeteringIntegration({
      serviceId: "managed-relay-runtime-quota-metering-test",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeQuotaAndMeteringIntegration({
      serviceId: "managed-relay-runtime-quota-metering-test",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeQuotaAndMeteringIntegration({
      serviceId: "managed-relay-runtime-quota-metering-test",
      generatedAtMs: 2000,
      routeRuntime: "not-wired",
    }),
  /requires encrypted frame routing/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeQuotaAndMeteringIntegration({
      serviceId: "managed-relay-runtime-quota-metering-test",
      generatedAtMs: 2000,
      quotaRuntime: "not-wired",
    }),
  /must wire active session frame byte metering/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeQuotaAndMeteringIntegration({
      serviceId: "managed-relay-runtime-quota-metering-test",
      generatedAtMs: 2000,
      selectedRuntime: "managed",
    }),
  /selected runtime must stay deferred/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeQuotaAndMeteringIntegration({
      serviceId: "managed-relay-runtime-quota-metering-test",
      generatedAtMs: 2000,
      productDefault: "relay",
    }),
  /product default must stay live-loopback/,
);
const managedRuntimeSupportAbuseIntegration =
  relayManagedRuntimeSupportAndAbuseOperationsIntegration();
assert.equal(managedRuntimeSupportAbuseIntegration.deploymentMode, "managed");
assert.equal(managedRuntimeSupportAbuseIntegration.readiness, "integration");
assert.equal(managedRuntimeSupportAbuseIntegration.productDefault, "live-loopback");
assert.equal(managedRuntimeSupportAbuseIntegration.selectedRuntime, "deferred");
assert.equal(managedRuntimeSupportAbuseIntegration.runtimeDefault, "not-selected");
assert.equal(
  managedRuntimeSupportAbuseIntegration.implementationStatus,
  "managed-runtime-support-and-abuse-operations-integrated-no-pwa-exposure",
);
assert.equal(
  managedRuntimeSupportAbuseIntegration.supportRuntime,
  "support-redaction-access-review-wired",
);
assert.equal(
  managedRuntimeSupportAbuseIntegration.abuseRuntime,
  "billing-abuse-boundary-review-wired",
);
assert.equal(
  managedRuntimeSupportAbuseIntegration.startupContract.supportOperationsHandler,
  "support-redaction-access-review-wired",
);
assert.equal(
  managedRuntimeSupportAbuseIntegration.startupContract.abuseOperationsHandler,
  "billing-abuse-boundary-review-wired",
);
assert.equal(managedRuntimeSupportAbuseIntegration.startupContract.publicBind, false);
assert.equal(managedRuntimeSupportAbuseIntegration.startupContract.endpointMode, "disabled");
assert.equal(managedRuntimeSupportAbuseIntegration.startupContract.pwaExposure, "disabled");
assert.equal(
  managedRuntimeSupportAbuseIntegration.supportOperationsContract.runtimeVisibility,
  "aggregate-only",
);
assert.equal(
  managedRuntimeSupportAbuseIntegration.supportOperationsContract.identifierPolicy,
  "hashed-identifiers-only",
);
assert.ok(
  managedRuntimeSupportAbuseIntegration.supportOperationsContract.supportOperationFields.includes(
    "support_actor_id_hash",
  ),
);
assert.equal(
  managedRuntimeSupportAbuseIntegration.supportOperationsContract.supportOperationFields.includes(
    "support_actor_id",
  ),
  false,
);
assert.equal(
  managedRuntimeSupportAbuseIntegration.abuseOperationsContract.sourceDataPolicy,
  "billing-meter-deltas-not-reclassified-as-abuse-source-data",
);
assert.ok(
  managedRuntimeSupportAbuseIntegration.abuseOperationsContract.abuseOperationFields.includes(
    "rate_limit_denial_count",
  ),
);
assert.equal(
  managedRuntimeSupportAbuseIntegration.healthSurface.allowedFields.includes(
    "payload_json",
  ),
  false,
);
assert.ok(
  managedRuntimeSupportAbuseIntegration.healthSurface.prohibitedFields.includes(
    "payload_json",
  ),
);
assert.deepEqual(managedRuntimeSupportAbuseIntegration.remainingImplementationPhases, [
  "managed-runtime-pwa-exposure-gate",
]);
assert.ok(
  managedRuntimeSupportAbuseIntegration.evidenceChecks.includes(
    "next-pwa-exposure-gate-slice-selected",
  ),
);
assert.equal(managedRuntimeSupportAbuseIntegration.implementationCanContinue, true);
assert.equal(managedRuntimeSupportAbuseIntegration.selectedRuntimeCanChange, false);
assert.equal(
  managedRuntimeSupportAbuseIntegration.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
const managedRuntimeSupportAbuseConfig =
  createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
    serviceId: "managed-relay-runtime-support-abuse-test",
    generatedAtMs: 2000,
  });
assert.equal(
  managedRuntimeSupportAbuseConfig.support_runtime,
  "support-redaction-access-review-wired",
);
assert.equal(
  managedRuntimeSupportAbuseConfig.abuse_runtime,
  "billing-abuse-boundary-review-wired",
);
assert.equal(managedRuntimeSupportAbuseConfig.endpoint_mode, "disabled");
assert.equal(managedRuntimeSupportAbuseConfig.public_bind_enabled, false);
assert.equal(managedRuntimeSupportAbuseConfig.pwa_exposure, "disabled");
assert.equal(
  managedRuntimeSupportAbuseConfig.support_operations_contract.payload_visibility,
  "payload-free",
);
assert.equal(
  managedRuntimeSupportAbuseConfig.abuse_operations_contract.payload_visibility,
  "payload-free",
);
const managedRuntimeSupportAbuseVisibleJson = JSON.stringify({
  supportOperationsContract: {
    ...managedRuntimeSupportAbuseConfig.support_operations_contract,
    prohibited_operation_fields: undefined,
  },
  abuseOperationsContract: {
    ...managedRuntimeSupportAbuseConfig.abuse_operations_contract,
    prohibited_operation_fields: undefined,
  },
  operationsHealth: managedRuntimeSupportAbuseConfig.operations_health,
  allowedOperationsFields: managedRuntimeSupportAbuseConfig.allowed_operations_fields,
});
assert.equal(
  managedRuntimeSupportAbuseVisibleJson.includes("payload_ciphertext_hex"),
  false,
);
assert.equal(
  managedRuntimeSupportAbuseVisibleJson.includes('"support_actor_id"'),
  false,
);
assert.throws(
  () =>
    createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
      serviceId: "managed-relay-runtime-support-abuse-test",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
      serviceId: "managed-relay-runtime-support-abuse-test",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
      serviceId: "managed-relay-runtime-support-abuse-test",
      generatedAtMs: 2000,
      supportRuntime: "not-wired",
    }),
  /must wire support access review/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
      serviceId: "managed-relay-runtime-support-abuse-test",
      generatedAtMs: 2000,
      abuseRuntime: "not-wired",
    }),
  /must wire billing abuse boundary review/,
);
const managedRuntimePwaExposureGate = relayManagedRuntimePwaExposureGate();
assert.equal(managedRuntimePwaExposureGate.deploymentMode, "managed");
assert.equal(managedRuntimePwaExposureGate.readiness, "exposure-gate");
assert.equal(managedRuntimePwaExposureGate.productDefault, "live-loopback");
assert.equal(
  managedRuntimePwaExposureGate.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(managedRuntimePwaExposureGate.runtimeDefault, "not-selected");
assert.equal(
  managedRuntimePwaExposureGate.implementationStatus,
  "managed-runtime-pwa-exposure-gate-passed-explicit-opt-in-only",
);
assert.equal(
  managedRuntimePwaExposureGate.pwaExposureDecision,
  "enabled-for-explicit-opt-in-setup-copy-only",
);
assert.equal(managedRuntimePwaExposureGate.pwaExposure, "explicit-opt-in");
assert.equal(managedRuntimePwaExposureGate.endpointMode, "operator-setup-required");
assert.equal(managedRuntimePwaExposureGate.endpointAutoStart, false);
assert.equal(managedRuntimePwaExposureGate.publicBind, false);
assert.equal(managedRuntimePwaExposureGate.rollbackDefault, "live-loopback");
assert.equal(
  managedRuntimePwaExposureGate.supportAbuseIntegration.implementationCanContinue,
  true,
);
assert.equal(
  managedRuntimePwaExposureGate.supportAbuseIntegration.startupContract.pwaExposure,
  "disabled",
);
assert.equal(managedRuntimePwaExposureGate.startupContract.publicBind, false);
assert.equal(
  managedRuntimePwaExposureGate.startupContract.endpointMode,
  "operator-setup-required",
);
assert.equal(managedRuntimePwaExposureGate.startupContract.endpointAutoStart, false);
assert.equal(
  managedRuntimePwaExposureGate.startupContract.pwaExposure,
  "explicit-opt-in",
);
assert.equal(managedRuntimePwaExposureGate.pwaSurface.visible, true);
assert.equal(managedRuntimePwaExposureGate.pwaSurface.mode, "explicit-opt-in");
assert.equal(
  managedRuntimePwaExposureGate.pwaSurface.copy.state_text,
  "Explicit opt-in ready",
);
assert.equal(
  managedRuntimePwaExposureGate.pwaSurface.copy.default_text,
  "Product default remains live-loopback",
);
assert.ok(
  managedRuntimePwaExposureGate.pwaSurface.visibleFields.includes(
    "pwa_exposure",
  ),
);
assert.ok(
  managedRuntimePwaExposureGate.pwaSurface.visibleFields.includes(
    "endpoint_auto_start",
  ),
);
assert.ok(
  managedRuntimePwaExposureGate.pwaSurface.prohibitedFields.includes(
    "signed_session_ticket",
  ),
);
assert.equal(
  managedRuntimePwaExposureGate.pwaSurface.visibleFields.includes(
    "signed_session_ticket",
  ),
  false,
);
assert.equal(managedRuntimePwaExposureGate.healthSurface.endpointAutoStart, false);
assert.equal(managedRuntimePwaExposureGate.healthSurface.publicBind, false);
assert.deepEqual(managedRuntimePwaExposureGate.remainingImplementationPhases, []);
assert.equal(managedRuntimePwaExposureGate.implementationCanContinue, true);
assert.equal(managedRuntimePwaExposureGate.selectedRuntimeCanChange, true);
assert.equal(
  managedRuntimePwaExposureGate.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(managedRuntimePwaExposureGate.productDefaultCanChange, false);
assert.equal(
  managedRuntimePwaExposureGate.nextLocalSlice,
  "managed-relay-runtime-browser-operator-evidence",
);
assert.ok(
  managedRuntimePwaExposureGate.completedImplementationEvidence.includes(
    "managed-runtime-pwa-exposure-gate",
  ),
);
for (const evidenceCheck of [
  "support-and-abuse-operations-integration-complete",
  "pwa-copy-and-setup-text-updated",
  "managed-runtime-exposure-gate-passed",
  "managed-runtime-remains-explicit-opt-in-only",
  "managed-runtime-endpoint-auto-start-disabled",
  "managed-runtime-public-bind-disabled",
  "pwa-surface-excludes-payloads-secrets-and-raw-identifiers",
  "live-loopback-rollback-documented",
  "next-browser-operator-evidence-slice-selected",
]) {
  assert.ok(
    managedRuntimePwaExposureGate.evidenceChecks.includes(evidenceCheck),
    `managed runtime pwa exposure gate missing evidence: ${evidenceCheck}`,
  );
}
const managedRuntimePwaExposureConfig = createManagedRelayRuntimePwaExposureGate({
  serviceId: "managed-relay-runtime-pwa-exposure-test",
  generatedAtMs: 2000,
});
assert.equal(managedRuntimePwaExposureConfig.exposure_gate_version, 1);
assert.equal(managedRuntimePwaExposureConfig.deployment_mode, "managed");
assert.equal(managedRuntimePwaExposureConfig.readiness, "pwa-exposure-gate");
assert.equal(
  managedRuntimePwaExposureConfig.selected_runtime,
  "explicit-opt-in-managed",
);
assert.equal(managedRuntimePwaExposureConfig.runtime_default, "not-selected");
assert.equal(managedRuntimePwaExposureConfig.pwa_exposure, "explicit-opt-in");
assert.equal(
  managedRuntimePwaExposureConfig.endpoint_mode,
  "operator-setup-required",
);
assert.equal(managedRuntimePwaExposureConfig.endpoint_auto_start, false);
assert.equal(managedRuntimePwaExposureConfig.public_bind_enabled, false);
assert.equal(managedRuntimePwaExposureConfig.pwa_surface.visible, true);
assert.equal(
  managedRuntimePwaExposureConfig.setup_contract.setup_visibility,
  "copy-and-status-only",
);
assert.equal(
  managedRuntimePwaExposureConfig.setup_contract.manual_connect_required,
  true,
);
assert.equal(
  managedRuntimePwaExposureConfig.rollback_contract.rollback_transport,
  "live-loopback",
);
const managedRuntimePwaVisibleJson = JSON.stringify({
  pwaSurface: {
    ...managedRuntimePwaExposureConfig.pwa_surface,
    prohibited_fields: undefined,
  },
  setupContract: managedRuntimePwaExposureConfig.setup_contract,
  rollbackContract: managedRuntimePwaExposureConfig.rollback_contract,
  exposureHealth: managedRuntimePwaExposureConfig.exposure_health,
  allowedPwaFields: managedRuntimePwaExposureConfig.allowed_pwa_fields,
});
for (const prohibited of [
  "payload_ciphertext_hex",
  "payload_nonce_hex",
  "payload_key_hex",
  "signed_session_ticket",
  "session_token",
  '"support_actor_id"',
  '"session_id"',
  '"daemon_device_id"',
  '"companion_device_id"',
]) {
  assert.equal(
    managedRuntimePwaVisibleJson.includes(prohibited),
    false,
    `pwa exposure visible surface leaked ${prohibited}`,
  );
}
assert.throws(
  () =>
    createManagedRelayRuntimePwaExposureGate({
      serviceId: "managed-relay-runtime-pwa-exposure-test",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must require operator setup/,
);
assert.throws(
  () =>
    createManagedRelayRuntimePwaExposureGate({
      serviceId: "managed-relay-runtime-pwa-exposure-test",
      generatedAtMs: 2000,
      endpointAutoStart: true,
    }),
  /endpoint auto start must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimePwaExposureGate({
      serviceId: "managed-relay-runtime-pwa-exposure-test",
      generatedAtMs: 2000,
      publicBind: true,
    }),
  /public bind must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimePwaExposureGate({
      serviceId: "managed-relay-runtime-pwa-exposure-test",
      generatedAtMs: 2000,
      pwaExposure: "disabled",
    }),
  /must stay explicit opt-in/,
);
assert.throws(
  () =>
    createManagedRelayRuntimePwaExposureGate({
      serviceId: "managed-relay-runtime-pwa-exposure-test",
      generatedAtMs: 2000,
      selectedRuntime: "managed",
    }),
  /selected runtime must stay explicit opt-in/,
);
const managedRuntimeBrowserOperatorEvidence =
  relayManagedRuntimeBrowserOperatorEvidence();
assert.equal(managedRuntimeBrowserOperatorEvidence.deploymentMode, "managed");
assert.equal(
  managedRuntimeBrowserOperatorEvidence.readiness,
  "browser-operator-evidence",
);
assert.equal(managedRuntimeBrowserOperatorEvidence.productDefault, "live-loopback");
assert.equal(
  managedRuntimeBrowserOperatorEvidence.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(managedRuntimeBrowserOperatorEvidence.runtimeDefault, "not-selected");
assert.equal(
  managedRuntimeBrowserOperatorEvidence.implementationStatus,
  "managed-runtime-browser-operator-evidence-captured-explicit-opt-in-only",
);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.pwaExposureDecision,
  "visible-and-verified-explicit-opt-in-setup-copy-only",
);
assert.equal(managedRuntimeBrowserOperatorEvidence.pwaExposure, "explicit-opt-in");
assert.equal(
  managedRuntimeBrowserOperatorEvidence.endpointMode,
  "operator-setup-required",
);
assert.equal(managedRuntimeBrowserOperatorEvidence.endpointAutoStart, false);
assert.equal(managedRuntimeBrowserOperatorEvidence.publicBind, false);
assert.equal(managedRuntimeBrowserOperatorEvidence.rollbackDefault, "live-loopback");
assert.equal(
  managedRuntimeBrowserOperatorEvidence.exposureGate.readiness,
  "exposure-gate",
);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.exposureGate.nextLocalSlice,
  "managed-relay-runtime-browser-operator-evidence",
);
for (const selector of [
  "#relay-managed-title",
  "#relay-managed-state",
  "#relay-managed-default-mode",
  "#relay-managed-exposure",
  "#relay-managed-endpoint-mode",
  "#relay-managed-public-bind",
  "#relay-managed-auto-start",
  "#relay-managed-rollback",
  "#relay-managed-next",
  "#relay-managed-evidence-list",
  "#relay-managed-copy",
]) {
  assert.ok(
    managedRuntimeBrowserOperatorEvidence.requiredSelectors.includes(selector),
    `browser operator evidence missing selector ${selector}`,
  );
}
assert.deepEqual(
  managedRuntimeBrowserOperatorEvidence.browserEvidence.requiredScreenshots,
  [
    "managed-relay-browser-operator-evidence.png",
    "managed-relay-browser-operator-evidence-mobile.png",
  ],
);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.browserEvidence.expectedVisibleText.state,
  "Explicit opt-in ready",
);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.browserEvidence.expectedVisibleText
    .productDefault,
  "live-loopback",
);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.browserEvidence.expectedVisibleText
    .endpointMode,
  "operator-setup-required",
);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.browserEvidence.expectedVisibleText
    .publicBind,
  "off",
);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.browserEvidence.expectedVisibleText
    .autoStart,
  "off",
);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.browserEvidence.mobileOverflowAllowed,
  false,
);
for (const prohibited of [
  "payload_json",
  "payload_ciphertext_hex",
  "payload_nonce_hex",
  "payload_key_hex",
  "signed_session_ticket",
  "session_token",
  "support_actor_id",
  "daemon_device_id",
  "companion_device_id",
]) {
  assert.ok(
    managedRuntimeBrowserOperatorEvidence.prohibitedVisibleTokens.includes(
      prohibited,
    ),
    `browser operator evidence missing prohibited token ${prohibited}`,
  );
}
assert.equal(
  managedRuntimeBrowserOperatorEvidence.operatorEvidence.operatorSetupRequired,
  true,
);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.operatorEvidence.endpointAutoStart,
  false,
);
assert.equal(managedRuntimeBrowserOperatorEvidence.operatorEvidence.publicBind, false);
assert.ok(
  managedRuntimeBrowserOperatorEvidence.operatorEvidence.operatorCopyIncludes.includes(
    "Managed relay setup requires an operator-issued setup payload.",
  ),
);
assert.ok(
  managedRuntimeBrowserOperatorEvidence.completedImplementationEvidence.includes(
    "managed-runtime-browser-operator-evidence",
  ),
);
for (const evidenceCheck of [
  "pwa-exposure-gate-complete",
  "managed-relay-panel-visible-in-browser",
  "managed-relay-panel-visible-on-mobile",
  "managed-relay-panel-has-no-mobile-horizontal-overflow",
  "managed-relay-state-is-explicit-opt-in-ready",
  "managed-relay-product-default-remains-live-loopback",
  "managed-relay-endpoint-auto-start-remains-disabled",
  "managed-relay-public-bind-remains-disabled",
  "managed-relay-visible-body-excludes-prohibited-data",
  "managed-relay-operator-copy-requires-operator-issued-setup",
  "next-operator-setup-contract-slice-selected",
]) {
  assert.ok(
    managedRuntimeBrowserOperatorEvidence.evidenceChecks.includes(evidenceCheck),
    `managed runtime browser operator evidence missing evidence: ${evidenceCheck}`,
  );
}
assert.equal(managedRuntimeBrowserOperatorEvidence.implementationCanContinue, true);
assert.equal(managedRuntimeBrowserOperatorEvidence.selectedRuntimeCanChange, true);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(managedRuntimeBrowserOperatorEvidence.productDefaultCanChange, false);
assert.equal(
  managedRuntimeBrowserOperatorEvidence.nextLocalSlice,
  "managed-relay-runtime-operator-setup-contract",
);
const managedRuntimeOperatorSetupContract =
  relayManagedRuntimeOperatorSetupContract();
assert.equal(managedRuntimeOperatorSetupContract.deploymentMode, "managed");
assert.equal(
  managedRuntimeOperatorSetupContract.readiness,
  "operator-setup-contract",
);
assert.equal(managedRuntimeOperatorSetupContract.productDefault, "live-loopback");
assert.equal(
  managedRuntimeOperatorSetupContract.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(managedRuntimeOperatorSetupContract.runtimeDefault, "not-selected");
assert.equal(
  managedRuntimeOperatorSetupContract.implementationStatus,
  "managed-runtime-operator-setup-contract-ready-explicit-activation-only",
);
assert.equal(
  managedRuntimeOperatorSetupContract.pwaExposureDecision,
  "explicit-opt-in-operator-setup-contract-only",
);
assert.equal(managedRuntimeOperatorSetupContract.pwaExposure, "explicit-opt-in");
assert.equal(
  managedRuntimeOperatorSetupContract.endpointMode,
  "operator-setup-required",
);
assert.equal(managedRuntimeOperatorSetupContract.endpointAutoStart, false);
assert.equal(managedRuntimeOperatorSetupContract.publicBind, false);
assert.equal(managedRuntimeOperatorSetupContract.manualConnectRequired, true);
assert.equal(
  managedRuntimeOperatorSetupContract.endpointActivation,
  "operator-owned-explicit-connect-only",
);
assert.equal(
  managedRuntimeOperatorSetupContract.browserOperatorEvidence.readiness,
  "browser-operator-evidence",
);
assert.equal(
  managedRuntimeOperatorSetupContract.browserOperatorEvidence.nextLocalSlice,
  "managed-relay-runtime-operator-setup-contract",
);
for (const field of [
  "setup_version",
  "deployment_mode",
  "relay_endpoint_url",
  "tenant_id",
  "session_id_hash",
  "daemon_device_id_hash",
  "companion_device_id_hash",
  "verifier_key_id",
  "verifier_key_version",
  "issued_at_ms",
  "expires_at_ms",
  "operator_setup_text",
  "rollback_transport",
]) {
  assert.ok(
    managedRuntimeOperatorSetupContract.requiredSetupFields.includes(field),
    `operator setup contract missing required field ${field}`,
  );
  assert.ok(
    managedRuntimeOperatorSetupContract.setupPayloadContract.required_fields.includes(
      field,
    ),
    `setup payload contract missing required field ${field}`,
  );
}
for (const field of [
  "setup_label",
  "support_contact",
  "not_before_ms",
]) {
  assert.ok(
    managedRuntimeOperatorSetupContract.optionalSetupFields.includes(field),
    `operator setup contract missing optional field ${field}`,
  );
}
for (const prohibited of [
  "payload_json",
  "payload_ciphertext_hex",
  "payload_nonce_hex",
  "payload_key_hex",
  "signed_session_ticket",
  "raw_session_token",
  "session_token",
  "support_actor_id",
  "session_id",
  "daemon_device_id",
  "companion_device_id",
]) {
  assert.ok(
    managedRuntimeOperatorSetupContract.prohibitedSetupFields.includes(
      prohibited,
    ),
    `operator setup contract missing prohibited field ${prohibited}`,
  );
}
assert.equal(
  managedRuntimeOperatorSetupContract.setupPayloadContract.payload_visibility,
  "metadata-only",
);
assert.equal(
  managedRuntimeOperatorSetupContract.setupPayloadContract.identifier_policy,
  "hashed-identifiers-only",
);
assert.equal(
  managedRuntimeOperatorSetupContract.setupPayloadContract
    .authentication_material_policy,
  "not-in-pwa-setup-contract",
);
assert.equal(
  managedRuntimeOperatorSetupContract.endpointContract.required_scheme,
  "wss",
);
assert.equal(
  managedRuntimeOperatorSetupContract.endpointContract.endpoint_auto_start,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupContract.endpointContract.public_bind_enabled,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupContract.endpointContract.connect_requires_user_action,
  true,
);
assert.equal(
  managedRuntimeOperatorSetupContract.activationContract.manual_connect_required,
  true,
);
assert.equal(
  managedRuntimeOperatorSetupContract.activationContract
    .imported_setup_does_not_start_endpoint,
  true,
);
assert.equal(
  managedRuntimeOperatorSetupContract.activationContract
    .imported_setup_does_not_change_product_default,
  true,
);
assert.equal(
  managedRuntimeOperatorSetupContract.rollbackContract.rollback_transport,
  "live-loopback",
);
assert.equal(
  managedRuntimeOperatorSetupContract.healthSurface.payload_visibility,
  "metadata-only",
);
assert.equal(
  managedRuntimeOperatorSetupContract.healthSurface.identifier_policy,
  "hashed-identifiers-only",
);
const managedRuntimeOperatorSetupConfig =
  createManagedRelayRuntimeOperatorSetupContract({
    serviceId: "managed-relay-runtime-operator-setup-test",
    generatedAtMs: 2000,
    issuedAtMs: 2000,
    expiresAtMs: 4000,
  });
assert.equal(
  managedRuntimeOperatorSetupConfig.operator_setup_contract_version,
  1,
);
assert.equal(managedRuntimeOperatorSetupConfig.deployment_mode, "managed");
assert.equal(
  managedRuntimeOperatorSetupConfig.readiness,
  "operator-setup-contract",
);
assert.equal(managedRuntimeOperatorSetupConfig.product_default, "live-loopback");
assert.equal(
  managedRuntimeOperatorSetupConfig.selected_runtime,
  "explicit-opt-in-managed",
);
assert.equal(managedRuntimeOperatorSetupConfig.pwa_exposure, "explicit-opt-in");
assert.equal(
  managedRuntimeOperatorSetupConfig.endpoint_mode,
  "operator-setup-required",
);
assert.equal(managedRuntimeOperatorSetupConfig.endpoint_auto_start, false);
assert.equal(managedRuntimeOperatorSetupConfig.public_bind_enabled, false);
assert.equal(
  managedRuntimeOperatorSetupConfig.setup_payload_contract.example
    .relay_endpoint_url,
  "wss://managed-relay.example/relay",
);
assert.equal(
  managedRuntimeOperatorSetupConfig.setup_payload_contract.example
    .session_id_hash,
  "sha256:1111111111111111",
);
const managedRuntimeOperatorVisibleJson = JSON.stringify({
  setupPayloadContract: {
    ...managedRuntimeOperatorSetupConfig.setup_payload_contract,
    prohibited_fields: undefined,
  },
  endpointContract: managedRuntimeOperatorSetupConfig.endpoint_contract,
  activationContract: managedRuntimeOperatorSetupConfig.activation_contract,
  rollbackContract: managedRuntimeOperatorSetupConfig.rollback_contract,
  pwaSurfaceContract: {
    ...managedRuntimeOperatorSetupConfig.pwa_surface_contract,
    prohibited_visible_fields: undefined,
  },
  operatorSetupHealth:
    managedRuntimeOperatorSetupConfig.operator_setup_health,
  allowedSetupFields: managedRuntimeOperatorSetupConfig.allowed_setup_fields,
});
for (const prohibited of [
  "payload_ciphertext_hex",
  "payload_nonce_hex",
  "payload_key_hex",
  "signed_session_ticket",
  "raw_session_token",
  "session_token",
  '"support_actor_id"',
  '"session_id"',
  '"daemon_device_id"',
  '"companion_device_id"',
]) {
  assert.equal(
    managedRuntimeOperatorVisibleJson.includes(prohibited),
    false,
    `operator setup visible contract leaked ${prohibited}`,
  );
}
assert.throws(
  () =>
    createManagedRelayRuntimeOperatorSetupContract({
      serviceId: "managed-relay-runtime-operator-setup-test",
      generatedAtMs: 2000,
      relayEndpointUrl: "ws://managed-relay.example/relay",
    }),
  /endpoint must be wss/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeOperatorSetupContract({
      serviceId: "managed-relay-runtime-operator-setup-test",
      generatedAtMs: 2000,
      endpointAutoStart: true,
    }),
  /endpoint auto start must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeOperatorSetupContract({
      serviceId: "managed-relay-runtime-operator-setup-test",
      generatedAtMs: 2000,
      publicBind: true,
    }),
  /public bind must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeOperatorSetupContract({
      serviceId: "managed-relay-runtime-operator-setup-test",
      generatedAtMs: 2000,
      sessionIdHash: "session-alpha",
    }),
  /session_id_hash/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeOperatorSetupContract({
      serviceId: "managed-relay-runtime-operator-setup-test",
      generatedAtMs: 2000,
      selectedRuntime: "managed",
    }),
  /selected runtime must stay explicit opt-in/,
);
for (const evidenceCheck of [
  "browser-operator-evidence-complete",
  "operator-setup-contract-versioned",
  "operator-setup-required-fields-defined",
  "operator-setup-optional-fields-defined",
  "operator-setup-allows-metadata-only",
  "operator-setup-uses-hashed-identifiers-only",
  "operator-setup-requires-wss-endpoint",
  "operator-setup-excludes-signed-tickets-tokens-and-key-material",
  "operator-setup-import-does-not-auto-start-endpoint",
  "operator-setup-import-does-not-enable-public-bind",
  "operator-setup-preserves-live-loopback-rollback",
  "next-operator-setup-import-preflight-slice-selected",
]) {
  assert.ok(
    managedRuntimeOperatorSetupContract.evidenceChecks.includes(evidenceCheck),
    `managed runtime operator setup contract missing evidence: ${evidenceCheck}`,
  );
}
assert.ok(
  managedRuntimeOperatorSetupContract.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-contract",
  ),
);
assert.equal(managedRuntimeOperatorSetupContract.implementationCanContinue, true);
assert.equal(managedRuntimeOperatorSetupContract.selectedRuntimeCanChange, true);
assert.equal(managedRuntimeOperatorSetupContract.productDefaultCanChange, false);
assert.equal(
  managedRuntimeOperatorSetupContract.nextLocalSlice,
  "managed-relay-runtime-operator-setup-import-preflight",
);
const managedOperatorSetupPayload = {
  setup_version: 1,
  deployment_mode: "managed",
  relay_endpoint_url: "wss://managed-relay.example/relay",
  tenant_id: "tenant-managed-relay",
  session_id_hash: "sha256:1111111111111111",
  daemon_device_id_hash: "sha256:2222222222222222",
  companion_device_id_hash: "sha256:3333333333333333",
  verifier_key_id: "managed-relay-key-a",
  verifier_key_version: 1,
  issued_at_ms: 2000,
  expires_at_ms: 4000,
  operator_setup_text: "Managed relay setup requires operator-issued activation.",
  rollback_transport: "live-loopback",
  setup_label: "managed-preflight",
  support_contact: "support-managed-relay",
  not_before_ms: 2000,
};
assert.deepEqual(
  validateManagedRelayRuntimeOperatorSetupMetadata(managedOperatorSetupPayload),
  managedOperatorSetupPayload,
);
assert.deepEqual(
  parseManagedRelayRuntimeOperatorSetupInput(
    JSON.stringify(managedOperatorSetupPayload),
  ),
  managedOperatorSetupPayload,
);
const managedSetupEncoded = encodeURIComponent(JSON.stringify(managedOperatorSetupPayload));
assert.deepEqual(
  parseManagedRelayRuntimeOperatorSetupInput("", `?setup=${managedSetupEncoded}`),
  managedOperatorSetupPayload,
);
assert.deepEqual(
  parseManagedRelayRuntimeOperatorSetupInput(
    `aiterminal://relay?relaySetup=${managedSetupEncoded}`,
  ),
  managedOperatorSetupPayload,
);
const managedImportReady = managedRelayRuntimeOperatorSetupImportPreflight(
  managedOperatorSetupPayload,
  2500,
);
assert.equal(managedImportReady.status, "ready");
assert.equal(managedImportReady.importReady, true);
assert.equal(managedImportReady.connectEnabled, false);
assert.equal(managedImportReady.endpointAutoStart, false);
assert.equal(managedImportReady.publicBind, false);
assert.equal(managedImportReady.manualConnectRequired, true);
assert.equal(managedImportReady.setupRendering, "sanitized-summary-only");
assert.deepEqual(managedImportReady.blockers, []);
assert.equal(
  managedImportReady.sanitizedSetup.relay_endpoint_url,
  "wss://managed-relay.example/relay",
);
const managedImportExpired = managedRelayRuntimeOperatorSetupImportPreflight(
  managedOperatorSetupPayload,
  4000,
);
assert.equal(managedImportExpired.status, "blocked");
assert.ok(
  managedImportExpired.blockers.includes("managed_operator_setup_expired"),
);
assert.equal(managedImportExpired.connectEnabled, false);
const managedImportNotBefore = managedRelayRuntimeOperatorSetupImportPreflight(
  {
    ...managedOperatorSetupPayload,
    not_before_ms: 3000,
  },
  2500,
);
assert.ok(
  managedImportNotBefore.blockers.includes("managed_operator_setup_not_before"),
);
assert.throws(
  () =>
    parseManagedRelayRuntimeOperatorSetupInput(
      JSON.stringify({
        ...managedOperatorSetupPayload,
        relay_endpoint_url: "ws://managed-relay.example/relay",
      }),
    ),
  /relay_endpoint_url/,
);
assert.throws(
  () =>
    parseManagedRelayRuntimeOperatorSetupInput(
      JSON.stringify({
        ...managedOperatorSetupPayload,
        signed_session_ticket: "not-allowed",
      }),
    ),
  /prohibited field/,
);
assert.throws(
  () =>
    parseManagedRelayRuntimeOperatorSetupInput(
      JSON.stringify({
        ...managedOperatorSetupPayload,
        session_id: "managed-session-raw",
      }),
    ),
  /prohibited field/,
);
assert.throws(
  () =>
    parseManagedRelayRuntimeOperatorSetupInput(
      JSON.stringify({
        ...managedOperatorSetupPayload,
        extra_field: "not-allowed",
      }),
    ),
  /field not allowed/,
);
const managedRuntimeOperatorSetupImportPreflight =
  relayManagedRuntimeOperatorSetupImportPreflight();
assert.equal(
  managedRuntimeOperatorSetupImportPreflight.deploymentMode,
  "managed",
);
assert.equal(
  managedRuntimeOperatorSetupImportPreflight.readiness,
  "operator-setup-import-preflight",
);
assert.equal(
  managedRuntimeOperatorSetupImportPreflight.implementationStatus,
  "managed-runtime-operator-setup-import-preflight-ready-status-only",
);
assert.equal(
  managedRuntimeOperatorSetupImportPreflight.setupRendering,
  "sanitized-summary-only",
);
assert.equal(
  managedRuntimeOperatorSetupImportPreflight.importPreflight.parser,
  "parseManagedRelayRuntimeOperatorSetupInput",
);
assert.equal(
  managedRuntimeOperatorSetupImportPreflight.importPreflight.connectEnabledAfterImport,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupImportPreflight.importPreflight
    .originalJsonRenderedAfterImport,
  false,
);
for (const selector of [
  "#relay-managed-setup-input",
  "#relay-managed-load-button",
  "#relay-managed-clear-button",
  "#relay-managed-import-state",
  "#relay-managed-setup-endpoint",
  "#relay-managed-tenant",
  "#relay-managed-session-hash",
  "#relay-managed-daemon-hash",
  "#relay-managed-companion-hash",
  "#relay-managed-verifier-key",
  "#relay-managed-setup-expires",
  "#relay-managed-activation",
  "#relay-managed-setup-blocker-list",
  "#relay-managed-setup-summary",
]) {
  assert.ok(
    managedRuntimeOperatorSetupImportPreflight.requiredSelectors.includes(
      selector,
    ),
    `managed operator setup import preflight missing selector ${selector}`,
  );
}
for (const prohibited of [
  "payload_json",
  "payload_ciphertext_hex",
  "payload_nonce_hex",
  "payload_key_hex",
  "signed_session_ticket",
  "raw_session_token",
  "session_token",
  "support_actor_id",
  "session_id",
  "daemon_device_id",
  "companion_device_id",
]) {
  assert.ok(
    managedRuntimeOperatorSetupImportPreflight.prohibitedVisibleTokens.includes(
      prohibited,
    ),
    `managed import preflight missing prohibited token ${prohibited}`,
  );
}
for (const evidenceCheck of [
  "operator-setup-contract-complete",
  "managed-operator-setup-parser-accepts-contract-payload",
  "managed-operator-setup-parser-rejects-unknown-fields",
  "managed-operator-setup-parser-rejects-prohibited-fields",
  "managed-operator-setup-preflight-requires-wss-endpoint",
  "managed-operator-setup-preflight-requires-unexpired-window",
  "managed-operator-setup-preflight-keeps-connect-disabled",
  "managed-operator-setup-import-keeps-endpoint-auto-start-disabled",
  "managed-operator-setup-import-keeps-public-bind-disabled",
  "managed-operator-setup-import-renders-sanitized-summary-only",
  "managed-operator-setup-import-preserves-live-loopback-rollback",
  "next-managed-operator-setup-browser-evidence-slice-selected",
]) {
  assert.ok(
    managedRuntimeOperatorSetupImportPreflight.evidenceChecks.includes(
      evidenceCheck,
    ),
    `managed import preflight missing evidence ${evidenceCheck}`,
  );
}
assert.ok(
  managedRuntimeOperatorSetupImportPreflight.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-import-preflight",
  ),
);
assert.equal(
  managedRuntimeOperatorSetupImportPreflight.nextLocalSlice,
  "managed-relay-runtime-operator-setup-browser-evidence",
);
const managedRuntimeOperatorSetupBrowserEvidence =
  relayManagedRuntimeOperatorSetupBrowserEvidence();
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.deploymentMode,
  "managed",
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.readiness,
  "operator-setup-browser-evidence",
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.implementationStatus,
  "managed-runtime-operator-setup-browser-evidence-captured-status-only",
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.productDefault,
  "live-loopback",
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.pwaExposure,
  "explicit-opt-in",
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.endpointMode,
  "operator-setup-required",
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.endpointAutoStart,
  false,
);
assert.equal(managedRuntimeOperatorSetupBrowserEvidence.publicBind, false);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.manualConnectRequired,
  true,
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.setupRendering,
  "sanitized-summary-only",
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.importPreflight.nextLocalSlice,
  "managed-relay-runtime-operator-setup-browser-evidence",
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.operatorEvidence
    .originalSetupJsonRenderedAfterImport,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.operatorEvidence
    .connectEnabledAfterImport,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.browserEvidence
    .mobileOverflowAllowed,
  false,
);
for (const screenshot of [
  "managed-relay-operator-setup-browser-evidence.png",
  "managed-relay-operator-setup-browser-evidence-mobile.png",
]) {
  assert.ok(
    managedRuntimeOperatorSetupBrowserEvidence.requiredScreenshots.includes(
      screenshot,
    ),
    `managed operator setup browser evidence missing screenshot ${screenshot}`,
  );
}
for (const selector of [
  "#relay-managed-state",
  "#relay-managed-copy",
  "#relay-managed-evidence-list",
  "#relay-managed-setup-input",
  "#relay-managed-load-button",
  "#relay-managed-clear-button",
  "#relay-managed-import-state",
  "#relay-managed-setup-endpoint",
  "#relay-managed-tenant",
  "#relay-managed-session-hash",
  "#relay-managed-daemon-hash",
  "#relay-managed-companion-hash",
  "#relay-managed-verifier-key",
  "#relay-managed-setup-expires",
  "#relay-managed-activation",
  "#relay-managed-setup-blocker-list",
  "#relay-managed-setup-summary",
]) {
  assert.ok(
    managedRuntimeOperatorSetupBrowserEvidence.requiredSelectors.includes(
      selector,
    ),
    `managed operator setup browser evidence missing selector ${selector}`,
  );
}
for (const expectedText of [
  "Ready",
  "live-loopback",
  "wss://managed-relay.example/relay",
  "tenant-managed-relay",
  "managed-relay-key-a@1",
  "manual-connect",
  "Managed setup imported (metadata hidden)",
  "Managed relay setup import ready",
  "endpoint URL:",
  "session hash:",
  "activation: manual connect required",
]) {
  const expectedVisibleText =
    managedRuntimeOperatorSetupBrowserEvidence.browserEvidence
      .expectedVisibleText;
  const expectedVisibleJson = JSON.stringify(expectedVisibleText);
  assert.ok(
    expectedVisibleJson.includes(expectedText),
    `managed operator setup browser evidence missing visible text ${expectedText}`,
  );
}
for (const prohibited of [
  "setup_version",
  "deployment_mode",
  "relay_endpoint_url",
  "tenant_id",
  "session_id_hash",
  "daemon_device_id_hash",
  "companion_device_id_hash",
  "verifier_key_id",
  "verifier_key_version",
  "operator_setup_text",
  "payload_json",
  "signed_session_ticket",
  "raw_session_token",
  "session_token",
]) {
  assert.ok(
    managedRuntimeOperatorSetupBrowserEvidence.prohibitedVisibleTokens.includes(
      prohibited,
    ),
    `managed browser evidence missing prohibited token ${prohibited}`,
  );
}
for (const evidenceCheck of [
  "operator-setup-import-preflight-complete",
  "managed-operator-setup-import-visible-in-browser",
  "managed-operator-setup-import-visible-on-mobile",
  "managed-operator-setup-import-has-no-mobile-horizontal-overflow",
  "managed-operator-setup-import-state-ready",
  "managed-operator-setup-original-json-hidden-after-load",
  "managed-operator-setup-summary-is-sanitized",
  "managed-operator-setup-visible-body-excludes-prohibited-data",
  "managed-operator-setup-connect-controls-remain-out-of-scope",
  "managed-operator-setup-endpoint-auto-start-remains-disabled",
  "managed-operator-setup-public-bind-remains-disabled",
  "next-managed-operator-setup-connection-controls-slice-selected",
]) {
  assert.ok(
    managedRuntimeOperatorSetupBrowserEvidence.evidenceChecks.includes(
      evidenceCheck,
    ),
    `managed browser evidence missing evidence ${evidenceCheck}`,
  );
}
assert.ok(
  managedRuntimeOperatorSetupBrowserEvidence.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-browser-evidence",
  ),
);
assert.equal(
  managedRuntimeOperatorSetupBrowserEvidence.nextLocalSlice,
  "managed-relay-runtime-operator-setup-connection-controls",
);
const managedControlsReady =
  managedRelayRuntimeOperatorSetupConnectionControls(
    managedOperatorSetupPayload,
    { manualConnectRequested: false },
    2500,
  );
assert.equal(managedControlsReady.status, "ready");
assert.equal(managedControlsReady.importReady, true);
assert.equal(managedControlsReady.connectControlEnabled, true);
assert.equal(managedControlsReady.disconnectControlEnabled, false);
assert.equal(managedControlsReady.connectionStateText, "Ready");
assert.equal(managedControlsReady.lastEventText, "setup-ready");
assert.equal(managedControlsReady.networkConnectionStarted, false);
assert.equal(managedControlsReady.webSocketCreated, false);
assert.equal(managedControlsReady.endpointAutoStart, false);
assert.equal(managedControlsReady.publicBind, false);
assert.equal(managedControlsReady.relayEndpointUrl, "wss://managed-relay.example/relay");
const managedControlsRequested =
  managedRelayRuntimeOperatorSetupConnectionControls(
    managedOperatorSetupPayload,
    { manualConnectRequested: true },
    2500,
  );
assert.equal(managedControlsRequested.status, "manual-connect-requested");
assert.equal(managedControlsRequested.manualConnectRequested, true);
assert.equal(managedControlsRequested.connectControlEnabled, false);
assert.equal(managedControlsRequested.disconnectControlEnabled, true);
assert.equal(
  managedControlsRequested.connectionStateText,
  "Manual connect requested",
);
assert.equal(
  managedControlsRequested.lastEventText,
  "manual-connect-requested",
);
assert.equal(managedControlsRequested.networkConnectionStarted, false);
assert.equal(managedControlsRequested.webSocketCreated, false);
const managedControlsExpired =
  managedRelayRuntimeOperatorSetupConnectionControls(
    managedOperatorSetupPayload,
    { manualConnectRequested: true },
    4000,
  );
assert.equal(managedControlsExpired.status, "blocked");
assert.equal(managedControlsExpired.importReady, false);
assert.equal(managedControlsExpired.manualConnectRequested, false);
assert.equal(managedControlsExpired.connectControlEnabled, false);
assert.equal(managedControlsExpired.disconnectControlEnabled, false);
assert.ok(
  managedControlsExpired.blockers.includes("managed_operator_setup_expired"),
);
const managedRuntimeOperatorSetupConnectionControls =
  relayManagedRuntimeOperatorSetupConnectionControls();
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.deploymentMode,
  "managed",
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.readiness,
  "operator-setup-connection-controls",
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.implementationStatus,
  "managed-runtime-operator-setup-connection-controls-ready-status-only",
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.productDefault,
  "live-loopback",
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.endpointAutoStart,
  false,
);
assert.equal(managedRuntimeOperatorSetupConnectionControls.publicBind, false);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.networkConnectionStartedOnRequest,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.connectionControls
    .requestEnabledAfterReadyImport,
  true,
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.connectionControls
    .cancelEnabledAfterReadyImport,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.connectionControls
    .cancelEnabledAfterManualRequest,
  true,
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.connectionControls
    .requestCreatesWebSocket,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.connectionControls
    .requestStartsEndpoint,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.connectionControls
    .mobileOverflowAllowed,
  false,
);
for (const selector of [
  "#relay-managed-request-connect-button",
  "#relay-managed-cancel-connect-button",
  "#relay-managed-connection-state",
  "#relay-managed-last-event",
]) {
  assert.ok(
    managedRuntimeOperatorSetupConnectionControls.requiredSelectors.includes(
      selector,
    ),
    `managed connection controls missing selector ${selector}`,
  );
}
for (const screenshot of [
  "managed-relay-operator-setup-connection-controls.png",
  "managed-relay-operator-setup-connection-controls-mobile.png",
]) {
  assert.ok(
    managedRuntimeOperatorSetupConnectionControls.requiredScreenshots.includes(
      screenshot,
    ),
    `managed connection controls missing screenshot ${screenshot}`,
  );
}
for (const evidenceCheck of [
  "operator-setup-browser-evidence-complete",
  "managed-operator-setup-request-connect-enabled-after-ready-import",
  "managed-operator-setup-manual-request-updates-connection-state",
  "managed-operator-setup-manual-request-does-not-create-websocket",
  "managed-operator-setup-manual-request-does-not-start-endpoint",
  "managed-operator-setup-cancel-request-restores-ready-state",
  "managed-operator-setup-connection-controls-have-no-mobile-overflow",
  "next-managed-operator-setup-session-handshake-slice-selected",
]) {
  assert.ok(
    managedRuntimeOperatorSetupConnectionControls.evidenceChecks.includes(
      evidenceCheck,
    ),
    `managed connection controls missing evidence ${evidenceCheck}`,
  );
}
assert.ok(
  managedRuntimeOperatorSetupConnectionControls.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-connection-controls",
  ),
);
assert.equal(
  managedRuntimeOperatorSetupConnectionControls.nextLocalSlice,
  "managed-relay-runtime-operator-setup-session-handshake",
);
const managedHandshakePayload =
  managedRelayRuntimeOperatorSetupSessionHandshakePayload(
    managedOperatorSetupPayload,
    2500,
  );
assert.ok(
  managedHandshakePayload.includes(
    "ai-terminal-managed-relay-session-handshake-v1",
  ),
);
assert.ok(managedHandshakePayload.includes("tenant=tenant-managed-relay"));
assert.ok(
  managedHandshakePayload.includes("session_hash=sha256:1111111111111111"),
);
assert.equal(managedHandshakePayload.includes("operator_setup_text"), false);
assert.equal(managedHandshakePayload.includes("support_contact"), false);
const managedHandshakeBlocked =
  await managedRelayRuntimeOperatorSetupSessionHandshake(
    managedOperatorSetupPayload,
    { manualConnectRequested: false },
    2500,
    webcrypto,
  );
assert.equal(managedHandshakeBlocked.status, "blocked");
assert.equal(managedHandshakeBlocked.handshakeReady, false);
assert.ok(
  managedHandshakeBlocked.blockers.includes(
    "managed_operator_setup_manual_connect_required",
  ),
);
assert.equal(managedHandshakeBlocked.networkConnectionStarted, false);
assert.equal(managedHandshakeBlocked.webSocketCreated, false);
const managedHandshakeReady =
  await managedRelayRuntimeOperatorSetupSessionHandshake(
    managedOperatorSetupPayload,
    { manualConnectRequested: true },
    2500,
    webcrypto,
  );
assert.equal(managedHandshakeReady.status, "handshake-ready");
assert.equal(managedHandshakeReady.handshakeReady, true);
assert.equal(managedHandshakeReady.sessionCapabilityReady, true);
assert.match(managedHandshakeReady.capabilityHandle, /^managed-cap:[0-9a-f]{24}$/);
assert.match(managedHandshakeReady.transcriptHash, /^sha256:[0-9a-f]{64}$/);
assert.equal(
  managedHandshakeReady.capabilityEnvelope.request_type,
  "managed-session-capability-request",
);
assert.equal(
  managedHandshakeReady.capabilityEnvelope.capability_handle,
  managedHandshakeReady.capabilityHandle,
);
assert.equal(
  managedHandshakeReady.capabilityEnvelope.transcript_hash,
  managedHandshakeReady.transcriptHash,
);
assert.equal(managedHandshakeReady.capabilityEnvelopeVisible, false);
assert.equal(managedHandshakeReady.signedTicketVisible, false);
assert.equal(managedHandshakeReady.rawTokenVisible, false);
assert.equal(managedHandshakeReady.payloadVisible, false);
assert.equal(managedHandshakeReady.privateKeyMaterialVisible, false);
assert.equal(managedHandshakeReady.endpointAutoStart, false);
assert.equal(managedHandshakeReady.publicBind, false);
assert.equal(managedHandshakeReady.networkConnectionStarted, false);
assert.equal(managedHandshakeReady.webSocketCreated, false);
const managedRuntimeOperatorSetupSessionHandshake =
  relayManagedRuntimeOperatorSetupSessionHandshake();
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.deploymentMode,
  "managed",
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.readiness,
  "operator-setup-session-handshake",
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.implementationStatus,
  "managed-runtime-operator-setup-session-handshake-ready-metadata-only",
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.handshakeMode,
  "manual-request-capability-envelope",
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.sessionCapabilityVisibility,
  "handle-and-transcript-hash-only",
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.networkConnectionStartedOnHandshake,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.webSocketCreatedOnHandshake,
  false,
);
assert.equal(managedRuntimeOperatorSetupSessionHandshake.signedTicketVisible, false);
assert.equal(managedRuntimeOperatorSetupSessionHandshake.rawTokenVisible, false);
assert.equal(managedRuntimeOperatorSetupSessionHandshake.payloadVisible, false);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.privateKeyMaterialVisible,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.sessionHandshake.createsWebSocket,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.sessionHandshake.startsEndpoint,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.sessionHandshake
    .mobileOverflowAllowed,
  false,
);
for (const selector of [
  "#relay-managed-start-handshake-button",
  "#relay-managed-reset-handshake-button",
  "#relay-managed-handshake-state",
  "#relay-managed-capability-handle",
  "#relay-managed-handshake-transcript",
]) {
  assert.ok(
    managedRuntimeOperatorSetupSessionHandshake.requiredSelectors.includes(
      selector,
    ),
    `managed session handshake missing selector ${selector}`,
  );
}
for (const evidenceCheck of [
  "operator-setup-connection-controls-complete",
  "managed-operator-setup-session-handshake-requires-ready-import",
  "managed-operator-setup-session-handshake-requires-manual-connect-request",
  "managed-operator-setup-session-handshake-creates-capability-envelope",
  "managed-operator-setup-session-handshake-displays-handle-only",
  "managed-operator-setup-session-handshake-displays-transcript-hash-only",
  "managed-operator-setup-session-handshake-does-not-render-envelope-json",
  "managed-operator-setup-session-handshake-does-not-render-signed-ticket",
  "managed-operator-setup-session-handshake-does-not-render-raw-token",
  "managed-operator-setup-session-handshake-does-not-create-websocket",
  "next-managed-operator-setup-approval-flow-evidence-slice-selected",
]) {
  assert.ok(
    managedRuntimeOperatorSetupSessionHandshake.evidenceChecks.includes(
      evidenceCheck,
    ),
    `managed session handshake missing evidence ${evidenceCheck}`,
  );
}
assert.ok(
  managedRuntimeOperatorSetupSessionHandshake.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-session-handshake",
  ),
);
assert.equal(
  managedRuntimeOperatorSetupSessionHandshake.nextLocalSlice,
  "managed-relay-runtime-operator-setup-approval-flow-evidence",
);
assert.throws(
  () => managedRelayRuntimeOperatorSetupApprovalRequest(managedHandshakeBlocked),
  /ready session handshake/,
);
const managedApprovalRequest =
  managedRelayRuntimeOperatorSetupApprovalRequest(managedHandshakeReady, {
    expiresAt: 60000,
    deviceEpoch: 7,
  });
assert.equal(managedApprovalRequest.command_masked, "managed relay approval evidence command");
assert.equal(managedApprovalRequest.context_hash, managedHandshakeReady.transcriptHash);
assert.equal(managedApprovalRequest.nonce.length, 32);
assert.equal(managedApprovalRequest.expires_at, 60000);
assert.equal(managedApprovalRequest.device_epoch, 7);
assert.equal(
  JSON.stringify(managedApprovalRequest).includes(managedHandshakeReady.capabilityHandle),
  false,
);
assert.equal(
  JSON.stringify(managedApprovalRequest).includes("capability_envelope"),
  false,
);
const managedApprovalBlocked =
  managedRelayRuntimeOperatorSetupApprovalFlowEvidenceFromHandshake(
    managedHandshakeBlocked,
  );
assert.equal(managedApprovalBlocked.status, "blocked");
assert.equal(managedApprovalBlocked.approvalFlowReady, false);
assert.ok(
  managedApprovalBlocked.blockers.includes(
    "managed_operator_setup_session_handshake_required",
  ),
);
const managedApprovalReady =
  managedRelayRuntimeOperatorSetupApprovalFlowEvidenceFromHandshake(
    managedHandshakeReady,
    { expiresAt: 60000 },
  );
assert.equal(managedApprovalReady.status, "approval-flow-ready");
assert.equal(managedApprovalReady.approvalFlowReady, true);
assert.equal(managedApprovalReady.approvalSourceText, "Managed Relay");
assert.equal(managedApprovalReady.approvalStateText, "Approval request ready");
assert.equal(managedApprovalReady.lastEventText, "approval-request-ready");
assert.equal(
  managedApprovalReady.approvalRequest.context_hash,
  managedHandshakeReady.transcriptHash,
);
assert.equal(managedApprovalReady.approvalPayloadVisibleInManagedSetupSurface, false);
assert.equal(managedApprovalReady.approvalResponseVisibleInManagedSetupSurface, false);
assert.equal(managedApprovalReady.approvalResponseDelivery, "manual-signed-response-copy-only");
assert.equal(managedApprovalReady.capabilityEnvelopeVisible, false);
assert.equal(managedApprovalReady.signedTicketVisible, false);
assert.equal(managedApprovalReady.rawTokenVisible, false);
assert.equal(managedApprovalReady.payloadVisible, false);
assert.equal(managedApprovalReady.privateKeyMaterialVisible, false);
assert.equal(managedApprovalReady.networkConnectionStarted, false);
assert.equal(managedApprovalReady.webSocketCreated, false);
const managedApprovalReadyFromSetup =
  await managedRelayRuntimeOperatorSetupApprovalFlowEvidence(
    managedOperatorSetupPayload,
    { manualConnectRequested: true },
    2500,
    webcrypto,
  );
assert.equal(managedApprovalReadyFromSetup.status, "approval-flow-ready");
assert.equal(managedApprovalReadyFromSetup.sessionHandshake.handshakeReady, true);
assert.equal(
  managedApprovalReadyFromSetup.approvalRequest.context_hash,
  managedApprovalReadyFromSetup.sessionHandshake.transcriptHash,
);
const managedRuntimeOperatorSetupApprovalFlowEvidence =
  relayManagedRuntimeOperatorSetupApprovalFlowEvidence();
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.readiness,
  "operator-setup-approval-flow-evidence",
);
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.implementationStatus,
  "managed-runtime-operator-setup-approval-flow-evidence-ready-manual-only",
);
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.approvalFlowMode,
  "manual-approval-request-via-session-capability",
);
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.approvalResponseDelivery,
  "manual-signed-response-copy-only",
);
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.networkConnectionStartedOnApproval,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.webSocketCreatedOnApproval,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.approvalFlow.createsWebSocket,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.approvalFlow.startsEndpoint,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.approvalFlow.mobileOverflowAllowed,
  false,
);
for (const selector of [
  "#relay-managed-load-approval-button",
  "#relay-managed-approval-state",
  "#relay-managed-approval-source",
  "#relay-managed-approval-context",
  "#approval-source",
  "#approval-response",
  "#approval-verify-command",
]) {
  assert.ok(
    managedRuntimeOperatorSetupApprovalFlowEvidence.requiredSelectors.includes(
      selector,
    ),
    `managed approval flow missing selector ${selector}`,
  );
}
for (const evidenceCheck of [
  "operator-setup-session-handshake-complete",
  "managed-operator-setup-approval-flow-requires-session-handshake",
  "managed-operator-setup-approval-flow-uses-session-capability-boundary",
  "managed-operator-setup-approval-flow-loads-existing-approval-panel",
  "managed-operator-setup-approval-flow-signs-approve-response",
  "managed-operator-setup-approval-flow-signs-reject-response",
  "managed-operator-setup-approval-flow-does-not-create-websocket",
  "next-managed-operator-setup-runbook-closeout-slice-selected",
]) {
  assert.ok(
    managedRuntimeOperatorSetupApprovalFlowEvidence.evidenceChecks.includes(
      evidenceCheck,
    ),
    `managed approval flow missing evidence ${evidenceCheck}`,
  );
}
assert.ok(
  managedRuntimeOperatorSetupApprovalFlowEvidence.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-approval-flow-evidence",
  ),
);
assert.equal(
  managedRuntimeOperatorSetupApprovalFlowEvidence.nextLocalSlice,
  "managed-relay-runtime-operator-setup-runbook-closeout",
);
const managedRuntimeOperatorSetupRunbookCloseout =
  relayManagedRuntimeOperatorSetupRunbookCloseout();
assert.equal(
  managedRuntimeOperatorSetupRunbookCloseout.readiness,
  "operator-setup-runbook-closeout",
);
assert.equal(
  managedRuntimeOperatorSetupRunbookCloseout.implementationStatus,
  "managed-runtime-operator-setup-runbook-closeout-ready",
);
assert.equal(
  managedRuntimeOperatorSetupRunbookCloseout.runbookSection,
  "Managed Relay Operator Setup Evidence Map",
);
assert.equal(
  managedRuntimeOperatorSetupRunbookCloseout.approvalResponseDelivery,
  "manual-signed-response-copy-only",
);
assert.equal(
  managedRuntimeOperatorSetupRunbookCloseout.networkConnectionStartedOnCloseout,
  false,
);
assert.equal(
  managedRuntimeOperatorSetupRunbookCloseout.webSocketCreatedOnCloseout,
  false,
);
assert.equal(managedRuntimeOperatorSetupRunbookCloseout.endpointAutoStart, false);
assert.equal(managedRuntimeOperatorSetupRunbookCloseout.publicBind, false);
for (const command of [
  "npm run check:pwa-relay-managed-runtime-operator-setup-contract",
  "npm run check:pwa-relay-managed-runtime-operator-setup-import-preflight",
  "npm run smoke:pwa-relay-managed-runtime-operator-setup-browser-evidence",
  "npm run smoke:pwa-relay-managed-runtime-operator-setup-connection-controls",
  "npm run smoke:pwa-relay-managed-runtime-operator-setup-session-handshake",
  "npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-flow-evidence",
]) {
  assert.ok(
    managedRuntimeOperatorSetupRunbookCloseout.requiredRunbookCommands.includes(
      command,
    ),
    `managed runbook closeout missing command ${command}`,
  );
}
for (const evidenceCheck of [
  "operator-setup-approval-flow-evidence-complete",
  "managed-operator-setup-runbook-section-present",
  "managed-operator-setup-runbook-lists-contract-check",
  "managed-operator-setup-runbook-lists-approval-flow-smoke",
  "managed-operator-setup-runbook-keeps-network-delivery-out-of-scope",
  "next-managed-operator-setup-approval-response-delivery-boundary-slice-selected",
]) {
  assert.ok(
    managedRuntimeOperatorSetupRunbookCloseout.evidenceChecks.includes(
      evidenceCheck,
    ),
    `managed runbook closeout missing evidence ${evidenceCheck}`,
  );
}
assert.ok(
  managedRuntimeOperatorSetupRunbookCloseout.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-runbook-closeout",
  ),
);
assert.equal(
  managedRuntimeOperatorSetupRunbookCloseout.nextLocalSlice,
  "managed-relay-runtime-operator-setup-approval-response-delivery-boundary",
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
const managedPayloadKeyHex = await managedRelayDeriveSessionPayloadKeyHex(
  relayLoopSetup.daemonConnect.session_id,
  peerKeys.identity.noisePubkeyHex,
  generatedKeys.keyMaterial,
  webcrypto,
);
const managedPeerPayloadKeyHex = await managedRelayDeriveSessionPayloadKeyHex(
  relayLoopSetup.daemonConnect.session_id,
  generatedKeys.identity.noisePubkeyHex,
  peerKeys.keyMaterial,
  webcrypto,
);
const managedWrongSessionPayloadKeyHex = await managedRelayDeriveSessionPayloadKeyHex(
  `${relayLoopSetup.daemonConnect.session_id}-other`,
  peerKeys.identity.noisePubkeyHex,
  generatedKeys.keyMaterial,
  webcrypto,
);
assert.equal(managedPayloadKeyHex, managedPeerPayloadKeyHex);
assert.notEqual(managedPayloadKeyHex, managedWrongSessionPayloadKeyHex);
const managedEncryptedRequest = await managedRelayEncryptedFrameFromLiveMessage(
  relayLoopSetup.daemonConnect.session_id,
  "daemon",
  1,
  1601,
  1601 + 30000,
  liveApprovalRequestMessage(approvalRequest),
  managedPayloadKeyHex,
  { nonceHex: "22".repeat(12), webCrypto: webcrypto },
);
assert.doesNotThrow(() => validateManagedRelayEncryptedFrame(managedEncryptedRequest));
assert.equal("payload_json" in managedEncryptedRequest, false);
assert.equal(managedEncryptedRequest.payload_ciphertext_alg, "aes-256-gcm");
assert.equal(managedEncryptedRequest.payload_key_scope, "client-held-session-key");
assert.match(managedEncryptedRequest.payload_ciphertext_hex, /^[0-9a-f]+$/);
const managedEncryptedRequestJson = managedRelayEncryptedFrameJson(managedEncryptedRequest);
assert.equal(managedEncryptedRequestJson.includes("payload_json"), false);
assert.equal(managedEncryptedRequestJson.includes("rm -rf build"), false);
assert.deepEqual(parseManagedRelayEncryptedFrame(managedEncryptedRequestJson), managedEncryptedRequest);
const managedEncryptedRequestRoute = managedRelayEncryptedFrameRouteEnvelope(managedEncryptedRequest);
assert.equal("payload_json" in managedEncryptedRequestRoute, false);
assert.equal("payload_ciphertext_hex" in managedEncryptedRequestRoute, false);
assert.equal(managedEncryptedRequestRoute.payload_ciphertext_alg, "aes-256-gcm");
assert.ok(managedEncryptedRequestRoute.payload_ciphertext_bytes > 16);
assert.deepEqual(
  await managedRelayEncryptedFramePayloadMessage(
    managedEncryptedRequest,
    managedPayloadKeyHex,
    webcrypto,
  ),
  liveApprovalRequestMessage(approvalRequest),
);
const managedEncryptedResponse = await managedRelayEncryptedFrameFromLiveMessage(
  relayLoopSetup.companionConnect.session_id,
  "companion",
  1,
  1602,
  1602 + 30000,
  liveApprovalResponseMessage(signedApprove),
  managedPayloadKeyHex,
  { nonceHex: "33".repeat(12), webCrypto: webcrypto },
);
assert.equal(managedRelayEncryptedFrameJson(managedEncryptedResponse).includes("approval_response_payload"), false);
assert.deepEqual(
  await managedRelayEncryptedFramePayloadMessage(
    managedEncryptedResponse,
    managedPayloadKeyHex,
    webcrypto,
  ),
  liveApprovalResponseMessage(signedApprove),
);
await assert.rejects(
  () =>
    managedRelayEncryptedFramePayloadMessage(
      managedEncryptedRequest,
      managedWrongSessionPayloadKeyHex,
      webcrypto,
    ),
  /decrypt failed/,
);
await assert.rejects(
  () =>
    managedRelayEncryptedFramePayloadMessage(
      {
        ...managedEncryptedRequest,
        sequence: 2,
      },
      managedPayloadKeyHex,
      webcrypto,
    ),
  /decrypt failed/,
);
assert.throws(
  () =>
    validateManagedRelayEncryptedFrame({
      ...managedEncryptedRequest,
      payload_json: liveTransportJson(liveApprovalRequestMessage(approvalRequest)),
    }),
  /plaintext field not allowed/,
);
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
