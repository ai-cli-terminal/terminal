import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  relayDeploymentShapeDecision,
  relayManagedClientKeyAgreementRuntimeSmoke,
  relayManagedMetadataMinimizationReview,
  relayManagedPayloadBlindFrameEncryptionSpike,
  relayManagedPublicVerifierKeyRegistryRuntimeSmoke,
  relayManagedRevocationAndRotationPropagationSmoke,
  relayManagedRuntimeReadinessGate,
  relayManagedTenantSessionRegistrationQuotaSmoke,
} from "../pwa/app.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const artifactRoot = path.join(repoRoot, "artifacts", "ra-pwa-relay-next-mode-planning");
const evidencePath =
  process.env.RA_PWA_RELAY_NEXT_MODE_PLANNING_PATH ||
  path.join(artifactRoot, "ra-pwa-relay-next-mode-planning.json");

const decision = relayDeploymentShapeDecision();
const runtimeReadinessGate = relayManagedRuntimeReadinessGate();
const payloadBlindFrameEncryptionSpike = relayManagedPayloadBlindFrameEncryptionSpike();
const clientKeyAgreementRuntimeSmoke = relayManagedClientKeyAgreementRuntimeSmoke();
const metadataMinimizationReview = relayManagedMetadataMinimizationReview();
const publicVerifierKeyRegistryRuntimeSmoke =
  relayManagedPublicVerifierKeyRegistryRuntimeSmoke();
const revocationAndRotationPropagationSmoke =
  relayManagedRevocationAndRotationPropagationSmoke();
const tenantSessionRegistrationQuotaSmoke =
  relayManagedTenantSessionRegistrationQuotaSmoke();
assert.equal(decision.selectedMode, "self-hosted");
assert.equal(decision.productDefault, "live-loopback");
assert.deepEqual(decision.deferredModes, ["private-network", "managed"]);
assert.ok(decision.guardrails.includes("product_default_remains_live_loopback"));
assert.ok(decision.guardrails.includes("relay_ui_requires_selected_self_hosted_mode"));
assert.equal(
  tenantSessionRegistrationQuotaSmoke.nextLocalSlice,
  "managed-relay-active-session-and-byte-quota-smoke",
);
assert.ok(runtimeReadinessGate.completedRuntimeEvidence.includes("tenant-session-registration-quota-smoke"));
assert.equal(
  runtimeReadinessGate.remainingRuntimeEvidence.includes("tenant-session-registration-quota-smoke"),
  false,
);
assert.ok(runtimeReadinessGate.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"));

const evidence = {
  status: "planned",
  generatedAt: new Date().toISOString(),
  objective: "Choose the next Relay/M2 mode-planning slice after managed tenant session registration quota smoke",
  currentReadyMode: "self-hosted",
  productDefault: decision.productDefault,
  selectedNextMode: "managed",
  deferredMode: "managed-runtime",
  rationale: [
    "Private-network relay and the managed relay operations/control-plane/abuse-retention/payload-confidentiality/verifier-key/billing-quota/runtime-readiness-gate/payload-blind-frame-encryption/client-key-agreement/metadata-minimization/public-verifier-registry/revocation-rotation/tenant-registration-quota slices are complete.",
    "Managed relay remains deferred because active session and byte quota, usage export, support review, and billing/abuse boundary evidence are still missing.",
    "The product default remains live-loopback while managed relay stays a deferred service path.",
  ],
  requiredNextEvidence: [
    "managed relay active session and byte quota smoke",
    "live-loopback remains product default",
    "managed relay remains deferred until remaining runtime evidence exists",
  ],
  runtimeReadinessGate: {
    gateStatus: runtimeReadinessGate.gateStatus,
    implementationCanStart: runtimeReadinessGate.implementationCanStart,
    completedRuntimeEvidence: runtimeReadinessGate.completedRuntimeEvidence,
    remainingRuntimeEvidence: runtimeReadinessGate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: runtimeReadinessGate.remainingRuntimeBlockers,
  },
  payloadBlindFrameEncryptionSpike: {
    completedRuntimeEvidence: payloadBlindFrameEncryptionSpike.completedRuntimeEvidence,
    closedReadinessBlockers: payloadBlindFrameEncryptionSpike.closedReadinessBlockers,
    implementationCanStart: payloadBlindFrameEncryptionSpike.implementationCanStart,
  },
  clientKeyAgreementRuntimeSmoke: {
    completedRuntimeEvidence: clientKeyAgreementRuntimeSmoke.completedRuntimeEvidence,
    closedReadinessBlockers: clientKeyAgreementRuntimeSmoke.closedReadinessBlockers,
    implementationCanStart: clientKeyAgreementRuntimeSmoke.implementationCanStart,
  },
  metadataMinimizationReview: {
    completedRuntimeEvidence: metadataMinimizationReview.completedRuntimeEvidence,
    closedReadinessBlockers: metadataMinimizationReview.closedReadinessBlockers,
    implementationCanStart: metadataMinimizationReview.implementationCanStart,
  },
  publicVerifierKeyRegistryRuntimeSmoke: {
    completedRuntimeEvidence: publicVerifierKeyRegistryRuntimeSmoke.completedRuntimeEvidence,
    closedReadinessBlockers: publicVerifierKeyRegistryRuntimeSmoke.closedReadinessBlockers,
    implementationCanStart: publicVerifierKeyRegistryRuntimeSmoke.implementationCanStart,
  },
  revocationAndRotationPropagationSmoke: {
    completedRuntimeEvidence: revocationAndRotationPropagationSmoke.completedRuntimeEvidence,
    closedReadinessBlockers: revocationAndRotationPropagationSmoke.closedReadinessBlockers,
    implementationCanStart: revocationAndRotationPropagationSmoke.implementationCanStart,
  },
  tenantSessionRegistrationQuotaSmoke: {
    completedRuntimeEvidence: tenantSessionRegistrationQuotaSmoke.completedRuntimeEvidence,
    closedReadinessBlockers: tenantSessionRegistrationQuotaSmoke.closedReadinessBlockers,
    implementationCanStart: tenantSessionRegistrationQuotaSmoke.implementationCanStart,
  },
  nextLocalSlice: tenantSessionRegistrationQuotaSmoke.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_NEXT_MODE_PLANNING_OK ${evidencePath}`);
