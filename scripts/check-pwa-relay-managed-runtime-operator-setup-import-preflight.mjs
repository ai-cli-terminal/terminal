import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  managedRelayRuntimeOperatorSetupImportPreflight,
  parseManagedRelayRuntimeOperatorSetupInput,
  relayManagedRuntimeOperatorSetupContract,
  relayManagedRuntimeOperatorSetupImportPreflight,
  validateManagedRelayRuntimeOperatorSetupMetadata,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-operator-setup-import-preflight",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_IMPORT_PREFLIGHT_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-operator-setup-import-preflight.json",
  );

const setupContract = relayManagedRuntimeOperatorSetupContract();
const importSummary = relayManagedRuntimeOperatorSetupImportPreflight();
const indexHtml = await readFile(path.join(repoRoot, "pwa", "index.html"), "utf8");

const setupPayload = {
  setup_version: 1,
  deployment_mode: "managed",
  relay_endpoint_url: "wss://managed-relay.example/relay",
  tenant_id: "tenant-managed-relay",
  session_id_hash: "sha256:1111111111111111",
  daemon_device_id_hash: "sha256:2222222222222222",
  companion_device_id_hash: "sha256:3333333333333333",
  verifier_key_id: "managed-relay-key-a",
  verifier_key_version: 1,
  issued_at_ms: 2000,
  expires_at_ms: 4000,
  operator_setup_text: "Managed relay setup requires operator-issued activation.",
  rollback_transport: "live-loopback",
  setup_label: "managed-preflight",
  support_contact: "support-managed-relay",
  not_before_ms: 2000,
};

assert.equal(setupContract.readiness, "operator-setup-contract");
assert.equal(
  setupContract.nextLocalSlice,
  "managed-relay-runtime-operator-setup-import-preflight",
);
assert.equal(importSummary.deploymentMode, "managed");
assert.equal(importSummary.readiness, "operator-setup-import-preflight");
assert.equal(importSummary.productDefault, "live-loopback");
assert.equal(importSummary.selectedRuntime, "explicit-opt-in-managed");
assert.equal(importSummary.runtimeDefault, "not-selected");
assert.equal(
  importSummary.implementationStatus,
  "managed-runtime-operator-setup-import-preflight-ready-status-only",
);
assert.equal(
  importSummary.pwaExposureDecision,
  "explicit-opt-in-managed-setup-import-status-only",
);
assert.equal(importSummary.pwaExposure, "explicit-opt-in");
assert.equal(importSummary.endpointMode, "operator-setup-required");
assert.equal(importSummary.endpointAutoStart, false);
assert.equal(importSummary.publicBind, false);
assert.equal(importSummary.manualConnectRequired, true);
assert.equal(importSummary.setupRendering, "sanitized-summary-only");
assert.equal(
  importSummary.endpointActivation,
  "manual-connect-required-not-started-by-import",
);
assert.equal(
  importSummary.nextLocalSlice,
  "managed-relay-runtime-operator-setup-browser-evidence",
);
assert.ok(
  importSummary.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-import-preflight",
  ),
);

for (const selector of importSummary.requiredSelectors) {
  assert.ok(indexHtml.includes(`id="${selector.slice(1)}"`), `PWA index missing ${selector}`);
}

for (const guardrail of [
  "product_default_remains_live_loopback",
  "managed_relay_is_explicit_opt_in_only",
  "operator_setup_import_is_preflight_only",
  "managed_setup_original_json_not_rendered_after_import",
  "managed_setup_summary_is_sanitized",
  "operator_setup_import_requires_wss_endpoint",
  "operator_setup_import_uses_hashed_identifiers_only",
  "operator_setup_import_excludes_signed_tickets_tokens_and_key_material",
  "operator_setup_import_does_not_auto_start_endpoint",
  "operator_setup_import_does_not_enable_public_bind",
  "rollback_to_live_loopback_required",
]) {
  assert.ok(importSummary.guardrails.includes(guardrail), `import preflight missing guardrail: ${guardrail}`);
}

assert.deepEqual(validateManagedRelayRuntimeOperatorSetupMetadata(setupPayload), setupPayload);
assert.deepEqual(
  parseManagedRelayRuntimeOperatorSetupInput(JSON.stringify(setupPayload)),
  setupPayload,
);
const encodedSetup = encodeURIComponent(JSON.stringify(setupPayload));
assert.deepEqual(
  parseManagedRelayRuntimeOperatorSetupInput("", `?relaySetup=${encodedSetup}`),
  setupPayload,
);

