import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayRuntimeEncryptedFrameRouting,
  liveApprovalRequestMessage,
  managedRelayEncryptedFrameFromLiveMessage,
  managedRelayEncryptedFrameRouteEnvelope,
  relayManagedRuntimeControlPlaneContractWiring,
  relayManagedRuntimeEncryptedFrameRouting,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimeReadinessGate,
  routeManagedRelayRuntimeEncryptedFrame,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-encrypted-frame-routing",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_ENCRYPTED_FRAME_ROUTING_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-encrypted-frame-routing.json",
  );

const gate = relayManagedRuntimeReadinessGate();
const plan = relayManagedRuntimeImplementationPlan();
const controlPlaneWiringSummary = relayManagedRuntimeControlPlaneContractWiring();
const routingSummary = relayManagedRuntimeEncryptedFrameRouting();
const routing = createManagedRelayRuntimeEncryptedFrameRouting({
  serviceId: "managed-relay-runtime-encrypted-routing-check",
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
    "check:pwa-relay-managed-runtime-encrypted-frame-routing",
  ),
);

assert.equal(controlPlaneWiringSummary.deploymentMode, "managed");
assert.equal(controlPlaneWiringSummary.readiness, "wiring");
assert.equal(
  controlPlaneWiringSummary.controlPlaneRuntime,
  "tenant-session-registration-contract-wired",
);
assert.equal(controlPlaneWiringSummary.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");

assert.equal(routingSummary.deploymentMode, "managed");
assert.equal(routingSummary.readiness, "routing");
assert.equal(routingSummary.productDefault, "live-loopback");
assert.equal(routingSummary.selectedRuntime, "deferred");
assert.equal(routingSummary.runtimeDefault, "not-selected");
assert.equal(
  routingSummary.implementationStatus,
  "managed-runtime-encrypted-frame-routing-wired-no-pwa-exposure",
);
assert.equal(
  routingSummary.controlPlaneRuntime,
  "tenant-session-registration-contract-wired",
);
assert.equal(routingSummary.routeRuntime, "encrypted-frame-routing-wired");
assert.equal(routingSummary.implementationCanContinue, true);
assert.equal(routingSummary.selectedRuntimeCanChange, false);
assert.equal(routingSummary.nextLocalSlice, "managed-relay-runtime-pwa-exposure-gate");
assert.ok(
  routingSummary.completedImplementationEvidence.includes(
    "managed-runtime-service-scaffold",
  ),
);
assert.ok(
  routingSummary.completedImplementationEvidence.includes(
    "managed-runtime-control-plane-contract-wiring",
  ),
);
assert.ok(
  routingSummary.completedImplementationEvidence.includes(
    "managed-runtime-encrypted-frame-routing",
  ),
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "encrypted_frame_routing_has_no_pwa_exposure",
  "route_visible_fields_are_allowlisted",
  "payload_ciphertext_hex_is_internal_delivery_only",
  "payload_nonce_hex_is_internal_delivery_only",
  "payload_key_material_never_enters_route_runtime",
  "plaintext_payload_fields_rejected_before_route",
  "expired_frames_fail_closed_before_route",
  "rollback_to_live_loopback_required",
]) {
  assert.ok(routingSummary.guardrails.includes(guardrail), `routing missing guardrail: ${guardrail}`);
}

assert.equal(routing.routing_version, 1);
assert.equal(routing.deployment_mode, "managed");
assert.equal(routing.readiness, "encrypted-frame-routing");
assert.equal(routing.product_default, "live-loopback");
assert.equal(routing.selected_runtime, "deferred");
assert.equal(routing.runtime_default, "not-selected");
assert.equal(routing.endpoint_mode, "disabled");
assert.equal(routing.public_bind_enabled, false);
assert.equal(routing.pwa_exposure, "disabled");
assert.equal(routing.control_plane_runtime, "tenant-session-registration-contract-wired");
assert.equal(routing.route_runtime, "encrypted-frame-routing-wired");
assert.equal(routing.rollback_transport, "live-loopback");
assert.equal(routing.route_contract.contract_state, "wired");
assert.equal(routing.route_contract.accepted_frame_type, "managed-encrypted-relay-frame");
assert.ok(routing.allowed_route_visible_fields.includes("payload_ciphertext_bytes"));
assert.equal(routing.allowed_route_visible_fields.includes("payload_ciphertext_hex"), false);
assert.ok(routing.prohibited_route_visible_fields.includes("payload_ciphertext_hex"));
assert.ok(routing.prohibited_route_visible_fields.includes("payload_nonce_hex"));
assert.equal(routing.route_contract.delivery_boundary.encrypted_frame_forwarded, true);
assert.equal(routing.route_contract.delivery_boundary.plaintext_payload_visible, false);
assert.equal(routing.route_contract.delivery_boundary.operator_visible_ciphertext, false);
assert.equal(routing.route_health.payload_visibility, "opaque-ciphertext-metadata-only");
assert.equal(routing.route_health.support_visibility, "aggregate-only");
assert.equal(JSON.stringify(routing.route_health).includes("payload_ciphertext_hex"), false);
assert.equal(JSON.stringify(routing.route_contract.delivery_boundary).includes("payload_key_hex"), false);

