import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  relayManagedRuntimeReadinessGate,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-managed-runtime-readiness-gate");
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_READINESS_GATE_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-runtime-readiness-gate.json");

const gate = relayManagedRuntimeReadinessGate();

assert.equal(gate.deploymentMode, "managed");
assert.equal(gate.readiness, "gate");
assert.equal(gate.productDefault, "live-loopback");
assert.equal(gate.selectedRuntime, "deferred");
assert.equal(gate.gateStatus, "blocked-until-runtime-evidence");
assert.equal(gate.implementationDecision, "managed-runtime-implementation-not-started");
assert.equal(gate.runtimeDefault, "not-selected");
assert.equal(gate.implementationCanStart, false);
assert.equal(gate.readinessDecision, "blocked-by-runtime-evidence");
assert.equal(gate.nextLocalSlice, "managed-relay-public-verifier-key-registry-runtime-smoke");
assert.deepEqual(gate.missingPlanningInputs, []);
assert.ok(gate.completedRuntimeEvidence.includes("payload-blind-frame-encryption-smoke"));
assert.ok(gate.completedRuntimeEvidence.includes("client-key-agreement-runtime-smoke"));
assert.equal(gate.remainingRuntimeEvidence.includes("payload-blind-frame-encryption-smoke"), false);
assert.equal(gate.remainingRuntimeEvidence.includes("client-key-agreement-runtime-smoke"), false);
assert.ok(gate.completedRuntimeEvidence.includes("metadata-minimization-review"));
assert.equal(gate.remainingRuntimeEvidence.includes("metadata-minimization-review"), false);
assert.ok(gate.remainingRuntimeEvidence.includes("public-verifier-key-registry-runtime-smoke"));
assert.ok(gate.resolvedRuntimeBlockers.includes("e2e_payload_encryption_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("client_key_agreement_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("metadata_minimization_review_missing"));
assert.ok(gate.resolvedRuntimeBlockers.includes("confidentiality_smoke_missing"));
assert.equal(gate.remainingRuntimeBlockers.includes("e2e_payload_encryption_missing"), false);
assert.equal(gate.remainingRuntimeBlockers.includes("client_key_agreement_missing"), false);
assert.equal(gate.remainingRuntimeBlockers.includes("metadata_minimization_review_missing"), false);

for (const input of [
  "control-plane-ownership",
  "tenant-isolation",
  "abuse-handling",
  "support-workflows",
  "retention-policy",
  "payload-confidentiality-plan",
  "public-verifier-key-operations",
  "billing-and-quota-policy",
]) {
  assert.ok(gate.requiredPlanningInputs.includes(input), `readiness gate missing required input: ${input}`);
  assert.ok(gate.completedPlanningInputs.includes(input), `readiness gate missing completed input: ${input}`);
}

for (const evidence of [
  "payload-blind-frame-encryption-smoke",
  "client-key-agreement-runtime-smoke",
  "metadata-minimization-review",
  "public-verifier-key-registry-runtime-smoke",
  "revocation-and-rotation-propagation-smoke",
  "tenant-session-registration-quota-smoke",
  "active-session-and-byte-quota-smoke",
  "tenant-aggregate-usage-export-smoke",
  "support-redaction-and-access-review-evidence",
  "billing-abuse-boundary-review",
]) {
  assert.ok(gate.requiredRuntimeEvidence.includes(evidence), `readiness gate missing evidence: ${evidence}`);
}

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "no_managed_runtime_until_readiness_gate_green",
  "runtime_evidence_required_before_pwa_exposure",
  "payload_blind_runtime_required",
  "public_verifier_key_runtime_required",
  "quota_enforcement_runtime_required",
  "aggregate_usage_export_required",
  "support_redaction_required",
]) {
  assert.ok(gate.guardrails.includes(guardrail), `readiness gate missing guardrail: ${guardrail}`);
}

for (const blocker of [
  "e2e_payload_encryption_missing",
  "client_key_agreement_missing",
  "metadata_minimization_review_missing",
  "confidentiality_smoke_missing",
  "support_redaction_evidence_missing",
  "managed_key_registry_runtime_missing",
  "key_revocation_propagation_smoke_missing",
  "rotation_overlap_smoke_missing",
  "managed_usage_meter_runtime_missing",
  "quota_enforcement_smoke_missing",
  "tenant_usage_export_smoke_missing",
  "billing_abuse_boundary_review_missing",
  "runtime_rate_limit_enforcement_missing",
  "abuse_escalation_runbook_missing",
  "tenant_deletion_workflow_missing",
  "support_access_review_missing",
]) {
  assert.ok(gate.auditedRuntimeBlockers.includes(blocker), `readiness gate missing blocker: ${blocker}`);
}

assert.ok(
  gate.runtimeReadinessDomains.payloadConfidentiality.evidence.includes(
    "payload-blind-frame-encryption-smoke",
  ),
);
assert.ok(
  gate.runtimeReadinessDomains.payloadConfidentiality.completedEvidence.includes(
    "payload-blind-frame-encryption-smoke",
  ),
);
assert.ok(
  gate.runtimeReadinessDomains.payloadConfidentiality.completedEvidence.includes(
    "client-key-agreement-runtime-smoke",
  ),
);
assert.ok(
  gate.runtimeReadinessDomains.payloadConfidentiality.completedEvidence.includes(
    "metadata-minimization-review",
  ),
);
assert.ok(
  gate.runtimeReadinessDomains.verifierKeys.evidence.includes(
    "public-verifier-key-registry-runtime-smoke",
  ),
);
assert.ok(
  gate.runtimeReadinessDomains.quotaAndUsage.evidence.includes(
    "active-session-and-byte-quota-smoke",
  ),
);
assert.ok(
  gate.runtimeReadinessDomains.abuseRetentionAndSupport.evidence.includes(
    "support-redaction-and-access-review-evidence",
  ),
);

const evidence = {
  status: "blocked",
  generatedAt: new Date().toISOString(),
  objective: "Audit managed relay runtime blockers and define the minimum readiness gate before implementation",
  gate: {
    deploymentMode: gate.deploymentMode,
    readiness: gate.readiness,
    selectedRuntime: gate.selectedRuntime,
    gateStatus: gate.gateStatus,
    implementationDecision: gate.implementationDecision,
    implementationCanStart: gate.implementationCanStart,
    readinessDecision: gate.readinessDecision,
  },
  requiredPlanningInputs: gate.requiredPlanningInputs,
  completedPlanningInputs: gate.completedPlanningInputs,
  missingPlanningInputs: gate.missingPlanningInputs,
  requiredRuntimeEvidence: gate.requiredRuntimeEvidence,
  completedRuntimeEvidence: gate.completedRuntimeEvidence,
  remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
  runtimeReadinessDomains: gate.runtimeReadinessDomains,
  auditedRuntimeBlockers: gate.auditedRuntimeBlockers,
  resolvedRuntimeBlockers: gate.resolvedRuntimeBlockers,
  remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
  guardrails: gate.guardrails,
  productDefault: gate.productDefault,
  nextLocalSlice: gate.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_READINESS_GATE_OK ${evidencePath}`);
