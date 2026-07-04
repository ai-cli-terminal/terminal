import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayRuntimeServiceScaffold,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimeReadinessGate,
  relayManagedRuntimeServiceScaffold,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-service-scaffold",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_SERVICE_SCAFFOLD_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-runtime-service-scaffold.json");

const gate = relayManagedRuntimeReadinessGate();
const plan = relayManagedRuntimeImplementationPlan();
const scaffoldSummary = relayManagedRuntimeServiceScaffold();
const scaffold = createManagedRelayRuntimeServiceScaffold({
  serviceId: "managed-relay-runtime-scaffold-check",
  generatedAtMs: 2000,
});

assert.equal(gate.gateStatus, "runtime-evidence-green");
assert.equal(gate.implementationCanStart, true);
assert.equal(gate.nextLocalSlice, "managed-relay-runtime-quota-and-metering-integration");
assert.deepEqual(gate.remainingRuntimeEvidence, []);
assert.deepEqual(gate.remainingRuntimeBlockers, []);

assert.equal(plan.readiness, "plan");
assert.equal(plan.implementationCanStart, true);
assert.equal(plan.nextLocalSlice, "managed-relay-runtime-quota-and-metering-integration");
assert.equal(plan.selectedRuntime, "deferred");
assert.equal(plan.selectedRuntimeCanChange, false);

assert.equal(scaffoldSummary.deploymentMode, "managed");
assert.equal(scaffoldSummary.readiness, "scaffold");
assert.equal(scaffoldSummary.productDefault, "live-loopback");
assert.equal(scaffoldSummary.selectedRuntime, "deferred");
assert.equal(scaffoldSummary.runtimeDefault, "not-selected");
assert.equal(
  scaffoldSummary.implementationStatus,
  "managed-runtime-service-scaffold-ready-no-pwa-exposure",
);
assert.equal(scaffoldSummary.serviceState, "scaffold-ready");
assert.equal(scaffoldSummary.serviceProcessPolicy, "explicit-operator-only-no-product-default");
assert.equal(
  scaffoldSummary.pwaExposureDecision,
  "disabled-until-managed-runtime-exposure-gate",
);
assert.equal(scaffoldSummary.rollbackDefault, "live-loopback");
assert.equal(scaffoldSummary.implementationCanContinue, true);
assert.equal(scaffoldSummary.selectedRuntimeCanChange, false);
assert.equal(
  scaffoldSummary.nextLocalSlice,
  "managed-relay-runtime-quota-and-metering-integration",
);
assert.ok(
  scaffoldSummary.completedImplementationEvidence.includes(
    "managed-runtime-service-scaffold",
  ),
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "managed_service_scaffold_has_no_pwa_exposure",
  "managed_service_does_not_bind_public_routes_by_default",
  "health_surface_is_aggregate_only",
  "payload_blind_boundary_preserved",
  "public_verifier_key_boundary_preserved",
  "quota_and_usage_boundaries_preserved",
  "rollback_to_live_loopback_required",
]) {
  assert.ok(scaffoldSummary.guardrails.includes(guardrail), `service scaffold missing guardrail: ${guardrail}`);
}

