import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  approvalResponseForRequest,
  generateCompanionKeyMaterial,
  liveApprovalRequestMessage,
  liveApprovalResponseMessage,
  managedRelayDeriveSessionPayloadKeyHex,
  managedRelayEncryptedFrameFromLiveMessage,
  managedRelayEncryptedFrameJson,
  managedRelayEncryptedFramePayloadMessage,
  managedRelayEncryptedFrameRouteEnvelope,
  relayManagedClientKeyAgreementRuntimeSmoke,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-client-key-agreement-runtime-smoke",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_CLIENT_KEY_AGREEMENT_RUNTIME_SMOKE_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-client-key-agreement-runtime-smoke.json");

const smoke = relayManagedClientKeyAgreementRuntimeSmoke();
assert.equal(smoke.deploymentMode, "managed");
assert.equal(smoke.readiness, "smoke");
assert.equal(smoke.productDefault, "live-loopback");
assert.equal(smoke.selectedRuntime, "deferred");
assert.equal(smoke.implementationStatus, "client-key-agreement-smoke-ready-runtime-still-deferred");
assert.equal(smoke.keyAgreementAlg, "x25519-hkdf-sha256");
assert.equal(smoke.hkdfHash, "SHA-256");
assert.equal(smoke.payloadKeyScope, "client-held-session-key");
assert.equal(smoke.implementationCanStart, false);
assert.equal(smoke.nextLocalSlice, "managed-relay-metadata-minimization-review");
assert.ok(smoke.completedRuntimeEvidence.includes("client-key-agreement-runtime-smoke"));
assert.ok(smoke.closedReadinessBlockers.includes("client_key_agreement_missing"));
assert.equal(smoke.remainingRuntimeEvidence.includes("client-key-agreement-runtime-smoke"), false);
assert.ok(smoke.remainingRuntimeEvidence.includes("metadata-minimization-review"));

for (const guardrail of [
  "session_bound_payload_key_required",
  "daemon_and_companion_derive_same_payload_key",
  "managed_relay_receives_public_keys_only",
  "route_metadata_cannot_derive_payload_key",
  "payload_key_not_serialized_to_frame_or_route",
]) {
  assert.ok(smoke.guardrails.includes(guardrail), `smoke missing guardrail: ${guardrail}`);
}

const daemon = await generateCompanionKeyMaterial(webcrypto);
const companion = await generateCompanionKeyMaterial(webcrypto);
const sessionId = "managed-session-key-agreement-alpha";
const otherSessionId = "managed-session-key-agreement-beta";
const daemonPayloadKeyHex = await managedRelayDeriveSessionPayloadKeyHex(
  sessionId,
  companion.identity.noisePubkeyHex,
  daemon.keyMaterial,
  webcrypto,
);
const companionPayloadKeyHex = await managedRelayDeriveSessionPayloadKeyHex(
  sessionId,
  daemon.identity.noisePubkeyHex,
  companion.keyMaterial,
  webcrypto,
);
const otherSessionPayloadKeyHex = await managedRelayDeriveSessionPayloadKeyHex(
  otherSessionId,
  companion.identity.noisePubkeyHex,
  daemon.keyMaterial,
  webcrypto,
);

assert.match(daemonPayloadKeyHex, /^[0-9a-f]{64}$/);
assert.equal(daemonPayloadKeyHex, companionPayloadKeyHex);
assert.notEqual(daemonPayloadKeyHex, otherSessionPayloadKeyHex);

const routeKeyMaterial = {
  session_id: sessionId,
  daemon_noise_pubkey_hex: daemon.identity.noisePubkeyHex,
  companion_noise_pubkey_hex: companion.identity.noisePubkeyHex,
};
assert.equal("payload_key_hex" in routeKeyMaterial, false);
assert.equal("shared_secret_hex" in routeKeyMaterial, false);
assert.equal("daemon_noise_private_key" in routeKeyMaterial, false);
assert.equal("companion_noise_private_key" in routeKeyMaterial, false);
await assert.rejects(
  () =>
    managedRelayDeriveSessionPayloadKeyHex(
      sessionId,
      companion.identity.noisePubkeyHex,
      { noise: { publicKey: daemon.keyMaterial.noise.publicKey } },
      webcrypto,
    ),
  /private key/,
);

