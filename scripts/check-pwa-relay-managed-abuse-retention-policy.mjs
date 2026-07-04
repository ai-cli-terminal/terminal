import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { relayManagedAbuseRetentionPolicy } from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-managed-abuse-retention-policy");
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_ABUSE_RETENTION_POLICY_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-abuse-retention-policy.json");

const policy = relayManagedAbuseRetentionPolicy();

assert.equal(policy.deploymentMode, "managed");
assert.equal(policy.readiness, "policy");
assert.equal(policy.productDefault, "live-loopback");
assert.equal(policy.selectedRuntime, "deferred");
assert.equal(policy.abuseHandling, "tenant-scoped-rate-limits-and-operator-escalation");
assert.equal(policy.retentionBoundary, "aggregate-audit-only-no-payload-json");
assert.equal(policy.deletionBoundary, "tenant-and-session-metadata-deletion-required");
assert.equal(policy.supportBoundary, "audited-aggregate-only-support-workflows");
assert.equal(policy.enforcementDefault, "fail-closed-before-managed-runtime");
assert.equal(policy.nextLocalSlice, "managed-relay-runtime-encrypted-frame-routing");

for (const scope of ["tenant", "daemon-device", "session", "source-ip", "verifier-key"]) {
  assert.ok(policy.rateLimitScopes.includes(scope), `managed abuse policy missing scope: ${scope}`);
}

for (const signal of [
  "invalid-ticket-rate",
  "session-registration-failure-rate",
  "frame-replay-or-duplicate-sequence-rate",
  "expired-frame-drop-rate",
  "tenant-quota-exhaustion",
]) {
  assert.ok(policy.abuseSignals.includes(signal), `managed abuse policy missing signal: ${signal}`);
}

for (const deletionRequirement of [
  "tenant-deletion-removes-session-metadata",
  "verifier-key-revocation-stops-new-sessions",
  "support-export-excludes-payloads-and-secrets",
  "retention-expiry-purges-audit-and-case-metadata",
]) {
  assert.ok(
    policy.deletionRequirements.includes(deletionRequirement),
    `managed retention policy missing deletion requirement: ${deletionRequirement}`,
  );
}

for (const constraint of [
  "support-access-audited",
  "tenant-admin-approval-required",
  "aggregate-state-only",
  "no-payload-json-or-secret-material",
  "breakglass-time-bounded",
]) {
  assert.ok(
    policy.supportWorkflowConstraints.includes(constraint),
    `managed support policy missing constraint: ${constraint}`,
  );
}

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "tenant_scoped_abuse_limits_required",
  "no_payload_or_secret_retention",
  "support_access_requires_audit_and_tenant_scope",
  "deletion_requirements_before_runtime",
]) {
  assert.ok(policy.guardrails.includes(guardrail), `managed abuse policy missing guardrail: ${guardrail}`);
}

assert.equal(policy.retentionWindows.healthAggregatesDays, 30);
assert.equal(policy.retentionWindows.controlPlaneAuditDays, 90);
assert.equal(policy.retentionWindows.abuseCaseMetadataDays, 180);
assert.equal(policy.retentionWindows.supportCaseMetadataDays, 90);
assert.equal(policy.retentionWindows.payloadJson, "not-retained");
assert.equal(policy.retentionWindows.sessionTokens, "not-retained");
assert.equal(policy.retentionWindows.approvalSignatures, "not-retained");
assert.equal(policy.retentionWindows.privateKeyMaterial, "not-retained");
assert.equal(policy.retentionWindows.hmacSecrets, "not-retained");
assert.equal(policy.retentionWindows.fullSetupJson, "not-retained");

assert.ok(policy.requiredBeforeRuntime.includes("payload-confidentiality-plan"));
assert.ok(policy.completedFollowupContracts.includes("payload-confidentiality-plan"));
assert.ok(policy.blockers.includes("support_access_review_missing"));

const evidence = {
  status: "policy",
  generatedAt: new Date().toISOString(),
  objective: "Define managed relay abuse handling, retention, deletion, and support workflow policy",
  policy: {
    deploymentMode: policy.deploymentMode,
    readiness: policy.readiness,
    selectedRuntime: policy.selectedRuntime,
    abuseHandling: policy.abuseHandling,
    retentionBoundary: policy.retentionBoundary,
    deletionBoundary: policy.deletionBoundary,
    supportBoundary: policy.supportBoundary,
    enforcementDefault: policy.enforcementDefault,
  },
  rateLimitScopes: policy.rateLimitScopes,
  abuseSignals: policy.abuseSignals,
  retentionWindows: policy.retentionWindows,
  deletionRequirements: policy.deletionRequirements,
  supportWorkflowConstraints: policy.supportWorkflowConstraints,
  completedFollowupContracts: policy.completedFollowupContracts,
  guardrails: policy.guardrails,
  blockers: policy.blockers,
  nextLocalSlice: policy.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_ABUSE_RETENTION_POLICY_OK ${evidencePath}`);
