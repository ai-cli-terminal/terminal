import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayRuntimeControlPlaneContractWiring,
  relayManagedRuntimeControlPlaneContractWiring,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimeReadinessGate,
  relayManagedRuntimeServiceScaffold,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-control-plane-contract-wiring",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_CONTROL_PLANE_CONTRACT_WIRING_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-control-plane-contract-wiring.json",
  );

const gate = relayManagedRuntimeReadinessGate();
const plan = relayManagedRuntimeImplementationPlan();
const scaffoldSummary = relayManagedRuntimeServiceScaffold();
const wiringSummary = relayManagedRuntimeControlPlaneContractWiring();
const wiring = createManagedRelayRuntimeControlPlaneContractWiring({
  serviceId: "managed-relay-runtime-control-plane-check",
  generatedAtMs: 2000,
});

assert.equal(gate.gateStatus, "runtime-evidence-green");
assert.equal(gate.implementationCanStart, true);
assert.equal(gate.nextLocalSlice, "managed-relay-runtime-support-and-abuse-operations-integration");
assert.deepEqual(gate.remainingRuntimeEvidence, []);
assert.deepEqual(gate.remainingRuntimeBlockers, []);

assert.equal(plan.readiness, "plan");
assert.equal(plan.implementationCanStart, true);
assert.equal(plan.nextLocalSlice, "managed-relay-runtime-support-and-abuse-operations-integration");
assert.equal(plan.selectedRuntime, "deferred");
assert.equal(plan.selectedRuntimeCanChange, false);
assert.ok(
  plan.regressionChecks.includes(
    "check:pwa-relay-managed-runtime-control-plane-contract-wiring",
  ),
);

assert.equal(scaffoldSummary.deploymentMode, "managed");
assert.equal(scaffoldSummary.readiness, "scaffold");
assert.equal(scaffoldSummary.productDefault, "live-loopback");
assert.equal(scaffoldSummary.selectedRuntime, "deferred");
assert.equal(scaffoldSummary.runtimeDefault, "not-selected");
assert.equal(scaffoldSummary.serviceState, "scaffold-ready");
assert.equal(scaffoldSummary.nextLocalSlice, "managed-relay-runtime-support-and-abuse-operations-integration");
assert.equal(scaffoldSummary.startupContract.publicBind, false);
assert.equal(scaffoldSummary.startupContract.endpointMode, "disabled");
assert.equal(scaffoldSummary.startupContract.pwaExposure, "disabled");

assert.equal(wiringSummary.deploymentMode, "managed");
assert.equal(wiringSummary.readiness, "wiring");
assert.equal(wiringSummary.productDefault, "live-loopback");
assert.equal(wiringSummary.selectedRuntime, "deferred");
assert.equal(wiringSummary.runtimeDefault, "not-selected");
assert.equal(
  wiringSummary.implementationStatus,
  "managed-runtime-control-plane-contract-wired-no-pwa-exposure",
);
assert.equal(
  wiringSummary.controlPlaneRuntime,
  "tenant-session-registration-contract-wired",
);
assert.equal(wiringSummary.routeRuntime, "not-wired");
assert.equal(wiringSummary.implementationCanContinue, true);
assert.equal(wiringSummary.selectedRuntimeCanChange, false);
assert.equal(wiringSummary.nextLocalSlice, "managed-relay-runtime-support-and-abuse-operations-integration");
assert.ok(
  wiringSummary.completedImplementationEvidence.includes(
    "managed-runtime-service-scaffold",
  ),
);
assert.ok(
  wiringSummary.completedImplementationEvidence.includes(
    "managed-runtime-control-plane-contract-wiring",
  ),
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "managed_control_plane_has_no_pwa_exposure",
  "route_runtime_remains_not_wired_until_encrypted_frame_routing",
  "tenant_session_metadata_only",
  "public_verifier_key_lookup_required",
  "quota_preflight_required_before_registration",
  "control_plane_audit_metadata_only",
  "rollback_to_live_loopback_required",
]) {
  assert.ok(wiringSummary.guardrails.includes(guardrail), `wiring missing guardrail: ${guardrail}`);
}

