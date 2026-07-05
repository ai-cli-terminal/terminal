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
  relayManagedRuntimeControlPlaneContractWiring,
  relayManagedRuntimeBrowserOperatorEvidence,
  relayManagedRuntimeEncryptedFrameRouting,
  relayManagedRuntimeImplementationPlan,
  relayManagedRuntimeOperatorSetupBrowserEvidence,
  relayManagedRuntimeOperatorSetupConnectionControls,
  relayManagedRuntimeOperatorSetupApprovalResponseDeliveryBoundary,
  relayManagedRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence,
  relayManagedRuntimeOperatorSetupApprovalFlowEvidence,
  relayManagedRuntimeOperatorSetupImportPreflight,
  relayManagedRuntimeOperatorSetupRunbookCloseout,
  relayManagedRuntimeOperatorSetupSessionHandshake,
  relayManagedRuntimeOperatorSetupContract,
  relayManagedRuntimePwaExposureGate,
  relayManagedRuntimeQuotaAndMeteringIntegration,
  relayManagedRuntimeReadinessGate,
  relayManagedRuntimeServiceScaffold,
  relayManagedRuntimeSupportAndAbuseOperationsIntegration,
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
const runtimeServiceScaffold = relayManagedRuntimeServiceScaffold();
const runtimeControlPlaneWiring = relayManagedRuntimeControlPlaneContractWiring();
const runtimeEncryptedFrameRouting = relayManagedRuntimeEncryptedFrameRouting();
const runtimeQuotaAndMeteringIntegration =
  relayManagedRuntimeQuotaAndMeteringIntegration();
const runtimeSupportAndAbuseOperationsIntegration =
  relayManagedRuntimeSupportAndAbuseOperationsIntegration();
const runtimePwaExposureGate = relayManagedRuntimePwaExposureGate();
const runtimeBrowserOperatorEvidence =
  relayManagedRuntimeBrowserOperatorEvidence();
const runtimeOperatorSetupContract =
  relayManagedRuntimeOperatorSetupContract();
const runtimeOperatorSetupImportPreflight =
  relayManagedRuntimeOperatorSetupImportPreflight();
const runtimeOperatorSetupBrowserEvidence =
  relayManagedRuntimeOperatorSetupBrowserEvidence();
const runtimeOperatorSetupConnectionControls =
  relayManagedRuntimeOperatorSetupConnectionControls();
const runtimeOperatorSetupSessionHandshake =
  relayManagedRuntimeOperatorSetupSessionHandshake();
const runtimeOperatorSetupApprovalFlowEvidence =
  relayManagedRuntimeOperatorSetupApprovalFlowEvidence();
const runtimeOperatorSetupRunbookCloseout =
  relayManagedRuntimeOperatorSetupRunbookCloseout();
const runtimeOperatorSetupApprovalResponseDeliveryBoundary =
  relayManagedRuntimeOperatorSetupApprovalResponseDeliveryBoundary();
const runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence =
  relayManagedRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence();
