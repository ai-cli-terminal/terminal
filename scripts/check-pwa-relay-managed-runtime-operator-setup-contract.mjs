import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  createManagedRelayRuntimeOperatorSetupContract,
  relayManagedRuntimeBrowserOperatorEvidence,
  relayManagedRuntimeOperatorSetupContract,
  relayManagedRuntimePwaExposureGate,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-operator-setup-contract",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_CONTRACT_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-operator-setup-contract.json",
  );

const exposureGate = relayManagedRuntimePwaExposureGate();
const browserEvidence = relayManagedRuntimeBrowserOperatorEvidence();
const setupSummary = relayManagedRuntimeOperatorSetupContract();
const setupContract = createManagedRelayRuntimeOperatorSetupContract({
  serviceId: "managed-relay-runtime-operator-setup-check",
  generatedAtMs: 2000,
  issuedAtMs: 2000,
  expiresAtMs: 4000,
});

assert.equal(exposureGate.readiness, "exposure-gate");
assert.equal(exposureGate.pwaExposure, "explicit-opt-in");
assert.equal(exposureGate.endpointMode, "operator-setup-required");
assert.equal(exposureGate.endpointAutoStart, false);
assert.equal(exposureGate.publicBind, false);

assert.equal(browserEvidence.readiness, "browser-operator-evidence");
assert.equal(
  browserEvidence.nextLocalSlice,
  "managed-relay-runtime-operator-setup-contract",
);
assert.equal(browserEvidence.endpointAutoStart, false);
assert.equal(browserEvidence.publicBind, false);

assert.equal(setupSummary.deploymentMode, "managed");
assert.equal(setupSummary.readiness, "operator-setup-contract");
assert.equal(setupSummary.productDefault, "live-loopback");
assert.equal(setupSummary.selectedRuntime, "explicit-opt-in-managed");
assert.equal(setupSummary.runtimeDefault, "not-selected");
assert.equal(
  setupSummary.implementationStatus,
  "managed-runtime-operator-setup-contract-ready-explicit-activation-only",
);
assert.equal(
  setupSummary.pwaExposureDecision,
  "explicit-opt-in-operator-setup-contract-only",
);
assert.equal(setupSummary.pwaExposure, "explicit-opt-in");
assert.equal(setupSummary.endpointMode, "operator-setup-required");
assert.equal(setupSummary.endpointAutoStart, false);
assert.equal(setupSummary.publicBind, false);
assert.equal(setupSummary.manualConnectRequired, true);
assert.equal(
  setupSummary.endpointActivation,
  "operator-owned-explicit-connect-only",
);
assert.equal(setupSummary.implementationCanContinue, true);
assert.equal(setupSummary.selectedRuntimeCanChange, true);
assert.equal(setupSummary.productDefaultCanChange, false);
assert.equal(
  setupSummary.nextLocalSlice,
  "managed-relay-runtime-operator-setup-import-preflight",
);
assert.ok(
  setupSummary.completedImplementationEvidence.includes(
    "managed-runtime-browser-operator-evidence",
  ),
);
assert.ok(
  setupSummary.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-contract",
  ),
);

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_is_explicit_opt_in_only",
  "operator_setup_payload_is_metadata_only",
  "operator_setup_uses_hashed_identifiers_only",
  "operator_setup_requires_wss_endpoint",
  "operator_setup_excludes_signed_tickets_tokens_and_key_material",
  "operator_setup_requires_manual_connect",
  "managed_relay_endpoint_auto_start_disabled",
  "managed_relay_public_bind_disabled",
  "rollback_to_live_loopback_required",
]) {
  assert.ok(setupSummary.guardrails.includes(guardrail), `operator setup missing guardrail: ${guardrail}`);
}

for (const field of [
  "setup_version",
  "deployment_mode",
  "relay_endpoint_url",
  "tenant_id",
  "session_id_hash",
  "daemon_device_id_hash",
  "companion_device_id_hash",
  "verifier_key_id",
  "verifier_key_version",
  "issued_at_ms",
  "expires_at_ms",
  "operator_setup_text",
  "rollback_transport",
]) {
  assert.ok(setupSummary.requiredSetupFields.includes(field), `operator setup missing required field: ${field}`);
  assert.ok(setupContract.setup_payload_contract.required_fields.includes(field), `contract missing required field: ${field}`);
}