assert.equal(wiring.wiring_version, 1);
assert.equal(wiring.deployment_mode, "managed");
assert.equal(wiring.readiness, "control-plane-contract-wiring");
assert.equal(wiring.product_default, "live-loopback");
assert.equal(wiring.selected_runtime, "deferred");
assert.equal(wiring.runtime_default, "not-selected");
assert.equal(wiring.endpoint_mode, "disabled");
assert.equal(wiring.public_bind_enabled, false);
assert.equal(wiring.pwa_exposure, "disabled");
assert.equal(wiring.route_runtime, "not-wired");
assert.equal(wiring.control_plane_runtime, "tenant-session-registration-contract-wired");
assert.equal(wiring.rollback_transport, "live-loopback");
assert.equal(wiring.service_scaffold_state, "scaffold-ready");
assert.ok(wiring.wired_contracts.includes("tenant-identity"));
assert.ok(wiring.wired_contracts.includes("session-registration"));
assert.ok(wiring.wired_contracts.includes("public-verifier-key-lookup"));
assert.ok(wiring.wired_contracts.includes("quota-preflight"));
assert.ok(wiring.wired_contracts.includes("audit-event"));
assert.equal(wiring.session_registration_contract.disabled_public_endpoint, true);
assert.equal(wiring.session_registration_contract.pwa_exposure, "disabled");
assert.equal(
  wiring.session_registration_contract.route_frame_handler,
  "disabled-until-encrypted-frame-routing",
);
assert.ok(wiring.session_registration_contract.input_fields.includes("tenant_id"));
assert.ok(wiring.session_registration_contract.input_fields.includes("verifier_key_id"));
assert.ok(wiring.session_registration_contract.output_fields.includes("quota_decision"));
assert.equal(wiring.public_verifier_key_lookup_contract.private_signing_material_allowed, false);
assert.equal(wiring.public_verifier_key_lookup_contract.hmac_material_allowed, false);
assert.equal(
  wiring.public_verifier_key_lookup_contract.failure_mode,
  "fail-closed-before-registration",
);
assert.equal(wiring.quota_preflight_contract.decision_point, "before-session-registration");
assert.equal(wiring.quota_preflight_contract.billing_meter_source, "registration-metadata-only");
assert.equal(wiring.audit_contract.payload_visibility, "payload-free");
assert.equal(wiring.audit_contract.support_visibility, "aggregate-only");
assert.equal(wiring.control_plane_health.payload_visibility, "payload-free");
assert.equal(wiring.control_plane_health.support_visibility, "aggregate-only");
assert.equal(wiring.control_plane_health.active_session_count, 0);
assert.equal(wiring.control_plane_health.pending_registration_count, 0);
assert.ok(wiring.allowed_control_plane_fields.includes("session_id_hash"));
assert.ok(wiring.prohibited_control_plane_fields.includes("payload_json"));
assert.equal(wiring.allowed_control_plane_fields.includes("payload_json"), false);
assert.equal(JSON.stringify(wiring.control_plane_health).includes("payload_json"), false);
assert.equal(JSON.stringify(wiring.session_registration_contract).includes("payload_key_hex"), false);
assert.equal(JSON.stringify(wiring.audit_contract).includes("raw_session_token"), false);

assert.deepEqual(wiringSummary.remainingImplementationPhases, [
  "managed-runtime-encrypted-frame-routing",
  "managed-runtime-quota-and-metering-integration",
  "managed-runtime-support-and-abuse-operations-integration",
  "managed-runtime-pwa-exposure-gate",
]);

for (const evidenceCheck of [
  "managed-service-scaffold-complete",
  "tenant-session-registration-contract-wired",
  "public-verifier-key-lookup-contract-wired",
  "quota-preflight-contract-wired",
  "control-plane-audit-is-payload-free",
  "route-frame-handler-remains-disabled",
  "pwa-exposure-remains-disabled",
  "next-encrypted-frame-routing-slice-selected",
]) {
  assert.ok(wiringSummary.evidenceChecks.includes(evidenceCheck), `wiring missing evidence: ${evidenceCheck}`);
}

assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-check",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-check",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-check",
      generatedAtMs: 2000,
      routeRuntime: "wired",
    }),
  /route runtime must stay not-wired/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-check",
      generatedAtMs: 2000,
      selectedRuntime: "managed",
    }),
  /selected runtime must stay deferred/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeControlPlaneContractWiring({
      serviceId: "managed-relay-runtime-control-plane-check",
      generatedAtMs: 2000,
      productDefault: "relay",
    }),
  /product default must stay live-loopback/,
);

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay runtime control-plane contract wiring without PWA exposure",
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
  serviceScaffold: {
    readiness: scaffoldSummary.readiness,
    serviceState: scaffoldSummary.serviceState,
    startupContract: scaffoldSummary.startupContract,
    healthSurface: scaffoldSummary.healthSurface,
  },
  controlPlaneWiring: wiring,
  summary: {
    readiness: wiringSummary.readiness,
    implementationStatus: wiringSummary.implementationStatus,
    controlPlaneRuntime: wiringSummary.controlPlaneRuntime,
    routeRuntime: wiringSummary.routeRuntime,
    pwaExposureDecision: wiringSummary.pwaExposureDecision,
    implementationCanContinue: wiringSummary.implementationCanContinue,
    selectedRuntimeCanChange: wiringSummary.selectedRuntimeCanChange,
  },
  startupContract: wiringSummary.startupContract,
  controlPlaneContract: wiringSummary.controlPlaneContract,
  healthSurface: wiringSummary.healthSurface,
  remainingImplementationPhases: wiringSummary.remainingImplementationPhases,
  evidenceChecks: wiringSummary.evidenceChecks,
  guardrails: wiringSummary.guardrails,
  productDefault: wiringSummary.productDefault,
  nextLocalSlice: wiringSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_CONTROL_PLANE_CONTRACT_WIRING_OK ${evidencePath}`);
