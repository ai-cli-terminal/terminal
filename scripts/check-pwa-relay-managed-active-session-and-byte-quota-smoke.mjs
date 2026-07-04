import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayActiveSessionAndByteQuotaState,
  evaluateManagedRelayActiveSessionAndByteQuota,
  relayManagedActiveSessionAndByteQuotaSmoke,
  relayManagedRuntimeReadinessGate,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-active-session-and-byte-quota-smoke",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_ACTIVE_SESSION_AND_BYTE_QUOTA_SMOKE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-active-session-and-byte-quota-smoke.json",
  );

const smoke = relayManagedActiveSessionAndByteQuotaSmoke();
const gate = relayManagedRuntimeReadinessGate();

assert.equal(smoke.deploymentMode, "managed");
assert.equal(smoke.readiness, "smoke");
assert.equal(smoke.productDefault, "live-loopback");
assert.equal(smoke.selectedRuntime, "deferred");
assert.equal(
  smoke.implementationStatus,
  "active-session-and-byte-quota-smoke-ready-runtime-still-deferred",
);
assert.equal(
  smoke.quotaBoundary,
  "tenant-and-daemon-active-session-plus-frame-byte-preflight",
);
assert.equal(smoke.implementationCanStart, true);
assert.equal(smoke.nextLocalSlice, "managed-relay-runtime-encrypted-frame-routing");
assert.ok(smoke.completedRuntimeEvidence.includes("active-session-and-byte-quota-smoke"));
assert.ok(smoke.closedReadinessBlockers.includes("managed_usage_meter_runtime_missing"));
assert.equal(
  smoke.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"),
  false,
);
assert.equal(smoke.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.equal(smoke.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);

assert.ok(gate.completedRuntimeEvidence.includes("active-session-and-byte-quota-smoke"));
assert.equal(
  gate.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"),
  false,
);
assert.ok(gate.completedRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"), false);
assert.equal(gate.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.equal(gate.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("managed_usage_meter_runtime_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("managed_usage_meter_runtime_missing"), false);
assert.ok(gate.resolvedRuntimeBlockers.includes("tenant_usage_export_smoke_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("tenant_usage_export_smoke_missing"), false);
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
  "tenant_active_session_quota_checked_before_activation",
  "daemon_device_active_session_quota_checked_before_activation",
  "frame_and_byte_quota_checked_before_route",
  "quota_denials_fail_closed_before_frame_routing",
  "usage_meter_deltas_exclude_payloads_and_secrets",
  "billing_meters_record_frame_and_byte_counts_without_payloads",
  "abuse_rate_limit_signals_remain_separate_from_billing_meters",
]) {
  assert.ok(smoke.guardrails.includes(guardrail), `active quota smoke missing guardrail: ${guardrail}`);
}

for (const field of [
  "tenant_id",
  "daemon_device_id",
  "window_start_ms",
  "window_end_ms",
  "tenant_active_session_limit",
  "tenant_active_sessions",
  "daemon_device_active_session_limit",
  "daemon_device_active_sessions",
  "relay_frame_limit",
  "relay_frames_used",
  "relay_byte_limit",
  "relay_bytes_used",
  "billing_meter",
  "abuse_signals",
]) {
  assert.ok(
    smoke.quotaContract.quotaStateFields.includes(field),
    `active quota smoke missing quota state field: ${field}`,
  );
}

for (const field of [
  "tenant_id",
  "session_id",
  "daemon_device_id",
  "verifier_key_id",
  "verifier_key_version",
  "frame_sequence",
  "payload_ciphertext_bytes",
]) {
  assert.ok(
    smoke.quotaContract.routeRequestFields.includes(field),
    `active quota smoke missing route field: ${field}`,
  );
}

for (const evidence of [
  "within-limit-frame-route-accepted-before-routing",
  "tenant-active-session-limit-rejects-before-session-activation",
  "daemon-device-active-session-limit-rejects-before-session-activation",
  "relay-frame-limit-rejects-before-routing",
  "relay-byte-limit-rejects-before-routing",
  "quota-denial-audit-excludes-payloads-and-secrets",
  "accepted-routes-increment-active-frame-byte-billing-meters",
  "quota-denials-increment-billing-meter-not-abuse-rate-limit",
]) {
  assert.ok(smoke.smokeEvidence.includes(evidence), `active quota smoke missing evidence: ${evidence}`);
}

const quotaState = createManagedRelayActiveSessionAndByteQuotaState({
  tenantId: "tenant-managed-active",
  daemonDeviceId: "daemon-managed-active-1",
  windowStartMs: 1000,
  windowEndMs: 2000,
  tenantActiveSessionLimit: 3,
  tenantActiveSessions: 2,
  daemonDeviceActiveSessionLimit: 2,
  daemonDeviceActiveSessions: 1,
  relayFrameLimit: 5,
  relayFramesUsed: 4,
  relayByteLimit: 1000,
  relayBytesUsed: 800,
  billingMeter: {
    active_session_count: 2,
    relay_frame_count: 4,
    relay_byte_count: 800,
    quota_denial_count: 0,
  },
  abuseSignals: {
    rate_limit_denial_count: 0,
    invalid_ticket_count: 1,
  },
});
const route = {
  tenant_id: "tenant-managed-active",
  session_id: "managed-active-quota-session-1",
  daemon_device_id: "daemon-managed-active-1",
  verifier_key_id: "managed-active-key-1",
  verifier_key_version: 4,
  frame_sequence: 5,
  payload_ciphertext_bytes: 100,
};

const accepted = evaluateManagedRelayActiveSessionAndByteQuota(quotaState, route, 1500);
assert.equal(accepted.decision, "accept");
assert.equal(accepted.relayAllowed, true);
assert.equal(accepted.reason, "within-active-session-and-byte-quota");
assert.equal(accepted.billingMeterDelta.active_session_count, 1);
assert.equal(accepted.billingMeterDelta.relay_frame_count, 1);
assert.equal(accepted.billingMeterDelta.relay_byte_count, 100);
assert.equal(accepted.billingMeterDelta.quota_denial_count, 0);
assert.equal(accepted.abuseSignalDelta.rate_limit_denial_count, 0);
assert.equal(accepted.abuseSignalDelta.invalid_ticket_count, 0);
assert.equal(accepted.auditEvent.relay_bytes_after_decision, 900);

const rejectCases = [
  [
    "tenant-active-session-quota-exceeded",
    {
      tenantActiveSessionLimit: 3,
      tenantActiveSessions: 3,
      daemonDeviceActiveSessionLimit: 2,
      daemonDeviceActiveSessions: 1,
      relayFrameLimit: 5,
      relayFramesUsed: 4,
      relayByteLimit: 1000,
      relayBytesUsed: 800,
    },
    1500,
  ],
  [
    "daemon-device-active-session-quota-exceeded",
    {
      tenantActiveSessionLimit: 3,
      tenantActiveSessions: 2,
      daemonDeviceActiveSessionLimit: 1,
      daemonDeviceActiveSessions: 1,
      relayFrameLimit: 5,
      relayFramesUsed: 4,
      relayByteLimit: 1000,
      relayBytesUsed: 800,
    },
    1500,
  ],
  [
    "relay-frame-quota-exceeded",
    {
      tenantActiveSessionLimit: 3,
      tenantActiveSessions: 2,
      daemonDeviceActiveSessionLimit: 2,
      daemonDeviceActiveSessions: 1,
      relayFrameLimit: 5,
      relayFramesUsed: 5,
      relayByteLimit: 1000,
      relayBytesUsed: 800,
    },
    1500,
  ],
  [
    "relay-byte-quota-exceeded",
    {
      tenantActiveSessionLimit: 3,
      tenantActiveSessions: 2,
      daemonDeviceActiveSessionLimit: 2,
      daemonDeviceActiveSessions: 1,
      relayFrameLimit: 5,
      relayFramesUsed: 4,
      relayByteLimit: 850,
      relayBytesUsed: 800,
    },
    1500,
  ],
  [
    "quota-window-not-effective",
    {
      tenantActiveSessionLimit: 3,
      tenantActiveSessions: 2,
      daemonDeviceActiveSessionLimit: 2,
      daemonDeviceActiveSessions: 1,
      relayFrameLimit: 5,
      relayFramesUsed: 4,
      relayByteLimit: 1000,
      relayBytesUsed: 800,
    },
    900,
  ],
];

for (const [reason, overrides, nowMs] of rejectCases) {
  const rejected = evaluateManagedRelayActiveSessionAndByteQuota(
    createManagedRelayActiveSessionAndByteQuotaState({
      tenantId: "tenant-managed-active",
      daemonDeviceId: "daemon-managed-active-1",
      windowStartMs: 1000,
      windowEndMs: 2000,
      ...overrides,
    }),
    {
      ...route,
      session_id: `managed-active-quota-${reason}`,
    },
    nowMs,
  );
  assert.equal(rejected.decision, "reject");
  assert.equal(rejected.relayAllowed, false);
  assert.equal(rejected.reason, reason);
  assert.equal(rejected.billingMeterDelta.active_session_count, 0);
  assert.equal(rejected.billingMeterDelta.relay_frame_count, 0);
  assert.equal(rejected.billingMeterDelta.relay_byte_count, 0);
  assert.equal(rejected.billingMeterDelta.quota_denial_count, 1);
  assert.equal(rejected.abuseSignalDelta.rate_limit_denial_count, 0);
  assert.equal(rejected.abuseSignalDelta.invalid_ticket_count, 0);
}

assert.throws(
  () =>
    evaluateManagedRelayActiveSessionAndByteQuota(
      quotaState,
      {
        ...route,
        payload_json: { command: "not allowed" },
      },
      1500,
    ),
  /prohibited payload or secret data/,
);

const auditJson = JSON.stringify(accepted.auditEvent);
for (const prohibited of ["payload_json", "command_text", "session_token", "secret", "mac_hex"]) {
  assert.equal(auditJson.includes(prohibited), false);
}

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay active session and byte quota smoke before runtime implementation",
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
  guardrails: smoke.guardrails,
  quotaContract: smoke.quotaContract,
  accepted: {
    decision: accepted.decision,
    reason: accepted.reason,
    billingMeterDelta: accepted.billingMeterDelta,
    abuseSignalDelta: accepted.abuseSignalDelta,
  },
  rejectReasons: rejectCases.map(([reason]) => reason),
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
console.log(`RA_PWA_RELAY_MANAGED_ACTIVE_SESSION_AND_BYTE_QUOTA_SMOKE_OK ${evidencePath}`);
