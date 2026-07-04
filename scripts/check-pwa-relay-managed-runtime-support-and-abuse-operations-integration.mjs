import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayBillingAbuseBoundaryReview,
  createManagedRelayRuntimeSupportAndAbuseOperationsIntegration,
  createManagedRelaySupportRedactionAccessReview,
  createManagedRelayTenantAggregateUsageExport,
  relayManagedBillingAbuseBoundaryReview,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimeQuotaAndMeteringIntegration,
  relayManagedRuntimeReadinessGate,
  relayManagedRuntimeSupportAndAbuseOperationsIntegration,
  relayManagedSupportRedactionAndAccessReviewEvidence,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-support-and-abuse-operations-integration",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_SUPPORT_AND_ABUSE_OPERATIONS_INTEGRATION_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-support-and-abuse-operations-integration.json",
  );

const gate = relayManagedRuntimeReadinessGate();
const plan = relayManagedRuntimeImplementationPlan();
const quotaSummary = relayManagedRuntimeQuotaAndMeteringIntegration();
const supportSummary = relayManagedSupportRedactionAndAccessReviewEvidence();
const billingAbuseSummary = relayManagedBillingAbuseBoundaryReview();
const integrationSummary = relayManagedRuntimeSupportAndAbuseOperationsIntegration();
const integration = createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
  serviceId: "managed-relay-runtime-support-abuse-check",
  generatedAtMs: 2000,
});