assert.equal(scaffold.scaffold_version, 1);
assert.equal(scaffold.deployment_mode, "managed");
assert.equal(scaffold.readiness, "service-scaffold");
assert.equal(scaffold.product_default, "live-loopback");
assert.equal(scaffold.selected_runtime, "deferred");
assert.equal(scaffold.runtime_default, "not-selected");
assert.equal(scaffold.endpoint_mode, "disabled");
assert.equal(scaffold.public_bind_enabled, false);
assert.equal(scaffold.pwa_exposure, "disabled");
assert.equal(scaffold.route_runtime, "not-wired");
assert.equal(scaffold.control_plane_runtime, "not-wired");
assert.equal(scaffold.lifecycle.starts_without_public_listener, true);
assert.equal(scaffold.lifecycle.starts_without_pwa_exposure, true);
assert.equal(scaffold.lifecycle.rollback_transport, "live-loopback");
assert.equal(
  scaffold.route_handlers.session_registration,
  "disabled-until-control-plane-contract-wiring",
);
assert.equal(
  scaffold.route_handlers.frame_route,
  "disabled-until-encrypted-frame-routing",
);
assert.equal(scaffold.route_handlers.health, "aggregate-only");
assert.equal(scaffold.health_surface.payload_visibility, "payload-free");
assert.equal(scaffold.health_surface.support_visibility, "aggregate-only");
assert.equal(scaffold.health_surface.tenant_count, 0);
assert.equal(scaffold.health_surface.relay_byte_count, 0);
assert.ok(scaffold.allowed_health_fields.includes("relay_byte_count"));
assert.ok(scaffold.prohibited_health_fields.includes("payload_json"));
assert.equal(scaffold.allowed_health_fields.includes("payload_json"), false);
assert.equal(JSON.stringify(scaffold.health_surface).includes("payload_json"), false);
assert.equal(JSON.stringify(scaffold.health_surface).includes("payload_key_hex"), false);
assert.equal(JSON.stringify(scaffold.route_handlers).includes("raw_session_token"), false);

assert.deepEqual(scaffoldSummary.remainingImplementationPhases, [
  "managed-runtime-control-plane-contract-wiring",
  "managed-runtime-encrypted-frame-routing",
  "managed-runtime-quota-and-metering-integration",
  "managed-runtime-support-and-abuse-operations-integration",
  "managed-runtime-pwa-exposure-gate",
]);

for (const evidenceCheck of [
  "managed-service-scaffold-starts-without-pwa-exposure",
  "managed-service-scaffold-has-no-public-bind",
  "health-surface-is-aggregate-only",
  "route-and-registration-handlers-remain-disabled",
  "live-loopback-rollback-remains-default",
  "next-control-plane-contract-wiring-slice-selected",
]) {
  assert.ok(scaffoldSummary.evidenceChecks.includes(evidenceCheck), `service scaffold missing evidence: ${evidenceCheck}`);
}

assert.throws(
  () =>
    createManagedRelayRuntimeServiceScaffold({
      serviceId: "managed-relay-runtime-scaffold-check",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeServiceScaffold({
      serviceId: "managed-relay-runtime-scaffold-check",
      generatedAtMs: 2000,
      publicBind: true,
    }),
  /public bind must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeServiceScaffold({
      serviceId: "managed-relay-runtime-scaffold-check",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeServiceScaffold({
      serviceId: "managed-relay-runtime-scaffold-check",
      generatedAtMs: 2000,
      productDefault: "relay",
    }),
  /product default must stay live-loopback/,
);

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify the managed relay runtime service scaffold without PWA exposure",
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
  },
  scaffold: {
    deploymentMode: scaffoldSummary.deploymentMode,
    readiness: scaffoldSummary.readiness,
    selectedRuntime: scaffoldSummary.selectedRuntime,
    runtimeDefault: scaffoldSummary.runtimeDefault,
    implementationStatus: scaffoldSummary.implementationStatus,
    serviceState: scaffoldSummary.serviceState,
    pwaExposureDecision: scaffoldSummary.pwaExposureDecision,
    implementationCanContinue: scaffoldSummary.implementationCanContinue,
    selectedRuntimeCanChange: scaffoldSummary.selectedRuntimeCanChange,
  },
  startupContract: scaffoldSummary.startupContract,
  healthSurface: scaffoldSummary.healthSurface,
  serviceScaffold: scaffoldSummary.serviceScaffold,
  remainingImplementationPhases: scaffoldSummary.remainingImplementationPhases,
  evidenceChecks: scaffoldSummary.evidenceChecks,
  guardrails: scaffoldSummary.guardrails,
  productDefault: scaffoldSummary.productDefault,
  nextLocalSlice: scaffoldSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_SERVICE_SCAFFOLD_OK ${evidencePath}`);
