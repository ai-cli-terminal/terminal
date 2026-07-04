import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayActiveSessionAndByteQuotaState,
  createManagedRelayRuntimeQuotaAndMeteringIntegration,
  liveApprovalRequestMessage,
  managedRelayEncryptedFrameFromLiveMessage,
  relayManagedRuntimeEncryptedFrameRouting,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimeQuotaAndMeteringIntegration,
  relayManagedRuntimeReadinessGate,
  routeManagedRelayRuntimeQuotaMeteredFrame,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-quota-and-metering-integration",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_QUOTA_AND_METERING_INTEGRATION_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-quota-and-metering-integration.json",
  );

const gate = relayManagedRuntimeReadinessGate();
const plan = relayManagedRuntimeImplementationPlan();
const encryptedRoutingSummary = relayManagedRuntimeEncryptedFrameRouting();
const integrationSummary = relayManagedRuntimeQuotaAndMeteringIntegration();
const integration = createManagedRelayRuntimeQuotaAndMeteringIntegration({
  serviceId: "managed-relay-runtime-quota-metering-check",
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
    "check:pwa-relay-managed-runtime-quota-and-metering-integration",
  ),
);

assert.equal(encryptedRoutingSummary.deploymentMode, "managed");
assert.equal(encryptedRoutingSummary.readiness, "routing");
assert.equal(encryptedRoutingSummary.routeRuntime, "encrypted-frame-routing-wired");
assert.equal(encryptedRoutingSummary.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");

assert.equal(integrationSummary.deploymentMode, "managed");
assert.equal(integrationSummary.readiness, "integration");
assert.equal(integrationSummary.productDefault, "live-loopback");
assert.equal(integrationSummary.selectedRuntime, "deferred");
assert.equal(integrationSummary.runtimeDefault, "not-selected");
assert.equal(
  integrationSummary.implementationStatus,
  "managed-runtime-quota-and-metering-integrated-no-pwa-exposure",
);
assert.equal(
  integrationSummary.controlPlaneRuntime,
  "tenant-session-registration-contract-wired",
);
assert.equal(integrationSummary.routeRuntime, "encrypted-frame-routing-wired");
assert.equal(integrationSummary.quotaRuntime, "active-session-frame-byte-metering-wired");
assert.equal(integrationSummary.implementationCanContinue, true);
assert.equal(integrationSummary.selectedRuntimeCanChange, false);
assert.equal(integrationSummary.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");
assert.ok(
  integrationSummary.completedImplementationEvidence.includes(
    "managed-runtime-quota-and-metering-integration",
  ),
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "quota_metering_has_no_pwa_exposure",
  "active_session_quota_checked_before_frame_delivery",
  "frame_and_byte_quota_checked_before_frame_delivery",
  "quota_denials_fail_closed_before_route_delivery",
  "billing_meters_record_aggregate_counts_only",
  "abuse_signals_remain_separate_from_billing_meters",
  "payload_ciphertext_hex_excluded_from_metering_surface",
  "rollback_to_live_loopback_required",
]) {
  assert.ok(integrationSummary.guardrails.includes(guardrail), `quota metering missing guardrail: ${guardrail}`);
}

assert.equal(integration.metering_version, 1);
assert.equal(integration.deployment_mode, "managed");
assert.equal(integration.readiness, "quota-and-metering-integration");
assert.equal(integration.product_default, "live-loopback");
assert.equal(integration.selected_runtime, "deferred");
assert.equal(integration.runtime_default, "not-selected");
assert.equal(integration.endpoint_mode, "disabled");
assert.equal(integration.public_bind_enabled, false);
assert.equal(integration.pwa_exposure, "disabled");
assert.equal(integration.control_plane_runtime, "tenant-session-registration-contract-wired");
assert.equal(integration.route_runtime, "encrypted-frame-routing-wired");
assert.equal(integration.quota_runtime, "active-session-frame-byte-metering-wired");
assert.equal(integration.quota_metering_contract.decision_point, "before-encrypted-frame-delivery");
assert.ok(integration.quota_metering_contract.quota_scopes.includes("relay-byte-count"));
assert.ok(integration.quota_metering_contract.metered_usage_dimensions.includes("relay_byte_count"));
assert.equal(
  integration.quota_metering_contract.billing_abuse_boundary,
  "billing-meter-deltas-and-abuse-signal-deltas-are-separate",
);
assert.equal(integration.metering_health.payload_visibility, "opaque-ciphertext-metadata-only");
assert.equal(integration.metering_health.support_visibility, "aggregate-only");
assert.ok(integration.allowed_metering_fields.includes("billing_meter_delta"));
assert.ok(integration.prohibited_metering_fields.includes("payload_ciphertext_hex"));
assert.equal(JSON.stringify(integration.metering_health).includes("payload_ciphertext_hex"), false);
assert.equal(JSON.stringify(integration.quota_metering_contract).includes("payload_key_hex"), false);

const frame = await managedRelayEncryptedFrameFromLiveMessage(
  "managed-runtime-metered-check",
  "daemon",
  1,
  5000,
  35000,
  liveApprovalRequestMessage({
    approval_id: [109, 101, 116, 101, 114, 101, 100],
    nonce: Array.from({ length: 32 }, (_, index) => index + 2),
    command_masked: "deploy production --tenant metered",
    context_hash: "ctx-managed-metered-check",
    expires_at: 35000,
    device_epoch: 8,
  }),
  "48".repeat(32),
  { nonceHex: "49".repeat(12), webCrypto: webcrypto },
);
const quotaState = createManagedRelayActiveSessionAndByteQuotaState({
  tenantId: "tenant-runtime-metered",
  daemonDeviceId: "daemon-runtime-metered-1",
  windowStartMs: 5000,
  windowEndMs: 40000,
  tenantActiveSessionLimit: 4,
  tenantActiveSessions: 2,
  daemonDeviceActiveSessionLimit: 3,
  daemonDeviceActiveSessions: 1,
  relayFrameLimit: 8,
  relayFramesUsed: 3,
  relayByteLimit: 4096,
  relayBytesUsed: 512,
});
const routeMetadata = {
  tenantId: "tenant-runtime-metered",
  daemonDeviceId: "daemon-runtime-metered-1",
  verifierKeyId: "managed-meter-key-check",
  verifierKeyVersion: 2,
};
const accepted = routeManagedRelayRuntimeQuotaMeteredFrame(
  frame,
  quotaState,
  routeMetadata,
  { nowMs: 5100 },
);
assert.equal(accepted.route_decision, "accepted");
assert.equal(accepted.route_state, "quota-metered-encrypted-frame-routed");
assert.equal(accepted.route_delivery.frame_delivery, "encrypted-frame-forwarded-after-quota");
assert.equal(accepted.quota_decision.decision, "accept");
assert.equal(accepted.quota_decision.relay_allowed, true);
assert.equal(accepted.quota_decision.billing_meter_delta.active_session_count, 1);
assert.equal(accepted.quota_decision.billing_meter_delta.relay_frame_count, 1);
assert.equal(
  accepted.quota_decision.billing_meter_delta.relay_byte_count,
  accepted.route_envelope.payload_ciphertext_bytes,
);
assert.equal(accepted.quota_decision.billing_meter_delta.quota_denial_count, 0);
assert.equal(accepted.quota_decision.abuse_signal_delta.rate_limit_denial_count, 0);
assert.equal(accepted.metering_event.decision, "accept");
assert.equal(accepted.metering_event.payload_ciphertext_bytes, accepted.route_envelope.payload_ciphertext_bytes);
assert.equal(JSON.stringify(accepted).includes("payload_ciphertext_hex"), false);
assert.equal(JSON.stringify(accepted).includes("payload_nonce_hex"), false);
assert.equal(JSON.stringify(accepted).includes("payload_key_hex"), false);
assert.equal(JSON.stringify(accepted).includes("deploy production"), false);
assert.equal(JSON.stringify(accepted).includes("ctx-managed-metered-check"), false);

const rejected = routeManagedRelayRuntimeQuotaMeteredFrame(
  frame,
  createManagedRelayActiveSessionAndByteQuotaState({
    tenantId: "tenant-runtime-metered",
    daemonDeviceId: "daemon-runtime-metered-1",
    windowStartMs: 5000,
    windowEndMs: 40000,
    tenantActiveSessionLimit: 4,
    tenantActiveSessions: 2,
    daemonDeviceActiveSessionLimit: 3,
    daemonDeviceActiveSessions: 1,
    relayFrameLimit: 8,
    relayFramesUsed: 3,
    relayByteLimit: 520,
    relayBytesUsed: 512,
  }),
  routeMetadata,
  { nowMs: 5100 },
);
assert.equal(rejected.route_decision, "rejected");
assert.equal(rejected.route_state, "quota-rejected-before-frame-delivery");
assert.equal(rejected.route_delivery.frame_delivery, "not-delivered-quota-fail-closed");
assert.equal(rejected.route_delivery.decrypt_at, "not-delivered");
assert.equal(rejected.quota_decision.decision, "reject");
assert.equal(rejected.quota_decision.relay_allowed, false);
assert.equal(rejected.quota_decision.reason, "relay-byte-quota-exceeded");
assert.equal(rejected.quota_decision.billing_meter_delta.active_session_count, 0);
assert.equal(rejected.quota_decision.billing_meter_delta.relay_frame_count, 0);
assert.equal(rejected.quota_decision.billing_meter_delta.relay_byte_count, 0);
assert.equal(rejected.quota_decision.billing_meter_delta.quota_denial_count, 1);
assert.equal(rejected.quota_decision.abuse_signal_delta.rate_limit_denial_count, 0);

assert.throws(
  () =>
    routeManagedRelayRuntimeQuotaMeteredFrame(frame, quotaState, routeMetadata, {
      nowMs: 35000,
    }),
  /expired before route/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeQuotaAndMeteringIntegration({
      serviceId: "managed-relay-runtime-quota-metering-check",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeQuotaAndMeteringIntegration({
      serviceId: "managed-relay-runtime-quota-metering-check",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeQuotaAndMeteringIntegration({
      serviceId: "managed-relay-runtime-quota-metering-check",
      generatedAtMs: 2000,
      quotaRuntime: "not-wired",
    }),
  /must wire active session frame byte metering/,
);

assert.deepEqual(integrationSummary.remainingImplementationPhases, [
  "managed-runtime-support-and-abuse-operations-integration",
  "managed-runtime-pwa-exposure-gate",
]);

for (const evidenceCheck of [
  "managed-service-scaffold-complete",
  "control-plane-contract-wiring-complete",
  "encrypted-frame-routing-complete",
  "active-session-quota-checked-before-frame-delivery",
  "frame-and-byte-quota-checked-before-frame-delivery",
  "quota-denials-fail-closed-before-route-delivery",
  "billing-meter-deltas-are-aggregate-only",
  "abuse-signal-deltas-remain-separate",
  "pwa-exposure-remains-disabled",
  "support-and-abuse-operations-integration-complete",
]) {
  assert.ok(integrationSummary.evidenceChecks.includes(evidenceCheck), `quota metering missing evidence: ${evidenceCheck}`);
}

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay runtime quota and metering integration without PWA exposure",
  gate: {
    gateStatus: gate.gateStatus,
    implementationCanStart: gate.implementationCanStart,
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
  },
  implementationPlan: {
    readiness: plan.readiness,
    implementationStatus: plan.implementationStatus,
    selectedRuntime: plan.selectedRuntime,
    selectedRuntimeCanChange: plan.selectedRuntimeCanChange,
    regressionChecks: plan.regressionChecks,
  },
  encryptedFrameRouting: {
    readiness: encryptedRoutingSummary.readiness,
    routeRuntime: encryptedRoutingSummary.routeRuntime,
    startupContract: encryptedRoutingSummary.startupContract,
  },
  quotaAndMeteringIntegration: integration,
  acceptedRoute: accepted,
  rejectedRoute: rejected,
  summary: {
    readiness: integrationSummary.readiness,
    implementationStatus: integrationSummary.implementationStatus,
    routeRuntime: integrationSummary.routeRuntime,
    quotaRuntime: integrationSummary.quotaRuntime,
    pwaExposureDecision: integrationSummary.pwaExposureDecision,
    implementationCanContinue: integrationSummary.implementationCanContinue,
    selectedRuntimeCanChange: integrationSummary.selectedRuntimeCanChange,
  },
  startupContract: integrationSummary.startupContract,
  quotaMeteringContract: integrationSummary.quotaMeteringContract,
  healthSurface: integrationSummary.healthSurface,
  remainingImplementationPhases: integrationSummary.remainingImplementationPhases,
  evidenceChecks: integrationSummary.evidenceChecks,
  guardrails: integrationSummary.guardrails,
  productDefault: integrationSummary.productDefault,
  nextLocalSlice: integrationSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_QUOTA_AND_METERING_INTEGRATION_OK ${evidencePath}`);