assert.equal(gate.gateStatus, "runtime-evidence-green");
assert.equal(gate.implementationCanStart, true);
assert.equal(gate.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");
assert.deepEqual(gate.remainingRuntimeEvidence, []);
assert.deepEqual(gate.remainingRuntimeBlockers, []);

assert.equal(plan.readiness, "plan");
assert.equal(plan.implementationCanStart, true);
assert.equal(plan.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");
assert.equal(plan.selectedRuntime, "deferred");
assert.equal(plan.selectedRuntimeCanChange, false);
assert.ok(
  plan.regressionChecks.includes(
    "check:pwa-relay-managed-runtime-support-and-abuse-operations-integration",
  ),
);

assert.equal(quotaSummary.deploymentMode, "managed");
assert.equal(quotaSummary.readiness, "integration");
assert.equal(quotaSummary.quotaRuntime, "active-session-frame-byte-metering-wired");
assert.equal(quotaSummary.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");

assert.equal(supportSummary.deploymentMode, "managed");
assert.equal(supportSummary.readiness, "evidence");
assert.equal(supportSummary.supportBoundary, "aggregate-redacted-support-view-with-audited-access");
assert.equal(supportSummary.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");
assert.ok(
  supportSummary.completedRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
);

assert.equal(billingAbuseSummary.deploymentMode, "managed");
assert.equal(billingAbuseSummary.readiness, "review");
assert.equal(
  billingAbuseSummary.billingAbuseBoundary,
  "billing-usage-and-abuse-signals-separate-non-reclassifiable",
);
assert.equal(billingAbuseSummary.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");
assert.ok(billingAbuseSummary.completedRuntimeEvidence.includes("billing-abuse-boundary-review"));

assert.equal(integrationSummary.deploymentMode, "managed");
assert.equal(integrationSummary.readiness, "integration");
assert.equal(integrationSummary.productDefault, "live-loopback");
assert.equal(integrationSummary.selectedRuntime, "deferred");
assert.equal(integrationSummary.runtimeDefault, "not-selected");
assert.equal(
  integrationSummary.implementationStatus,
  "managed-runtime-support-and-abuse-operations-integrated-no-pwa-exposure",
);
assert.equal(integrationSummary.supportRuntime, "support-redaction-access-review-wired");
assert.equal(integrationSummary.abuseRuntime, "billing-abuse-boundary-review-wired");
assert.equal(integrationSummary.implementationCanContinue, true);
assert.equal(integrationSummary.selectedRuntimeCanChange, false);
assert.equal(integrationSummary.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");
assert.ok(
  integrationSummary.completedImplementationEvidence.includes(
    "managed-runtime-support-and-abuse-operations-integration",
  ),
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "support_abuse_operations_have_no_pwa_exposure",
  "support_views_are_aggregate_only",
  "support_access_is_time_bounded_and_audited",
  "support_identifiers_are_hashed",
  "abuse_operations_are_not_billing_source_data",
  "abuse_counters_exclude_payloads_tokens_and_key_material",
  "billing_abuse_boundary_remains_non_reclassifiable",
  "rollback_to_live_loopback_required",
]) {
  assert.ok(integrationSummary.guardrails.includes(guardrail), `support abuse integration missing guardrail: ${guardrail}`);
}

assert.equal(integration.operations_version, 1);
assert.equal(integration.deployment_mode, "managed");
assert.equal(integration.readiness, "support-and-abuse-operations-integration");
assert.equal(integration.product_default, "live-loopback");
assert.equal(integration.selected_runtime, "deferred");
assert.equal(integration.runtime_default, "not-selected");
assert.equal(integration.endpoint_mode, "disabled");
assert.equal(integration.public_bind_enabled, false);
assert.equal(integration.pwa_exposure, "disabled");
assert.equal(integration.support_runtime, "support-redaction-access-review-wired");
assert.equal(integration.abuse_runtime, "billing-abuse-boundary-review-wired");
assert.equal(
  integration.support_operations_contract.runtime_visibility,
  "aggregate-only",
);
assert.equal(
  integration.support_operations_contract.identifier_policy,
  "hashed-identifiers-only",
);
assert.equal(
  integration.abuse_operations_contract.source_data_policy,
  "billing-meter-deltas-not-reclassified-as-abuse-source-data",
);
assert.equal(integration.operations_health.payload_visibility, "payload-free");
assert.equal(integration.operations_health.support_visibility, "aggregate-only");
assert.ok(integration.allowed_operations_fields.includes("support_actor_id_hash"));
assert.ok(integration.allowed_operations_fields.includes("rate_limit_denial_count"));
assert.ok(integration.prohibited_operations_fields.includes("payload_ciphertext_hex"));
const visibleOperationSurfaceJson = JSON.stringify({
  supportOperationsContract: {
    ...integration.support_operations_contract,
    prohibited_operation_fields: undefined,
  },
  abuseOperationsContract: {
    ...integration.abuse_operations_contract,
    prohibited_operation_fields: undefined,
  },
  operationsHealth: integration.operations_health,
  allowedOperationsFields: integration.allowed_operations_fields,
});
assert.equal(visibleOperationSurfaceJson.includes("payload_ciphertext_hex"), false);
assert.equal(visibleOperationSurfaceJson.includes('"support_actor_id"'), false);
assert.equal(visibleOperationSurfaceJson.includes("payload_key_hex"), false);

const tenantUsageExport = createManagedRelayTenantAggregateUsageExport({
  tenantId: "tenant-managed-runtime-ops",
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

const supportView = createManagedRelaySupportRedactionAccessReview({
  tenantId: "tenant-managed-runtime-ops",
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
  billingUsage: tenantUsageExport.billing_usage,
  abuseSignals: tenantUsageExport.abuse_signal_summary,
});

const boundaryReview = createManagedRelayBillingAbuseBoundaryReview({
  tenantId: "tenant-managed-runtime-ops",
  windowStartMs: 1000,
  windowEndMs: 2000,
  generatedAtMs: 2100,
  planId: "managed-relay-plan-a",
  billingUsage: tenantUsageExport.billing_usage,
  abuseSignals: tenantUsageExport.abuse_signal_summary,
  tenantUsageExport,
  supportView,
});

assert.equal(supportView.support_visibility, "aggregate-only");
assert.equal(supportView.redaction_state, "redacted");
assert.equal(supportView.payload_visibility, "payload-free");
assert.equal("support_actor_id" in supportView, false);
assert.equal(boundaryReview.support_evidence_boundary.support_evidence_is_not_billing_source, true);
assert.equal(boundaryReview.billing_abuse_boundary.abuse_signals_are_not_billing_meters, true);

assert.throws(
  () =>
    createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
      serviceId: "managed-relay-runtime-support-abuse-check",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
      serviceId: "managed-relay-runtime-support-abuse-check",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
      serviceId: "managed-relay-runtime-support-abuse-check",
      generatedAtMs: 2000,
      supportRuntime: "not-wired",
    }),
  /must wire support access review/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeSupportAndAbuseOperationsIntegration({
      serviceId: "managed-relay-runtime-support-abuse-check",
      generatedAtMs: 2000,
      abuseRuntime: "not-wired",
    }),
  /must wire billing abuse boundary review/,
);

assert.deepEqual(integrationSummary.remainingImplementationPhases, [
  "managed-runtime-pwa-exposure-gate",
]);

for (const evidenceCheck of [
  "managed-service-scaffold-complete",
  "control-plane-contract-wiring-complete",
  "encrypted-frame-routing-complete",
  "quota-and-metering-integration-complete",
  "support-redaction-access-review-regression-passed",
  "billing-abuse-boundary-review-regression-passed",
  "support-operations-view-is-aggregate-only",
  "abuse-operations-are-not-billing-source-data",
  "support-abuse-operations-exclude-payloads-and-raw-identifiers",
  "pwa-exposure-remains-disabled",
  "next-pwa-exposure-gate-slice-selected",
]) {
  assert.ok(integrationSummary.evidenceChecks.includes(evidenceCheck), `support abuse integration missing evidence: ${evidenceCheck}`);
}

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay runtime support and abuse operations integration without PWA exposure",
  gate: {
    gateStatus: gate.gateStatus,
    implementationCanStart: gate.implementationCanStart,
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
    nextLocalSlice: gate.nextLocalSlice,
  },
  implementationPlan: {
    readiness: plan.readiness,
    implementationStatus: plan.implementationStatus,
    selectedRuntime: plan.selectedRuntime,
    selectedRuntimeCanChange: plan.selectedRuntimeCanChange,
    regressionChecks: plan.regressionChecks,
  },
  quotaAndMetering: {
    readiness: quotaSummary.readiness,
    quotaRuntime: quotaSummary.quotaRuntime,
    startupContract: quotaSummary.startupContract,
  },
  supportEvidence: {
    readiness: supportSummary.readiness,
    supportBoundary: supportSummary.supportBoundary,
    supportAccessContract: supportSummary.supportAccessContract,
  },
  billingAbuseReview: {
    readiness: billingAbuseSummary.readiness,
    billingAbuseBoundary: billingAbuseSummary.billingAbuseBoundary,
    boundaryContract: billingAbuseSummary.boundaryContract,
  },
  supportAbuseIntegration: integration,
  supportView,
  boundaryReview,
  summary: {
    readiness: integrationSummary.readiness,
    implementationStatus: integrationSummary.implementationStatus,
    supportRuntime: integrationSummary.supportRuntime,
    abuseRuntime: integrationSummary.abuseRuntime,
    pwaExposureDecision: integrationSummary.pwaExposureDecision,
    implementationCanContinue: integrationSummary.implementationCanContinue,
    selectedRuntimeCanChange: integrationSummary.selectedRuntimeCanChange,
  },
  startupContract: integrationSummary.startupContract,
  supportOperationsContract: integrationSummary.supportOperationsContract,
  abuseOperationsContract: integrationSummary.abuseOperationsContract,
  healthSurface: integrationSummary.healthSurface,
  remainingImplementationPhases: integrationSummary.remainingImplementationPhases,
  evidenceChecks: integrationSummary.evidenceChecks,
  guardrails: integrationSummary.guardrails,
  productDefault: integrationSummary.productDefault,
  nextLocalSlice: integrationSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_SUPPORT_AND_ABUSE_OPERATIONS_INTEGRATION_OK ${evidencePath}`);
