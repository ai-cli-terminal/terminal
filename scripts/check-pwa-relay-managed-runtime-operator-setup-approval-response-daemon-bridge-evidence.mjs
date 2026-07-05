import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  approvalResponseForRequest,
  generateCompanionKeyMaterial,
  managedRelayDeriveSessionPayloadKeyHex,
  managedRelayRuntimeOperatorSetupApprovalFlowEvidence,
  managedRelayRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence,
  managedRelayRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence,
  relayManagedRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence,
  relayManagedRuntimeOperatorSetupApprovalResponseEndpointBrowserEvidence,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-operator-setup-approval-response-daemon-bridge-evidence",
);
const evidencePath =
  process.env
    .RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_APPROVAL_RESPONSE_DAEMON_BRIDGE_EVIDENCE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-operator-setup-approval-response-daemon-bridge-evidence.json",
  );

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
  setup_label: "managed-daemon-bridge",
  support_contact: "support-managed-relay",
  not_before_ms: 2000,
};

const previousBrowser =
  relayManagedRuntimeOperatorSetupApprovalResponseEndpointBrowserEvidence();
const summary = relayManagedRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence();
assert.equal(
  previousBrowser.nextLocalSlice,
  "managed-relay-runtime-operator-setup-approval-response-daemon-bridge-evidence",
);
assert.equal(summary.readiness, "operator-setup-approval-response-daemon-bridge-evidence");
assert.equal(
  summary.implementationStatus,
  "managed-runtime-operator-setup-approval-response-daemon-bridge-evidence-ready",
);
assert.equal(
  summary.nextLocalSlice,
  "managed-relay-runtime-operator-setup-production-closeout",
);
assert.equal(summary.endpointAutoStart, false);
assert.equal(summary.publicBind, false);
assert.equal(summary.manualCopyFallback, "manual-signed-response-copy-available");
assert.equal(
  summary.approvalVerificationBoundary,
  "existing-daemon-approval-verify-boundary",
);
assert.equal(
  summary.daemonBridgeStatus,
  "verified-existing-approval-validation-boundary",
);
assert.ok(
  summary.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-approval-response-daemon-bridge-evidence",
  ),
);

const companion = await generateCompanionKeyMaterial(webcrypto);
const daemon = await generateCompanionKeyMaterial(webcrypto);
const approvalFlow = await managedRelayRuntimeOperatorSetupApprovalFlowEvidence(
  setupPayload,
  { manualConnectRequested: true },
  2500,
  webcrypto,
);
assert.equal(approvalFlow.status, "approval-flow-ready");
const signedResponse = await approvalResponseForRequest(
  approvalFlow.approvalRequest,
  true,
  companion.keyMaterial,
  webcrypto,
);
const sessionId = "managed-approval-response-daemon-bridge-check";
const companionPayloadKeyHex = await managedRelayDeriveSessionPayloadKeyHex(
  sessionId,
  daemon.identity.noisePubkeyHex,
  companion.keyMaterial,
  webcrypto,
);
const endpointDelivery =
  await managedRelayRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence(
    approvalFlow,
    signedResponse,
    {
      operatorEndpointReady: true,
      manualConnectRequested: true,
      sessionId,
      payloadKeyHex: companionPayloadKeyHex,
      nowMs: 7600,
      nonceHex: "66".repeat(12),
    },
    webcrypto,
  );
assert.equal(endpointDelivery.status, "endpoint-delivery-ready");
assert.equal(endpointDelivery.daemonReceivedApprovalResponse, true);
assert.equal(endpointDelivery.deliveredResponseMatchesApprovalRequest, true);

const daemonBridgeEvidence =
  await managedRelayRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence(
    approvalFlow,
    signedResponse,
    endpointDelivery,
    {
      approvalKeyMaterial: companion.keyMaterial,
      currentContextHash: approvalFlow.approvalRequest.context_hash,
    },
    webcrypto,
  );

assert.equal(daemonBridgeEvidence.status, "daemon-bridge-ready");
assert.equal(daemonBridgeEvidence.deliveryBoundaryReady, true);
assert.equal(daemonBridgeEvidence.endpointDeliveryReady, true);
assert.equal(daemonBridgeEvidence.approvalResponseDaemonBridgeReady, true);
assert.equal(daemonBridgeEvidence.signatureVerifiedByApprovalBoundary, true);
assert.equal(daemonBridgeEvidence.contextHashVerified, true);
assert.equal(daemonBridgeEvidence.daemonReceivedApprovalResponse, true);
assert.equal(daemonBridgeEvidence.deliveredResponseMatchesApprovalRequest, true);
assert.equal(daemonBridgeEvidence.routeEnvelopeVisibleToDaemonBridgeEvidence, false);
assert.equal(daemonBridgeEvidence.payloadKeyVisibleToDaemonBridgeEvidence, false);
assert.equal(daemonBridgeEvidence.payloadCiphertextVisibleToDaemonBridgeEvidence, false);
assert.equal(daemonBridgeEvidence.approvalResponsePayloadLogged, false);
assert.equal(daemonBridgeEvidence.manualCopyFallbackAvailable, true);
assert.deepEqual(daemonBridgeEvidence.blockers, []);
assert.equal(Object.hasOwn(daemonBridgeEvidence, "routeEnvelope"), false);