const approvalRequest = {
  approval_id: [109, 97, 110, 97, 103, 101, 100, 45, 107, 101, 121],
  nonce: Array.from({ length: 32 }, (_, i) => 255 - i),
  command_masked: "rotate managed relay session key",
  context_hash: "ctx-managed-key-agreement",
  expires_at: 1782804241456,
  device_epoch: 9,
};
const signedResponse = await approvalResponseForRequest(
  approvalRequest,
  true,
  companion.keyMaterial,
  webcrypto,
);
const requestFrame = await managedRelayEncryptedFrameFromLiveMessage(
  sessionId,
  "daemon",
  1,
  1801,
  1801 + 30000,
  liveApprovalRequestMessage(approvalRequest),
  daemonPayloadKeyHex,
  { nonceHex: "99".repeat(12), webCrypto: webcrypto },
);
const responseFrame = await managedRelayEncryptedFrameFromLiveMessage(
  sessionId,
  "companion",
  1,
  1802,
  1802 + 30000,
  liveApprovalResponseMessage(signedResponse),
  companionPayloadKeyHex,
  { nonceHex: "aa".repeat(12), webCrypto: webcrypto },
);

for (const frameJson of [
  managedRelayEncryptedFrameJson(requestFrame),
  managedRelayEncryptedFrameJson(responseFrame),
]) {
  assert.equal(frameJson.includes("payload_key_hex"), false);
  assert.equal(frameJson.includes("shared_secret_hex"), false);
  assert.equal(frameJson.includes("rotate managed relay session key"), false);
  assert.equal(frameJson.includes("ctx-managed-key-agreement"), false);
}

for (const route of [
  managedRelayEncryptedFrameRouteEnvelope(requestFrame),
  managedRelayEncryptedFrameRouteEnvelope(responseFrame),
]) {
  assert.equal("payload_key_hex" in route, false);
  assert.equal("shared_secret_hex" in route, false);
  assert.equal("payload_ciphertext_hex" in route, false);
  assert.ok(route.payload_ciphertext_bytes > 16);
}

assert.deepEqual(
  await managedRelayEncryptedFramePayloadMessage(requestFrame, companionPayloadKeyHex, webcrypto),
  liveApprovalRequestMessage(approvalRequest),
);
assert.deepEqual(
  await managedRelayEncryptedFramePayloadMessage(responseFrame, daemonPayloadKeyHex, webcrypto),
  liveApprovalResponseMessage(signedResponse),
);
await assert.rejects(
  () => managedRelayEncryptedFramePayloadMessage(requestFrame, otherSessionPayloadKeyHex, webcrypto),
  /decrypt failed/,
);

const evidence = {
  status: "smoke-complete-runtime-deferred",
  generatedAt: new Date().toISOString(),
  objective:
    "Prove managed relay payload keys are derived by daemon/companion boundaries and not route-visible relay state",
  smoke: {
    deploymentMode: smoke.deploymentMode,
    readiness: smoke.readiness,
    selectedRuntime: smoke.selectedRuntime,
    implementationStatus: smoke.implementationStatus,
    keyAgreementAlg: smoke.keyAgreementAlg,
    hkdfHash: smoke.hkdfHash,
    payloadKeyScope: smoke.payloadKeyScope,
    implementationCanStart: smoke.implementationCanStart,
  },
  completedRuntimeEvidence: smoke.completedRuntimeEvidence,
  closedReadinessBlockers: smoke.closedReadinessBlockers,
  remainingRuntimeEvidence: smoke.remainingRuntimeEvidence,
  remainingRuntimeBlockers: smoke.remainingRuntimeBlockers,
  keyAgreement: smoke.keyAgreement,
  routeKeyMaterial,
  smokeEvidence: {
    daemonAndCompanionDerivedSameKey: daemonPayloadKeyHex === companionPayloadKeyHex,
    differentSessionDerivedDifferentKey: daemonPayloadKeyHex !== otherSessionPayloadKeyHex,
    routeMaterialExcludesPrivateKeys: true,
    routeMaterialExcludesPayloadKey: true,
    derivedKeyEncryptedRequestRoute: managedRelayEncryptedFrameRouteEnvelope(requestFrame),
    derivedKeyEncryptedResponseRoute: managedRelayEncryptedFrameRouteEnvelope(responseFrame),
    wrongSessionKeyFailsClosed: true,
  },
  guardrails: smoke.guardrails,
  productDefault: smoke.productDefault,
  nextLocalSlice: smoke.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_CLIENT_KEY_AGREEMENT_RUNTIME_SMOKE_OK ${evidencePath}`);
