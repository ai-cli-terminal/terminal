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
  managedRelayRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence,
  relayManagedRuntimeOperatorSetupApprovalResponseDeliveryBoundary,
  relayManagedRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(
  repoRoot,
  "artifacts",
  "ra-pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence",
);
const evidencePath =
  process.env
    .RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_APPROVAL_RESPONSE_ENDPOINT_DELIVERY_EVIDENCE_PATH ||
  path.join(
    artifactRoot,
    "ra-pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence.json",
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
  setup_label: "managed-endpoint-delivery",
  support_contact: "support-managed-relay",
  not_before_ms: 2000,
};

const summary = relayManagedRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence();
const previousBoundary = relayManagedRuntimeOperatorSetupApprovalResponseDeliveryBoundary();
assert.equal(
  previousBoundary.nextLocalSlice,
  "managed-relay-runtime-operator-setup-approval-response-endpoint-delivery-evidence",
);
assert.equal(summary.readiness, "operator-setup-approval-response-endpoint-delivery-evidence");
assert.equal(
  summary.nextLocalSlice,
  "managed-relay-runtime-operator-setup-approval-response-endpoint-browser-evidence",
);
assert.equal(summary.endpointAutoStart, false);
assert.equal(summary.endpointStartedByAutoStart, false);
assert.equal(summary.publicBind, false);
assert.equal(summary.manualCopyFallback, "manual-signed-response-copy-available");
assert.equal(summary.networkDeliveryStatus, "verified-explicit-managed-endpoint-delivery");
assert.ok(
  summary.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence",
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
const sessionId = "managed-approval-response-endpoint-delivery-check";
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
      nonceHex: "55".repeat(12),
    },
    webcrypto,
  );

assert.equal(endpointDelivery.status, "endpoint-delivery-ready");
assert.equal(endpointDelivery.deliveryBoundaryReady, true);
assert.equal(endpointDelivery.operatorEndpointReady, true);
assert.equal(endpointDelivery.manualConnectRequested, true);
assert.equal(endpointDelivery.endpointAutoStart, false);
assert.equal(endpointDelivery.endpointStartedByAutoStart, false);
assert.equal(endpointDelivery.publicBind, false);
assert.equal(endpointDelivery.publicBindEnabledOnDelivery, false);
assert.equal(endpointDelivery.networkConnectionStartedOnDelivery, true);
assert.equal(endpointDelivery.webSocketCreatedOnDelivery, true);
assert.equal(endpointDelivery.endpointStartedOnDelivery, true);
assert.equal(endpointDelivery.encryptedFrameDelivery, true);
assert.equal(endpointDelivery.routeDecision, "accepted");
assert.equal(endpointDelivery.routeEnvelope.session_id, sessionId);
assert.equal("payload_json" in endpointDelivery.routeEnvelope, false);
assert.equal("payload_ciphertext_hex" in endpointDelivery.routeEnvelope, false);
assert.equal(endpointDelivery.plaintextPayloadVisibleToRelay, false);
assert.equal(endpointDelivery.approvalResponsePayloadVisibleToRelay, false);
assert.equal(endpointDelivery.payloadKeyVisibleToRelay, false);
assert.equal(endpointDelivery.payloadCiphertextVisibleToOperator, false);
assert.equal(endpointDelivery.daemonReceivedMessageType, "approval_response");
assert.equal(endpointDelivery.daemonReceivedApprovalResponse, true);
assert.equal(endpointDelivery.deliveredResponseMatchesApprovalRequest, true);
assert.deepEqual(endpointDelivery.blockers, []);

const blockedDelivery =
  await managedRelayRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence(
    approvalFlow,
    signedResponse,
    {
      manualConnectRequested: true,
      sessionId,
      payloadKeyHex: companionPayloadKeyHex,
      nowMs: 7600,
    },
    webcrypto,
  );
assert.equal(blockedDelivery.status, "blocked");
assert.ok(
  blockedDelivery.blockers.includes(
    "managed_operator_setup_operator_started_endpoint_required",
  ),
);

const evidence = {
  status: "endpoint-delivery-evidence-ready",
  generatedAt: new Date().toISOString(),
  objective:
    "Prove managed operator setup approval responses can use an explicit operator-started encrypted endpoint path without exposing payloads or enabling auto-start/public bind",
  summary: {
    readiness: summary.readiness,
    implementationStatus: summary.implementationStatus,
    deliveryMode: summary.deliveryMode,
    approvalResponseDelivery: summary.approvalResponseDelivery,
    manualCopyFallback: summary.manualCopyFallback,
    networkDeliveryStatus: summary.networkDeliveryStatus,
    endpointAutoStart: summary.endpointAutoStart,
    endpointStartedByAutoStart: summary.endpointStartedByAutoStart,
    publicBind: summary.publicBind,
    nextLocalSlice: summary.nextLocalSlice,
  },
  endpointDeliveryEvidence: {
    deliveryBoundaryReady: endpointDelivery.deliveryBoundaryReady,
    operatorEndpointReady: endpointDelivery.operatorEndpointReady,
    manualConnectRequested: endpointDelivery.manualConnectRequested,
    endpointStartedByOperator: endpointDelivery.endpointStartedByOperator,
    endpointStartedByAutoStart: endpointDelivery.endpointStartedByAutoStart,
    endpointAutoStart: endpointDelivery.endpointAutoStart,
    publicBind: endpointDelivery.publicBind,
    publicBindEnabledOnDelivery: endpointDelivery.publicBindEnabledOnDelivery,
    encryptedFrameDelivery: endpointDelivery.encryptedFrameDelivery,
    routeDecision: endpointDelivery.routeDecision,
    routeState: endpointDelivery.routeState,
    routeEnvelope: endpointDelivery.routeEnvelope,
    routeVisibleFields: endpointDelivery.routeVisibleFields,
    routeVisiblePayload: endpointDelivery.routeVisiblePayload,
    plaintextPayloadVisibleToRelay: endpointDelivery.plaintextPayloadVisibleToRelay,
    approvalResponsePayloadVisibleToRelay:
      endpointDelivery.approvalResponsePayloadVisibleToRelay,
    payloadKeyVisibleToRelay: endpointDelivery.payloadKeyVisibleToRelay,
    payloadCiphertextVisibleToOperator:
      endpointDelivery.payloadCiphertextVisibleToOperator,
    daemonReceivedMessageType: endpointDelivery.daemonReceivedMessageType,
    daemonReceivedApprovalResponse: endpointDelivery.daemonReceivedApprovalResponse,
    deliveredResponseMatchesApprovalRequest:
      endpointDelivery.deliveredResponseMatchesApprovalRequest,
  },
  blockedDeliveryEvidence: {
    status: blockedDelivery.status,
    blockers: blockedDelivery.blockers,
  },
  completedImplementationEvidence: summary.completedImplementationEvidence,
  guardrails: summary.guardrails,
  evidenceChecks: summary.evidenceChecks,
  productDefault: summary.productDefault,
  nextLocalSlice: summary.nextLocalSlice,
};

const deliveredSurfaceJson = JSON.stringify({
  endpointDeliveryEvidence: evidence.endpointDeliveryEvidence,
  blockedDeliveryEvidence: evidence.blockedDeliveryEvidence,
});
for (const prohibited of [
  "payload_key_hex",
  "shared_secret_hex",
  "payload_ciphertext_hex",
  "approval_response_payload",
  "private_key_material",
]) {
  assert.equal(deliveredSurfaceJson.includes(prohibited), false);
}

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(
  `RA_PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_APPROVAL_RESPONSE_ENDPOINT_DELIVERY_EVIDENCE_OK ${evidencePath}`,
);