const frame = await managedRelayEncryptedFrameFromLiveMessage(
  "managed-runtime-route-check",
  "daemon",
  1,
  4000,
  34000,
  liveApprovalRequestMessage({
    approval_id: [114, 111, 117, 116, 101, 45, 99, 104, 101, 99, 107],
    nonce: Array.from({ length: 32 }, (_, index) => index + 1),
    command_masked: "rotate production secret --tenant alpha",
    context_hash: "ctx-managed-routing-check",
    expires_at: 34000,
    device_epoch: 4,
  }),
  "46".repeat(32),
  { nonceHex: "47".repeat(12), webCrypto: webcrypto },
);
const route = routeManagedRelayRuntimeEncryptedFrame(frame, { nowMs: 4100 });
assert.equal(route.route_decision, "accepted");
assert.equal(route.route_state, "encrypted-frame-routed");
assert.equal(route.endpoint_mode, "disabled");
assert.equal(route.pwa_exposure, "disabled");
assert.equal(route.route_runtime, "encrypted-frame-routing-wired");
assert.deepEqual(route.route_envelope, managedRelayEncryptedFrameRouteEnvelope(frame));
assert.deepEqual(route.route_visible_fields, routing.allowed_route_visible_fields);
assert.equal(route.route_delivery.payload_visibility, "opaque-ciphertext-only");
assert.equal(route.route_delivery.plaintext_payload_visible, false);
assert.equal(route.route_delivery.operator_visible_ciphertext, false);
assert.equal(route.audit_event.event_type, "managed-encrypted-frame-route");
assert.equal(route.audit_event.decision, "accepted");
assert.equal(route.audit_event.payload_ciphertext_bytes, route.route_envelope.payload_ciphertext_bytes);
assert.equal(JSON.stringify(route).includes("payload_ciphertext_hex"), false);
assert.equal(JSON.stringify(route).includes("payload_nonce_hex"), false);
assert.equal(JSON.stringify(route).includes("payload_key_hex"), false);
assert.equal(JSON.stringify(route).includes("rotate production secret"), false);
assert.equal(JSON.stringify(route).includes("ctx-managed-routing-check"), false);

assert.throws(
  () => routeManagedRelayRuntimeEncryptedFrame(frame, { nowMs: 34000 }),
  /expired before route/,
);
assert.throws(
  () =>
    routeManagedRelayRuntimeEncryptedFrame({
      ...frame,
      payload_json: "{}",
    }),
  /plaintext field not allowed: payload_json/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-check",
      generatedAtMs: 2000,
      endpointMode: "enabled",
    }),
  /endpoint mode must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-check",
      generatedAtMs: 2000,
      pwaExposure: "enabled",
    }),
  /pwa exposure must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-check",
      generatedAtMs: 2000,
      routeRuntime: "not-wired",
    }),
  /route runtime must wire encrypted frame routing/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-check",
      generatedAtMs: 2000,
      selectedRuntime: "managed",
    }),
  /selected runtime must stay deferred/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeEncryptedFrameRouting({
      serviceId: "managed-relay-runtime-encrypted-routing-check",
      generatedAtMs: 2000,
      productDefault: "relay",
    }),
  /product default must stay live-loopback/,
);

assert.deepEqual(routingSummary.remainingImplementationPhases, [
  "managed-runtime-quota-and-metering-integration",
  "managed-runtime-support-and-abuse-operations-integration",
  "managed-runtime-pwa-exposure-gate",
]);

for (const evidenceCheck of [
  "managed-service-scaffold-complete",
  "control-plane-contract-wiring-complete",
  "encrypted-frame-routing-wired",
  "route-visible-field-allowlist-enforced",
  "plaintext-payload-fields-rejected-before-route",
  "expired-frames-fail-closed-before-route",
  "pwa-exposure-remains-disabled",
  "next-quota-and-metering-integration-slice-selected",
]) {
  assert.ok(routingSummary.evidenceChecks.includes(evidenceCheck), `routing missing evidence: ${evidenceCheck}`);
}

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay runtime encrypted frame routing without PWA exposure",
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
  controlPlaneWiring: {
    readiness: controlPlaneWiringSummary.readiness,
    controlPlaneRuntime: controlPlaneWiringSummary.controlPlaneRuntime,
    startupContract: controlPlaneWiringSummary.startupContract,
  },
  encryptedFrameRouting: routing,
  routeDecision: route,
  summary: {
    readiness: routingSummary.readiness,
    implementationStatus: routingSummary.implementationStatus,
    controlPlaneRuntime: routingSummary.controlPlaneRuntime,
    routeRuntime: routingSummary.routeRuntime,
    pwaExposureDecision: routingSummary.pwaExposureDecision,
    implementationCanContinue: routingSummary.implementationCanContinue,
    selectedRuntimeCanChange: routingSummary.selectedRuntimeCanChange,
  },
  startupContract: routingSummary.startupContract,
  routeContract: routingSummary.routeContract,
  healthSurface: routingSummary.healthSurface,
  remainingImplementationPhases: routingSummary.remainingImplementationPhases,
  evidenceChecks: routingSummary.evidenceChecks,
  guardrails: routingSummary.guardrails,
  productDefault: routingSummary.productDefault,
  nextLocalSlice: routingSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_ENCRYPTED_FRAME_ROUTING_OK ${evidencePath}`);
