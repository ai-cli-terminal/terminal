import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  relayManagedRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence,
  relayManagedRuntimeOperatorSetupProductionCloseout,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-operator-setup-production-closeout",
);
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_PRODUCTION_CLOSEOUT_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-operator-setup-production-closeout.json",
  );

const daemonBridge =
  relayManagedRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence();
const closeout = relayManagedRuntimeOperatorSetupProductionCloseout();

assert.equal(
  daemonBridge.nextLocalSlice,
  "managed-relay-runtime-operator-setup-production-closeout",
);
assert.equal(closeout.readiness, "operator-setup-production-closeout");
assert.equal(
  closeout.implementationStatus,
  "managed-runtime-operator-setup-production-closeout-ready",
);
assert.equal(
  closeout.productionCloseoutStatus,
  "managed-operator-setup-local-chain-closed",
);
assert.equal(closeout.localManagedOperatorSetupComplete, true);
assert.equal(closeout.productDefault, "live-loopback");
assert.equal(closeout.selectedRuntime, "explicit-opt-in-managed");
assert.equal(closeout.runtimeDefault, "not-selected");
assert.equal(closeout.endpointAutoStart, false);
assert.equal(closeout.endpointStartedByAutoStart, false);
assert.equal(closeout.publicBind, false);
assert.equal(closeout.publicBindEnabledOnDelivery, false);
assert.equal(closeout.manualCopyFallback, "manual-signed-response-copy-available");
assert.equal(
  closeout.approvalVerificationBoundary,
  "existing-daemon-approval-verify-boundary",
);
assert.equal(
  closeout.productionCloseout.allLocalManagedOperatorSetupEvidenceComplete,
  true,
);
assert.deepEqual(
  closeout.productionCloseout.remainingManagedOperatorSetupEvidence,
  [],
);
assert.equal(
  closeout.operatorEvidence.fullManagedOperatorSetupEvidenceChainComplete,
  true,
);
assert.equal(closeout.operatorEvidence.routeEnvelopeVisible, false);
assert.equal(closeout.operatorEvidence.payloadKeyVisible, false);
assert.equal(closeout.operatorEvidence.payloadCiphertextVisible, false);
assert.equal(closeout.operatorEvidence.privateKeyMaterialVisible, false);

for (const evidenceName of [
  "managed-runtime-operator-setup-contract",
  "managed-runtime-operator-setup-import-preflight",
  "managed-runtime-operator-setup-browser-evidence",
  "managed-runtime-operator-setup-connection-controls",
  "managed-runtime-operator-setup-session-handshake",
  "managed-runtime-operator-setup-approval-flow-evidence",
  "managed-runtime-operator-setup-runbook-closeout",
  "managed-runtime-operator-setup-approval-response-delivery-boundary",
  "managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence",
  "managed-runtime-operator-setup-approval-response-endpoint-browser-evidence",
  "managed-runtime-operator-setup-approval-response-daemon-bridge-evidence",
]) {
  assert.ok(
    closeout.requiredEvidenceChain.includes(evidenceName),
    `missing required evidence ${evidenceName}`,
  );
  assert.ok(
    closeout.completedImplementationEvidence.includes(evidenceName),
    `missing completed evidence ${evidenceName}`,
  );
}
assert.ok(
  closeout.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-production-closeout",
  ),
);

for (const command of [
  "npm run check:pwa-relay-managed-runtime-operator-setup-contract",
  "npm run check:pwa-relay-managed-runtime-operator-setup-import-preflight",
  "npm run smoke:pwa-relay-managed-runtime-operator-setup-browser-evidence",
  "npm run smoke:pwa-relay-managed-runtime-operator-setup-connection-controls",
  "npm run smoke:pwa-relay-managed-runtime-operator-setup-session-handshake",
  "npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-flow-evidence",
  "npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence",
  "npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-browser-evidence",
  "npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-daemon-bridge-evidence",
  "npm run check:pwa-relay-managed-runtime-operator-setup-production-closeout",
  "npm run check:pwa-relay-deployment-runbook",
]) {
  assert.ok(
    closeout.requiredRunbookCommands.includes(command),
    `missing required runbook command ${command}`,
  );
}

for (const evidenceCheck of [
  "operator-setup-approval-response-daemon-bridge-evidence-complete",
  "managed-operator-setup-production-closeout-links-contract",
  "managed-operator-setup-production-closeout-links-endpoint-delivery",
  "managed-operator-setup-production-closeout-links-endpoint-browser",
  "managed-operator-setup-production-closeout-links-daemon-bridge",
  "managed-operator-setup-production-closeout-keeps-live-loopback-default",
  "managed-operator-setup-production-closeout-keeps-explicit-opt-in",
  "managed-operator-setup-production-closeout-keeps-endpoint-auto-start-disabled",
  "managed-operator-setup-production-closeout-keeps-public-bind-disabled",
  "managed-operator-setup-production-closeout-keeps-manual-copy-fallback",
  "managed-operator-setup-production-closeout-uses-existing-approval-validation-boundary",
  "managed-operator-setup-production-closeout-hides-route-envelope",
  "managed-operator-setup-production-closeout-does-not-log-payload-key",
  "managed-operator-setup-production-closeout-does-not-log-ciphertext",
  "next-release-followup-external-evidence-closeout-selected",
]) {
  assert.ok(
    closeout.evidenceChecks.includes(evidenceCheck),
    `missing evidence check ${evidenceCheck}`,
  );
}

const evidence = {
  status: "production-closeout-ready",
  generatedAt: new Date().toISOString(),
  objective:
    "Close the managed relay runtime operator setup local evidence chain while preserving explicit opt-in, live-loopback default, manual fallback, and payload-blind evidence boundaries",
  summary: {
    readiness: closeout.readiness,
    implementationStatus: closeout.implementationStatus,
    productionCloseoutStatus: closeout.productionCloseoutStatus,
    localManagedOperatorSetupComplete: closeout.localManagedOperatorSetupComplete,
    productDefault: closeout.productDefault,
    selectedRuntime: closeout.selectedRuntime,
    runtimeDefault: closeout.runtimeDefault,
    endpointAutoStart: closeout.endpointAutoStart,
    publicBind: closeout.publicBind,
    manualCopyFallback: closeout.manualCopyFallback,
    approvalVerificationBoundary: closeout.approvalVerificationBoundary,
    nextLocalSlice: closeout.nextLocalSlice,
  },
  daemonBridgeEvidence: closeout.daemonBridgeEvidence,
  productionCloseout: closeout.productionCloseout,
  operatorEvidence: closeout.operatorEvidence,
  requiredEvidenceChain: closeout.requiredEvidenceChain,
  requiredRunbookCommands: closeout.requiredRunbookCommands,
  completedImplementationEvidence: closeout.completedImplementationEvidence,
  guardrails: closeout.guardrails,
  evidenceChecks: closeout.evidenceChecks,
};

const closeoutSurfaceJson = JSON.stringify({
  productionCloseout: evidence.productionCloseout,
  operatorEvidence: evidence.operatorEvidence,
});
for (const prohibited of [
  "route_envelope",
  "payload_key_hex",
  "shared_secret_hex",
  "payload_ciphertext_hex",
  "payload_ciphertext_bytes",
  "approval_response_payload",
  "private_key_material",
  "raw_session_token",
]) {
  assert.equal(closeoutSurfaceJson.includes(prohibited), false);
}

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(
  `RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_PRODUCTION_CLOSEOUT_OK ${evidencePath}`,
);
