import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayRuntimePwaExposureGate,
  relayDeploymentShapeDecision,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimePwaExposureGate,
  relayManagedRuntimeReadinessGate,
  relayManagedRuntimeSupportAndAbuseOperationsIntegration,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-pwa-exposure-gate",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_PWA_EXPOSURE_GATE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-pwa-exposure-gate.json",
  );

const decision = relayDeploymentShapeDecision();
const gate = relayManagedRuntimeReadinessGate();
const plan = relayManagedRuntimeImplementationPlan();
const supportAbuseSummary = relayManagedRuntimeSupportAndAbuseOperationsIntegration();
const exposureSummary = relayManagedRuntimePwaExposureGate();
const exposureGate = createManagedRelayRuntimePwaExposureGate({
  serviceId: "managed-relay-runtime-pwa-exposure-check",
  generatedAtMs: 2000,
});
const indexHtml = await readFile(path.join(repoRoot, "pwa", "index.html"), "utf8");

assert.equal(gate.gateStatus, "runtime-evidence-green");
assert.equal(gate.implementationCanStart, true);
assert.deepEqual(gate.remainingRuntimeEvidence, []);
assert.deepEqual(gate.remainingRuntimeBlockers, []);

assert.equal(plan.implementationCanStart, true);
assert.equal(plan.productDefault, "live-loopback");
assert.equal(plan.selectedRuntime, "deferred");
assert.equal(plan.selectedRuntimeCanChange, false);
assert.ok(
  plan.regressionChecks.includes(
    "check:pwa-relay-managed-runtime-pwa-exposure-gate",
  ),
);
assert.ok(plan.exposureGates.includes("pwa_copy_and_setup_text_updated"));
assert.ok(plan.exposureGates.includes("live_loopback_rollback_documented"));

assert.equal(decision.selectedMode, "self-hosted");
assert.equal(decision.productDefault, "live-loopback");
assert.ok(decision.pwaVisibleModes.includes("managed"));
assert.ok(decision.explicitOptInModes.includes("managed"));

assert.equal(supportAbuseSummary.readiness, "integration");
assert.equal(supportAbuseSummary.implementationCanContinue, true);
assert.equal(supportAbuseSummary.startupContract.pwaExposure, "disabled");
assert.ok(
  supportAbuseSummary.completedImplementationEvidence.includes(
    "managed-runtime-support-and-abuse-operations-integration",
  ),
);

assert.equal(exposureSummary.deploymentMode, "managed");
assert.equal(exposureSummary.readiness, "exposure-gate");
assert.equal(exposureSummary.productDefault, "live-loopback");
assert.equal(exposureSummary.selectedRuntime, "explicit-opt-in-managed");
assert.equal(exposureSummary.runtimeDefault, "not-selected");
assert.equal(exposureSummary.pwaExposure, "explicit-opt-in");
assert.equal(exposureSummary.endpointMode, "operator-setup-required");
assert.equal(exposureSummary.endpointAutoStart, false);
assert.equal(exposureSummary.publicBind, false);
assert.equal(exposureSummary.rollbackDefault, "live-loopback");
assert.equal(exposureSummary.selectedRuntimeCanChange, true);
assert.equal(exposureSummary.selectedRuntimeChangeBoundary, "explicit-opt-in-only");
assert.equal(exposureSummary.productDefaultCanChange, false);
assert.equal(
  exposureSummary.nextLocalSlice,
  "managed-relay-runtime-browser-operator-evidence",
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_is_explicit_opt_in_only",
  "managed_relay_pwa_exposes_setup_copy_only",
  "managed_relay_endpoint_auto_start_disabled",
  "managed_relay_public_bind_disabled",
  "managed_runtime_requires_operator_setup",
  "pwa_surface_excludes_payloads_and_secrets",
  "pwa_surface_excludes_raw_identifiers",
  "support_abuse_boundaries_preserved_after_exposure",
  "rollback_to_live_loopback_required",
]) {
  assert.ok(exposureSummary.guardrails.includes(guardrail), `exposure gate missing guardrail: ${guardrail}`);
}

for (const evidenceCheck of [
  "managed-service-scaffold-complete",
  "control-plane-contract-wiring-complete",
  "encrypted-frame-routing-complete",
  "quota-and-metering-integration-complete",
  "support-and-abuse-operations-integration-complete",
  "pwa-copy-and-setup-text-updated",
  "managed-runtime-exposure-gate-passed",
  "managed-runtime-remains-explicit-opt-in-only",
  "managed-runtime-endpoint-auto-start-disabled",
  "managed-runtime-public-bind-disabled",
  "pwa-surface-excludes-payloads-secrets-and-raw-identifiers",
  "live-loopback-rollback-documented",
  "next-browser-operator-evidence-slice-selected",
]) {
  assert.ok(exposureSummary.evidenceChecks.includes(evidenceCheck), `exposure gate missing evidence: ${evidenceCheck}`);
}

