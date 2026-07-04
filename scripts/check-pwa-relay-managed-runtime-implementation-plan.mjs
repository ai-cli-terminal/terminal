import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  relayManagedBillingAbuseBoundaryReview,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimeReadinessGate,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-implementation-plan",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_IMPLEMENTATION_PLAN_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-runtime-implementation-plan.json");

const gate = relayManagedRuntimeReadinessGate();
const billingAbuseBoundaryReview = relayManagedBillingAbuseBoundaryReview();
const plan = relayManagedRuntimeImplementationPlan();

assert.equal(plan.deploymentMode, "managed");
assert.equal(plan.readiness, "plan");
assert.equal(plan.productDefault, "live-loopback");
assert.equal(plan.selectedRuntime, "deferred");
assert.equal(plan.runtimeDefault, "not-selected");
assert.equal(
  plan.implementationStatus,
  "managed-runtime-implementation-plan-ready-runtime-still-deferred",
);
assert.equal(
  plan.implementationBoundary,
  "managed-service-plan-ready-with-pwa-exposure-deferred",
);
assert.equal(
  plan.pwaExposureDecision,
  "deferred-until-runtime-scaffold-and-exposure-gate",
);
assert.equal(plan.implementationCanStart, true);
assert.equal(plan.selectedRuntimeCanChange, false);
assert.equal(plan.nextLocalSlice, "managed-relay-runtime-encrypted-frame-routing");
assert.ok(plan.completedPlanningEvidence.includes("managed-runtime-implementation-plan"));

assert.equal(gate.gateStatus, "runtime-evidence-green");
assert.equal(gate.implementationDecision, "managed-runtime-implementation-can-start");
assert.equal(gate.implementationCanStart, true);
assert.equal(gate.readinessDecision, "ready-for-managed-runtime-implementation");
assert.equal(gate.nextLocalSlice, "managed-relay-runtime-encrypted-frame-routing");
assert.deepEqual(gate.remainingRuntimeEvidence, []);
assert.deepEqual(gate.remainingRuntimeBlockers, []);

assert.equal(billingAbuseBoundaryReview.implementationCanStart, true);
assert.equal(
  billingAbuseBoundaryReview.nextLocalSlice,
  "managed-relay-runtime-encrypted-frame-routing",
);

assert.equal(plan.readinessGate.gateStatus, gate.gateStatus);
assert.equal(plan.readinessGate.implementationCanStart, true);
assert.deepEqual(plan.readinessGate.remainingRuntimeEvidence, []);
assert.deepEqual(plan.readinessGate.remainingRuntimeBlockers, []);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "managed_runtime_not_exposed_until_plan_gate",
  "payload_blind_boundary_preserved",
  "public_verifier_key_boundary_preserved",
  "quota_and_usage_boundaries_preserved",
  "support_and_billing_abuse_boundaries_preserved",
  "rollback_to_live_loopback_required",
]) {
  assert.ok(plan.guardrails.includes(guardrail), `implementation plan missing guardrail: ${guardrail}`);
}

assert.deepEqual(
  plan.implementationPhases.map(({ phase }) => phase),
  [
    "managed-runtime-service-scaffold",
    "managed-runtime-control-plane-contract-wiring",
    "managed-runtime-encrypted-frame-routing",
    "managed-runtime-quota-and-metering-integration",
    "managed-runtime-support-and-abuse-operations-integration",
    "managed-runtime-pwa-exposure-gate",
  ],
);
assert.equal(
  plan.implementationPhases[0].exitEvidence.includes(
    "managed-service-process-starts-without-pwa-exposure",
  ),
  true,
);
assert.equal(
  plan.implementationPhases.at(-1).exitEvidence.includes(
    "live-loopback-rollback-documented",
  ),
  true,
);

for (const allowed of [
  "relay_protocol_version",
  "session_id",
  "sender",
  "sequence",
  "sent_at_ms",
  "expires_at_ms",
  "payload_ciphertext_alg",
  "payload_key_scope",
  "payload_ciphertext_bytes",
]) {
  assert.ok(
    plan.serviceBoundary.allowedManagedVisibleFields.includes(allowed),
    `implementation plan missing allowed visible field: ${allowed}`,
  );
}

for (const prohibited of [
  "payload_json",
  "command_text",
  "context_json",
  "approval_response_payload",
  "payload_key_hex",
  "shared_secret_hex",
  "private_key_material",
  "raw_session_token",
  "full_setup_json",
  "hmac_secret",
  "mac_hex",
]) {
  assert.ok(
    plan.serviceBoundary.prohibitedManagedVisibleFields.includes(prohibited),
    `implementation plan missing prohibited visible field: ${prohibited}`,
  );
  assert.equal(
    plan.serviceBoundary.allowedManagedVisibleFields.includes(prohibited),
    false,
    `implementation plan allowed prohibited visible field: ${prohibited}`,
  );
}

for (const gateName of [
  "readiness_gate_green",
  "managed_runtime_contract_check_passed",
  "payload_blind_frame_routing_smoke_passed",
  "quota_metering_integration_smoke_passed",
  "support_billing_abuse_regression_passed",
  "pwa_copy_and_setup_text_updated",
  "live_loopback_rollback_documented",
]) {
  assert.ok(plan.exposureGates.includes(gateName), `implementation plan missing exposure gate: ${gateName}`);
}

for (const check of [
  "check:pwa-relay-managed-runtime-readiness-gate",
  "check:pwa-relay-managed-billing-abuse-boundary-review",
  "check:pwa-relay-managed-runtime-implementation-plan",
  "check:pwa-relay-managed-runtime-service-scaffold",
  "check:pwa-relay-next-mode-planning",
  "test:pwa",
]) {
  assert.ok(plan.regressionChecks.includes(check), `implementation plan missing regression check: ${check}`);
}

const evidence = {
  status: "planned",
  generatedAt: new Date().toISOString(),
  objective: "Define the managed relay runtime implementation plan before service scaffold work",
  plan: {
    deploymentMode: plan.deploymentMode,
    readiness: plan.readiness,
    selectedRuntime: plan.selectedRuntime,
    runtimeDefault: plan.runtimeDefault,
    implementationStatus: plan.implementationStatus,
    implementationBoundary: plan.implementationBoundary,
    pwaExposureDecision: plan.pwaExposureDecision,
    selectedRuntimeCanChange: plan.selectedRuntimeCanChange,
    implementationCanStart: plan.implementationCanStart,
  },
  readinessGate: plan.readinessGate,
  serviceBoundary: plan.serviceBoundary,
  implementationPhases: plan.implementationPhases,
  exposureGates: plan.exposureGates,
  regressionChecks: plan.regressionChecks,
  guardrails: plan.guardrails,
  productDefault: plan.productDefault,
  nextLocalSlice: plan.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_IMPLEMENTATION_PLAN_OK ${evidencePath}`);
