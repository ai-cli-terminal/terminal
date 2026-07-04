import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { relayManagedBillingQuotaPolicy } from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-managed-billing-quota-policy");
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_BILLING_QUOTA_POLICY_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-billing-quota-policy.json");

const policy = relayManagedBillingQuotaPolicy();

assert.equal(policy.deploymentMode, "managed");
assert.equal(policy.readiness, "policy");
assert.equal(policy.productDefault, "live-loopback");
assert.equal(policy.selectedRuntime, "deferred");
assert.equal(policy.billingModel, "tenant-scoped-metered-usage-before-managed-runtime");
assert.equal(policy.quotaEnforcement, "tenant-and-session-quota-fail-closed-before-runtime");
assert.equal(policy.usageVisibility, "tenant-aggregate-usage-no-payload-or-secret-data");
assert.equal(policy.quotaOwner, "tenant-admin-owned-service-enforced-limits");
assert.equal(policy.billingBoundary, "control-plane-usage-metadata-only");
assert.equal(policy.nextLocalSlice, "managed-relay-payload-blind-frame-encryption-spike");

for (const scope of ["tenant", "daemon-device", "session", "verifier-key", "source-ip"]) {
  assert.ok(policy.requiredQuotaScopes.includes(scope), `managed billing policy missing scope: ${scope}`);
}

for (const dimension of [
  "session-registration-count",
  "active-session-count",
  "relay-frame-count",
  "relay-byte-count",
  "invalid-ticket-count",
  "quota-denial-count",
]) {
  assert.ok(
    policy.meteredUsageDimensions.includes(dimension),
    `managed billing policy missing usage dimension: ${dimension}`,
  );
}

for (const prohibited of [
  "payload_json",
  "command_text",
  "context_json",
  "approval_response_payload",
  "private_key_material",
  "raw_session_token",
  "full_setup_json",
]) {
  assert.ok(
    policy.prohibitedBillingData.includes(prohibited),
    `managed billing policy missing prohibited data marker: ${prohibited}`,
  );
}

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "tenant_usage_metadata_only",
  "quota_enforcement_fail_closed",
  "billing_records_exclude_payloads_and_secrets",
  "quota_policy_required_before_runtime",
  "abuse_limits_remain_separate_from_billing",
]) {
  assert.ok(policy.guardrails.includes(guardrail), `managed billing policy missing guardrail: ${guardrail}`);
}

assert.equal(policy.quotaDefaults.tenantSessionRegistrationsPerHour, 1000);
assert.equal(policy.quotaDefaults.activeSessionsPerTenant, 100);
assert.equal(policy.quotaDefaults.activeSessionsPerDaemonDevice, 10);
assert.equal(policy.quotaDefaults.relayFrameBytesPerSession, 50 * 1024 * 1024);
assert.equal(policy.quotaDefaults.invalidTicketsPerTenantPerHour, 100);
assert.equal(policy.quotaDefaults.quotaExceededBehavior, "reject-new-session-or-frame");

for (const requirement of [
  "enforce-tenant-session-registration-quota",
  "enforce-active-session-quota",
  "enforce-frame-and-byte-quota",
  "record-quota-denial-audit-event",
  "separate-abuse-rate-limits-from-billing-meters",
  "export-tenant-aggregate-usage-without-payloads",
]) {
  assert.ok(
    policy.enforcementRequirements.includes(requirement),
    `managed billing policy missing enforcement requirement: ${requirement}`,
  );
}

for (const requirement of [
  "usage-rollups-retained-400-days",
  "raw-control-plane-meter-events-retained-90-days",
  "quota-denial-audit-retained-90-days",
  "payload-and-secret-data-not-retained",
]) {
  assert.ok(
    policy.retentionRequirements.includes(requirement),
    `managed billing policy missing retention requirement: ${requirement}`,
  );
}

assert.equal(policy.trustBoundaries.tenantAdmin, "reviews-tenant-usage-and-configures-plan-limits");
assert.equal(policy.trustBoundaries.serviceOperator, "enforces-aggregate-quotas-without-payload-access");
assert.equal(policy.trustBoundaries.supportOperator, "sees-tenant-aggregate-usage-only");
assert.equal(policy.trustBoundaries.managedRelayService, "meters-routing-events-and-quota-denials-only");

for (const blocker of [
  "managed_usage_meter_runtime_missing",
  "quota_enforcement_smoke_missing",
  "tenant_usage_export_smoke_missing",
  "billing_abuse_boundary_review_missing",
]) {
  assert.ok(policy.implementationBlockers.includes(blocker), `managed billing policy missing blocker: ${blocker}`);
}

const evidence = {
  status: "policy",
  generatedAt: new Date().toISOString(),
  objective: "Define managed relay billing, quota, usage metering, and tenant usage policy",
  policy: {
    deploymentMode: policy.deploymentMode,
    readiness: policy.readiness,
    selectedRuntime: policy.selectedRuntime,
    billingModel: policy.billingModel,
    quotaEnforcement: policy.quotaEnforcement,
    usageVisibility: policy.usageVisibility,
    quotaOwner: policy.quotaOwner,
    billingBoundary: policy.billingBoundary,
  },
  requiredQuotaScopes: policy.requiredQuotaScopes,
  meteredUsageDimensions: policy.meteredUsageDimensions,
  prohibitedBillingData: policy.prohibitedBillingData,
  quotaDefaults: policy.quotaDefaults,
  enforcementRequirements: policy.enforcementRequirements,
  retentionRequirements: policy.retentionRequirements,
  trustBoundaries: policy.trustBoundaries,
  guardrails: policy.guardrails,
  implementationBlockers: policy.implementationBlockers,
  nextLocalSlice: policy.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_BILLING_QUOTA_POLICY_OK ${evidencePath}`);
