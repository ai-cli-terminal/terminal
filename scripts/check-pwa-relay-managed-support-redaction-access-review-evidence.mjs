import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelaySupportRedactionAccessReview,
  relayManagedRuntimeReadinessGate,
  relayManagedSupportRedactionAndAccessReviewEvidence,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-support-redaction-access-review-evidence",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_SUPPORT_REDACTION_ACCESS_REVIEW_EVIDENCE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-support-redaction-access-review-evidence.json",
  );

const evidenceSummary = relayManagedSupportRedactionAndAccessReviewEvidence();
const gate = relayManagedRuntimeReadinessGate();

assert.equal(evidenceSummary.deploymentMode, "managed");
assert.equal(evidenceSummary.readiness, "evidence");
assert.equal(evidenceSummary.productDefault, "live-loopback");
assert.equal(evidenceSummary.selectedRuntime, "deferred");
assert.equal(
  evidenceSummary.implementationStatus,
  "support-redaction-access-review-ready-runtime-still-deferred",
);
assert.equal(
  evidenceSummary.supportBoundary,
  "aggregate-redacted-support-view-with-audited-access",
);
assert.equal(evidenceSummary.implementationCanStart, true);
assert.equal(evidenceSummary.nextLocalSlice, "managed-relay-runtime-implementation-plan");
assert.ok(
  evidenceSummary.completedRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
);
assert.ok(evidenceSummary.closedReadinessBlockers.includes("support_audit_boundary_missing"));
assert.ok(evidenceSummary.closedReadinessBlockers.includes("support_access_review_missing"));
assert.ok(
  evidenceSummary.closedReadinessBlockers.includes("support_redaction_evidence_missing"),
);
assert.equal(
  evidenceSummary.remainingRuntimeEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
  false,
);
assert.equal(evidenceSummary.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);

assert.ok(gate.completedRuntimeEvidence.includes("support-redaction-and-access-review-evidence"));
assert.equal(
  gate.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"),
  false,
);
assert.equal(gate.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("support_audit_boundary_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("support_access_review_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("support_redaction_evidence_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("support_audit_boundary_missing"), false);
assert.equal(gate.remainingRuntimeBlockers.includes("support_access_review_missing"), false);
assert.equal(gate.remainingRuntimeBlockers.includes("support_redaction_evidence_missing"), false);
assert.ok(
  gate.runtimeReadinessDomains.abuseRetentionAndSupport.completedEvidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "support_views_are_aggregate_only",
  "support_identifiers_are_hashed",
  "support_access_requires_tenant_admin_approval",
  "support_access_is_time_bounded_and_audited",
  "support_views_exclude_payloads_secrets_tokens_and_raw_tickets",
]) {
  assert.ok(evidenceSummary.guardrails.includes(guardrail), `support evidence missing guardrail: ${guardrail}`);
}

for (const field of [
  "support_actor_id_hash",
  "session_id_hash",
  "daemon_device_id_hash",
  "companion_device_id_hash",
]) {
  assert.ok(
    evidenceSummary.supportAccessContract.hashedIdentifierFields.includes(field),
    `support evidence missing hashed field: ${field}`,
  );
}

const supportView = createManagedRelaySupportRedactionAccessReview({
  tenantId: "tenant-managed-support",
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
  billingUsage: {
    session_registration_count: 4,
    active_session_count: 2,
    relay_frame_count: 13,
    relay_byte_count: 2048,
    invalid_ticket_count: 1,
    quota_denial_count: 3,
  },
  abuseSignals: {
    rate_limit_denial_count: 2,
    invalid_ticket_count: 1,
    abuse_case_count: 0,
  },
});

assert.equal(supportView.support_visibility, "aggregate-only");
assert.equal(supportView.redaction_state, "redacted");
assert.equal(supportView.payload_visibility, "payload-free");
assert.equal(supportView.session_id_hash, "sha256:bbbbbbbbbbbbbbbb");
assert.equal(supportView.daemon_device_id_hash, "sha256:cccccccccccccccc");
assert.equal(supportView.companion_device_id_hash, "sha256:dddddddddddddddd");
assert.equal(supportView.billing_usage_summary.relay_byte_count, 2048);
assert.equal(supportView.billing_usage_summary.quota_denial_count, 3);
assert.equal(supportView.abuse_signal_summary.rate_limit_denial_count, 2);
assert.equal(supportView.access_review_audit.event_type, "support-access-reviewed");
assert.equal(supportView.access_review_audit.decision, "approved");
assert.equal(
  supportView.access_review_audit.tenant_admin_approval_id,
  "approval-2026-07",
);

for (const rawField of ["session_id", "daemon_device_id", "companion_device_id", "support_actor_id"]) {
  assert.equal(rawField in supportView, false);
}

assert.throws(
  () =>
    createManagedRelaySupportRedactionAccessReview({
      tenantId: "tenant-managed-support",
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
      session_id: "raw-session-not-allowed",
    }),
  /raw support identifier/,
);

assert.throws(
  () =>
    createManagedRelaySupportRedactionAccessReview({
      tenantId: "tenant-managed-support",
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
      payload_json: { command: "not allowed" },
    }),
  /prohibited payload or secret data/,
);

const supportViewJson = JSON.stringify(supportView);
for (const prohibited of ["payload_json", "command_text", "session_token", "secret", "mac_hex"]) {
  assert.equal(supportViewJson.includes(prohibited), false);
}

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay support redaction and access review evidence before runtime implementation",
  evidence: {
    deploymentMode: evidenceSummary.deploymentMode,
    readiness: evidenceSummary.readiness,
    selectedRuntime: evidenceSummary.selectedRuntime,
    implementationStatus: evidenceSummary.implementationStatus,
    supportBoundary: evidenceSummary.supportBoundary,
    implementationCanStart: evidenceSummary.implementationCanStart,
  },
  completedRuntimeEvidence: evidenceSummary.completedRuntimeEvidence,
  closedReadinessBlockers: evidenceSummary.closedReadinessBlockers,
  remainingRuntimeEvidence: evidenceSummary.remainingRuntimeEvidence,
  remainingRuntimeBlockers: evidenceSummary.remainingRuntimeBlockers,
  guardrails: evidenceSummary.guardrails,
  supportAccessContract: evidenceSummary.supportAccessContract,
  supportView,
  gate: {
    completedRuntimeEvidence: gate.completedRuntimeEvidence,
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    resolvedRuntimeBlockers: gate.resolvedRuntimeBlockers,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
  },
  productDefault: evidenceSummary.productDefault,
  nextLocalSlice: evidenceSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_SUPPORT_REDACTION_ACCESS_REVIEW_EVIDENCE_OK ${evidencePath}`);
