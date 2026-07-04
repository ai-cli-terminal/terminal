import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createEd25519SignedRelaySessionTicket,
  createManagedRelayPublicVerifierKeyRegistry,
  createRelaySessionTicket,
  createSignedRelaySessionTicket,
  generateCompanionKeyMaterial,
  lookupManagedRelayPublicVerifierKey,
  relayManagedPublicVerifierKeyRegistryRuntimeSmoke,
  relayManagedRuntimeReadinessGate,
  validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-public-verifier-key-registry-runtime-smoke",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_PUBLIC_VERIFIER_KEY_REGISTRY_RUNTIME_SMOKE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-public-verifier-key-registry-runtime-smoke.json",
  );

const smoke = relayManagedPublicVerifierKeyRegistryRuntimeSmoke();
const gate = relayManagedRuntimeReadinessGate();

assert.equal(smoke.deploymentMode, "managed");
assert.equal(smoke.readiness, "smoke");
assert.equal(smoke.productDefault, "live-loopback");
assert.equal(smoke.selectedRuntime, "deferred");
assert.equal(smoke.implementationStatus, "public-verifier-key-registry-smoke-ready-runtime-still-deferred");
assert.equal(smoke.verifierKeyAlg, "ed25519");
assert.equal(smoke.registryBoundary, "tenant-key-id-version-public-verifiers-only");
assert.equal(smoke.implementationCanStart, false);
assert.equal(smoke.nextLocalSlice, "managed-relay-support-redaction-and-access-review-evidence");
assert.ok(smoke.completedRuntimeEvidence.includes("public-verifier-key-registry-runtime-smoke"));
assert.ok(smoke.closedReadinessBlockers.includes("managed_key_registry_runtime_missing"));
assert.equal(smoke.remainingRuntimeEvidence.includes("public-verifier-key-registry-runtime-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("revocation-and-rotation-propagation-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("tenant-session-registration-quota-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"), false);
assert.ok(smoke.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"));
assert.ok(gate.completedRuntimeEvidence.includes("public-verifier-key-registry-runtime-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("public-verifier-key-registry-runtime-smoke"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("managed_key_registry_runtime_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("managed_key_registry_runtime_missing"), false);
assert.ok(gate.completedRuntimeEvidence.includes("revocation-and-rotation-propagation-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("revocation-and-rotation-propagation-smoke"), false);
assert.ok(gate.completedRuntimeEvidence.includes("tenant-session-registration-quota-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("tenant-session-registration-quota-smoke"), false);
assert.ok(gate.completedRuntimeEvidence.includes("active-session-and-byte-quota-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"), false);
assert.ok(gate.completedRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"), false);
assert.ok(gate.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"));
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
  "registry_contains_public_verifier_keys_only",
  "private_signing_keys_excluded_from_registry",
  "hmac_secrets_excluded_from_registry",
  "ticket_key_id_version_required",
  "missing_or_revoked_key_fails_closed",
  "key_id_version_audit_metadata_preserved",
]) {
  assert.ok(smoke.guardrails.includes(guardrail), `public verifier smoke missing guardrail: ${guardrail}`);
}

const tenantId = "tenant-managed-demo";
const keyId = "managed-ed25519-key";
const keyVersion = 7;
const issuedAtMs = 1000;
const expiresAtMs = 2000;
const nowMs = 1500;
const signingKeys = await generateCompanionKeyMaterial(webcrypto);
const companion = await generateCompanionKeyMaterial(webcrypto);
const ticket = createRelaySessionTicket({
  sessionId: "managed-public-verifier-session",
  sessionToken: "token_managed_public_verifier_1234567890",
  issuedAtMs,
  expiresAtMs,
  daemonPubkeyHex: "a".repeat(64),
  companionDeviceId: companion.identity.deviceId,
  companionNoisePubkeyHex: companion.identity.noisePubkeyHex,
  companionApprovalPubkeyHex: companion.identity.approvalPubkeyHex,
});
const signedTicket = await createEd25519SignedRelaySessionTicket(
  ticket,
  signingKeys.keyMaterial,
  { keyId, keyVersion },
  webcrypto,
);
const registry = createManagedRelayPublicVerifierKeyRegistry([
  {
    tenant_id: tenantId,
    key_id: keyId,
    key_version: keyVersion,
    public_key_alg: "ed25519",
    public_key_hex: signingKeys.identity.approvalPubkeyHex,
    state: "active",
    not_before_ms: issuedAtMs,
    expires_at_ms: expiresAtMs,
  },
]);

const lookupResult = lookupManagedRelayPublicVerifierKey(
  registry,
  { tenantId, keyId, keyVersion },
  nowMs,
);
assert.equal(lookupResult.public_key_hex, signingKeys.identity.approvalPubkeyHex);
assert.deepEqual(
  await validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
    signedTicket,
    registry,
    { tenantId, nowMs },
    webcrypto,
  ),
  ticket,
);

assert.throws(() =>
  lookupManagedRelayPublicVerifierKey(
    registry,
    { tenantId, keyId: "missing-key", keyVersion },
    nowMs,
  ),
);

const revokedRegistry = createManagedRelayPublicVerifierKeyRegistry([
  {
    ...registry.entries[0],
    state: "revoked",
  },
]);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
      signedTicket,
      revokedRegistry,
      { tenantId, nowMs },
      webcrypto,
    ),
  /inactive/,
);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
      {
        ...signedTicket,
        ticket: {
          ...signedTicket.ticket,
          session_token: "token_tampered_public_verifier_1234567890",
        },
      },
      registry,
      { tenantId, nowMs },
      webcrypto,
    ),
  /signature mismatch/,
);

const hmacTicket = await createSignedRelaySessionTicket(
  ticket,
  "managed-public-verifier-hmac-secret-demo",
  webcrypto,
);
await assert.rejects(
  () =>
    validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
      hmacTicket,
      registry,
      { tenantId, nowMs },
      webcrypto,
    ),
  /requires ed25519/,
);
assert.throws(() =>
  createManagedRelayPublicVerifierKeyRegistry([
    {
      ...registry.entries[0],
      private_signing_key: "not-allowed",
    },
  ]),
);
assert.throws(() =>
  createManagedRelayPublicVerifierKeyRegistry([
    {
      ...registry.entries[0],
      hmac_secret: "not-allowed",
    },
  ]),
);

const registryJson = JSON.stringify(registry);
assert.equal(registryJson.includes("private_signing_key"), false);
assert.equal(registryJson.includes("hmac_secret"), false);
assert.equal(registryJson.includes("secret"), false);
assert.equal(registryJson.includes(signedTicket.mac_hex), false);

const evidence = {
  status: "smoke-complete-runtime-deferred",
  generatedAt: new Date().toISOString(),
  objective: "Prove managed relay public verifier-key registry lookup and Ed25519 ticket verification",
  smoke: {
    deploymentMode: smoke.deploymentMode,
    readiness: smoke.readiness,
    selectedRuntime: smoke.selectedRuntime,
    implementationStatus: smoke.implementationStatus,
    registryBoundary: smoke.registryBoundary,
    verifierKeyAlg: smoke.verifierKeyAlg,
    implementationCanStart: smoke.implementationCanStart,
  },
  completedRuntimeEvidence: smoke.completedRuntimeEvidence,
  closedReadinessBlockers: smoke.closedReadinessBlockers,
  remainingRuntimeEvidence: smoke.remainingRuntimeEvidence,
  remainingRuntimeBlockers: smoke.remainingRuntimeBlockers,
  registryContract: smoke.registryContract,
  registryEntry: lookupResult,
  signedTicketMetadata: {
    mac_alg: signedTicket.mac_alg,
    key_id: signedTicket.key_id,
    key_version: signedTicket.key_version,
  },
  smokeEvidence: smoke.smokeEvidence,
  guardrails: smoke.guardrails,
  productDefault: smoke.productDefault,
  nextLocalSlice: smoke.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_PUBLIC_VERIFIER_KEY_REGISTRY_RUNTIME_SMOKE_OK ${evidencePath}`);
