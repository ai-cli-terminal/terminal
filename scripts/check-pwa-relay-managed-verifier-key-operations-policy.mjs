import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { relayManagedVerifierKeyOperationsPolicy } from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-verifier-key-operations-policy",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_VERIFIER_KEY_OPERATIONS_POLICY_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-verifier-key-operations-policy.json");

const policy = relayManagedVerifierKeyOperationsPolicy();

assert.equal(policy.deploymentMode, "managed");
assert.equal(policy.readiness, "policy");
assert.equal(policy.productDefault, "live-loopback");
assert.equal(policy.selectedRuntime, "deferred");
assert.equal(policy.verifierKeyOwner, "tenant-admin-owned-daemon-issued-signing-keys");
assert.equal(policy.verifierKeyDistribution, "managed-relay-public-verifier-keys-only");
assert.equal(policy.keyMaterialBoundary, "private-signing-keys-never-enter-managed-relay");
assert.equal(policy.rotationPolicy, "overlapping-key-id-versions-with-explicit-retirement");
assert.equal(policy.revocationPolicy, "revoked-key-ids-stop-new-session-registration");
assert.equal(policy.auditBoundary, "key-id-version-events-without-private-key-material");
assert.equal(policy.nextLocalSlice, "managed-relay-support-redaction-and-access-review-evidence");

for (const state of ["pending", "active", "rotating", "retiring", "revoked"]) {
  assert.ok(policy.requiredKeyStates.includes(state), `managed verifier key policy missing state: ${state}`);
}

for (const operation of [
  "register-public-verifier-key",
  "activate-key-version",
  "rotate-with-overlap-window",
  "revoke-key-id",
  "reject-retired-key-registration",
  "audit-key-version-change",
]) {
  assert.ok(
    policy.requiredKeyOperations.includes(operation),
    `managed verifier key policy missing operation: ${operation}`,
  );
}

for (const prohibited of [
  "private_signing_key",
  "hmac_secret",
  "raw_session_token",
  "payload_json",
  "approval_signature_payload",
]) {
  assert.ok(
    policy.prohibitedVerifierKeyData.includes(prohibited),
    `managed verifier key policy missing prohibited data marker: ${prohibited}`,
  );
}

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "public_verifier_keys_only_in_relay_service",
  "private_signing_keys_never_leave_daemon_or_tenant_admin",
  "key_id_version_required_for_tickets",
  "revoked_keys_fail_closed_for_new_sessions",
  "key_rotation_requires_overlap_window",
]) {
  assert.ok(policy.guardrails.includes(guardrail), `managed verifier key policy missing guardrail: ${guardrail}`);
}

assert.equal(policy.keyRotationRequirements.overlapWindowHours, 24);
assert.equal(policy.keyRotationRequirements.maxActiveKeysPerTenant, 2);
assert.equal(policy.keyRotationRequirements.keyIdRequired, true);
assert.equal(policy.keyRotationRequirements.keyVersionRequired, true);
assert.equal(policy.keyRotationRequirements.revokedKeyRegistration, "fail-closed");
assert.equal(policy.keyRotationRequirements.oldKeyRetirement, "no-new-sessions-after-retirement");

for (const requirement of [
  "publish-public-verifier-key-by-tenant-and-key-id",
  "pin-ticket-key-id-and-version",
  "validate-session-ticket-against-active-key-version",
  "remove-private-key-material-from-service-config",
  "record-key-version-audit-events",
  "propagate-revocation-before-runtime",
]) {
  assert.ok(
    policy.distributionRequirements.includes(requirement),
    `managed verifier key policy missing distribution requirement: ${requirement}`,
  );
}

assert.equal(policy.trustBoundaries.tenantAdmin, "owns-key-registration-rotation-and-revocation");
assert.equal(policy.trustBoundaries.daemonOwner, "issues-session-tickets-with-current-key-id-version");
assert.equal(policy.trustBoundaries.managedRelayService, "verifies-public-keys-only");
assert.equal(policy.trustBoundaries.supportOperator, "sees-key-id-version-state-only");

for (const blocker of [
  "managed_key_registry_runtime_missing",
  "key_revocation_propagation_smoke_missing",
  "rotation_overlap_smoke_missing",
]) {
  assert.ok(policy.implementationBlockers.includes(blocker), `managed verifier key policy missing blocker: ${blocker}`);
}

assert.ok(policy.completedFollowupContracts.includes("billing-and-quota-policy"));

const evidence = {
  status: "policy",
  generatedAt: new Date().toISOString(),
  objective: "Define managed relay verifier-key ownership, rotation, distribution, and revocation policy",
  policy: {
    deploymentMode: policy.deploymentMode,
    readiness: policy.readiness,
    selectedRuntime: policy.selectedRuntime,
    verifierKeyOwner: policy.verifierKeyOwner,
    verifierKeyDistribution: policy.verifierKeyDistribution,
    keyMaterialBoundary: policy.keyMaterialBoundary,
    rotationPolicy: policy.rotationPolicy,
    revocationPolicy: policy.revocationPolicy,
    auditBoundary: policy.auditBoundary,
  },
  requiredKeyStates: policy.requiredKeyStates,
  requiredKeyOperations: policy.requiredKeyOperations,
  prohibitedVerifierKeyData: policy.prohibitedVerifierKeyData,
  keyRotationRequirements: policy.keyRotationRequirements,
  distributionRequirements: policy.distributionRequirements,
  trustBoundaries: policy.trustBoundaries,
  guardrails: policy.guardrails,
  implementationBlockers: policy.implementationBlockers,
  completedFollowupContracts: policy.completedFollowupContracts,
  nextLocalSlice: policy.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_VERIFIER_KEY_OPERATIONS_POLICY_OK ${evidencePath}`);
