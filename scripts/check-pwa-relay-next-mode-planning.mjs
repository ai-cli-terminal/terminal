import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  relayDeploymentShapeDecision,
  relayManagedActiveSessionAndByteQuotaSmoke,
  relayManagedBillingAbuseBoundaryReview,
  relayManagedClientKeyAgreementRuntimeSmoke,
  relayManagedMetadataMinimizationReview,
  relayManagedPayloadBlindFrameEncryptionSpike,
  relayManagedPublicVerifierKeyRegistryRuntimeSmoke,
  relayManagedRevocationAndRotationPropagationSmoke,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimeReadinessGate,
  relayManagedSupportRedactionAndAccessReviewEvidence,
  relayManagedTenantAggregateUsageExportSmoke,
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
const activeSessionAndByteQuotaSmoke =
  relayManagedActiveSessionAndByteQuotaSmoke();
const tenantAggregateUsageExportSmoke =
  relayManagedTenantAggregateUsageExportSmoke();
const supportRedactionAndAccessReviewEvidence =
  relayManagedSupportRedactionAndAccessReviewEvidence();
const billingAbuseBoundaryReview = relayManagedBillingAbuseBoundaryReview();
const runtimeImplementationPlan = relayManagedRuntimeImplementationPlan();
assert.equal(decision.selectedMode, "self-hosted");
assert.equal(decision.productDefault, "live-loopback");
assert.deepEqual(decision.deferredModes, ["private-network", "managed"]);
assert.ok(decision.guardrails.includes("product_default_remains_live_loopback"));
assert.ok(decision.guardrails.includes("relay_ui_requires_selected_self_hosted_mode"));
assert.equal(
  tenantAggregateUsageExportSmoke.nextLocalSlice,
  "managed-relay-runtime-service-scaffold",
);
assert.equal(
  supportRedactionAndAccessReviewEvidence.nextLocalSlice,
  "managed-relay-runtime-service-scaffold",
);
assert.equal(
  billingAbuseBoundaryReview.nextLocalSlice,
  "managed-relay-runtime-service-scaffold",
);
assert.equal(runtimeImplementationPlan.nextLocalSlice, "managed-relay-runtime-service-scaffold");
assert.equal(runtimeImplementationPlan.selectedRuntime, "deferred");
assert.equal(runtimeImplementationPlan.selectedRuntimeCanChange, false);
assert.equal(runtimeImplementationPlan.implementationCanStart, true);
assert.ok(
  runtimeImplementationPlan.completedPlanningEvidence.includes(
    "managed-runtime-implementation-plan",
  ),
);
assert.ok(runtimeReadinessGate.completedRuntimeEvidence.includes("tenant-session-registration-quota-smoke"));
assert.equal(
  runtimeReadinessGate.remainingRuntimeEvidence.includes("tenant-session-registration-quota-smoke"),
  false,
);
assert.ok(runtimeReadinessGate.completedRuntimeEvidence.includes("active-session-and-byte-quota-smoke"));
assert.equal(
  runtimeReadinessGate.remainingRuntimeEvidence.includes("active-session-and-byte-quota-smoke"),
  false,
);
assert.ok(runtimeReadinessGate.completedRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"));
assert.equal(
  runtimeReadinessGate.remainingRuntimeEvidence.includes("tenant-aggregate-usage-export-smoke"),
  false,
);
assert.ok(runtimeReadinessGate.completedRuntimeEvidence.includes("support-redaction-and-access-review-evidence"));
assert.equal(runtimeReadinessGate.remainingRuntimeEvidence.includes("support-redaction-and-access-review-evidence"), false);
assert.ok(runtimeReadinessGate.completedRuntimeEvidence.includes("billing-abuse-boundary-review"));
assert.equal(runtimeReadinessGate.remainingRuntimeEvidence.includes("billing-abuse-boundary-review"), false);
assert.deepEqual(runtimeReadinessGate.remainingRuntimeEvidence, []);
assert.deepEqual(runtimeReadinessGate.remainingRuntimeBlockers, []);
assert.equal(runtimeReadinessGate.implementationCanStart, true);

const evidence = {
  status: "planned",
  generatedAt: new Date().toISOString(),
  objective: "Choose the next Relay/M2 mode-planning slice after managed runtime implementation planning",
  currentReadyMode: "self-hosted",
  productDefault: decision.productDefault,
  selectedNextMode: "managed",
  deferredMode: "managed-runtime",
  rationale: [
    "Private-network relay and all managed relay readiness evidence slices through billing/abuse boundary review are complete.",
    "The managed runtime readiness gate is green and the implementation plan now defines service boundary, phases, exposure gates, and regressions.",
    "The product default remains live-loopback and selectedRuntime remains deferred until a later exposure gate explicitly changes it.",
  ],
  requiredNextEvidence: [
    "managed relay runtime service scaffold",
    "live-loopback remains product default",
    "managed relay remains deferred until scaffold and exposure gates explicitly change exposure",
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
  activeSessionAndByteQuotaSmoke: {
    completedRuntimeEvidence: activeSessionAndByteQuotaSmoke.completedRuntimeEvidence,
    closedReadinessBlockers: activeSessionAndByteQuotaSmoke.closedReadinessBlockers,
    implementationCanStart: activeSessionAndByteQuotaSmoke.implementationCanStart,
  },
  tenantAggregateUsageExportSmoke: {
    completedRuntimeEvidence: tenantAggregateUsageExportSmoke.completedRuntimeEvidence,
    closedReadinessBlockers: tenantAggregateUsageExportSmoke.closedReadinessBlockers,
    implementationCanStart: tenantAggregateUsageExportSmoke.implementationCanStart,
  },
  supportRedactionAndAccessReviewEvidence: {
    completedRuntimeEvidence: supportRedactionAndAccessReviewEvidence.completedRuntimeEvidence,
    closedReadinessBlockers: supportRedactionAndAccessReviewEvidence.closedReadinessBlockers,
    implementationCanStart: supportRedactionAndAccessReviewEvidence.implementationCanStart,
  },
  billingAbuseBoundaryReview: {
    completedRuntimeEvidence: billingAbuseBoundaryReview.completedRuntimeEvidence,
    closedReadinessBlockers: billingAbuseBoundaryReview.closedReadinessBlockers,
    implementationCanStart: billingAbuseBoundaryReview.implementationCanStart,
  },
  runtimeImplementationPlan: {
    readiness: runtimeImplementationPlan.readiness,
    implementationStatus: runtimeImplementationPlan.implementationStatus,
    implementationBoundary: runtimeImplementationPlan.implementationBoundary,
    pwaExposureDecision: runtimeImplementationPlan.pwaExposureDecision,
    selectedRuntimeCanChange: runtimeImplementationPlan.selectedRuntimeCanChange,
    implementationCanStart: runtimeImplementationPlan.implementationCanStart,
    implementationPhases: runtimeImplementationPlan.implementationPhases.map(
      ({ phase }) => phase,
    ),
    exposureGates: runtimeImplementationPlan.exposureGates,
    regressionChecks: runtimeImplementationPlan.regressionChecks,
  },
  nextLocalSlice: runtimeImplementationPlan.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_NEXT_MODE_PLANNING_OK ${evidencePath}`);