const missingKeyEvidence =
  await managedRelayRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence(
    approvalFlow,
    signedResponse,
    endpointDelivery,
    {
      currentContextHash: approvalFlow.approvalRequest.context_hash,
    },
    webcrypto,
  );
assert.equal(missingKeyEvidence.status, "blocked");
assert.ok(
  missingKeyEvidence.blockers.includes(
    "managed_operator_setup_daemon_bridge_approval_key_required",
  ),
);

const contextMismatchEvidence =
  await managedRelayRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence(
    approvalFlow,
    signedResponse,
    endpointDelivery,
    {
      approvalKeyMaterial: companion.keyMaterial,
      currentContextHash: `sha256:${"0".repeat(64)}`,
    },
    webcrypto,
  );
assert.equal(contextMismatchEvidence.status, "blocked");
assert.ok(
  contextMismatchEvidence.blockers.includes(
    "managed_operator_setup_daemon_bridge_context_hash_mismatch",
  ),
);

const evidence = {
  status: "daemon-bridge-evidence-ready",
  generatedAt: new Date().toISOString(),
  objective:
    "Prove managed endpoint-delivered approval responses are consumed through the existing daemon approval verification boundary without exposing route, key, ciphertext, token, or private material",
  summary: {
    readiness: summary.readiness,
    implementationStatus: summary.implementationStatus,
    deliveryMode: summary.deliveryMode,
    approvalResponseDelivery: summary.approvalResponseDelivery,
    manualCopyFallback: summary.manualCopyFallback,
    networkDeliveryStatus: summary.networkDeliveryStatus,
    approvalVerificationBoundary: summary.approvalVerificationBoundary,
    daemonBridgeStatus: summary.daemonBridgeStatus,
    endpointAutoStart: summary.endpointAutoStart,
    publicBind: summary.publicBind,
    nextLocalSlice: summary.nextLocalSlice,
  },
  endpointDeliveryEvidence: {
    status: endpointDelivery.status,
    deliveryBoundaryReady: endpointDelivery.deliveryBoundaryReady,
    endpointDeliveryReady: endpointDelivery.approvalResponseEndpointDeliveryReady,
    daemonReceivedApprovalResponse: endpointDelivery.daemonReceivedApprovalResponse,
    deliveredResponseMatchesApprovalRequest:
      endpointDelivery.deliveredResponseMatchesApprovalRequest,
  },
  daemonBridgeEvidence: {
    status: daemonBridgeEvidence.status,
    deliveryBoundaryReady: daemonBridgeEvidence.deliveryBoundaryReady,
    endpointDeliveryReady: daemonBridgeEvidence.endpointDeliveryReady,
    daemonReceivedApprovalResponse:
      daemonBridgeEvidence.daemonReceivedApprovalResponse,
    deliveredResponseMatchesApprovalRequest:
      daemonBridgeEvidence.deliveredResponseMatchesApprovalRequest,
    signedApprovalResponseValid: daemonBridgeEvidence.signedApprovalResponseValid,
    signatureVerifiedByApprovalBoundary:
      daemonBridgeEvidence.signatureVerifiedByApprovalBoundary,
    contextHashVerified: daemonBridgeEvidence.contextHashVerified,
    approvalVerificationBoundary: daemonBridgeEvidence.approvalVerificationBoundary,
    daemonBridgeStatus: daemonBridgeEvidence.daemonBridgeStatus,
    manualCopyFallbackAvailable: daemonBridgeEvidence.manualCopyFallbackAvailable,
    routeEnvelopeVisibleToDaemonBridgeEvidence:
      daemonBridgeEvidence.routeEnvelopeVisibleToDaemonBridgeEvidence,
    payloadKeyVisibleToDaemonBridgeEvidence:
      daemonBridgeEvidence.payloadKeyVisibleToDaemonBridgeEvidence,
    payloadCiphertextVisibleToDaemonBridgeEvidence:
      daemonBridgeEvidence.payloadCiphertextVisibleToDaemonBridgeEvidence,
    approvalResponsePayloadLogged: daemonBridgeEvidence.approvalResponsePayloadLogged,
    privateKeyMaterialVisible: daemonBridgeEvidence.privateKeyMaterialVisible,
  },
  blockedEvidence: {
    missingKey: {
      status: missingKeyEvidence.status,
      blockers: missingKeyEvidence.blockers,
    },
    contextMismatch: {
      status: contextMismatchEvidence.status,
      blockers: contextMismatchEvidence.blockers,
    },
  },
  completedImplementationEvidence: summary.completedImplementationEvidence,
  guardrails: summary.guardrails,
  evidenceChecks: summary.evidenceChecks,
  requiredRustBoundaries: summary.requiredRustBoundaries,
  productDefault: summary.productDefault,
  nextLocalSlice: summary.nextLocalSlice,
};

const bridgeSurfaceJson = JSON.stringify({
  daemonBridgeEvidence: evidence.daemonBridgeEvidence,
  blockedEvidence: evidence.blockedEvidence,
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
  assert.equal(bridgeSurfaceJson.includes(prohibited), false);
}

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(
  `RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_APPROVAL_RESPONSE_DAEMON_BRIDGE_EVIDENCE_OK ${evidencePath}`,
);
