import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayTenantAggregateUsageExport,
  relayManagedRuntimeReadinessGate,
  relayManagedTenantAggregateUsageExportSmoke,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-tenant-aggregate-usage-export-smoke",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_TENANT_AGGREGATE_USAGE_EXPORT_SMOKE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-tenant-aggregate-usage-export-smoke.json",
  );

const smoke = relayManagedTenantAggregateUsageExportSmoke();
const gate = relayManagedRuntimeReadinessGate();

assert.equal(smoke.deploymentMode, "managed");
assert.equal(smoke.readiness, "smoke");
assert.equal(smoke.productDefault, "live-loopback");
assert.equal(smoke.selectedRuntime, "deferred");
assert.equal(
  smoke.implementationStatus,
  "tenant-aggregate-usage-export-smoke-ready-runtime-still-deferred",
);
assert.equal(
  smoke.exportBoundary,
  "tenant-aggregate-usage-counters-without-payloads-or-secrets",
);
assert.equal(smoke.implementationCanStart, true);
assert.equal(smoke.nextLocalSlice, "managed-relay-runtime-support-and-abuse-operations-integration");
assert.ok(smoke.completedRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"));
assert.ok(smoke.closedReadinessBlockers.includes("tenant_usage_export_smoke_missing"));
assert.equal(
  smoke.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"),
  false,
);
assert.equal(smoke.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);

assert.ok(gate.completedRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"));
assert.equal(
  gate.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"),
  false,
);
assert.equal(gate.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.equal(gate.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("tenant_usage_export_smoke_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("tenant_usage_export_smoke_missing"), false);
assert.ok(
  gate.runtimeReadinessDomains.quotaAndUsage.completedEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "tenant_usage_export_is_aggregate_only",
  "tenant_usage_export_excludes_payloads_and_secrets",
  "session_active_frame_byte_and_quota_denial_counters_exported",
  "billing_usage_and_abuse_signals_are_separate_sections",
  "support_views_remain_aggregate_only",
]) {
  assert.ok(smoke.guardrails.includes(guardrail), `tenant usage export missing guardrail: ${guardrail}`);
}

for (const field of [
  "session_registration_count",
  "active_session_count",
  "relay_frame_count",
  "relay_byte_count",
  "invalid_ticket_count",
  "quota_denial_count",
]) {
  assert.ok(
    smoke.exportContract.billingUsageFields.includes(field),
    `tenant usage export missing billing field: ${field}`,
  );
}

for (const field of [
  "rate_limit_denial_count",
  "invalid_ticket_count",
  "abuse_case_count",
]) {
  assert.ok(
    smoke.exportContract.abuseSignalFields.includes(field),
    `tenant usage export missing abuse field: ${field}`,
  );
}

const usageExport = createManagedRelayTenantAggregateUsageExport({
  tenantId: "tenant-managed-usage",
  windowStartMs: 1000,
  windowEndMs: 2000,
  generatedAtMs: 2100,
  planId: "managed-relay-plan-a",
  billingMeter: {
    session_registration_count: 7,
    active_session_count: 3,
    relay_frame_count: 23,
    relay_byte_count: 4096,
    invalid_ticket_count: 2,
    quota_denial_count: 4,
  },
  abuseSignals: {
    rate_limit_denial_count: 5,
    invalid_ticket_count: 2,
    abuse_case_count: 1,
  },
});

assert.equal(usageExport.export_scope, "tenant-aggregate-usage");
assert.equal(usageExport.tenant_id, "tenant-managed-usage");
assert.equal(usageExport.payload_visibility, "payload-free");
assert.equal(usageExport.support_visibility, "aggregate-only");
assert.equal(usageExport.billing_usage.session_registration_count, 7);
assert.equal(usageExport.billing_usage.active_session_count, 3);
assert.equal(usageExport.billing_usage.relay_frame_count, 23);
assert.equal(usageExport.billing_usage.relay_byte_count, 4096);
assert.equal(usageExport.billing_usage.quota_denial_count, 4);
assert.equal(usageExport.abuse_signal_summary.rate_limit_denial_count, 5);
assert.equal(usageExport.billing_abuse_boundary.abuse_signals_are_not_billing_meters, true);
assert.ok(usageExport.billing_abuse_boundary.billing_usage_fields.includes("relay_byte_count"));
assert.ok(
  usageExport.billing_abuse_boundary.abuse_signal_fields.includes(
    "rate_limit_denial_count",
  ),
);

assert.throws(
  () =>
    createManagedRelayTenantAggregateUsageExport({
      tenantId: "tenant-managed-usage",
      windowStartMs: 1000,
      windowEndMs: 2000,
      generatedAtMs: 2100,
      planId: "managed-relay-plan-a",
      billingMeter: {
        session_registration_count: 1,
      },
      payload_json: { command: "not allowed" },
    }),
  /prohibited payload or secret data/,
);

const exportJson = JSON.stringify(usageExport);
for (const prohibited of ["payload_json", "command_text", "session_token", "secret", "mac_hex"]) {
  assert.equal(exportJson.includes(prohibited), false);
}

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay tenant aggregate usage export before runtime implementation",
  smoke: {
    deploymentMode: smoke.deploymentMode,
    readiness: smoke.readiness,
    selectedRuntime: smoke.selectedRuntime,
    implementationStatus: smoke.implementationStatus,
    exportBoundary: smoke.exportBoundary,
    implementationCanStart: smoke.implementationCanStart,
  },
  completedRuntimeEvidence: smoke.completedRuntimeEvidence,
  closedReadinessBlockers: smoke.closedReadinessBlockers,
  remainingRuntimeEvidence: smoke.remainingRuntimeEvidence,
  remainingRuntimeBlockers: smoke.remainingRuntimeBlockers,
  guardrails: smoke.guardrails,
  exportContract: smoke.exportContract,
  usageExport,
  gate: {
    completedRuntimeEvidence: gate.completedRuntimeEvidence,
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    resolvedRuntimeBlockers: gate.resolvedRuntimeBlockers,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
  },
  productDefault: smoke.productDefault,
  nextLocalSlice: smoke.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_TENANT_AGGREGATE_USAGE_EXPORT_SMOKE_OK ${evidencePath}`);