const readyPreflight = managedRelayRuntimeOperatorSetupImportPreflight(
  setupPayload,
  2500,
);
assert.equal(readyPreflight.status, "ready");
assert.equal(readyPreflight.importReady, true);
assert.equal(readyPreflight.connectEnabled, false);
assert.equal(readyPreflight.endpointAutoStart, false);
assert.equal(readyPreflight.publicBind, false);
assert.equal(readyPreflight.manualConnectRequired, true);
assert.equal(readyPreflight.setupRendering, "sanitized-summary-only");
assert.deepEqual(readyPreflight.blockers, []);
assert.equal(readyPreflight.sanitizedSetup.session_id_hash, "sha256:1111111111111111");

const expiredPreflight = managedRelayRuntimeOperatorSetupImportPreflight(
  setupPayload,
  4000,
);
assert.equal(expiredPreflight.status, "blocked");
assert.ok(expiredPreflight.blockers.includes("managed_operator_setup_expired"));
assert.equal(expiredPreflight.connectEnabled, false);

const notBeforePreflight = managedRelayRuntimeOperatorSetupImportPreflight(
  { ...setupPayload, not_before_ms: 3000 },
  2500,
);
assert.equal(notBeforePreflight.status, "blocked");
assert.ok(notBeforePreflight.blockers.includes("managed_operator_setup_not_before"));
assert.equal(notBeforePreflight.connectEnabled, false);

for (const invalid of [
  { ...setupPayload, relay_endpoint_url: "ws://managed-relay.example/relay" },
  { ...setupPayload, deployment_mode: "self-hosted" },
  { ...setupPayload, rollback_transport: "relay" },
  { ...setupPayload, signed_session_ticket: "not-allowed" },
  { ...setupPayload, session_id: "managed-session-raw" },
  { ...setupPayload, daemon_device_id: "daemon-raw" },
  { ...setupPayload, extra_field: "not-allowed" },
]) {
  assert.throws(
    () => parseManagedRelayRuntimeOperatorSetupInput(JSON.stringify(invalid)),
    /managed relay setup/,
  );
  const blocked = managedRelayRuntimeOperatorSetupImportPreflight(invalid, 2500);
  assert.equal(blocked.status, "blocked");
  assert.equal(blocked.importReady, false);
  assert.equal(blocked.connectEnabled, false);
}

const visibleSummaryJson = JSON.stringify({
  importPreflight: {
    ...importSummary.importPreflight,
    prohibitedVisibleTokens: undefined,
  },
  readyPreflight: {
    ...readyPreflight,
    sanitizedSetup: readyPreflight.sanitizedSetup,
  },
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
    visibleSummaryJson.includes(prohibited),
    false,
    `import preflight visible summary leaked ${prohibited}`,
  );
}

for (const evidenceCheck of [
  "operator-setup-contract-complete",
  "managed-operator-setup-parser-accepts-contract-payload",
  "managed-operator-setup-parser-rejects-unknown-fields",
  "managed-operator-setup-parser-rejects-prohibited-fields",
  "managed-operator-setup-preflight-requires-wss-endpoint",
  "managed-operator-setup-preflight-requires-unexpired-window",
  "managed-operator-setup-preflight-keeps-connect-disabled",
  "managed-operator-setup-import-keeps-endpoint-auto-start-disabled",
  "managed-operator-setup-import-keeps-public-bind-disabled",
  "managed-operator-setup-import-renders-sanitized-summary-only",
  "managed-operator-setup-import-preserves-live-loopback-rollback",
  "next-managed-operator-setup-browser-evidence-slice-selected",
]) {
  assert.ok(importSummary.evidenceChecks.includes(evidenceCheck), `import preflight missing evidence: ${evidenceCheck}`);
}

const evidence = {
  status: "passed",
  generatedAt: new Date().toISOString(),
  objective: "Verify managed relay operator setup import preflight before browser evidence",
  setupContract: {
    readiness: setupContract.readiness,
    implementationStatus: setupContract.implementationStatus,
    nextLocalSlice: setupContract.nextLocalSlice,
  },
  importSummary: {
    readiness: importSummary.readiness,
    implementationStatus: importSummary.implementationStatus,
    selectedRuntime: importSummary.selectedRuntime,
    pwaExposureDecision: importSummary.pwaExposureDecision,
    setupRendering: importSummary.setupRendering,
    endpointAutoStart: importSummary.endpointAutoStart,
    publicBind: importSummary.publicBind,
    manualConnectRequired: importSummary.manualConnectRequired,
    nextLocalSlice: importSummary.nextLocalSlice,
  },
  setupPayload: {
    ...setupPayload,
    operator_setup_text: "redacted-from-evidence-summary",
  },
  readyPreflight,
  expiredPreflight,
  notBeforePreflight,
  requiredSelectors: importSummary.requiredSelectors,
  evidenceChecks: importSummary.evidenceChecks,
  guardrails: importSummary.guardrails,
  productDefault: importSummary.productDefault,
  nextLocalSlice: importSummary.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_IMPORT_PREFLIGHT_OK ${evidencePath}`);
