import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createEd25519SignedRelaySessionTicket,
  createManagedRelayPublicVerifierKeyRegistry,
  createManagedRelayPublicVerifierKeyRegistrySnapshot,
  createRelaySessionTicket,
  generateCompanionKeyMaterial,
  lookupManagedRelayPublicVerifierKeyFromRegistrySnapshot,
  relayManagedRevocationAndRotationPropagationSmoke,
  relayManagedRuntimeReadinessGate,
  validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-revocation-and-rotation-propagation-smoke",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_REVOCATION_AND_ROTATION_PROPAGATION_SMOKE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-revocation-and-rotation-propagation-smoke.json",
  );

const smoke = relayManagedRevocationAndRotationPropagationSmoke();
const gate = relayManagedRuntimeReadinessGate();

assert.equal(smoke.deploymentMode, "managed");
assert.equal(smoke.readiness, "smoke");
assert.equal(smoke.productDefault, "live-loopback");
assert.equal(smoke.selectedRuntime, "deferred");
assert.equal(smoke.implementationStatus, "revocation-and-rotation-propagation-smoke-ready-runtime-still-deferred");
assert.equal(smoke.propagationBoundary, "snapshot-based-tenant-key-version-state");
assert.equal(smoke.implementationCanStart, true);
assert.equal(smoke.nextLocalSlice, "managed-relay-runtime-service-scaffold");
assert.ok(smoke.completedRuntimeEvidence.includes("revocation-and-rotation-propagation-smoke"));
assert.ok(smoke.closedReadinessBlockers.includes("key_revocation_propagation_smoke_missing"));
assert.ok(smoke.closedReadinessBlockers.includes("rotation_overlap_smoke_missing"));
assert.equal(smoke.remainingRuntimeEvidence.includes("revocation-and-rotation-propagation-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("tenant-session-registration-quota-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);
assert.ok(gate.completedRuntimeEvidence.includes("revocation-and-rotation-propagation-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("revocation-and-rotation-propagation-smoke"), false);
assert.ok(gate.completedRuntimeEvidence.includes("tenant-session-registration-quota-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("tenant-session-registration-quota-smoke"), false);
assert.ok(gate.completedRuntimeEvidence.includes("active-session-and-byte-quota-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"), false);
assert.ok(gate.completedRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"), false);
assert.equal(gate.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.equal(gate.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("managed_usage_meter_runtime_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("managed_usage_meter_runtime_missing"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("tenant_usage_export_smoke_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("tenant_usage_export_smoke_missing"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("key_revocation_propagation_smoke_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("rotation_overlap_smoke_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("quota_enforcement_smoke_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("key_revocation_propagation_smoke_missing"), false);
assert.equal(gate.remainingRuntimeBlockers.includes("rotation_overlap_smoke_missing"), false);
assert.equal(gate.remainingRuntimeBlockers.includes("quota_enforcement_smoke_missing"), false);

for (const guardrail of [
  "active_and_rotating_keys_overlap_during_rotation",
  "retiring_keys_fail_closed_for_new_sessions",
  "revoked_keys_fail_closed_after_snapshot_propagation",
  "registry_snapshot_id_preserved_in_audit",
  "tenant_key_id_version_audit_metadata_preserved",
]) {
  assert.ok(smoke.guardrails.includes(guardrail), `revocation/rotation smoke missing guardrail: ${guardrail}`);
}

const tenantId = "tenant-managed-rotation";
const keyId = "managed-rotation-key";
const oldKeyVersion = 1;
const newKeyVersion = 2;
const issuedAtMs = 1000;
const expiresAtMs = 4000;
const overlapNowMs = 1500;
const retiringNowMs = 2500;
const revokedNowMs = 3500;
const oldSigningKeys = await generateCompanionKeyMaterial(webcrypto);
const newSigningKeys = await generateCompanionKeyMaterial(webcrypto);
const companion = await generateCompanionKeyMaterial(webcrypto);
const ticket = createRelaySessionTicket({
  sessionId: "managed-rotation-session",
  sessionToken: "token_managed_rotation_session_1234567890",
  issuedAtMs,
  expiresAtMs,
  daemonPubkeyHex: "a".repeat(64),
  companionDeviceId: companion.identity.deviceId,
  companionNoisePubkeyHex: companion.identity.noisePubkeyHex,
  companionApprovalPubkeyHex: companion.identity.approvalPubkeyHex,
});
const oldSignedTicket = await createEd25519SignedRelaySessionTicket(
  ticket,
  oldSigningKeys.keyMaterial,
  { keyId, keyVersion: oldKeyVersion },
  webcrypto,
);
const newSignedTicket = await createEd25519SignedRelaySessionTicket(
  ticket,
  newSigningKeys.keyMaterial,
  { keyId, keyVersion: newKeyVersion },
  webcrypto,
);

const overlapSnapshot = createManagedRelayPublicVerifierKeyRegistrySnapshot(
  createManagedRelayPublicVerifierKeyRegistry([
    {
      tenant_id: tenantId,
      key_id: keyId,
      key_version: oldKeyVersion,
      public_key_alg: "ed25519",
      public_key_hex: oldSigningKeys.identity.approvalPubkeyHex,
      state: "active",
      not_before_ms: issuedAtMs,
      expires_at_ms: expiresAtMs,
    },
    {
      tenant_id: tenantId,
      key_id: keyId,
      key_version: newKeyVersion,
      public_key_alg: "ed25519",
      public_key_hex: newSigningKeys.identity.approvalPubkeyHex,
      state: "rotating",
      not_before_ms: issuedAtMs,
      expires_at_ms: expiresAtMs,
    },
  ]),
  {
    snapshotId: "rotation-snapshot-overlap",
    effectiveAtMs: 1400,
    reason: "rotation-overlap",
  },
);
const oldOverlapValidation =
  await validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
    oldSignedTicket,
    overlapSnapshot,
    { tenantId, nowMs: overlapNowMs },
    webcrypto,
  );
const newOverlapValidation =
  await validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
    newSignedTicket,
    overlapSnapshot,
    { tenantId, nowMs: overlapNowMs },
    webcrypto,
  );
assert.deepEqual(oldOverlapValidation.ticket, ticket);
assert.deepEqual(newOverlapValidation.ticket, ticket);
assert.equal(newOverlapValidation.auditEvent.key_state, "rotating");
assert.equal(newOverlapValidation.auditEvent.registry_snapshot_id, "rotation-snapshot-overlap");

const retiringSnapshot = createManagedRelayPublicVerifierKeyRegistrySnapshot(
  createManagedRelayPublicVerifierKeyRegistry([
    {
      ...overlapSnapshot.entries[0],
      state: "retiring",
    },
    {
      ...overlapSnapshot.entries[1],
      state: "active",
    },
  ]),
  {
    snapshotId: "rotation-snapshot-retiring",
    effectiveAtMs: 2400,
    previousSnapshotId: "rotation-snapshot-overlap",
    reason: "old-key-retiring",
  },
);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
      oldSignedTicket,
      retiringSnapshot,
      { tenantId, nowMs: retiringNowMs },
      webcrypto,
    ),
  /inactive/,
);

const revokedSnapshot = createManagedRelayPublicVerifierKeyRegistrySnapshot(
  createManagedRelayPublicVerifierKeyRegistry([
    {
      ...overlapSnapshot.entries[0],
      state: "revoked",
    },
    {
      ...overlapSnapshot.entries[1],
      state: "active",
    },
  ]),
  {
    snapshotId: "rotation-snapshot-revoked",
    effectiveAtMs: 3400,
    previousSnapshotId: "rotation-snapshot-retiring",
    reason: "old-key-revoked",
  },
);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
      oldSignedTicket,
      revokedSnapshot,
      { tenantId, nowMs: revokedNowMs },
      webcrypto,
    ),
  /inactive/,
);
const newRevokedSnapshotValidation =
  await validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
    newSignedTicket,
    revokedSnapshot,
    { tenantId, nowMs: revokedNowMs },
    webcrypto,
  );
assert.deepEqual(newRevokedSnapshotValidation.ticket, ticket);
assert.equal(newRevokedSnapshotValidation.auditEvent.key_state, "active");
assert.equal(newRevokedSnapshotValidation.auditEvent.key_id, keyId);
assert.equal(newRevokedSnapshotValidation.auditEvent.key_version, newKeyVersion);
assert.equal(newRevokedSnapshotValidation.auditEvent.registry_snapshot_id, "rotation-snapshot-revoked");
assert.equal(
  lookupManagedRelayPublicVerifierKeyFromRegistrySnapshot(
    revokedSnapshot,
    { tenantId, keyId, keyVersion: newKeyVersion },
    revokedNowMs,
  ).registry_effective_at_ms,
  3400,
);

const snapshotJson = JSON.stringify(revokedSnapshot);
assert.equal(snapshotJson.includes("private_signing_key"), false);
assert.equal(snapshotJson.includes("hmac_secret"), false);
assert.equal(snapshotJson.includes("secret"), false);
assert.equal(snapshotJson.includes(oldSignedTicket.mac_hex), false);
assert.equal(snapshotJson.includes(newSignedTicket.mac_hex), false);

const evidence = {
  status: "smoke-complete-runtime-deferred",
  generatedAt: new Date().toISOString(),
  objective: "Prove managed relay revocation and rotation propagation across verifier registry snapshots",
  smoke: {
    deploymentMode: smoke.deploymentMode,
    readiness: smoke.readiness,
    selectedRuntime: smoke.selectedRuntime,
    implementationStatus: smoke.implementationStatus,
    propagationBoundary: smoke.propagationBoundary,
    implementationCanStart: smoke.implementationCanStart,
  },
  completedRuntimeEvidence: smoke.completedRuntimeEvidence,
  closedReadinessBlockers: smoke.closedReadinessBlockers,
  remainingRuntimeEvidence: smoke.remainingRuntimeEvidence,
  remainingRuntimeBlockers: smoke.remainingRuntimeBlockers,
  propagationContract: smoke.propagationContract,
  snapshots: [
    {
      snapshot_id: overlapSnapshot.snapshot_id,
      states: overlapSnapshot.entries.map((entry) => `${entry.key_version}:${entry.state}`),
    },
    {
      snapshot_id: retiringSnapshot.snapshot_id,
      states: retiringSnapshot.entries.map((entry) => `${entry.key_version}:${entry.state}`),
    },
    {
      snapshot_id: revokedSnapshot.snapshot_id,
      states: revokedSnapshot.entries.map((entry) => `${entry.key_version}:${entry.state}`),
    },
  ],
  acceptedAuditEvents: [
    oldOverlapValidation.auditEvent,
    newOverlapValidation.auditEvent,
    newRevokedSnapshotValidation.auditEvent,
  ],
  smokeEvidence: smoke.smokeEvidence,
  guardrails: smoke.guardrails,
  productDefault: smoke.productDefault,
  nextLocalSlice: smoke.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_REVOCATION_AND_ROTATION_PROPAGATION_SMOKE_OK ${evidencePath}`);