assert.equal(decision.selectedMode, "self-hosted");
assert.equal(decision.productDefault, "live-loopback");
assert.deepEqual(decision.deferredModes, ["private-network", "managed"]);
assert.ok(decision.pwaVisibleModes.includes("managed"));
assert.ok(decision.explicitOptInModes.includes("managed"));
assert.ok(decision.guardrails.includes("product_default_remains_live_loopback"));
assert.ok(decision.guardrails.includes("relay_ui_requires_selected_self_hosted_mode"));
assert.equal(
  tenantAggregateUsageExportSmoke.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
assert.equal(
  supportRedactionAndAccessReviewEvidence.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
assert.equal(
  billingAbuseBoundaryReview.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
assert.equal(
  runtimeImplementationPlan.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
assert.equal(runtimeImplementationPlan.selectedRuntime, "deferred");
assert.equal(runtimeImplementationPlan.selectedRuntimeCanChange, false);
assert.equal(runtimeImplementationPlan.implementationCanStart, true);
assert.ok(
  runtimeImplementationPlan.completedPlanningEvidence.includes(
    "managed-runtime-implementation-plan",
  ),
);
assert.equal(
  runtimeServiceScaffold.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
assert.equal(runtimeServiceScaffold.selectedRuntime, "deferred");
assert.equal(runtimeServiceScaffold.selectedRuntimeCanChange, false);
assert.equal(runtimeServiceScaffold.implementationCanContinue, true);
assert.ok(
  runtimeServiceScaffold.completedImplementationEvidence.includes(
    "managed-runtime-service-scaffold",
  ),
);
assert.equal(
  runtimeControlPlaneWiring.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
assert.equal(runtimeControlPlaneWiring.selectedRuntime, "deferred");
assert.equal(runtimeControlPlaneWiring.selectedRuntimeCanChange, false);
assert.equal(runtimeControlPlaneWiring.implementationCanContinue, true);
assert.ok(
  runtimeControlPlaneWiring.completedImplementationEvidence.includes(
    "managed-runtime-control-plane-contract-wiring",
  ),
);
assert.equal(
  runtimeControlPlaneWiring.startupContract.routeFrameHandler,
  "disabled-until-encrypted-frame-routing",
);
assert.equal(
  runtimeControlPlaneWiring.startupContract.pwaExposure,
  "disabled",
);
assert.equal(
  runtimeEncryptedFrameRouting.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
assert.equal(runtimeEncryptedFrameRouting.selectedRuntime, "deferred");
assert.equal(runtimeEncryptedFrameRouting.selectedRuntimeCanChange, false);
assert.equal(runtimeEncryptedFrameRouting.implementationCanContinue, true);
assert.equal(runtimeEncryptedFrameRouting.routeRuntime, "encrypted-frame-routing-wired");
assert.ok(
  runtimeEncryptedFrameRouting.completedImplementationEvidence.includes(
    "managed-runtime-encrypted-frame-routing",
  ),
);
assert.equal(
  runtimeEncryptedFrameRouting.startupContract.routeFrameHandler,
  "encrypted-frame-routing-wired",
);
assert.equal(runtimeEncryptedFrameRouting.startupContract.pwaExposure, "disabled");
assert.equal(
  runtimeQuotaAndMeteringIntegration.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
assert.equal(runtimeQuotaAndMeteringIntegration.selectedRuntime, "deferred");
assert.equal(runtimeQuotaAndMeteringIntegration.selectedRuntimeCanChange, false);
assert.equal(runtimeQuotaAndMeteringIntegration.implementationCanContinue, true);
assert.equal(
  runtimeQuotaAndMeteringIntegration.quotaRuntime,
  "active-session-frame-byte-metering-wired",
);
assert.ok(
  runtimeQuotaAndMeteringIntegration.completedImplementationEvidence.includes(
    "managed-runtime-quota-and-metering-integration",
  ),
);
assert.equal(
  runtimeQuotaAndMeteringIntegration.startupContract.quotaMeteringHandler,
  "active-session-frame-byte-metering-wired",
);
assert.equal(runtimeQuotaAndMeteringIntegration.startupContract.pwaExposure, "disabled");
assert.equal(
  runtimeSupportAndAbuseOperationsIntegration.nextLocalSlice,
  "managed-relay-runtime-pwa-exposure-gate",
);
assert.equal(runtimeSupportAndAbuseOperationsIntegration.selectedRuntime, "deferred");
assert.equal(runtimeSupportAndAbuseOperationsIntegration.selectedRuntimeCanChange, false);
assert.equal(runtimeSupportAndAbuseOperationsIntegration.implementationCanContinue, true);
assert.equal(
  runtimeSupportAndAbuseOperationsIntegration.startupContract.pwaExposure,
  "disabled",
);
assert.ok(
  runtimeSupportAndAbuseOperationsIntegration.completedImplementationEvidence.includes(
    "managed-runtime-support-and-abuse-operations-integration",
  ),
);
assert.equal(
  runtimePwaExposureGate.nextLocalSlice,
  "managed-relay-runtime-browser-operator-evidence",
);
assert.equal(runtimePwaExposureGate.selectedRuntime, "explicit-opt-in-managed");
assert.equal(runtimePwaExposureGate.selectedRuntimeCanChange, true);
assert.equal(runtimePwaExposureGate.selectedRuntimeChangeBoundary, "explicit-opt-in-only");
assert.equal(runtimePwaExposureGate.productDefaultCanChange, false);
assert.equal(runtimePwaExposureGate.pwaExposure, "explicit-opt-in");
assert.equal(runtimePwaExposureGate.endpointMode, "operator-setup-required");
assert.equal(runtimePwaExposureGate.endpointAutoStart, false);
assert.equal(runtimePwaExposureGate.publicBind, false);
assert.deepEqual(runtimePwaExposureGate.remainingImplementationPhases, []);
assert.ok(
  runtimePwaExposureGate.completedImplementationEvidence.includes(
    "managed-runtime-pwa-exposure-gate",
  ),
);
assert.equal(
  runtimeBrowserOperatorEvidence.nextLocalSlice,
  "managed-relay-runtime-operator-setup-contract",
);
assert.equal(runtimeBrowserOperatorEvidence.selectedRuntime, "explicit-opt-in-managed");
assert.equal(runtimeBrowserOperatorEvidence.selectedRuntimeCanChange, true);
assert.equal(
  runtimeBrowserOperatorEvidence.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(runtimeBrowserOperatorEvidence.productDefaultCanChange, false);
assert.equal(runtimeBrowserOperatorEvidence.pwaExposure, "explicit-opt-in");
assert.equal(runtimeBrowserOperatorEvidence.endpointMode, "operator-setup-required");
assert.equal(runtimeBrowserOperatorEvidence.endpointAutoStart, false);
assert.equal(runtimeBrowserOperatorEvidence.publicBind, false);
assert.deepEqual(runtimeBrowserOperatorEvidence.remainingImplementationPhases, []);
assert.ok(
  runtimeBrowserOperatorEvidence.completedImplementationEvidence.includes(
    "managed-runtime-browser-operator-evidence",
  ),
);
assert.equal(
  runtimeOperatorSetupContract.nextLocalSlice,
  "managed-relay-runtime-operator-setup-import-preflight",
);
assert.equal(
  runtimeOperatorSetupContract.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(runtimeOperatorSetupContract.selectedRuntimeCanChange, true);
assert.equal(
  runtimeOperatorSetupContract.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(runtimeOperatorSetupContract.productDefaultCanChange, false);
assert.equal(runtimeOperatorSetupContract.pwaExposure, "explicit-opt-in");
assert.equal(
  runtimeOperatorSetupContract.endpointMode,
  "operator-setup-required",
);
assert.equal(runtimeOperatorSetupContract.endpointAutoStart, false);
assert.equal(runtimeOperatorSetupContract.publicBind, false);
assert.equal(runtimeOperatorSetupContract.manualConnectRequired, true);
assert.deepEqual(runtimeOperatorSetupContract.remainingImplementationPhases, []);
assert.ok(
  runtimeOperatorSetupContract.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-contract",
  ),
);
assert.equal(
  runtimeOperatorSetupImportPreflight.nextLocalSlice,
  "managed-relay-runtime-operator-setup-browser-evidence",
);
assert.equal(
  runtimeOperatorSetupImportPreflight.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(runtimeOperatorSetupImportPreflight.selectedRuntimeCanChange, true);
assert.equal(
  runtimeOperatorSetupImportPreflight.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(runtimeOperatorSetupImportPreflight.productDefaultCanChange, false);
assert.equal(runtimeOperatorSetupImportPreflight.pwaExposure, "explicit-opt-in");
assert.equal(
  runtimeOperatorSetupImportPreflight.endpointMode,
  "operator-setup-required",
);
assert.equal(runtimeOperatorSetupImportPreflight.endpointAutoStart, false);
assert.equal(runtimeOperatorSetupImportPreflight.publicBind, false);
assert.equal(runtimeOperatorSetupImportPreflight.manualConnectRequired, true);
assert.equal(
  runtimeOperatorSetupImportPreflight.setupRendering,
  "sanitized-summary-only",
);
assert.deepEqual(runtimeOperatorSetupImportPreflight.remainingImplementationPhases, []);
assert.ok(
  runtimeOperatorSetupImportPreflight.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-import-preflight",
  ),
);
assert.equal(
  runtimeOperatorSetupBrowserEvidence.nextLocalSlice,
  "managed-relay-runtime-operator-setup-connection-controls",
);
assert.equal(
  runtimeOperatorSetupBrowserEvidence.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(runtimeOperatorSetupBrowserEvidence.selectedRuntimeCanChange, true);
assert.equal(
  runtimeOperatorSetupBrowserEvidence.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(runtimeOperatorSetupBrowserEvidence.productDefaultCanChange, false);
assert.equal(runtimeOperatorSetupBrowserEvidence.pwaExposure, "explicit-opt-in");
assert.equal(
  runtimeOperatorSetupBrowserEvidence.endpointMode,
  "operator-setup-required",
);
assert.equal(runtimeOperatorSetupBrowserEvidence.endpointAutoStart, false);
assert.equal(runtimeOperatorSetupBrowserEvidence.publicBind, false);
assert.equal(runtimeOperatorSetupBrowserEvidence.manualConnectRequired, true);
assert.equal(
  runtimeOperatorSetupBrowserEvidence.setupRendering,
  "sanitized-summary-only",
);
assert.equal(
  runtimeOperatorSetupBrowserEvidence.browserEvidence.mobileOverflowAllowed,
  false,
);
assert.deepEqual(runtimeOperatorSetupBrowserEvidence.remainingImplementationPhases, []);
assert.ok(
  runtimeOperatorSetupBrowserEvidence.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-browser-evidence",
  ),
);
assert.equal(
  runtimeOperatorSetupConnectionControls.nextLocalSlice,
  "managed-relay-runtime-operator-setup-session-handshake",
);
assert.equal(
  runtimeOperatorSetupConnectionControls.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(runtimeOperatorSetupConnectionControls.selectedRuntimeCanChange, true);
assert.equal(
  runtimeOperatorSetupConnectionControls.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(runtimeOperatorSetupConnectionControls.productDefaultCanChange, false);
assert.equal(runtimeOperatorSetupConnectionControls.pwaExposure, "explicit-opt-in");
assert.equal(
  runtimeOperatorSetupConnectionControls.endpointMode,
  "operator-setup-required",
);
assert.equal(runtimeOperatorSetupConnectionControls.endpointAutoStart, false);
assert.equal(runtimeOperatorSetupConnectionControls.publicBind, false);
assert.equal(runtimeOperatorSetupConnectionControls.manualConnectRequired, true);
assert.equal(
  runtimeOperatorSetupConnectionControls.connectionControlMode,
  "manual-request-status-only",
);
assert.equal(
  runtimeOperatorSetupConnectionControls.networkConnectionStartedOnRequest,
  false,
);
assert.equal(
  runtimeOperatorSetupConnectionControls.connectionControls.requestCreatesWebSocket,
  false,
);
assert.equal(
  runtimeOperatorSetupConnectionControls.connectionControls.mobileOverflowAllowed,
  false,
);
assert.deepEqual(runtimeOperatorSetupConnectionControls.remainingImplementationPhases, []);
assert.ok(
  runtimeOperatorSetupConnectionControls.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-connection-controls",
  ),
);
assert.equal(
  runtimeOperatorSetupSessionHandshake.nextLocalSlice,
  "managed-relay-runtime-operator-setup-approval-flow-evidence",
);
assert.equal(
  runtimeOperatorSetupSessionHandshake.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(runtimeOperatorSetupSessionHandshake.selectedRuntimeCanChange, true);
assert.equal(
  runtimeOperatorSetupSessionHandshake.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(runtimeOperatorSetupSessionHandshake.productDefaultCanChange, false);
assert.equal(runtimeOperatorSetupSessionHandshake.pwaExposure, "explicit-opt-in");
assert.equal(
  runtimeOperatorSetupSessionHandshake.endpointMode,
  "operator-setup-required",
);
assert.equal(runtimeOperatorSetupSessionHandshake.endpointAutoStart, false);
assert.equal(runtimeOperatorSetupSessionHandshake.publicBind, false);
assert.equal(runtimeOperatorSetupSessionHandshake.manualConnectRequired, true);
assert.equal(
  runtimeOperatorSetupSessionHandshake.handshakeMode,
  "manual-request-capability-envelope",
);
assert.equal(
  runtimeOperatorSetupSessionHandshake.sessionCapabilityVisibility,
  "handle-and-transcript-hash-only",
);
assert.equal(
  runtimeOperatorSetupSessionHandshake.networkConnectionStartedOnHandshake,
  false,
);
assert.equal(runtimeOperatorSetupSessionHandshake.webSocketCreatedOnHandshake, false);
assert.equal(runtimeOperatorSetupSessionHandshake.signedTicketVisible, false);
assert.equal(runtimeOperatorSetupSessionHandshake.rawTokenVisible, false);
assert.equal(runtimeOperatorSetupSessionHandshake.payloadVisible, false);
assert.equal(runtimeOperatorSetupSessionHandshake.privateKeyMaterialVisible, false);
assert.deepEqual(runtimeOperatorSetupSessionHandshake.remainingImplementationPhases, []);
assert.ok(
  runtimeOperatorSetupSessionHandshake.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-session-handshake",
  ),
);
assert.equal(
  runtimeOperatorSetupApprovalFlowEvidence.nextLocalSlice,
  "managed-relay-runtime-operator-setup-runbook-closeout",
);
assert.equal(
  runtimeOperatorSetupApprovalFlowEvidence.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.selectedRuntimeCanChange, true);
assert.equal(
  runtimeOperatorSetupApprovalFlowEvidence.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.productDefaultCanChange, false);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.pwaExposure, "explicit-opt-in");
assert.equal(
  runtimeOperatorSetupApprovalFlowEvidence.endpointMode,
  "operator-setup-required",
);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.endpointAutoStart, false);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.publicBind, false);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.manualConnectRequired, true);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.sessionHandshakeRequired, true);
assert.equal(
  runtimeOperatorSetupApprovalFlowEvidence.approvalFlowMode,
  "manual-approval-request-via-session-capability",
);
assert.equal(
  runtimeOperatorSetupApprovalFlowEvidence.approvalResponseDelivery,
  "manual-signed-response-copy-only",
);
assert.equal(
  runtimeOperatorSetupApprovalFlowEvidence.networkConnectionStartedOnApproval,
  false,
);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.webSocketCreatedOnApproval, false);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.signedTicketVisible, false);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.rawTokenVisible, false);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.payloadVisible, false);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.privateKeyMaterialVisible, false);
assert.equal(runtimeOperatorSetupApprovalFlowEvidence.capabilityEnvelopeVisible, false);
assert.deepEqual(runtimeOperatorSetupApprovalFlowEvidence.remainingImplementationPhases, []);
assert.ok(
  runtimeOperatorSetupApprovalFlowEvidence.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-approval-flow-evidence",
  ),
);
assert.equal(
  runtimeOperatorSetupRunbookCloseout.nextLocalSlice,
  "managed-relay-runtime-operator-setup-approval-response-delivery-boundary",
);
assert.equal(
  runtimeOperatorSetupRunbookCloseout.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(runtimeOperatorSetupRunbookCloseout.selectedRuntimeCanChange, true);
assert.equal(
  runtimeOperatorSetupRunbookCloseout.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(runtimeOperatorSetupRunbookCloseout.productDefaultCanChange, false);
assert.equal(runtimeOperatorSetupRunbookCloseout.pwaExposure, "explicit-opt-in");
assert.equal(
  runtimeOperatorSetupRunbookCloseout.endpointMode,
  "operator-setup-required",
);
assert.equal(runtimeOperatorSetupRunbookCloseout.endpointAutoStart, false);
assert.equal(runtimeOperatorSetupRunbookCloseout.publicBind, false);
assert.equal(runtimeOperatorSetupRunbookCloseout.manualConnectRequired, true);
assert.equal(runtimeOperatorSetupRunbookCloseout.sessionHandshakeRequired, true);
assert.equal(
  runtimeOperatorSetupRunbookCloseout.approvalResponseDelivery,
  "manual-signed-response-copy-only",
);
assert.equal(
  runtimeOperatorSetupRunbookCloseout.networkConnectionStartedOnCloseout,
  false,
);
assert.equal(runtimeOperatorSetupRunbookCloseout.webSocketCreatedOnCloseout, false);
assert.equal(runtimeOperatorSetupRunbookCloseout.signedTicketVisible, false);
assert.equal(runtimeOperatorSetupRunbookCloseout.rawTokenVisible, false);
assert.equal(runtimeOperatorSetupRunbookCloseout.payloadVisible, false);
assert.equal(runtimeOperatorSetupRunbookCloseout.privateKeyMaterialVisible, false);
assert.equal(runtimeOperatorSetupRunbookCloseout.capabilityEnvelopeVisible, false);
assert.deepEqual(runtimeOperatorSetupRunbookCloseout.remainingImplementationPhases, []);
assert.ok(
  runtimeOperatorSetupRunbookCloseout.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-runbook-closeout",
  ),
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.nextLocalSlice,
  "managed-relay-runtime-operator-setup-approval-response-endpoint-delivery-evidence",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.selectedRuntimeCanChange,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.productDefaultCanChange,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.pwaExposure,
  "explicit-opt-in",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.endpointMode,
  "operator-setup-required",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.endpointAutoStart,
  false,
);
assert.equal(runtimeOperatorSetupApprovalResponseDeliveryBoundary.publicBind, false);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.manualConnectRequired,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.sessionHandshakeRequired,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.deliveryMode,
  "manual-signed-response-copy-only",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.networkDeliveryStatus,
  "blocked-until-managed-endpoint-delivery-evidence",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.approvalResponseVisibleInApprovalPanel,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.approvalResponseVisibleInManagedSetupSurface,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.verifyCommandVisible,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.copyResponseControlVisible,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.networkConnectionStartedOnDelivery,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.webSocketCreatedOnDelivery,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.endpointStartedOnDelivery,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.publicBindEnabledOnDelivery,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.signedTicketVisible,
  false,
);
assert.equal(runtimeOperatorSetupApprovalResponseDeliveryBoundary.rawTokenVisible, false);
assert.equal(runtimeOperatorSetupApprovalResponseDeliveryBoundary.payloadVisible, false);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.privateKeyMaterialVisible,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.capabilityEnvelopeVisible,
  false,
);
assert.deepEqual(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.remainingImplementationPhases,
  [],
);
assert.ok(
  runtimeOperatorSetupApprovalResponseDeliveryBoundary.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-approval-response-delivery-boundary",
  ),
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.nextLocalSlice,
  "managed-relay-runtime-operator-setup-approval-response-endpoint-browser-evidence",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.selectedRuntime,
  "explicit-opt-in-managed",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.selectedRuntimeCanChange,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.selectedRuntimeChangeBoundary,
  "explicit-opt-in-only",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.productDefaultCanChange,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.pwaExposure,
  "explicit-opt-in",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointMode,
  "operator-setup-required",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointAutoStart,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointStartedByAutoStart,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.publicBind,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.publicBindEnabledOnDelivery,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.manualConnectRequired,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.sessionHandshakeRequired,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.deliveryMode,
  "explicit-managed-endpoint-encrypted-frame-with-manual-copy-fallback",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.approvalResponseDelivery,
  "explicit-managed-endpoint-encrypted-frame",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.manualCopyFallback,
  "manual-signed-response-copy-available",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.networkDeliveryStatus,
  "verified-explicit-managed-endpoint-delivery",
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.networkConnectionStartedOnDelivery,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.webSocketCreatedOnDelivery,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointStartedOnDelivery,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointStartedByOperator,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.encryptedFrameDelivery,
  true,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.plaintextPayloadVisibleToRelay,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.approvalResponsePayloadVisibleToRelay,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.payloadKeyVisibleToRelay,
  false,
);
assert.equal(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.payloadCiphertextVisibleToOperator,
  false,
);
assert.deepEqual(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.remainingImplementationPhases,
  [],
);
assert.ok(
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.completedImplementationEvidence.includes(
    "managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence",
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
  objective: "Choose the next Relay/M2 mode-planning slice after managed runtime operator setup approval response endpoint delivery evidence",
  currentReadyMode: "self-hosted",
  productDefault: decision.productDefault,
  selectedNextMode: "managed",
  deferredMode: "managed-runtime",
  rationale: [
    "Private-network relay and all managed relay readiness evidence slices through support/abuse operations integration are complete.",
    "The managed runtime readiness gate is green, the implementation plan, service scaffold, control-plane contract, encrypted frame routing, quota/metering integration, support/abuse operations, PWA exposure gate, browser/operator evidence, operator setup contract, import preflight, browser evidence, connection controls, session handshake, approval flow evidence, runbook closeout, approval response delivery boundary, and endpoint delivery evidence are complete.",
    "The product default remains live-loopback; managed relay is browser-verified only as an explicit opt-in setup path, operator setup import renders a sanitized metadata summary only, connection controls are manual request/cancel status controls, session handshake displays only a capability handle plus transcript hash, approval flow evidence uses the existing approval panel, and endpoint delivery evidence uses an explicit operator-started encrypted frame path with manual signed-response copy as fallback.",
  ],
  requiredNextEvidence: [
    "managed relay runtime operator setup approval response endpoint browser evidence",
    "live-loopback remains product default",
    "managed relay remains explicit opt-in with public bind and endpoint auto-start disabled",
    "browser evidence must show endpoint delivery controls without exposing capability envelope JSON, raw tokens, payload material, private key material, payload keys, or ciphertext",
    "browser evidence must preserve manual signed-response copy as fallback while proving the explicit endpoint path",
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
  runtimeServiceScaffold: {
    readiness: runtimeServiceScaffold.readiness,
    implementationStatus: runtimeServiceScaffold.implementationStatus,
    serviceState: runtimeServiceScaffold.serviceState,
    pwaExposureDecision: runtimeServiceScaffold.pwaExposureDecision,
    selectedRuntimeCanChange: runtimeServiceScaffold.selectedRuntimeCanChange,
    implementationCanContinue: runtimeServiceScaffold.implementationCanContinue,
    startupContract: runtimeServiceScaffold.startupContract,
    remainingImplementationPhases: runtimeServiceScaffold.remainingImplementationPhases,
    evidenceChecks: runtimeServiceScaffold.evidenceChecks,
  },
  runtimeControlPlaneWiring: {
    readiness: runtimeControlPlaneWiring.readiness,
    implementationStatus: runtimeControlPlaneWiring.implementationStatus,
    controlPlaneRuntime: runtimeControlPlaneWiring.controlPlaneRuntime,
    routeRuntime: runtimeControlPlaneWiring.routeRuntime,
    pwaExposureDecision: runtimeControlPlaneWiring.pwaExposureDecision,
    selectedRuntimeCanChange: runtimeControlPlaneWiring.selectedRuntimeCanChange,
    implementationCanContinue: runtimeControlPlaneWiring.implementationCanContinue,
    startupContract: runtimeControlPlaneWiring.startupContract,
    remainingImplementationPhases: runtimeControlPlaneWiring.remainingImplementationPhases,
    evidenceChecks: runtimeControlPlaneWiring.evidenceChecks,
  },
  runtimeEncryptedFrameRouting: {
    readiness: runtimeEncryptedFrameRouting.readiness,
    implementationStatus: runtimeEncryptedFrameRouting.implementationStatus,
    controlPlaneRuntime: runtimeEncryptedFrameRouting.controlPlaneRuntime,
    routeRuntime: runtimeEncryptedFrameRouting.routeRuntime,
    pwaExposureDecision: runtimeEncryptedFrameRouting.pwaExposureDecision,
    selectedRuntimeCanChange: runtimeEncryptedFrameRouting.selectedRuntimeCanChange,
    implementationCanContinue: runtimeEncryptedFrameRouting.implementationCanContinue,
    startupContract: runtimeEncryptedFrameRouting.startupContract,
    remainingImplementationPhases: runtimeEncryptedFrameRouting.remainingImplementationPhases,
    evidenceChecks: runtimeEncryptedFrameRouting.evidenceChecks,
  },
  runtimeQuotaAndMeteringIntegration: {
    readiness: runtimeQuotaAndMeteringIntegration.readiness,
    implementationStatus: runtimeQuotaAndMeteringIntegration.implementationStatus,
    controlPlaneRuntime: runtimeQuotaAndMeteringIntegration.controlPlaneRuntime,
    routeRuntime: runtimeQuotaAndMeteringIntegration.routeRuntime,
    quotaRuntime: runtimeQuotaAndMeteringIntegration.quotaRuntime,
    pwaExposureDecision: runtimeQuotaAndMeteringIntegration.pwaExposureDecision,
    selectedRuntimeCanChange: runtimeQuotaAndMeteringIntegration.selectedRuntimeCanChange,
    implementationCanContinue:
      runtimeQuotaAndMeteringIntegration.implementationCanContinue,
    startupContract: runtimeQuotaAndMeteringIntegration.startupContract,
    quotaMeteringContract:
      runtimeQuotaAndMeteringIntegration.quotaMeteringContract,
    remainingImplementationPhases:
      runtimeQuotaAndMeteringIntegration.remainingImplementationPhases,
    evidenceChecks: runtimeQuotaAndMeteringIntegration.evidenceChecks,
  },
  runtimeSupportAndAbuseOperationsIntegration: {
    readiness: runtimeSupportAndAbuseOperationsIntegration.readiness,
    implementationStatus:
      runtimeSupportAndAbuseOperationsIntegration.implementationStatus,
    supportRuntime: runtimeSupportAndAbuseOperationsIntegration.supportRuntime,
    abuseRuntime: runtimeSupportAndAbuseOperationsIntegration.abuseRuntime,
    pwaExposureDecision:
      runtimeSupportAndAbuseOperationsIntegration.pwaExposureDecision,
    selectedRuntimeCanChange:
      runtimeSupportAndAbuseOperationsIntegration.selectedRuntimeCanChange,
    implementationCanContinue:
      runtimeSupportAndAbuseOperationsIntegration.implementationCanContinue,
    startupContract: runtimeSupportAndAbuseOperationsIntegration.startupContract,
    remainingImplementationPhases:
      runtimeSupportAndAbuseOperationsIntegration.remainingImplementationPhases,
    evidenceChecks: runtimeSupportAndAbuseOperationsIntegration.evidenceChecks,
  },
  runtimePwaExposureGate: {
    readiness: runtimePwaExposureGate.readiness,
    implementationStatus: runtimePwaExposureGate.implementationStatus,
    selectedRuntime: runtimePwaExposureGate.selectedRuntime,
    pwaExposureDecision: runtimePwaExposureGate.pwaExposureDecision,
    pwaExposure: runtimePwaExposureGate.pwaExposure,
    endpointMode: runtimePwaExposureGate.endpointMode,
    endpointAutoStart: runtimePwaExposureGate.endpointAutoStart,
    publicBind: runtimePwaExposureGate.publicBind,
    selectedRuntimeCanChange: runtimePwaExposureGate.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimePwaExposureGate.selectedRuntimeChangeBoundary,
    productDefaultCanChange: runtimePwaExposureGate.productDefaultCanChange,
    startupContract: runtimePwaExposureGate.startupContract,
    pwaSurface: runtimePwaExposureGate.pwaSurface,
    remainingImplementationPhases:
      runtimePwaExposureGate.remainingImplementationPhases,
    evidenceChecks: runtimePwaExposureGate.evidenceChecks,
  },
  runtimeBrowserOperatorEvidence: {
    readiness: runtimeBrowserOperatorEvidence.readiness,
    implementationStatus: runtimeBrowserOperatorEvidence.implementationStatus,
    selectedRuntime: runtimeBrowserOperatorEvidence.selectedRuntime,
    pwaExposureDecision: runtimeBrowserOperatorEvidence.pwaExposureDecision,
    pwaExposure: runtimeBrowserOperatorEvidence.pwaExposure,
    endpointMode: runtimeBrowserOperatorEvidence.endpointMode,
    endpointAutoStart: runtimeBrowserOperatorEvidence.endpointAutoStart,
    publicBind: runtimeBrowserOperatorEvidence.publicBind,
    selectedRuntimeCanChange:
      runtimeBrowserOperatorEvidence.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeBrowserOperatorEvidence.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeBrowserOperatorEvidence.productDefaultCanChange,
    browserEvidence: runtimeBrowserOperatorEvidence.browserEvidence,
    operatorEvidence: runtimeBrowserOperatorEvidence.operatorEvidence,
    remainingImplementationPhases:
      runtimeBrowserOperatorEvidence.remainingImplementationPhases,
    evidenceChecks: runtimeBrowserOperatorEvidence.evidenceChecks,
  },
  runtimeOperatorSetupContract: {
    readiness: runtimeOperatorSetupContract.readiness,
    implementationStatus: runtimeOperatorSetupContract.implementationStatus,
    selectedRuntime: runtimeOperatorSetupContract.selectedRuntime,
    pwaExposureDecision: runtimeOperatorSetupContract.pwaExposureDecision,
    pwaExposure: runtimeOperatorSetupContract.pwaExposure,
    endpointMode: runtimeOperatorSetupContract.endpointMode,
    endpointAutoStart: runtimeOperatorSetupContract.endpointAutoStart,
    publicBind: runtimeOperatorSetupContract.publicBind,
    manualConnectRequired: runtimeOperatorSetupContract.manualConnectRequired,
    endpointActivation: runtimeOperatorSetupContract.endpointActivation,
    selectedRuntimeCanChange:
      runtimeOperatorSetupContract.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeOperatorSetupContract.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeOperatorSetupContract.productDefaultCanChange,
    setupPayloadContract: runtimeOperatorSetupContract.setupPayloadContract,
    endpointContract: runtimeOperatorSetupContract.endpointContract,
    activationContract: runtimeOperatorSetupContract.activationContract,
    remainingImplementationPhases:
      runtimeOperatorSetupContract.remainingImplementationPhases,
    evidenceChecks: runtimeOperatorSetupContract.evidenceChecks,
  },
  runtimeOperatorSetupImportPreflight: {
    readiness: runtimeOperatorSetupImportPreflight.readiness,
    implementationStatus:
      runtimeOperatorSetupImportPreflight.implementationStatus,
    selectedRuntime: runtimeOperatorSetupImportPreflight.selectedRuntime,
    pwaExposureDecision:
      runtimeOperatorSetupImportPreflight.pwaExposureDecision,
    pwaExposure: runtimeOperatorSetupImportPreflight.pwaExposure,
    endpointMode: runtimeOperatorSetupImportPreflight.endpointMode,
    endpointAutoStart: runtimeOperatorSetupImportPreflight.endpointAutoStart,
    publicBind: runtimeOperatorSetupImportPreflight.publicBind,
    manualConnectRequired:
      runtimeOperatorSetupImportPreflight.manualConnectRequired,
    setupRendering: runtimeOperatorSetupImportPreflight.setupRendering,
    endpointActivation:
      runtimeOperatorSetupImportPreflight.endpointActivation,
    selectedRuntimeCanChange:
      runtimeOperatorSetupImportPreflight.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeOperatorSetupImportPreflight.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeOperatorSetupImportPreflight.productDefaultCanChange,
    importPreflight: runtimeOperatorSetupImportPreflight.importPreflight,
    remainingImplementationPhases:
      runtimeOperatorSetupImportPreflight.remainingImplementationPhases,
    evidenceChecks: runtimeOperatorSetupImportPreflight.evidenceChecks,
  },
  runtimeOperatorSetupBrowserEvidence: {
    readiness: runtimeOperatorSetupBrowserEvidence.readiness,
    implementationStatus:
      runtimeOperatorSetupBrowserEvidence.implementationStatus,
    selectedRuntime: runtimeOperatorSetupBrowserEvidence.selectedRuntime,
    pwaExposureDecision:
      runtimeOperatorSetupBrowserEvidence.pwaExposureDecision,
    pwaExposure: runtimeOperatorSetupBrowserEvidence.pwaExposure,
    endpointMode: runtimeOperatorSetupBrowserEvidence.endpointMode,
    endpointAutoStart: runtimeOperatorSetupBrowserEvidence.endpointAutoStart,
    publicBind: runtimeOperatorSetupBrowserEvidence.publicBind,
    manualConnectRequired:
      runtimeOperatorSetupBrowserEvidence.manualConnectRequired,
    setupRendering: runtimeOperatorSetupBrowserEvidence.setupRendering,
    endpointActivation:
      runtimeOperatorSetupBrowserEvidence.endpointActivation,
    selectedRuntimeCanChange:
      runtimeOperatorSetupBrowserEvidence.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeOperatorSetupBrowserEvidence.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeOperatorSetupBrowserEvidence.productDefaultCanChange,
    importPreflight: runtimeOperatorSetupBrowserEvidence.importPreflight,
    browserEvidence: runtimeOperatorSetupBrowserEvidence.browserEvidence,
    operatorEvidence: runtimeOperatorSetupBrowserEvidence.operatorEvidence,
    remainingImplementationPhases:
      runtimeOperatorSetupBrowserEvidence.remainingImplementationPhases,
    evidenceChecks: runtimeOperatorSetupBrowserEvidence.evidenceChecks,
  },
  runtimeOperatorSetupConnectionControls: {
    readiness: runtimeOperatorSetupConnectionControls.readiness,
    implementationStatus:
      runtimeOperatorSetupConnectionControls.implementationStatus,
    selectedRuntime: runtimeOperatorSetupConnectionControls.selectedRuntime,
    pwaExposureDecision:
      runtimeOperatorSetupConnectionControls.pwaExposureDecision,
    pwaExposure: runtimeOperatorSetupConnectionControls.pwaExposure,
    endpointMode: runtimeOperatorSetupConnectionControls.endpointMode,
    endpointAutoStart: runtimeOperatorSetupConnectionControls.endpointAutoStart,
    publicBind: runtimeOperatorSetupConnectionControls.publicBind,
    manualConnectRequired:
      runtimeOperatorSetupConnectionControls.manualConnectRequired,
    setupRendering: runtimeOperatorSetupConnectionControls.setupRendering,
    endpointActivation:
      runtimeOperatorSetupConnectionControls.endpointActivation,
    connectionControlMode:
      runtimeOperatorSetupConnectionControls.connectionControlMode,
    networkConnectionStartedOnRequest:
      runtimeOperatorSetupConnectionControls.networkConnectionStartedOnRequest,
    selectedRuntimeCanChange:
      runtimeOperatorSetupConnectionControls.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeOperatorSetupConnectionControls.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeOperatorSetupConnectionControls.productDefaultCanChange,
    browserEvidence: runtimeOperatorSetupConnectionControls.browserEvidence,
    connectionControls:
      runtimeOperatorSetupConnectionControls.connectionControls,
    operatorEvidence: runtimeOperatorSetupConnectionControls.operatorEvidence,
    remainingImplementationPhases:
      runtimeOperatorSetupConnectionControls.remainingImplementationPhases,
    evidenceChecks: runtimeOperatorSetupConnectionControls.evidenceChecks,
  },
  runtimeOperatorSetupSessionHandshake: {
    readiness: runtimeOperatorSetupSessionHandshake.readiness,
    implementationStatus:
      runtimeOperatorSetupSessionHandshake.implementationStatus,
    selectedRuntime: runtimeOperatorSetupSessionHandshake.selectedRuntime,
    pwaExposureDecision:
      runtimeOperatorSetupSessionHandshake.pwaExposureDecision,
    pwaExposure: runtimeOperatorSetupSessionHandshake.pwaExposure,
    endpointMode: runtimeOperatorSetupSessionHandshake.endpointMode,
    endpointAutoStart: runtimeOperatorSetupSessionHandshake.endpointAutoStart,
    publicBind: runtimeOperatorSetupSessionHandshake.publicBind,
    manualConnectRequired:
      runtimeOperatorSetupSessionHandshake.manualConnectRequired,
    setupRendering: runtimeOperatorSetupSessionHandshake.setupRendering,
    endpointActivation:
      runtimeOperatorSetupSessionHandshake.endpointActivation,
    handshakeMode: runtimeOperatorSetupSessionHandshake.handshakeMode,
    sessionCapabilityVisibility:
      runtimeOperatorSetupSessionHandshake.sessionCapabilityVisibility,
    networkConnectionStartedOnHandshake:
      runtimeOperatorSetupSessionHandshake.networkConnectionStartedOnHandshake,
    webSocketCreatedOnHandshake:
      runtimeOperatorSetupSessionHandshake.webSocketCreatedOnHandshake,
    signedTicketVisible:
      runtimeOperatorSetupSessionHandshake.signedTicketVisible,
    rawTokenVisible: runtimeOperatorSetupSessionHandshake.rawTokenVisible,
    payloadVisible: runtimeOperatorSetupSessionHandshake.payloadVisible,
    privateKeyMaterialVisible:
      runtimeOperatorSetupSessionHandshake.privateKeyMaterialVisible,
    selectedRuntimeCanChange:
      runtimeOperatorSetupSessionHandshake.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeOperatorSetupSessionHandshake.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeOperatorSetupSessionHandshake.productDefaultCanChange,
    connectionControls:
      runtimeOperatorSetupSessionHandshake.connectionControls,
    sessionHandshake: runtimeOperatorSetupSessionHandshake.sessionHandshake,
    operatorEvidence: runtimeOperatorSetupSessionHandshake.operatorEvidence,
    remainingImplementationPhases:
      runtimeOperatorSetupSessionHandshake.remainingImplementationPhases,
    evidenceChecks: runtimeOperatorSetupSessionHandshake.evidenceChecks,
  },
  runtimeOperatorSetupApprovalFlowEvidence: {
    readiness: runtimeOperatorSetupApprovalFlowEvidence.readiness,
    implementationStatus:
      runtimeOperatorSetupApprovalFlowEvidence.implementationStatus,
    selectedRuntime: runtimeOperatorSetupApprovalFlowEvidence.selectedRuntime,
    pwaExposureDecision:
      runtimeOperatorSetupApprovalFlowEvidence.pwaExposureDecision,
    pwaExposure: runtimeOperatorSetupApprovalFlowEvidence.pwaExposure,
    endpointMode: runtimeOperatorSetupApprovalFlowEvidence.endpointMode,
    endpointAutoStart: runtimeOperatorSetupApprovalFlowEvidence.endpointAutoStart,
    publicBind: runtimeOperatorSetupApprovalFlowEvidence.publicBind,
    manualConnectRequired:
      runtimeOperatorSetupApprovalFlowEvidence.manualConnectRequired,
    sessionHandshakeRequired:
      runtimeOperatorSetupApprovalFlowEvidence.sessionHandshakeRequired,
    setupRendering: runtimeOperatorSetupApprovalFlowEvidence.setupRendering,
    approvalFlowMode:
      runtimeOperatorSetupApprovalFlowEvidence.approvalFlowMode,
    approvalRequestVisibility:
      runtimeOperatorSetupApprovalFlowEvidence.approvalRequestVisibility,
    approvalResponseDelivery:
      runtimeOperatorSetupApprovalFlowEvidence.approvalResponseDelivery,
    networkConnectionStartedOnApproval:
      runtimeOperatorSetupApprovalFlowEvidence.networkConnectionStartedOnApproval,
    webSocketCreatedOnApproval:
      runtimeOperatorSetupApprovalFlowEvidence.webSocketCreatedOnApproval,
    signedTicketVisible:
      runtimeOperatorSetupApprovalFlowEvidence.signedTicketVisible,
    rawTokenVisible: runtimeOperatorSetupApprovalFlowEvidence.rawTokenVisible,
    payloadVisible: runtimeOperatorSetupApprovalFlowEvidence.payloadVisible,
    privateKeyMaterialVisible:
      runtimeOperatorSetupApprovalFlowEvidence.privateKeyMaterialVisible,
    capabilityEnvelopeVisible:
      runtimeOperatorSetupApprovalFlowEvidence.capabilityEnvelopeVisible,
    selectedRuntimeCanChange:
      runtimeOperatorSetupApprovalFlowEvidence.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeOperatorSetupApprovalFlowEvidence.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeOperatorSetupApprovalFlowEvidence.productDefaultCanChange,
    sessionHandshake:
      runtimeOperatorSetupApprovalFlowEvidence.sessionHandshake,
    approvalFlow: runtimeOperatorSetupApprovalFlowEvidence.approvalFlow,
    operatorEvidence: runtimeOperatorSetupApprovalFlowEvidence.operatorEvidence,
    remainingImplementationPhases:
      runtimeOperatorSetupApprovalFlowEvidence.remainingImplementationPhases,
    evidenceChecks: runtimeOperatorSetupApprovalFlowEvidence.evidenceChecks,
  },
  runtimeOperatorSetupRunbookCloseout: {
    readiness: runtimeOperatorSetupRunbookCloseout.readiness,
    implementationStatus:
      runtimeOperatorSetupRunbookCloseout.implementationStatus,
    selectedRuntime: runtimeOperatorSetupRunbookCloseout.selectedRuntime,
    pwaExposureDecision:
      runtimeOperatorSetupRunbookCloseout.pwaExposureDecision,
    pwaExposure: runtimeOperatorSetupRunbookCloseout.pwaExposure,
    endpointMode: runtimeOperatorSetupRunbookCloseout.endpointMode,
    endpointAutoStart: runtimeOperatorSetupRunbookCloseout.endpointAutoStart,
    publicBind: runtimeOperatorSetupRunbookCloseout.publicBind,
    manualConnectRequired:
      runtimeOperatorSetupRunbookCloseout.manualConnectRequired,
    sessionHandshakeRequired:
      runtimeOperatorSetupRunbookCloseout.sessionHandshakeRequired,
    setupRendering: runtimeOperatorSetupRunbookCloseout.setupRendering,
    approvalFlowMode: runtimeOperatorSetupRunbookCloseout.approvalFlowMode,
    approvalResponseDelivery:
      runtimeOperatorSetupRunbookCloseout.approvalResponseDelivery,
    networkConnectionStartedOnCloseout:
      runtimeOperatorSetupRunbookCloseout.networkConnectionStartedOnCloseout,
    webSocketCreatedOnCloseout:
      runtimeOperatorSetupRunbookCloseout.webSocketCreatedOnCloseout,
    signedTicketVisible:
      runtimeOperatorSetupRunbookCloseout.signedTicketVisible,
    rawTokenVisible: runtimeOperatorSetupRunbookCloseout.rawTokenVisible,
    payloadVisible: runtimeOperatorSetupRunbookCloseout.payloadVisible,
    privateKeyMaterialVisible:
      runtimeOperatorSetupRunbookCloseout.privateKeyMaterialVisible,
    capabilityEnvelopeVisible:
      runtimeOperatorSetupRunbookCloseout.capabilityEnvelopeVisible,
    selectedRuntimeCanChange:
      runtimeOperatorSetupRunbookCloseout.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeOperatorSetupRunbookCloseout.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeOperatorSetupRunbookCloseout.productDefaultCanChange,
    approvalFlowEvidence:
      runtimeOperatorSetupRunbookCloseout.approvalFlowEvidence,
    evidenceMap: runtimeOperatorSetupRunbookCloseout.evidenceMap,
    requiredRunbookCommands:
      runtimeOperatorSetupRunbookCloseout.requiredRunbookCommands,
    remainingImplementationPhases:
      runtimeOperatorSetupRunbookCloseout.remainingImplementationPhases,
    evidenceChecks: runtimeOperatorSetupRunbookCloseout.evidenceChecks,
  },
  runtimeOperatorSetupApprovalResponseDeliveryBoundary: {
    readiness: runtimeOperatorSetupApprovalResponseDeliveryBoundary.readiness,
    implementationStatus:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.implementationStatus,
    selectedRuntime:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.selectedRuntime,
    pwaExposureDecision:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.pwaExposureDecision,
    pwaExposure: runtimeOperatorSetupApprovalResponseDeliveryBoundary.pwaExposure,
    endpointMode:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.endpointMode,
    endpointAutoStart:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.endpointAutoStart,
    publicBind: runtimeOperatorSetupApprovalResponseDeliveryBoundary.publicBind,
    manualConnectRequired:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.manualConnectRequired,
    sessionHandshakeRequired:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.sessionHandshakeRequired,
    setupRendering:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.setupRendering,
    approvalFlowMode:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.approvalFlowMode,
    deliveryMode:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.deliveryMode,
    approvalResponseDelivery:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.approvalResponseDelivery,
    networkDeliveryStatus:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.networkDeliveryStatus,
    approvalResponseVisibleInApprovalPanel:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.approvalResponseVisibleInApprovalPanel,
    approvalResponseVisibleInManagedSetupSurface:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.approvalResponseVisibleInManagedSetupSurface,
    verifyCommandVisible:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.verifyCommandVisible,
    copyResponseControlVisible:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.copyResponseControlVisible,
    networkConnectionStartedOnDelivery:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.networkConnectionStartedOnDelivery,
    webSocketCreatedOnDelivery:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.webSocketCreatedOnDelivery,
    endpointStartedOnDelivery:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.endpointStartedOnDelivery,
    publicBindEnabledOnDelivery:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.publicBindEnabledOnDelivery,
    signedTicketVisible:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.signedTicketVisible,
    rawTokenVisible:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.rawTokenVisible,
    payloadVisible:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.payloadVisible,
    privateKeyMaterialVisible:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.privateKeyMaterialVisible,
    capabilityEnvelopeVisible:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.capabilityEnvelopeVisible,
    selectedRuntimeCanChange:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.productDefaultCanChange,
    runbookCloseout:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.runbookCloseout,
    deliveryBoundary:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.deliveryBoundary,
    operatorEvidence:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.operatorEvidence,
    remainingImplementationPhases:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.remainingImplementationPhases,
    evidenceChecks:
      runtimeOperatorSetupApprovalResponseDeliveryBoundary.evidenceChecks,
  },
  runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence: {
    readiness:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.readiness,
    implementationStatus:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.implementationStatus,
    selectedRuntime:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.selectedRuntime,
    pwaExposureDecision:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.pwaExposureDecision,
    pwaExposure:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.pwaExposure,
    endpointMode:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointMode,
    endpointAutoStart:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointAutoStart,
    endpointStartedByAutoStart:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointStartedByAutoStart,
    publicBind:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.publicBind,
    manualConnectRequired:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.manualConnectRequired,
    sessionHandshakeRequired:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.sessionHandshakeRequired,
    setupRendering:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.setupRendering,
    approvalFlowMode:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.approvalFlowMode,
    deliveryMode:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.deliveryMode,
    approvalResponseDelivery:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.approvalResponseDelivery,
    manualCopyFallback:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.manualCopyFallback,
    networkDeliveryStatus:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.networkDeliveryStatus,
    approvalResponseVisibleInApprovalPanel:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.approvalResponseVisibleInApprovalPanel,
    approvalResponseVisibleInManagedSetupSurface:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.approvalResponseVisibleInManagedSetupSurface,
    verifyCommandVisible:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.verifyCommandVisible,
    copyResponseControlVisible:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.copyResponseControlVisible,
    networkConnectionStartedOnDelivery:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.networkConnectionStartedOnDelivery,
    webSocketCreatedOnDelivery:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.webSocketCreatedOnDelivery,
    endpointStartedOnDelivery:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointStartedOnDelivery,
    endpointStartedByOperator:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointStartedByOperator,
    publicBindEnabledOnDelivery:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.publicBindEnabledOnDelivery,
    encryptedFrameDelivery:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.encryptedFrameDelivery,
    routeVisiblePayload:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.routeVisiblePayload,
    plaintextPayloadVisibleToRelay:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.plaintextPayloadVisibleToRelay,
    approvalResponsePayloadVisibleToRelay:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.approvalResponsePayloadVisibleToRelay,
    payloadKeyVisibleToRelay:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.payloadKeyVisibleToRelay,
    payloadCiphertextVisibleToOperator:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.payloadCiphertextVisibleToOperator,
    signedTicketVisible:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.signedTicketVisible,
    rawTokenVisible:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.rawTokenVisible,
    payloadVisible:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.payloadVisible,
    privateKeyMaterialVisible:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.privateKeyMaterialVisible,
    capabilityEnvelopeVisible:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.capabilityEnvelopeVisible,
    selectedRuntimeCanChange:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.selectedRuntimeCanChange,
    selectedRuntimeChangeBoundary:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.selectedRuntimeChangeBoundary,
    productDefaultCanChange:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.productDefaultCanChange,
    deliveryBoundary:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.deliveryBoundary,
    endpointDeliveryEvidence:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.endpointDeliveryEvidence,
    operatorEvidence:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.operatorEvidence,
    remainingImplementationPhases:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.remainingImplementationPhases,
    evidenceChecks:
      runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.evidenceChecks,
  },
  nextLocalSlice:
    runtimeOperatorSetupApprovalResponseEndpointDeliveryEvidence.nextLocalSlice,
};

await mkdir(artifactRoot, { recursive: true });
await writeFile(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
console.log(`RA_PWA_RELAY_NEXT_MODE_PLANNING_OK ${evidencePath}`);
