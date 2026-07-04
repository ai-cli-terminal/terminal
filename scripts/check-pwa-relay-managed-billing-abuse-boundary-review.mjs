import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayBillingAbuseBoundaryReview,
  createManagedRelaySupportRedactionAccessReview,
  createManagedRelayTenantAggregateUsageExport,
  relayManagedBillingAbuseBoundaryReview,
  relayManagedRuntimeReadinessGate,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-billing-abuse-boundary-review",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_BILLING_ABUSE_BOUNDARY_REVIEW_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-billing-abuse-boundary-review.json");

const reviewSummary = relayManagedBillingAbuseBoundaryReview();
const gate = relayManagedRuntimeReadinessGate();

assert.equal(reviewSummary.deploymentMode, "managed");
assert.equal(reviewSummary.readiness, "review");
assert.equal(reviewSummary.productDefault, "live-loopback");
assert.equal(reviewSummary.selectedRuntime, "deferred");
assert.equal(
  reviewSummary.implementationStatus,
  "billing-abuse-boundary-review-ready-runtime-still-deferred",
);
assert.equal(
  reviewSummary.billingAbuseBoundary,
  "billing-usage-and-abuse-signals-separate-non-reclassifiable",
);
assert.equal(reviewSummary.implementationCanStart, true);
assert.equal(reviewSummary.nextLocalSlice, "managed-relay-runtime-quota-and-metering-integration");
assert.ok(reviewSummary.completedRuntimeEvidence.includes("billing-abuse-boundary-review"));
assert.ok(reviewSummary.closedReadinessBlockers.includes("billing_abuse_boundary_review_missing"));
assert.ok(reviewSummary.closedReadinessBlockers.includes("runtime_rate_limit_enforcement_missing"));
assert.ok(reviewSummary.closedReadinessBlockers.includes("abuse_escalation_runbook_missing"));
assert.ok(reviewSummary.closedReadinessBlockers.includes("tenant_deletion_workflow_missing"));
assert.deepEqual(reviewSummary.remainingRuntimeEvidence, []);
assert.deepEqual(reviewSummary.remainingRuntimeBlockers, []);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "billing_usage_and_abuse_signals_have_separate_sections",
  "support_evidence_is_not_a_billing_source",
  "abuse_escalation_is_case_metadata_only",
  "tenant_deletion_boundary_reviewed",
  "tenant_usage_export_remains_aggregate_only",
  "billing_abuse_review_excludes_payloads_tokens_and_key_material",
]) {
  assert.ok(reviewSummary.guardrails.includes(guardrail), `billing abuse review missing guardrail: ${guardrail}`);
}

