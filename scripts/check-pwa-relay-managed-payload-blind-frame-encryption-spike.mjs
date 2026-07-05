import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  approvalResponseForRequest,
  liveApprovalRequestMessage,
  liveApprovalResponseMessage,
  managedRelayEncryptedFrameFromLiveMessage,
  managedRelayEncryptedFrameJson,
  managedRelayEncryptedFramePayloadMessage,
  managedRelayEncryptedFrameRouteEnvelope,
  relayManagedPayloadBlindFrameEncryptionSpike,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-payload-blind-frame-encryption-spike",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_PAYLOAD_BLIND_FRAME_ENCRYPTION_SPIKE_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-payload-blind-frame-encryption-spike.json");

const spike = relayManagedPayloadBlindFrameEncryptionSpike();
assert.equal(spike.deploymentMode, "managed");
assert.equal(spike.readiness, "spike");
assert.equal(spike.productDefault, "live-loopback");
assert.equal(spike.selectedRuntime, "deferred");
assert.equal(spike.implementationStatus, "payload-blind-frame-envelope-ready-runtime-still-deferred");
assert.equal(spike.payloadCiphertextAlg, "aes-256-gcm");
assert.equal(spike.payloadKeyScope, "client-held-session-key");
assert.equal(spike.implementationCanStart, true);
assert.equal(spike.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");

for (const evidence of [
  "payload-blind-frame-encryption-smoke",
]) {
  assert.ok(spike.completedRuntimeEvidence.includes(evidence), `spike missing completed evidence: ${evidence}`);
}

for (const blocker of [
  "e2e_payload_encryption_missing",
  "confidentiality_smoke_missing",
]) {
  assert.ok(spike.closedReadinessBlockers.includes(blocker), `spike missing closed blocker: ${blocker}`);
  assert.equal(spike.remainingRuntimeBlockers.includes(blocker), false);
}

for (const guardrail of [
  "relay_routes_ciphertext_only",
  "payload_json_excluded_from_managed_frame",
  "client_held_payload_key_required",
  "aes_gcm_nonce_required_per_frame",
]) {
  assert.ok(spike.guardrails.includes(guardrail), `spike missing guardrail: ${guardrail}`);
}

const approvalRequest = {
  approval_id: [109, 97, 110, 97, 103, 101, 100, 45, 49],
  nonce: Array.from({ length: 32 }, (_, i) => i + 1),
  command_masked: "deploy production --tenant alpha",
  context_hash: "ctx-managed-alpha",
  expires_at: 1782804241456,
  device_epoch: 7,
};
const keyMaterial = await webcrypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"]);
const signedResponse = await approvalResponseForRequest(
  approvalRequest,
  true,
  {
    approval: {
      privateKey: keyMaterial.privateKey,
      publicKey: keyMaterial.publicKey,
    },
  },
  webcrypto,
);
const payloadKeyHex = "55".repeat(32);
const requestFrame = await managedRelayEncryptedFrameFromLiveMessage(
  "managed-session-alpha",
  "daemon",
  1,
  1701,
  1701 + 30000,
  liveApprovalRequestMessage(approvalRequest),
  payloadKeyHex,
  { nonceHex: "66".repeat(12), webCrypto: webcrypto },
);
const responseFrame = await managedRelayEncryptedFrameFromLiveMessage(
  "managed-session-alpha",
  "companion",
  1,
  1702,
  1702 + 30000,
  liveApprovalResponseMessage(signedResponse),
  payloadKeyHex,
  { nonceHex: "77".repeat(12), webCrypto: webcrypto },
);

const requestFrameJson = managedRelayEncryptedFrameJson(requestFrame);
const responseFrameJson = managedRelayEncryptedFrameJson(responseFrame);
assert.equal(requestFrameJson.includes("payload_json"), false);
assert.equal(requestFrameJson.includes("deploy production"), false);
assert.equal(requestFrameJson.includes("ctx-managed-alpha"), false);
assert.equal(responseFrameJson.includes("approval_response_payload"), false);

const requestRoute = managedRelayEncryptedFrameRouteEnvelope(requestFrame);
const responseRoute = managedRelayEncryptedFrameRouteEnvelope(responseFrame);
for (const route of [requestRoute, responseRoute]) {
  assert.equal("payload_json" in route, false);
  assert.equal("payload_ciphertext_hex" in route, false);
  assert.equal(route.payload_ciphertext_alg, "aes-256-gcm");
  assert.equal(route.payload_key_scope, "client-held-session-key");
  assert.ok(route.payload_ciphertext_bytes > 16);
}

assert.deepEqual(
  await managedRelayEncryptedFramePayloadMessage(requestFrame, payloadKeyHex, webcrypto),
  liveApprovalRequestMessage(approvalRequest),
);
assert.deepEqual(
  await managedRelayEncryptedFramePayloadMessage(responseFrameJson, payloadKeyHex, webcrypto),
  liveApprovalResponseMessage(signedResponse),
);
await assert.rejects(
  () => managedRelayEncryptedFramePayloadMessage(requestFrame, "88".repeat(32), webcrypto),
  /decrypt failed/,
);
await assert.rejects(
  () =>
    managedRelayEncryptedFramePayloadMessage(
      {
        ...requestFrame,
        sequence: 2,
      },
      payloadKeyHex,
      webcrypto,
    ),
  /decrypt failed/,
);

const evidence = {
  status: "spike-complete-runtime-deferred",
  generatedAt: new Date().toISOString(),
  objective: "Prove managed relay can route opaque encrypted frame payloads without payload_json visibility",
  spike: {
    deploymentMode: spike.deploymentMode,
    readiness: spike.readiness,
    selectedRuntime: spike.selectedRuntime,
    implementationStatus: spike.implementationStatus,
    payloadCiphertextAlg: spike.payloadCiphertextAlg,
    payloadKeyScope: spike.payloadKeyScope,
    implementationCanStart: spike.implementationCanStart,
  },
  completedRuntimeEvidence: spike.completedRuntimeEvidence,
  closedReadinessBlockers: spike.closedReadinessBlockers,
  remainingRuntimeEvidence: spike.remainingRuntimeEvidence,
  remainingRuntimeBlockers: spike.remainingRuntimeBlockers,
  frameEnvelope: spike.frameEnvelope,
  smokeEvidence: {
    requestRoute,
    responseRoute,
    requestJsonExcludesPayloadJson: !requestFrameJson.includes("payload_json"),
    requestJsonExcludesCommandText: !requestFrameJson.includes("deploy production"),
    wrongKeyFailsClosed: true,
    aadTamperFailsClosed: true,
  },
  guardrails: spike.guardrails,
  productDefault: spike.productDefault,
  nextLocalSlice: spike.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_PAYLOAD_BLIND_FRAME_ENCRYPTION_SPIKE_OK ${evidencePath}`);