for (const field of ["setup_label", "support_contact", "not_before_ms"]) {
  assert.ok(setupSummary.optionalSetupFields.includes(field), `operator setup missing optional field: ${field}`);
}

for (const prohibited of [
  "payload_json",
  "command_text",
  "context_json",
  "approval_response_payload",
  "payload_ciphertext_hex",
  "payload_nonce_hex",
  "payload_key_hex",
  "shared_secret_hex",
  "private_key_material",
  "raw_session_token",
  "session_token",
  "signed_session_ticket",
  "full_setup_json",
  "hmac_secret",
  "mac_hex",
  "support_actor_id",
  "session_id",
  "daemon_device_id",
  "companion_device_id",
]) {
  assert.ok(setupSummary.prohibitedSetupFields.includes(prohibited), `operator setup missing prohibited field: ${prohibited}`);
  assert.equal(
    setupSummary.setupPayloadContract.allowed_fields.includes(prohibited),
    false,
    `operator setup allowed prohibited field: ${prohibited}`,
  );
}

assert.equal(setupContract.operator_setup_contract_version, 1);
assert.equal(setupContract.deployment_mode, "managed");
assert.equal(setupContract.readiness, "operator-setup-contract");
assert.equal(setupContract.product_default, "live-loopback");
assert.equal(setupContract.selected_runtime, "explicit-opt-in-managed");
assert.equal(setupContract.runtime_default, "not-selected");
assert.equal(setupContract.pwa_exposure, "explicit-opt-in");
assert.equal(setupContract.endpoint_mode, "operator-setup-required");
assert.equal(setupContract.endpoint_auto_start, false);
assert.equal(setupContract.public_bind_enabled, false);
assert.equal(setupContract.setup_source, "service-operator-issued-managed-relay-setup");
assert.equal(setupContract.setup_payload_contract.payload_version, 1);
assert.equal(setupContract.setup_payload_contract.payload_visibility, "metadata-only");
assert.equal(
  setupContract.setup_payload_contract.identifier_policy,
  "hashed-identifiers-only",
);
assert.equal(
  setupContract.setup_payload_contract.authentication_material_policy,
  "not-in-pwa-setup-contract",
);
assert.equal(setupContract.endpoint_contract.required_scheme, "wss");
assert.equal(setupContract.endpoint_contract.endpoint_auto_start, false);
assert.equal(setupContract.endpoint_contract.public_bind_enabled, false);
assert.equal(setupContract.endpoint_contract.connect_requires_user_action, true);
assert.equal(setupContract.activation_contract.manual_connect_required, true);
assert.equal(
  setupContract.activation_contract.imported_setup_does_not_start_endpoint,
  true,
);
assert.equal(
  setupContract.activation_contract.imported_setup_does_not_change_product_default,
  true,
);
assert.equal(setupContract.rollback_contract.rollback_transport, "live-loopback");
assert.equal(setupContract.operator_setup_health.payload_visibility, "metadata-only");
assert.equal(
  setupContract.operator_setup_health.identifier_policy,
  "hashed-identifiers-only",
);

const visibleContractJson = JSON.stringify({
  setupPayloadContract: {
    ...setupContract.setup_payload_contract,
    prohibited_fields: undefined,
  },
  endpointContract: setupContract.endpoint_contract,
  activationContract: setupContract.activation_contract,
  rollbackContract: setupContract.rollback_contract,
  pwaSurfaceContract: {
    ...setupContract.pwa_surface_contract,
    prohibited_visible_fields: undefined,
  },
  operatorSetupHealth: setupContract.operator_setup_health,
  allowedSetupFields: setupContract.allowed_setup_fields,
});

