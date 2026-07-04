import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { relayManagedOperationsPlan } from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-managed-operations-planning");
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_OPERATIONS_PLANNING_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-operations-planning.json");

const plan = relayManagedOperationsPlan();

assert.equal(plan.deploymentMode, "managed");
assert.equal(plan.readiness, "planning");
assert.equal(plan.productDefault, "live-loopback");
assert.equal(plan.selectedRuntime, "deferred");
assert.equal(plan.privateNetworkRelay, "explicit-advanced-path-ready");
assert.equal(plan.implementationStatus, "operations-contract-ready-runtime-still-deferred");
assert.equal(plan.nextLocalSlice, "managed-relay-metadata-minimization-review");

for (const requirement of [
  "control-plane-ownership",
  "tenant-isolation",
  "abuse-handling",
  "support-workflows",
  "retention-policy",
  "billing-and-quota-policy",
  "public-verifier-key-operations",
  "payload-confidentiality-plan",
]) {
  assert.ok(
    plan.requiredBeforeImplementation.includes(requirement),
    `managed operations plan missing requirement: ${requirement}`,
  );
}

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_remains_deferred",
  "private_network_relay_remains_explicit_advanced_path",
  "self_hosted_relay_readiness_remains_separate",
  "no_managed_runtime_without_operations_contract",
]) {
  assert.ok(plan.guardrails.includes(guardrail), `managed operations plan missing guardrail: ${guardrail}`);
}

for (const completed of [
  "control-plane-ownership",
  "tenant-isolation",
  "abuse-handling",
  "support-workflows",
  "retention-policy",
  "payload-confidentiality-plan",
  "public-verifier-key-operations",
  "billing-and-quota-policy",
]) {
  assert.ok(
    plan.completedOperationContracts.includes(completed),
    `managed operations plan missing completed contract: ${completed}`,
  );
}

assert.deepEqual(plan.remainingOperationContracts, []);
assert.deepEqual(plan.blockers, []);

const evidence = {
  status: "planned",
  generatedAt: new Date().toISOString(),
  objective: "Define managed relay operations requirements before implementation",
  managedRelay: {
    deploymentMode: plan.deploymentMode,
    readiness: plan.readiness,
    selectedRuntime: plan.selectedRuntime,
    implementationStatus: plan.implementationStatus,
    blockers: plan.blockers,
  },
  guardrails: plan.guardrails,
  requiredBeforeImplementation: plan.requiredBeforeImplementation,
  completedOperationContracts: plan.completedOperationContracts,
  remainingOperationContracts: plan.remainingOperationContracts,
  operationAreas: plan.operationAreas,
  productDefault: plan.productDefault,
  nextLocalSlice: plan.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_OPERATIONS_PLANNING_OK ${evidencePath}`);
