import assert from "node:assert/strict";
import { webcrypto } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  generateCompanionKeyMaterial,
  liveApprovalRequestMessage,
  managedRelayDeriveSessionPayloadKeyHex,
  managedRelayEncryptedFrameFromLiveMessage,
  managedRelayEncryptedFrameJson,
  managedRelayEncryptedFrameRouteEnvelope,
  relayManagedMetadataMinimizationReview,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-managed-metadata-minimization-review");
const evidencePath =
  process.env.RA_PWA_RELAY_MANAGED_METADATA_MINIMIZATION_REVIEW_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-managed-metadata-minimization-review.json");

const review = relayManagedMetadataMinimizationReview();
assert.equal(review.deploymentMode, "managed");
assert.equal(review.readiness, "review");
assert.equal(review.productDefault, "live-loopback");
assert.equal(review.selectedRuntime, "deferred");
assert.equal(review.implementationStatus, "metadata-minimization-review-complete-runtime-still-deferred");
assert.equal(review.metadataBoundary, "allowlisted-route-control-billing-support-audit-metadata-only");
assert.equal(review.implementationCanStart, true);
assert.equal(review.nextLocalSlice, "managed-relay-runtime-support-and-abuse-operations-integration");
assert.ok(review.completedRuntimeEvidence.includes("metadata-minimization-review"));
assert.ok(review.closedReadinessBlockers.includes("metadata_minimization_review_missing"));
assert.equal(review.remainingRuntimeEvidence.includes("metadata-minimization-review"), false);
assert.equal(review.remainingRuntimeEvidence.includes("public-verifier-key-registry-runtime-smoke"), false);
assert.equal(review.remainingRuntimeEvidence.includes("revocation-and-rotation-propagation-smoke"), false);
assert.equal(review.remainingRuntimeEvidence.includes("tenant-session-registration-quota-smoke"), false);
assert.equal(review.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"), false);
assert.equal(review.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"), false);
assert.equal(review.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.equal(review.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);

for (const guardrail of [
  "metadata_surfaces_are_allowlisted",
  "route_metadata_excludes_payload_and_key_material",
  "support_metadata_excludes_raw_ciphertext",
  "billing_metadata_is_aggregate_only",
  "audit_metadata_excludes_payloads_and_secrets",
]) {
  assert.ok(review.guardrails.includes(guardrail), `metadata review missing guardrail: ${guardrail}`);
}

for (const prohibited of [
  "payload_json",
  "command_text",
  "context_json",
  "approval_response_payload",
  "payload_ciphertext_hex",
  "payload_key_hex",
  "shared_secret_hex",
  "private_key_material",
  "raw_session_token",
  "full_setup_json",
]) {
  assert.ok(
    review.prohibitedMetadataFields.includes(prohibited),
    `metadata review missing prohibited field: ${prohibited}`,
  );
}

const daemon = await generateCompanionKeyMaterial(webcrypto);
const companion = await generateCompanionKeyMaterial(webcrypto);
const sessionId = "managed-metadata-review-session";
const payloadKeyHex = await managedRelayDeriveSessionPayloadKeyHex(
  sessionId,
  companion.identity.noisePubkeyHex,
  daemon.keyMaterial,
  webcrypto,
);
const approvalRequest = {
  approval_id: [109, 101, 116, 97, 100, 97, 116, 97],
  nonce: Array.from({ length: 32 }, (_, i) => i + 1),
  command_masked: "metadata review must not leak command text",
  context_hash: "ctx-managed-metadata-review",
  expires_at: 1782804241456,
  device_epoch: 10,
};
const requestFrame = await managedRelayEncryptedFrameFromLiveMessage(
  sessionId,
  "daemon",
  1,
  1901,
  1901 + 30000,
  liveApprovalRequestMessage(approvalRequest),
  payloadKeyHex,
  { nonceHex: "bb".repeat(12), webCrypto: webcrypto },
);
const routeEnvelope = managedRelayEncryptedFrameRouteEnvelope(requestFrame);
assert.deepEqual(Object.keys(routeEnvelope), review.metadataSurfaces.routeEnvelope);
assert.equal("payload_ciphertext_hex" in routeEnvelope, false);
assert.equal("payload_nonce_hex" in routeEnvelope, false);
assert.equal("payload_key_hex" in routeEnvelope, false);
assert.equal("shared_secret_hex" in routeEnvelope, false);
assert.equal(routeEnvelope.payload_ciphertext_bytes > 16, true);

const controlPlaneMetadata = {
  tenant_id: "tenant_demo",
  session_id: sessionId,
  daemon_device_id_hash: "sha256:daemon-demo",
  companion_device_id_hash: "sha256:companion-demo",
  ticket_key_id: "ticket-key-2026-07",
  ticket_key_version: 3,
  session_state: "active",
  created_at_ms: 1901,
  expires_at_ms: 1901 + 30000,
};
const billingUsageMetadata = {
  tenant_id: "tenant_demo",
  billing_period: "2026-07",
  session_registration_count: 1,
  active_session_count: 1,
  relay_frame_count: 1,
  relay_byte_count: routeEnvelope.payload_ciphertext_bytes,
  invalid_ticket_count: 0,
  quota_denial_count: 0,
};
const supportViewMetadata = {
  tenant_id: "tenant_demo",
  session_id_hash: "sha256:session-demo",
  daemon_device_id_hash: "sha256:daemon-demo",
  companion_device_id_hash: "sha256:companion-demo",
  aggregate_error_class: "none",
  quota_state: "within-limit",
  key_id: "ticket-key-2026-07",
  key_version: 3,
  last_seen_at_ms: 1901,
};
const auditEventMetadata = {
  tenant_id: "tenant_demo",
  event_type: "frame-routed",
  session_id_hash: "sha256:session-demo",
  actor_role: "managed-relay-service",
  key_id: "ticket-key-2026-07",
  key_version: 3,
  occurred_at_ms: 1901,
  aggregate_error_class: "none",
};

const sensitiveValues = [
  approvalRequest.command_masked,
  approvalRequest.context_hash,
  payloadKeyHex,
  requestFrame.payload_ciphertext_hex,
  requestFrame.payload_nonce_hex,
  daemon.identity.noisePubkeyHex,
  companion.identity.noisePubkeyHex,
];

function assertAllowlistedMetadata(surfaceName, metadata, allowedFields) {
  const fields = Object.keys(metadata);
  assert.deepEqual(fields, allowedFields, `${surfaceName} metadata fields drifted`);
  for (const prohibited of review.prohibitedMetadataFields) {
    assert.equal(prohibited in metadata, false, `${surfaceName} exposes prohibited ${prohibited}`);
  }
  const json = JSON.stringify(metadata);
  for (const sensitive of sensitiveValues) {
    assert.equal(json.includes(sensitive), false, `${surfaceName} leaks sensitive value`);
  }
}

assertAllowlistedMetadata("control-plane", controlPlaneMetadata, review.metadataSurfaces.controlPlane);
assertAllowlistedMetadata("billing", billingUsageMetadata, review.metadataSurfaces.billingUsage);
assertAllowlistedMetadata("support", supportViewMetadata, review.metadataSurfaces.supportView);
assertAllowlistedMetadata("audit", auditEventMetadata, review.metadataSurfaces.auditEvent);

const encryptedFrameJson = managedRelayEncryptedFrameJson(requestFrame);
assert.equal(encryptedFrameJson.includes(approvalRequest.command_masked), false);
assert.equal(encryptedFrameJson.includes(approvalRequest.context_hash), false);
assert.equal(encryptedFrameJson.includes("payload_key_hex"), false);
assert.equal(encryptedFrameJson.includes("shared_secret_hex"), false);

const evidence = {
  status: "review-complete-runtime-deferred",
  generatedAt: new Date().toISOString(),
  objective: "Review managed relay route/control/billing/support/audit metadata allowlists",
  review: {
    deploymentMode: review.deploymentMode,
    readiness: review.readiness,
    selectedRuntime: review.selectedRuntime,
    implementationStatus: review.implementationStatus,
    metadataBoundary: review.metadataBoundary,
    implementationCanStart: review.implementationCanStart,
  },
  completedRuntimeEvidence: review.completedRuntimeEvidence,
  closedReadinessBlockers: review.closedReadinessBlockers,
  remainingRuntimeEvidence: review.remainingRuntimeEvidence,
  remainingRuntimeBlockers: review.remainingRuntimeBlockers,
  metadataSurfaces: review.metadataSurfaces,
  prohibitedMetadataFields: review.prohibitedMetadataFields,
  routeEnvelope,
  controlPlaneMetadata,
  billingUsageMetadata,
  supportViewMetadata,
  auditEventMetadata,
  minimizationEvidence: review.minimizationEvidence,
  guardrails: review.guardrails,
  productDefault: review.productDefault,
  nextLocalSlice: review.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_MANAGED_METADATA_MINIMIZATION_REVIEW_OK ${evidencePath}`);