assert.equal(gate.gateStatus, "runtime-evidence-green");
assert.equal(gate.implementationDecision, "managed-runtime-implementation-can-start");
assert.equal(gate.implementationCanStart, true);
assert.equal(gate.readinessDecision, "ready-for-managed-runtime-implementation");
assert.equal(gate.nextLocalSlice, "managed-relay-runtime-quota-and-metering-integration");
assert.ok(gate.completedRuntimeEvidence.includes("billing-abuse-boundary-review"));
assert.equal(gate.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);
assert.deepEqual(gate.remainingRuntimeEvidence, []);
assert.ok(gate.resolvedRuntimeBlockers.includes("billing_abuse_boundary_review_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("runtime_rate_limit_enforcement_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("abuse_escalation_runbook_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("tenant_deletion_workflow_missing"));
assert.deepEqual(gate.remainingRuntimeBlockers, []);
assert.ok(
  gate.runtimeReadinessDomains.abuseRetentionAndSupport.completedEvidence.includes(
    "billing-abuse-boundary-review",
  ),
);

const tenantUsageExport = createManagedRelayTenantAggregateUsageExport({
  tenantId: "tenant-managed-billing-abuse",
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
  tenantId: "tenant-managed-billing-abuse",
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
  tenantId: "tenant-managed-billing-abuse",
  windowStartMs: 1000,
  windowEndMs: 2000,
  generatedAtMs: 2100,
  planId: "managed-relay-plan-a",
  billingUsage: tenantUsageExport.billing_usage,
  abuseSignals: tenantUsageExport.abuse_signal_summary,
  tenantUsageExport,
  supportView,
});

assert.equal(boundaryReview.review_scope, "managed-relay-billing-abuse-boundary");
assert.equal(boundaryReview.billing_usage_summary.relay_byte_count, 4096);
assert.equal(boundaryReview.abuse_signal_summary.rate_limit_denial_count, 5);
assert.equal(boundaryReview.tenant_usage_export_boundary.payload_visibility, "payload-free");
assert.equal(boundaryReview.tenant_usage_export_boundary.support_visibility, "aggregate-only");
assert.equal(
  boundaryReview.tenant_usage_export_boundary.abuse_signals_are_not_billing_meters,
  true,
);
assert.equal(boundaryReview.support_evidence_boundary.redaction_state, "redacted");
assert.equal(boundaryReview.support_evidence_boundary.support_evidence_is_not_billing_source, true);
assert.equal(boundaryReview.boundary_decisions.tenant_deletion_workflow_reviewed, true);
assert.equal(boundaryReview.boundary_decisions.abuse_escalation_runbook_reviewed, true);
assert.ok(boundaryReview.billing_abuse_boundary.billing_usage_fields.includes("relay_byte_count"));
assert.ok(boundaryReview.billing_abuse_boundary.abuse_signal_fields.includes("rate_limit_denial_count"));
assert.ok(boundaryReview.billing_abuse_boundary.shared_review_fields.includes("invalid_ticket_count"));

assert.throws(
  () =>
    createManagedRelayBillingAbuseBoundaryReview({
      tenantId: "tenant-managed-billing-abuse",
      windowStartMs: 1000,
      windowEndMs: 2000,
      generatedAtMs: 2100,
      planId: "managed-relay-plan-a",
      billingUsage: {
        relay_byte_count: 4096,
        support_case_id: "support-case-2026-07",
      },
      abuseSignals: {
        rate_limit_denial_count: 5,
      },
      tenantUsageExport,
      supportView,
    }),
  /billing usage contains abuse or support data/,
);
assert.throws(
  () =>
    createManagedRelayBillingAbuseBoundaryReview({
      tenantId: "tenant-managed-billing-abuse",
      windowStartMs: 1000,
      windowEndMs: 2000,
      generatedAtMs: 2100,
      planId: "managed-relay-plan-a",
      billingUsage: {
        relay_byte_count: 4096,
      },
      abuseSignals: {
        relay_byte_count: 4096,
      },
      tenantUsageExport,
      supportView,
    }),
  /abuse signal contains billing data/,
);

const reviewJson = JSON.stringify(boundaryReview);
for (const prohibited of ["payload_json", "command_text", "session_token", "secret", "mac_hex"]) {
  assert.equal(reviewJson.includes(prohibited), false);
}

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay billing and abuse boundary review before runtime implementation",
  review: {
    deploymentMode: reviewSummary.deploymentMode,
    readiness: reviewSummary.readiness,
    selectedRuntime: reviewSummary.selectedRuntime,
    implementationStatus: reviewSummary.implementationStatus,
    billingAbuseBoundary: reviewSummary.billingAbuseBoundary,
    implementationCanStart: reviewSummary.implementationCanStart,
  },
  completedRuntimeEvidence: reviewSummary.completedRuntimeEvidence,
  closedReadinessBlockers: reviewSummary.closedReadinessBlockers,
  remainingRuntimeEvidence: reviewSummary.remainingRuntimeEvidence,
  remainingRuntimeBlockers: reviewSummary.remainingRuntimeBlockers,
  guardrails: reviewSummary.guardrails,
  boundaryContract: reviewSummary.boundaryContract,
  boundaryReview,
  gate: {
    gateStatus: gate.gateStatus,
    implementationDecision: gate.implementationDecision,
    implementationCanStart: gate.implementationCanStart,
    completedRuntimeEvidence: gate.completedRuntimeEvidence,
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    resolvedRuntimeBlockers: gate.resolvedRuntimeBlockers,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
  },
  productDefault: reviewSummary.productDefault,
  nextLocalSlice: reviewSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_BILLING_ABUSE_BOUNDARY_REVIEW_OK ${evidencePath}`);
