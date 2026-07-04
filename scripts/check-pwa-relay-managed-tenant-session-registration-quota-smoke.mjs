import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayTenantSessionRegistrationQuotaState,
  evaluateManagedRelayTenantSessionRegistrationQuota,
  relayManagedRuntimeReadinessGate,
  relayManagedTenantSessionRegistrationQuotaSmoke,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-tenant-session-registration-quota-smoke",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_TENANT_SESSION_REGISTRATION_QUOTA_SMOKE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-tenant-session-registration-quota-smoke.json",
  );

const smoke = relayManagedTenantSessionRegistrationQuotaSmoke();
const gate = relayManagedRuntimeReadinessGate();

assert.equal(smoke.deploymentMode, "managed");
assert.equal(smoke.readiness, "smoke");
assert.equal(smoke.productDefault, "live-loopback");
assert.equal(smoke.selectedRuntime, "deferred");
assert.equal(
  smoke.implementationStatus,
  "tenant-session-registration-quota-smoke-ready-runtime-still-deferred",
);
assert.equal(smoke.quotaBoundary, "tenant-scoped-session-registration-preflight");
assert.equal(smoke.implementationCanStart, true);
assert.equal(smoke.nextLocalSlice, "managed-relay-runtime-implementation-plan");
assert.ok(smoke.completedRuntimeEvidence.includes("tenant-session-registration-quota-smoke"));
assert.ok(smoke.closedReadinessBlockers.includes("quota_enforcement_smoke_missing"));
assert.equal(
  smoke.remainingRuntimeEvidence.includes("tenant-session-registration-quota-smoke"),
  false,
);
assert.equal(smoke.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);