for (const prohibited of [
  '"payload_json"',
  '"command_text"',
  '"context_json"',
  '"approval_response_payload"',
  '"payload_ciphertext_hex"',
  '"payload_nonce_hex"',
  '"payload_key_hex"',
  '"shared_secret_hex"',
  '"private_key_material"',
  '"raw_session_token"',
  '"session_token"',
  '"signed_session_ticket"',
  '"full_setup_json"',
  '"hmac_secret"',
  '"mac_hex"',
  '"support_actor_id"',
  '"session_id"',
  '"daemon_device_id"',
  '"companion_device_id"',
]) {
  assert.equal(
    visibleContractJson.includes(prohibited),
    false,
    `operator setup visible contract leaked ${prohibited}`,
  );
}

assert.throws(
  () =>
    createManagedRelayRuntimeOperatorSetupContract({
      serviceId: "managed-relay-runtime-operator-setup-check",
      generatedAtMs: 2000,
      relayEndpointUrl: "ws://managed-relay.example/relay",
    }),
  /endpoint must be wss/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeOperatorSetupContract({
      serviceId: "managed-relay-runtime-operator-setup-check",
      generatedAtMs: 2000,
      endpointAutoStart: true,
    }),
  /endpoint auto start must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeOperatorSetupContract({
      serviceId: "managed-relay-runtime-operator-setup-check",
      generatedAtMs: 2000,
      publicBind: true,
    }),
  /public bind must stay disabled/,
);
assert.throws(
  () =>
    createManagedRelayRuntimeOperatorSetupContract({
      serviceId: "managed-relay-runtime-operator-setup-check",
      generatedAtMs: 2000,
      sessionIdHash: "managed-session-raw",
    }),
  /session_id_hash/,
);

for (const evidenceCheck of [
  "browser-operator-evidence-complete",
  "operator-setup-contract-versioned",
  "operator-setup-required-fields-defined",
  "operator-setup-optional-fields-defined",
  "operator-setup-allows-metadata-only",
  "operator-setup-uses-hashed-identifiers-only",
  "operator-setup-requires-wss-endpoint",
  "operator-setup-excludes-signed-tickets-tokens-and-key-material",
  "operator-setup-import-does-not-auto-start-endpoint",
  "operator-setup-import-does-not-enable-public-bind",
  "operator-setup-preserves-live-loopback-rollback",
  "next-operator-setup-import-preflight-slice-selected",
]) {
  assert.ok(setupSummary.evidenceChecks.includes(evidenceCheck), `operator setup missing evidence: ${evidenceCheck}`);
}

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay runtime operator setup contract before import preflight",
  exposureGate: {
    readiness: exposureGate.readiness,
    pwaExposure: exposureGate.pwaExposure,
    endpointMode: exposureGate.endpointMode,
    endpointAutoStart: exposureGate.endpointAutoStart,
    publicBind: exposureGate.publicBind,
  },
  browserOperatorEvidence: {
    readiness: browserEvidence.readiness,
    implementationStatus: browserEvidence.implementationStatus,
    nextLocalSlice: browserEvidence.nextLocalSlice,
  },
  operatorSetupContract: setupContract,
  summary: {
    readiness: setupSummary.readiness,
    implementationStatus: setupSummary.implementationStatus,
    selectedRuntime: setupSummary.selectedRuntime,
    pwaExposureDecision: setupSummary.pwaExposureDecision,
    endpointAutoStart: setupSummary.endpointAutoStart,
    publicBind: setupSummary.publicBind,
    manualConnectRequired: setupSummary.manualConnectRequired,
    endpointActivation: setupSummary.endpointActivation,
    nextLocalSlice: setupSummary.nextLocalSlice,
  },
  setupPayloadContract: setupSummary.setupPayloadContract,
  endpointContract: setupSummary.endpointContract,
  activationContract: setupSummary.activationContract,
  rollbackContract: setupSummary.rollbackContract,
  pwaSurfaceContract: setupSummary.pwaSurfaceContract,
  healthSurface: setupSummary.healthSurface,
  evidenceChecks: setupSummary.evidenceChecks,
  guardrails: setupSummary.guardrails,
  productDefault: setupSummary.productDefault,
  nextLocalSlice: setupSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_CONTRACT_OK ${evidencePath}`);
