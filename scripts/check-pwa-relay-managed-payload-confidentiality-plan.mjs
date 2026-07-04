import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { relayManagedPayloadConfidentialityPlan } from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-managed-payload-confidentiality-plan");
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_PAYLOAD_CONFIDENTIALITY_PLAN_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-payload-confidentiality-plan.json");

const plan = relayManagedPayloadConfidentialityPlan();

assert.equal(plan.deploymentMode, "managed");
assert.equal(plan.readiness, "plan");
assert.equal(plan.productDefault, "live-loopback");
assert.equal(plan.selectedRuntime, "deferred");
assert.equal(plan.payloadConfidentiality, "required-payload-blind-managed-relay");
assert.equal(plan.operatorTrustBoundary, "relay-operator-cannot-read-payload-json-or-approval-content");
assert.equal(plan.serviceVisibility, "routing-metadata-and-aggregate-health-only");
assert.equal(plan.managedRuntimeRequirement, "end-to-end-encrypted-frame-payloads-before-runtime");
assert.equal(plan.fallbackDecision, "without-payload-blind-design-managed-relay-remains-deferred");
assert.equal(plan.keyAccessPolicy, "daemon-and-companion-only");
assert.equal(plan.nextLocalSlice, "managed-relay-runtime-service-scaffold");

for (const prohibited of [
  "payload_json",
  "command_text",
  "context_json",
  "approval_response_payload",
  "session_tokens",
  "private_key_material",
  "hmac_secrets",
  "full_setup_json",
]) {
  assert.ok(
    plan.prohibitedManagedRelayData.includes(prohibited),
    `managed payload confidentiality plan missing prohibited data marker: ${prohibited}`,
  );
}

for (const metadata of [
  "tenant_id",
  "session_id",
  "daemon_device_id_hash",
  "companion_device_id_hash",
  "frame_sequence",
  "frame_expiry_ms",
  "ticket_key_id",
  "aggregate_error_class",
]) {
  assert.ok(
    plan.allowedRelayMetadata.includes(metadata),
    `managed payload confidentiality plan missing allowed metadata marker: ${metadata}`,
  );
}

for (const requirement of [
  "frame-payload-e2e-encryption",
  "envelope-metadata-minimization",
  "client-held-payload-keys",
  "key-rotation-and-revocation",
  "confidentiality-regression-evidence",
  "support-payload-redaction",
]) {
  assert.ok(
    plan.requiredBeforeRuntime.includes(requirement),
    `managed payload confidentiality plan missing runtime requirement: ${requirement}`,
  );
}

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_runtime_remains_deferred",
  "payload_blind_managed_relay_required",
  "operator_trust_not_sufficient_for_managed_relay",
  "client_held_payload_keys_required",
  "metadata_minimization_required",
  "support_access_cannot_decrypt_payloads",
]) {
  assert.ok(
    plan.guardrails.includes(guardrail),
    `managed payload confidentiality plan missing guardrail: ${guardrail}`,
  );
}

assert.equal(plan.designDecisions.selfHostedRelay, "explicit-operator-trust-is-acceptable-for-debug-setup");
assert.equal(plan.designDecisions.privateNetworkRelay, "explicit-operator-trust-is-acceptable-for-advanced-setup");
assert.equal(plan.designDecisions.managedRelay, "payload-blind-end-to-end-confidentiality-required");

for (const confidentialityRequirement of [
  "encrypt-live-transport-payload-before-relay-frame",
  "relay-service-routes-opaque-ciphertext-only",
  "approval-request-command-context-remain-client-visible-only",
  "approval-response-remains-client-signed-and-opaque-to-relay",
  "support-exports-redact-ciphertext-and-metadata-identifiers",
  "no-operator-breakglass-to-decrypt-payloads",
]) {
  assert.ok(
    plan.confidentialityRequirements.includes(confidentialityRequirement),
    `managed payload confidentiality plan missing confidentiality requirement: ${confidentialityRequirement}`,
  );
}

for (const blocker of [
  "e2e_payload_encryption_missing",
  "client_key_agreement_missing",
  "metadata_minimization_review_missing",
  "confidentiality_smoke_missing",
  "support_redaction_evidence_missing",
]) {
  assert.ok(plan.implementationBlockers.includes(blocker), `managed payload plan missing blocker: ${blocker}`);
}

assert.ok(plan.completedFollowupContracts.includes("public-verifier-key-operations"));
assert.ok(plan.completedFollowupContracts.includes("billing-and-quota-policy"));

const evidence = {
  status: "planned",
  generatedAt: new Date().toISOString(),
  objective: "Define managed relay payload confidentiality and operator trust boundaries",
  plan: {
    deploymentMode: plan.deploymentMode,
    readiness: plan.readiness,
    selectedRuntime: plan.selectedRuntime,
    payloadConfidentiality: plan.payloadConfidentiality,
    operatorTrustBoundary: plan.operatorTrustBoundary,
    serviceVisibility: plan.serviceVisibility,
    managedRuntimeRequirement: plan.managedRuntimeRequirement,
    fallbackDecision: plan.fallbackDecision,
    keyAccessPolicy: plan.keyAccessPolicy,
  },
  prohibitedManagedRelayData: plan.prohibitedManagedRelayData,
  allowedRelayMetadata: plan.allowedRelayMetadata,
  requiredBeforeRuntime: plan.requiredBeforeRuntime,
  confidentialityRequirements: plan.confidentialityRequirements,
  designDecisions: plan.designDecisions,
  guardrails: plan.guardrails,
  implementationBlockers: plan.implementationBlockers,
  completedFollowupContracts: plan.completedFollowupContracts,
  nextLocalSlice: plan.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_PAYLOAD_CONFIDENTIALITY_PLAN_OK ${evidencePath}`);