assert.ok(gate.completedRuntimeEvidence.includes("tenant-session-registration-quota-smoke"));
assert.equal(
  gate.remainingRuntimeEvidence.includes("tenant-session-registration-quota-smoke"),
  false,
);
assert.ok(gate.completedRuntimeEvidence.includes("active-session-and-byte-quota-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"), false);
assert.ok(gate.completedRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"), false);
assert.equal(gate.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.equal(gate.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("quota_enforcement_smoke_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("quota_enforcement_smoke_missing"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("managed_usage_meter_runtime_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("managed_usage_meter_runtime_missing"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("tenant_usage_export_smoke_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("tenant_usage_export_smoke_missing"), false);
assert.ok(
  gate.runtimeReadinessDomains.quotaAndUsage.completedEvidence.includes(
    "tenant-session-registration-quota-smoke",
  ),
);
assert.ok(
  gate.runtimeReadinessDomains.quotaAndUsage.completedEvidence.includes(
    "active-session-and-byte-quota-smoke",
  ),
);
assert.ok(
  gate.runtimeReadinessDomains.quotaAndUsage.completedEvidence.includes(
    "tenant-aggregate-usage-export-smoke",
  ),
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "session_registration_quota_checked_before_registration",
  "quota_denials_fail_closed_before_session_creation",
  "quota_denial_audit_excludes_payloads_and_secrets",
  "abuse_rate_limit_signals_remain_separate_from_billing_meters",
  "billing_meters_record_quota_denials_without_payloads",
]) {
  assert.ok(smoke.guardrails.includes(guardrail), `quota smoke missing guardrail: ${guardrail}`);
}

for (const field of [
  "tenant_id",
  "window_start_ms",
  "window_end_ms",
  "registration_limit",
  "registrations_used",
  "billing_meter",
  "abuse_signals",
]) {
  assert.ok(
    smoke.quotaContract.quotaStateFields.includes(field),
    `quota smoke missing quota state field: ${field}`,
  );
}

for (const field of [
  "tenant_id",
  "session_id",
  "daemon_device_id",
  "verifier_key_id",
  "verifier_key_version",
  "source_ip_hash",
]) {
  assert.ok(
    smoke.quotaContract.registrationRequestFields.includes(field),
    `quota smoke missing registration field: ${field}`,
  );
}

for (const evidence of [
  "within-limit-registration-accepted-before-session-creation",
  "tenant-registration-limit-rejects-new-session-before-registration",
  "not-yet-effective-quota-window-fails-closed",
  "quota-denial-audit-preserves-tenant-session-key-metadata",
  "quota-denial-audit-excludes-payloads-and-secrets",
  "quota-denials-increment-billing-meter-not-abuse-rate-limit",
]) {
  assert.ok(smoke.smokeEvidence.includes(evidence), `quota smoke missing evidence: ${evidence}`);
}

const quotaState = createManagedRelayTenantSessionRegistrationQuotaState({
  tenantId: "tenant-managed-quota",
  windowStartMs: 1000,
  windowEndMs: 2000,
  registrationLimit: 2,
  registrationsUsed: 1,
  billingMeter: {
    session_registration_count: 10,
    quota_denial_count: 0,
  },
  abuseSignals: {
    rate_limit_denial_count: 0,
    invalid_ticket_count: 1,
  },
});
const registration = {
  tenant_id: "tenant-managed-quota",
  session_id: "managed-quota-session-1",
  daemon_device_id: "daemon-managed-quota-1",
  verifier_key_id: "managed-quota-key-1",
  verifier_key_version: 3,
  source_ip_hash: "b".repeat(64),
};

const accepted = evaluateManagedRelayTenantSessionRegistrationQuota(
  quotaState,
  registration,
  1500,
);
assert.equal(accepted.decision, "accept");
assert.equal(accepted.registrationAllowed, true);
assert.equal(accepted.reason, "within-tenant-session-registration-quota");
assert.equal(accepted.billingMeterDelta.session_registration_count, 1);
assert.equal(accepted.billingMeterDelta.quota_denial_count, 0);
assert.equal(accepted.abuseSignalDelta.rate_limit_denial_count, 0);
assert.equal(accepted.abuseSignalDelta.invalid_ticket_count, 0);
assert.equal(accepted.auditEvent.tenant_id, registration.tenant_id);
assert.equal(accepted.auditEvent.session_id, registration.session_id);
assert.equal(accepted.auditEvent.verifier_key_id, registration.verifier_key_id);
assert.equal(accepted.auditEvent.verifier_key_version, registration.verifier_key_version);
assert.equal(accepted.auditEvent.quota_remaining_after_decision, 0);

const exhaustedQuotaState = createManagedRelayTenantSessionRegistrationQuotaState({
  tenantId: "tenant-managed-quota",
  windowStartMs: 1000,
  windowEndMs: 2000,
  registrationLimit: 2,
  registrationsUsed: 2,
  billingMeter: {
    session_registration_count: 11,
    quota_denial_count: 0,
  },
  abuseSignals: {
    rate_limit_denial_count: 0,
    invalid_ticket_count: 1,
  },
});
const rejected = evaluateManagedRelayTenantSessionRegistrationQuota(
  exhaustedQuotaState,
  registration,
  1500,
);
assert.equal(rejected.decision, "reject");
assert.equal(rejected.registrationAllowed, false);
assert.equal(rejected.reason, "tenant-session-registration-quota-exceeded");
assert.equal(rejected.billingMeterDelta.session_registration_count, 0);
assert.equal(rejected.billingMeterDelta.quota_denial_count, 1);
assert.equal(rejected.abuseSignalDelta.rate_limit_denial_count, 0);
assert.equal(rejected.abuseSignalDelta.invalid_ticket_count, 0);
assert.equal(rejected.auditEvent.decision, "reject");
assert.equal(rejected.auditEvent.reason, "tenant-session-registration-quota-exceeded");
assert.equal(rejected.auditEvent.quota_remaining_after_decision, 0);

const beforeWindow = evaluateManagedRelayTenantSessionRegistrationQuota(
  quotaState,
  registration,
  900,
);
assert.equal(beforeWindow.decision, "reject");
assert.equal(beforeWindow.reason, "quota-window-not-effective");
assert.equal(beforeWindow.billingMeterDelta.quota_denial_count, 1);
assert.equal(beforeWindow.abuseSignalDelta.rate_limit_denial_count, 0);

assert.throws(
  () =>
    evaluateManagedRelayTenantSessionRegistrationQuota(
      quotaState,
      {
        ...registration,
        payload_json: { command: "not allowed" },
      },
      1500,
    ),
  /prohibited/,
);

const acceptedAuditJson = JSON.stringify(accepted.auditEvent);
const rejectedAuditJson = JSON.stringify(rejected.auditEvent);
for (const prohibited of [
  "payload_json",
  "command_text",
  "context_json",
  "approval_response_payload",
  "private_key_material",
  "raw_session_token",
  "session_token",
  "signed_session_ticket",
  "full_setup_json",
  "hmac_secret",
  "secret",
  "mac_hex",
  "token_managed_quota_1234567890",
  "c".repeat(64),
]) {
  assert.equal(acceptedAuditJson.includes(prohibited), false);
  assert.equal(rejectedAuditJson.includes(prohibited), false);
}

const evidence = {
  status: "smoke-complete-runtime-deferred",
  generatedAt: new Date().toISOString(),
  objective:
    "Prove managed relay tenant session registration quota preflight rejects over-limit sessions before registration",
  smoke: {
    deploymentMode: smoke.deploymentMode,
    readiness: smoke.readiness,
    selectedRuntime: smoke.selectedRuntime,
    implementationStatus: smoke.implementationStatus,
    quotaBoundary: smoke.quotaBoundary,
    implementationCanStart: smoke.implementationCanStart,
  },
  completedRuntimeEvidence: smoke.completedRuntimeEvidence,
  closedReadinessBlockers: smoke.closedReadinessBlockers,
  remainingRuntimeEvidence: smoke.remainingRuntimeEvidence,
  remainingRuntimeBlockers: smoke.remainingRuntimeBlockers,
  quotaContract: smoke.quotaContract,
  quotaState,
  acceptedDecision: accepted,
  rejectedDecision: rejected,
  beforeWindowDecision: beforeWindow,
  smokeEvidence: smoke.smokeEvidence,
  guardrails: smoke.guardrails,
  productDefault: smoke.productDefault,
  nextLocalSlice: smoke.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_TENANT_SESSION_REGISTRATION_QUOTA_SMOKE_OK ${evidencePath}`);