assert.equal(exposureGate.exposure_gate_version, 1);
assert.equal(exposureGate.deployment_mode, "managed");
assert.equal(exposureGate.readiness, "pwa-exposure-gate");
assert.equal(exposureGate.product_default, "live-loopback");
assert.equal(exposureGate.selected_runtime, "explicit-opt-in-managed");
assert.equal(exposureGate.runtime_default, "not-selected");
assert.equal(exposureGate.endpoint_mode, "operator-setup-required");
assert.equal(exposureGate.endpoint_auto_start, false);
assert.equal(exposureGate.public_bind_enabled, false);
assert.equal(exposureGate.pwa_exposure, "explicit-opt-in");
assert.equal(exposureGate.pwa_surface.visible, true);
assert.equal(exposureGate.pwa_surface.mode, "explicit-opt-in");
assert.equal(
  exposureGate.pwa_surface.copy.default_text,
  "Product default remains live-loopback",
);
assert.equal(
  exposureGate.setup_contract.setup_visibility,
  "copy-and-status-only",
);
assert.equal(exposureGate.setup_contract.manual_connect_required, true);
assert.equal(exposureGate.setup_contract.endpoint_auto_start, false);
assert.equal(exposureGate.setup_contract.public_bind_enabled, false);
assert.equal(exposureGate.rollback_contract.rollback_transport, "live-loopback");
assert.ok(exposureGate.allowed_pwa_fields.includes("pwa_exposure"));
assert.ok(exposureGate.allowed_pwa_fields.includes("endpoint_auto_start"));
assert.ok(exposureGate.prohibited_pwa_fields.includes("signed_session_ticket"));

const visiblePwaSurfaceJson = JSON.stringify({
  pwaSurface: {
    ...exposureGate.pwa_surface,
    prohibited_fields: undefined,
  },
  setupContract: exposureGate.setup_contract,
  rollbackContract: exposureGate.rollback_contract,
  exposureHealth: exposureGate.exposure_health,
  allowedPwaFields: exposureGate.allowed_pwa_fields,
});
for (const prohibited of [
  "payload_json",
  "payload_ciphertext_hex",
  "payload_nonce_hex",
  "payload_key_hex",
  "signed_session_ticket",
  "session_token",
  "hmac_secret",
  '"support_actor_id"',
  '"session_id"',
  '"daemon_device_id"',
  '"companion_device_id"',
]) {
  assert.equal(visiblePwaSurfaceJson.includes(prohibited), false, `PWA exposure surface leaked ${prohibited}`);
}

for (const selectorId of [
  "relay-managed-title",
  "relay-managed-state",
  "relay-managed-default-mode",
  "relay-managed-exposure",
  "relay-managed-endpoint-mode",
  "relay-managed-public-bind",
  "relay-managed-auto-start",
  "relay-managed-rollback",
  "relay-managed-next",
  "relay-managed-evidence-list",
  "relay-managed-copy",
]) {
  assert.ok(indexHtml.includes(`id="${selectorId}"`), `PWA index missing #${selectorId}`);
}
assert.ok(indexHtml.includes("Managed Relay"));
assert.ok(indexHtml.includes("Managed setup copy"));
assert.equal(indexHtml.includes("signed_session_ticket"), false);
assert.equal(indexHtml.includes("session_token"), false);
assert.equal(indexHtml.includes("payload_ciphertext_hex"), false);

assert.throws(
  () =>
    createManagedRelayRuntimePwaExposureGate({
      serviceId: "managed-relay-runtime-pwa-exposure-check",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must require operator setup/,
);
assert.throws(
  () =>
    createManagedRelayRuntimePwaExposureGate({
      serviceId: "managed-relay-runtime-pwa-exposure-check",
      generatedAtMs: 2000,
      endpointAutoStart: true,
    }),
  /endpoint auto start must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimePwaExposureGate({
      serviceId: "managed-relay-runtime-pwa-exposure-check",
      generatedAtMs: 2000,
      publicBind: true,
    }),
  /public bind must stay disabled/,
);

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay runtime PWA exposure gate for explicit opt-in setup copy",
  readinessGate: {
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
    exposureGates: plan.exposureGates,
    regressionChecks: plan.regressionChecks,
  },
  supportAbuseIntegration: {
    readiness: supportAbuseSummary.readiness,
    implementationStatus: supportAbuseSummary.implementationStatus,
    startupContract: supportAbuseSummary.startupContract,
  },
  exposureGate,
  summary: {
    readiness: exposureSummary.readiness,
    implementationStatus: exposureSummary.implementationStatus,
    selectedRuntime: exposureSummary.selectedRuntime,
    pwaExposureDecision: exposureSummary.pwaExposureDecision,
    pwaExposure: exposureSummary.pwaExposure,
    endpointMode: exposureSummary.endpointMode,
    endpointAutoStart: exposureSummary.endpointAutoStart,
    publicBind: exposureSummary.publicBind,
    selectedRuntimeCanChange: exposureSummary.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary: exposureSummary.selectedRuntimeChangeBoundary,
    productDefaultCanChange: exposureSummary.productDefaultCanChange,
  },
  startupContract: exposureSummary.startupContract,
  pwaSurface: exposureSummary.pwaSurface,
  healthSurface: exposureSummary.healthSurface,
  remainingImplementationPhases: exposureSummary.remainingImplementationPhases,
  evidenceChecks: exposureSummary.evidenceChecks,
  guardrails: exposureSummary.guardrails,
  productDefault: exposureSummary.productDefault,
  nextLocalSlice: exposureSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_PWA_EXPOSURE_GATE_OK ${evidencePath}`);
