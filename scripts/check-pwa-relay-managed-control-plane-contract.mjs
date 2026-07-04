import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { relayManagedControlPlaneContract } from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-managed-control-plane-contract");
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_CONTROL_PLANE_CONTRACT_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-control-plane-contract.json");

const contract = relayManagedControlPlaneContract();

assert.equal(contract.deploymentMode, "managed");
assert.equal(contract.readiness, "contract");
assert.equal(contract.productDefault, "live-loopback");
assert.equal(contract.selectedRuntime, "deferred");
assert.equal(contract.controlPlaneOwner, "required-before-runtime");
assert.equal(contract.tenantBoundary, "tenant-isolated-sessions-and-verifier-keys");
assert.equal(contract.sessionBoundary, "per-session-ticket-and-frame-isolation");
assert.equal(contract.operatorVisibleState, "aggregate-health-and-control-plane-events-only");
assert.equal(contract.auditBoundary, "no-payload-json-or-secret-material");
assert.equal(contract.nextLocalSlice, "managed-relay-metadata-minimization-review");

for (const role of ["service-operator", "tenant-admin", "daemon-owner", "support-operator"]) {
  assert.ok(contract.requiredRoles.includes(role), `managed control-plane contract missing role: ${role}`);
}

for (const required of [
  "tenant-identity",
  "session-registration",
  "verifier-key-distribution",
  "quota-and-rate-limit",
  "support-access",
  "audit-retention",
]) {
  assert.ok(
    contract.requiredContracts.includes(required),
    `managed control-plane contract missing required contract: ${required}`,
  );
}

for (const prohibited of [
  "payload_json",
  "session_tokens",
  "approval_signatures",
  "private_key_material",
  "hmac_secrets",
  "full_setup_json",
]) {
  assert.ok(
    contract.prohibitedControlPlaneData.includes(prohibited),
    `managed control-plane contract missing prohibited data marker: ${prohibited}`,
  );
}

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "tenant_data_isolation_required",
  "operator_state_excludes_payload_json",
  "support_access_requires_audit_boundary",
]) {
  assert.ok(contract.guardrails.includes(guardrail), `managed control-plane contract missing guardrail: ${guardrail}`);
}

assert.ok(contract.responsibilities.serviceOperator.includes("operate-relay-control-plane"));
assert.ok(contract.responsibilities.tenantAdmin.includes("rotate-tenant-verifier-keys"));
assert.ok(contract.responsibilities.daemonOwner.includes("validate-approval-responses"));
assert.ok(contract.responsibilities.supportOperator.includes("never-view-payload-json-or-secrets"));
assert.ok(contract.completedFollowupContracts.includes("billing-and-quota-policy"));

const evidence = {
  status: "contract",
  generatedAt: new Date().toISOString(),
  objective: "Define managed relay control-plane ownership, tenant boundaries, and audit constraints",
  contract: {
    deploymentMode: contract.deploymentMode,
    readiness: contract.readiness,
    selectedRuntime: contract.selectedRuntime,
    controlPlaneOwner: contract.controlPlaneOwner,
    tenantBoundary: contract.tenantBoundary,
    sessionBoundary: contract.sessionBoundary,
    operatorVisibleState: contract.operatorVisibleState,
    auditBoundary: contract.auditBoundary,
  },
  requiredRoles: contract.requiredRoles,
  requiredContracts: contract.requiredContracts,
  prohibitedControlPlaneData: contract.prohibitedControlPlaneData,
  guardrails: contract.guardrails,
  blockers: contract.blockers,
  completedFollowupContracts: contract.completedFollowupContracts,
  nextLocalSlice: contract.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_CONTROL_PLANE_CONTRACT_OK ${evidencePath}`);
