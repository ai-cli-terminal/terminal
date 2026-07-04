export const COMPANION_IDENTITY_KEY = "ai-terminal-companion-identity-v1";
export const COMPANION_IDENTITY_DB = "ai-terminal-companion-v1";
export const COMPANION_IDENTITY_STORE = "identity";
export const ACTIVE_IDENTITY_ID = "active";
export const LIVE_TRANSPORT_PROTOCOL_VERSION = 1;
export const RELAY_TRANSPORT_PROTOCOL_VERSION = 1;
export const DEFAULT_RELAY_FRAME_TTL_MS = 30_000;
export const DEFAULT_RELAY_SESSION_TTL_MS = 5 * 60 * 1000;
export const RELAY_TICKET_MAC_ALG_HMAC_SHA256 = "hmac-sha256";
export const RELAY_TICKET_MAC_ALG_ED25519 = "ed25519";
export const PWA_TRANSPORT_MODE_LIVE_LOOPBACK = "live-loopback";
export const PWA_TRANSPORT_MODE_RELAY = "relay";
export const PWA_RELAY_DEPLOYMENT_MODE_SELF_HOSTED = "self-hosted";
export const PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK = "private-network";
export const PWA_RELAY_DEPLOYMENT_MODE_MANAGED = "managed";
export const PWA_RELAY_DEPLOYMENT_MODES = Object.freeze([
  PWA_RELAY_DEPLOYMENT_MODE_SELF_HOSTED,
  PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK,
  PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
]);
export const PWA_RELAY_SELECTED_DEPLOYMENT_MODE = PWA_RELAY_DEPLOYMENT_MODE_SELF_HOSTED;
export const PWA_RELAY_DEPLOYMENT_DECISION = Object.freeze({
  selectedMode: PWA_RELAY_SELECTED_DEPLOYMENT_MODE,
  selectedSubstrate: "websocket",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  relayTransportReadiness: "planned",
  endpointPolicy: "wss-production-localhost-ws-development",
  ticketSecretOwner: "daemon",
  ticketVerifierMode: "ed25519-public-verifier-preferred",
  payloadConfidentiality: "explicit-self-hosted-operator-trust-decision",
  deferredModes: Object.freeze([
    PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK,
    PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  ]),
});
export const PWA_RELAY_PRIVATE_NETWORK_SETUP_CONTRACT = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK,
  readiness: "contract",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  endpointPolicy: "private-network-wss-localhost-ws-development-only",
  selectedRuntime: "deferred",
  managedRelay: "deferred",
  requiredSetupFields: Object.freeze([
    "transportMode",
    "deploymentMode",
    "relayEndpointUrl",
    "privateNetworkName",
    "signedSessionTicket",
    "companionIdentity",
    "operatorSetupText",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "private_network_relay_is_explicit_setup_path",
    "public_ws_blocked",
    "wss_required_for_non_localhost_endpoints",
    "managed_relay_remains_deferred",
  ]),
});
export const PWA_RELAY_MANAGED_OPERATIONS_PLAN = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "planning",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  privateNetworkRelay: "explicit-advanced-path-ready",
  implementationStatus: "blocked-until-operations-contract",
  requiredBeforeImplementation: Object.freeze([
    "control-plane-ownership",
    "tenant-isolation",
    "abuse-handling",
    "support-workflows",
    "retention-policy",
    "billing-and-quota-policy",
    "public-verifier-key-operations",
    "payload-confidentiality-plan",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_remains_deferred",
    "private_network_relay_remains_explicit_advanced_path",
    "self_hosted_relay_readiness_remains_separate",
    "no_managed_runtime_without_operations_contract",
  ]),
});
export const PWA_RELAY_MANAGED_CONTROL_PLANE_CONTRACT = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "contract",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  controlPlaneOwner: "required-before-runtime",
  tenantBoundary: "tenant-isolated-sessions-and-verifier-keys",
  sessionBoundary: "per-session-ticket-and-frame-isolation",
  operatorVisibleState: "aggregate-health-and-control-plane-events-only",
  auditBoundary: "no-payload-json-or-secret-material",
  requiredRoles: Object.freeze([
    "service-operator",
    "tenant-admin",
    "daemon-owner",
    "support-operator",
  ]),
  requiredContracts: Object.freeze([
    "tenant-identity",
    "session-registration",
    "verifier-key-distribution",
    "quota-and-rate-limit",
    "support-access",
    "audit-retention",
  ]),
  prohibitedControlPlaneData: Object.freeze([
    "payload_json",
    "session_tokens",
    "approval_signatures",
    "private_key_material",
    "hmac_secrets",
    "full_setup_json",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "tenant_data_isolation_required",
    "operator_state_excludes_payload_json",
    "support_access_requires_audit_boundary",
  ]),
});
export const PWA_RELAY_MANAGED_ABUSE_RETENTION_POLICY = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "policy",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  abuseHandling: "tenant-scoped-rate-limits-and-operator-escalation",
  retentionBoundary: "aggregate-audit-only-no-payload-json",
  deletionBoundary: "tenant-and-session-metadata-deletion-required",
  supportBoundary: "audited-aggregate-only-support-workflows",
  enforcementDefault: "fail-closed-before-managed-runtime",
  rateLimitScopes: Object.freeze([
    "tenant",
    "daemon-device",
    "session",
    "source-ip",
    "verifier-key",
  ]),
  abuseSignals: Object.freeze([
    "invalid-ticket-rate",
    "session-registration-failure-rate",
    "frame-replay-or-duplicate-sequence-rate",
    "expired-frame-drop-rate",
    "tenant-quota-exhaustion",
  ]),
  deletionRequirements: Object.freeze([
    "tenant-deletion-removes-session-metadata",
    "verifier-key-revocation-stops-new-sessions",
    "support-export-excludes-payloads-and-secrets",
    "retention-expiry-purges-audit-and-case-metadata",
  ]),
  supportWorkflowConstraints: Object.freeze([
    "support-access-audited",
    "tenant-admin-approval-required",
    "aggregate-state-only",
    "no-payload-json-or-secret-material",
    "breakglass-time-bounded",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "tenant_scoped_abuse_limits_required",
    "no_payload_or_secret_retention",
    "support_access_requires_audit_and_tenant_scope",
    "deletion_requirements_before_runtime",
  ]),
});
export const PWA_RELAY_MANAGED_PAYLOAD_CONFIDENTIALITY_PLAN = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "plan",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  payloadConfidentiality: "required-payload-blind-managed-relay",
  operatorTrustBoundary: "relay-operator-cannot-read-payload-json-or-approval-content",
  serviceVisibility: "routing-metadata-and-aggregate-health-only",
  managedRuntimeRequirement: "end-to-end-encrypted-frame-payloads-before-runtime",
  fallbackDecision: "without-payload-blind-design-managed-relay-remains-deferred",
  keyAccessPolicy: "daemon-and-companion-only",
  prohibitedManagedRelayData: Object.freeze([
    "payload_json",
    "command_text",
    "context_json",
    "approval_response_payload",
    "session_tokens",
    "private_key_material",
    "hmac_secrets",
    "full_setup_json",
  ]),
  allowedRelayMetadata: Object.freeze([
    "tenant_id",
    "session_id",
    "daemon_device_id_hash",
    "companion_device_id_hash",
    "frame_sequence",
    "frame_expiry_ms",
    "ticket_key_id",
    "aggregate_error_class",
  ]),
  requiredBeforeRuntime: Object.freeze([
    "frame-payload-e2e-encryption",
    "envelope-metadata-minimization",
    "client-held-payload-keys",
    "key-rotation-and-revocation",
    "confidentiality-regression-evidence",
    "support-payload-redaction",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "payload_blind_managed_relay_required",
    "operator_trust_not_sufficient_for_managed_relay",
    "client_held_payload_keys_required",
    "metadata_minimization_required",
    "support_access_cannot_decrypt_payloads",
  ]),
});
export const PWA_RELAY_MANAGED_VERIFIER_KEY_OPERATIONS_POLICY = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "policy",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  verifierKeyOwner: "tenant-admin-owned-daemon-issued-signing-keys",
  verifierKeyDistribution: "managed-relay-public-verifier-keys-only",
  keyMaterialBoundary: "private-signing-keys-never-enter-managed-relay",
  rotationPolicy: "overlapping-key-id-versions-with-explicit-retirement",
  revocationPolicy: "revoked-key-ids-stop-new-session-registration",
  auditBoundary: "key-id-version-events-without-private-key-material",
  requiredKeyStates: Object.freeze([
    "pending",
    "active",
    "rotating",
    "retiring",
    "revoked",
  ]),
  requiredKeyOperations: Object.freeze([
    "register-public-verifier-key",
    "activate-key-version",
    "rotate-with-overlap-window",
    "revoke-key-id",
    "reject-retired-key-registration",
    "audit-key-version-change",
  ]),
  prohibitedVerifierKeyData: Object.freeze([
    "private_signing_key",
    "hmac_secret",
    "raw_session_token",
    "payload_json",
    "approval_signature_payload",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "public_verifier_keys_only_in_relay_service",
    "private_signing_keys_never_leave_daemon_or_tenant_admin",
    "key_id_version_required_for_tickets",
    "revoked_keys_fail_closed_for_new_sessions",
    "key_rotation_requires_overlap_window",
  ]),
});
export const PWA_RELAY_MANAGED_BILLING_QUOTA_POLICY = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "policy",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  billingModel: "tenant-scoped-metered-usage-before-managed-runtime",
  quotaEnforcement: "tenant-and-session-quota-fail-closed-before-runtime",
  usageVisibility: "tenant-aggregate-usage-no-payload-or-secret-data",
  quotaOwner: "tenant-admin-owned-service-enforced-limits",
  billingBoundary: "control-plane-usage-metadata-only",
  requiredQuotaScopes: Object.freeze([
    "tenant",
    "daemon-device",
    "session",
    "verifier-key",
    "source-ip",
  ]),
  meteredUsageDimensions: Object.freeze([
    "session-registration-count",
    "active-session-count",
    "relay-frame-count",
    "relay-byte-count",
    "invalid-ticket-count",
    "quota-denial-count",
  ]),
  prohibitedBillingData: Object.freeze([
    "payload_json",
    "command_text",
    "context_json",
    "approval_response_payload",
    "private_key_material",
    "raw_session_token",
    "full_setup_json",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "tenant_usage_metadata_only",
    "quota_enforcement_fail_closed",
    "billing_records_exclude_payloads_and_secrets",
    "quota_policy_required_before_runtime",
    "abuse_limits_remain_separate_from_billing",
  ]),
});
export const PWA_RELAY_MANAGED_RUNTIME_READINESS_GATE = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "gate",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  gateStatus: "blocked-until-runtime-evidence",
  implementationDecision: "managed-runtime-implementation-not-started",
  runtimeDefault: "not-selected",
  requiredPlanningInputs: Object.freeze([
    "control-plane-ownership",
    "tenant-isolation",
    "abuse-handling",
    "support-workflows",
    "retention-policy",
    "payload-confidentiality-plan",
    "public-verifier-key-operations",
    "billing-and-quota-policy",
  ]),
  requiredRuntimeEvidence: Object.freeze([
    "payload-blind-frame-encryption-smoke",
    "client-key-agreement-runtime-smoke",
    "metadata-minimization-review",
    "public-verifier-key-registry-runtime-smoke",
    "revocation-and-rotation-propagation-smoke",
    "tenant-session-registration-quota-smoke",
    "active-session-and-byte-quota-smoke",
    "tenant-aggregate-usage-export-smoke",
    "support-redaction-and-access-review-evidence",
    "billing-abuse-boundary-review",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "no_managed_runtime_until_readiness_gate_green",
    "runtime_evidence_required_before_pwa_exposure",
    "payload_blind_runtime_required",
    "public_verifier_key_runtime_required",
    "quota_enforcement_runtime_required",
    "aggregate_usage_export_required",
    "support_redaction_required",
  ]),
});
export const MAX_RELAY_SESSION_ID_LENGTH = 96;
export const MIN_RELAY_SESSION_TOKEN_LENGTH = 32;
export const MAX_RELAY_SESSION_TOKEN_LENGTH = 128;
export const MAX_RELAY_DEVICE_ID_LENGTH = 96;
export const MIN_RELAY_TICKET_HMAC_KEY_BYTES = 32;
export const MAX_RELAY_PAYLOAD_JSON_BYTES = 1 << 20;
export const MANAGED_RELAY_PAYLOAD_CIPHERTEXT_ALG = "aes-256-gcm";
export const MANAGED_RELAY_PAYLOAD_KEY_SCOPE = "client-held-session-key";
export const MANAGED_RELAY_PAYLOAD_KEY_BYTES = 32;
export const MANAGED_RELAY_PAYLOAD_NONCE_BYTES = 12;
export const MAX_MANAGED_RELAY_PAYLOAD_CIPHERTEXT_BYTES = MAX_RELAY_PAYLOAD_JSON_BYTES + 16;
export const MANAGED_RELAY_PAYLOAD_KEY_AGREEMENT_ALG = "x25519-hkdf-sha256";
export const MANAGED_RELAY_PAYLOAD_KEY_HKDF_HASH = "SHA-256";
export const MANAGED_RELAY_PAYLOAD_KEY_HKDF_INFO = "ai-terminal-managed-relay-payload-key-v1";
export const MANAGED_RELAY_PUBLIC_VERIFIER_KEY_ALG = RELAY_TICKET_MAC_ALG_ED25519;
export const PWA_RELAY_MANAGED_PAYLOAD_BLIND_FRAME_ENCRYPTION_SPIKE = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "spike",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  implementationStatus: "payload-blind-frame-envelope-ready-runtime-still-deferred",
  payloadCiphertextAlg: MANAGED_RELAY_PAYLOAD_CIPHERTEXT_ALG,
  payloadKeyScope: MANAGED_RELAY_PAYLOAD_KEY_SCOPE,
  completedRuntimeEvidence: Object.freeze([
    "payload-blind-frame-encryption-smoke",
  ]),
  closedReadinessBlockers: Object.freeze([
    "e2e_payload_encryption_missing",
    "confidentiality_smoke_missing",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "relay_routes_ciphertext_only",
    "payload_json_excluded_from_managed_frame",
    "command_context_and_approval_payload_excluded_from_route",
    "client_held_payload_key_required",
    "aes_gcm_nonce_required_per_frame",
  ]),
});
export const PWA_RELAY_MANAGED_CLIENT_KEY_AGREEMENT_RUNTIME_SMOKE = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "smoke",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  implementationStatus: "client-key-agreement-smoke-ready-runtime-still-deferred",
  keyAgreementAlg: MANAGED_RELAY_PAYLOAD_KEY_AGREEMENT_ALG,
  hkdfHash: MANAGED_RELAY_PAYLOAD_KEY_HKDF_HASH,
  hkdfInfo: MANAGED_RELAY_PAYLOAD_KEY_HKDF_INFO,
  payloadCiphertextAlg: MANAGED_RELAY_PAYLOAD_CIPHERTEXT_ALG,
  payloadKeyScope: MANAGED_RELAY_PAYLOAD_KEY_SCOPE,
  completedRuntimeEvidence: Object.freeze([
    "client-key-agreement-runtime-smoke",
  ]),
  closedReadinessBlockers: Object.freeze([
    "client_key_agreement_missing",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "session_bound_payload_key_required",
    "daemon_and_companion_derive_same_payload_key",
    "managed_relay_receives_public_keys_only",
    "route_metadata_cannot_derive_payload_key",
    "payload_key_not_serialized_to_frame_or_route",
  ]),
});
export const PWA_RELAY_MANAGED_METADATA_MINIMIZATION_REVIEW = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "review",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  implementationStatus: "metadata-minimization-review-complete-runtime-still-deferred",
  metadataBoundary: "allowlisted-route-control-billing-support-audit-metadata-only",
  completedRuntimeEvidence: Object.freeze([
    "metadata-minimization-review",
  ]),
  closedReadinessBlockers: Object.freeze([
    "metadata_minimization_review_missing",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "metadata_surfaces_are_allowlisted",
    "route_metadata_excludes_payload_and_key_material",
    "support_metadata_excludes_raw_ciphertext",
    "billing_metadata_is_aggregate_only",
    "audit_metadata_excludes_payloads_and_secrets",
  ]),
});
export const PWA_RELAY_MANAGED_PUBLIC_VERIFIER_KEY_REGISTRY_RUNTIME_SMOKE = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "smoke",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  implementationStatus: "public-verifier-key-registry-smoke-ready-runtime-still-deferred",
  verifierKeyAlg: MANAGED_RELAY_PUBLIC_VERIFIER_KEY_ALG,
  registryBoundary: "tenant-key-id-version-public-verifiers-only",
  completedRuntimeEvidence: Object.freeze([
    "public-verifier-key-registry-runtime-smoke",
  ]),
  closedReadinessBlockers: Object.freeze([
    "managed_key_registry_runtime_missing",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "registry_contains_public_verifier_keys_only",
    "private_signing_keys_excluded_from_registry",
    "hmac_secrets_excluded_from_registry",
    "ticket_key_id_version_required",
    "missing_or_revoked_key_fails_closed",
    "key_id_version_audit_metadata_preserved",
  ]),
});
export const PWA_RELAY_MANAGED_REVOCATION_AND_ROTATION_PROPAGATION_SMOKE = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "smoke",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  implementationStatus: "revocation-and-rotation-propagation-smoke-ready-runtime-still-deferred",
  verifierKeyAlg: MANAGED_RELAY_PUBLIC_VERIFIER_KEY_ALG,
  propagationBoundary: "snapshot-based-tenant-key-version-state",
  completedRuntimeEvidence: Object.freeze([
    "revocation-and-rotation-propagation-smoke",
  ]),
  closedReadinessBlockers: Object.freeze([
    "key_revocation_propagation_smoke_missing",
    "rotation_overlap_smoke_missing",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "active_and_rotating_keys_overlap_during_rotation",
    "retiring_keys_fail_closed_for_new_sessions",
    "revoked_keys_fail_closed_after_snapshot_propagation",
    "registry_snapshot_id_preserved_in_audit",
    "tenant_key_id_version_audit_metadata_preserved",
  ]),
});
export const PWA_RELAY_MANAGED_TENANT_SESSION_REGISTRATION_QUOTA_SMOKE = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "smoke",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  implementationStatus: "tenant-session-registration-quota-smoke-ready-runtime-still-deferred",
  quotaBoundary: "tenant-scoped-session-registration-preflight",
  completedRuntimeEvidence: Object.freeze([
    "tenant-session-registration-quota-smoke",
  ]),
  closedReadinessBlockers: Object.freeze([
    "quota_enforcement_smoke_missing",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "session_registration_quota_checked_before_registration",
    "quota_denials_fail_closed_before_session_creation",
    "quota_denial_audit_excludes_payloads_and_secrets",
    "abuse_rate_limit_signals_remain_separate_from_billing_meters",
    "billing_meters_record_quota_denials_without_payloads",
  ]),
});
export const PWA_RELAY_MANAGED_ACTIVE_SESSION_AND_BYTE_QUOTA_SMOKE = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "smoke",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  implementationStatus: "active-session-and-byte-quota-smoke-ready-runtime-still-deferred",
  quotaBoundary: "tenant-and-daemon-active-session-plus-frame-byte-preflight",
  completedRuntimeEvidence: Object.freeze([
    "active-session-and-byte-quota-smoke",
  ]),
  closedReadinessBlockers: Object.freeze([
    "managed_usage_meter_runtime_missing",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "tenant_active_session_quota_checked_before_activation",
    "daemon_device_active_session_quota_checked_before_activation",
    "frame_and_byte_quota_checked_before_route",
    "quota_denials_fail_closed_before_frame_routing",
    "usage_meter_deltas_exclude_payloads_and_secrets",
    "billing_meters_record_frame_and_byte_counts_without_payloads",
    "abuse_rate_limit_signals_remain_separate_from_billing_meters",
  ]),
});
export const PWA_RELAY_MANAGED_TENANT_AGGREGATE_USAGE_EXPORT_SMOKE = Object.freeze({
  deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_MANAGED,
  readiness: "smoke",
  productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
  selectedRuntime: "deferred",
  implementationStatus: "tenant-aggregate-usage-export-smoke-ready-runtime-still-deferred",
  exportBoundary: "tenant-aggregate-usage-counters-without-payloads-or-secrets",
  completedRuntimeEvidence: Object.freeze([
    "tenant-aggregate-usage-export-smoke",
  ]),
  closedReadinessBlockers: Object.freeze([
    "tenant_usage_export_smoke_missing",
  ]),
  guardrails: Object.freeze([
    "product_default_remains_live_loopback",
    "managed_relay_runtime_remains_deferred",
    "tenant_usage_export_is_aggregate_only",
    "tenant_usage_export_excludes_payloads_and_secrets",
    "session_active_frame_byte_and_quota_denial_counters_exported",
    "billing_usage_and_abuse_signals_are_separate_sections",
    "support_views_remain_aggregate_only",
  ]),
});

export function decodePairPayloadFromUrl(urlText) {
  const url = new URL(urlText, "https://companion.local/");
  const payload = url.searchParams.get("payload");
  return payload || "";
}

export function decodeApprovalPayloadFromUrl(urlText) {
  const url = new URL(urlText, "https://companion.local/");
  return url.searchParams.get("approval") || url.searchParams.get("request") || "";
}

export function decodeRelaySetupPayloadFromUrl(urlText) {
  const url = new URL(urlText, "https://companion.local/");
  return url.searchParams.get("relaySetup") || url.searchParams.get("setup") || "";
}

export function parsePairingInput(text, currentSearch = "") {
  const raw = (text || "").trim();
  let candidate = raw;
  if (!candidate && currentSearch) {
    candidate = decodePairPayloadFromUrl(`https://companion.local/${currentSearch}`);
  } else if (candidate.startsWith("aiterminal://pair?") || candidate.includes("?payload=")) {
    candidate = decodePairPayloadFromUrl(candidate);
  }
  if (!candidate) {
    throw new Error("payload 없음");
  }

  let payload;
  try {
    payload = JSON.parse(candidate);
  } catch {
    throw new Error("payload JSON 파싱 실패");
  }
  validatePairingPayload(payload);
  return payload;
}

export function validatePairingPayload(payload) {
  if (payload.protocol_version !== 1) {
    throw new Error("지원하지 않는 protocol_version");
  }
  if (!/^[0-9]{6}$/.test(payload.pairing_code || "")) {
    throw new Error("pairing_code 형식 오류");
  }
  if (!/^[0-9a-f]{64}$/i.test(payload.daemon_pubkey_hex || "")) {
    throw new Error("daemon_pubkey_hex 형식 오류");
  }
  if (typeof payload.transport_addr !== "string" || payload.transport_addr.length < 6) {
    throw new Error("transport_addr 형식 오류");
  }
  if (!Number.isSafeInteger(payload.expires_at_ms) || payload.expires_at_ms <= 0) {
    throw new Error("expires_at_ms 형식 오류");
  }
}

export function parseApprovalInput(text, currentSearch = "") {
  const raw = (text || "").trim();
  let candidate = raw;
  if (!candidate && currentSearch) {
    candidate = decodeApprovalPayloadFromUrl(`https://companion.local/${currentSearch}`);
  } else if (candidate.startsWith("aiterminal://approve?") || candidate.includes("?approval=")) {
    candidate = decodeApprovalPayloadFromUrl(candidate);
  }
  if (!candidate) {
    throw new Error("approval request 없음");
  }
  let request;
  try {
    request = JSON.parse(candidate);
  } catch {
    throw new Error("approval request JSON 파싱 실패");
  }
  validateApprovalRequest(request);
  return request;
}

export function parseRelayRuntimeSetupInput(text, currentSearch = "") {
  const raw = (text || "").trim();
  let candidate = raw;
  if (!candidate && currentSearch) {
    candidate = decodeRelaySetupPayloadFromUrl(`https://companion.local/${currentSearch}`);
  } else if (
    candidate.startsWith("aiterminal://relay?") ||
    candidate.includes("?relaySetup=") ||
    candidate.includes("?setup=")
  ) {
    candidate = decodeRelaySetupPayloadFromUrl(candidate);
  }
  if (!candidate) {
    throw new Error("relay setup 없음");
  }
  let setup;
  try {
    setup = JSON.parse(candidate);
  } catch {
    throw new Error("relay setup JSON 파싱 실패");
  }
  validateRelayRuntimeSetupMetadata(setup);
  return setup;
}

export function parseRelayPrivateNetworkRuntimeSetupInput(text, currentSearch = "") {
  const raw = (text || "").trim();
  let candidate = raw;
  if (!candidate && currentSearch) {
    candidate = decodeRelaySetupPayloadFromUrl(`https://companion.local/${currentSearch}`);
  } else if (
    candidate.startsWith("aiterminal://relay?") ||
    candidate.includes("?relaySetup=") ||
    candidate.includes("?setup=")
  ) {
    candidate = decodeRelaySetupPayloadFromUrl(candidate);
  }
  if (!candidate) {
    throw new Error("private-network relay setup 없음");
  }
  let setup;
  try {
    setup = JSON.parse(candidate);
  } catch {
    throw new Error("private-network relay setup JSON 파싱 실패");
  }
  validateRelayPrivateNetworkRuntimeSetupMetadata(setup);
  return setup;
}

export function validateRelayRuntimeSetupMetadata(setup) {
  validateRelayRuntimeSetupCommonMetadata(setup, PWA_RELAY_SELECTED_DEPLOYMENT_MODE);
  if (Object.prototype.hasOwnProperty.call(setup, "privateNetworkName")) {
    throw new Error("self-hosted relay setup privateNetworkName 형식 오류");
  }
}

export function validateRelayPrivateNetworkRuntimeSetupMetadata(setup) {
  validateRelayRuntimeSetupCommonMetadata(setup, PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK);
  if (!validPrivateNetworkName(setup.privateNetworkName)) {
    throw new Error("private-network relay setup privateNetworkName 형식 오류");
  }
}

function validateRelayRuntimeSetupCommonMetadata(setup, expectedDeploymentMode) {
  if (!setup || typeof setup !== "object" || Array.isArray(setup)) {
    throw new Error("relay setup 형식 오류");
  }
  rejectRelaySetupSecretFields(setup);
  if (setup.relayProtocolVersion !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay setup protocol_version");
  }
  if (setup.transportMode !== PWA_TRANSPORT_MODE_RELAY) {
    throw new Error("relay setup transportMode 형식 오류");
  }
  if (setup.deploymentMode !== expectedDeploymentMode) {
    throw new Error("relay setup deploymentMode 형식 오류");
  }
  if (!validRelayWebSocketEndpointUrl(setup.relayEndpointUrl)) {
    throw new Error("relay setup endpoint URL 형식 오류");
  }
  validateSignedRelaySessionTicketMetadata(setup.signedSessionTicket);
  const ticket = setup.signedSessionTicket.ticket;
  if (setup.daemonConnect?.peer !== "daemon") {
    throw new Error("relay setup daemonConnect peer 형식 오류");
  }
  if (setup.companionConnect?.peer !== "companion") {
    throw new Error("relay setup companionConnect peer 형식 오류");
  }
  validateRelaySessionConnect(ticket, setup.daemonConnect, ticket.issued_at_ms);
  validateRelaySessionConnect(ticket, setup.companionConnect, ticket.issued_at_ms);
  if (!validCompanionIdentity(setup.companionIdentity)) {
    throw new Error("relay setup companionIdentity 형식 오류");
  }
  if (
    setup.companionIdentity.deviceId !== ticket.companion_device_id ||
    setup.companionIdentity.noisePubkeyHex !== ticket.companion_noise_pubkey_hex ||
    setup.companionIdentity.approvalPubkeyHex !== ticket.companion_approval_pubkey_hex
  ) {
    throw new Error("relay setup companionIdentity mismatch");
  }
  if (typeof setup.operatorSetupText !== "string" || setup.operatorSetupText.trim().length < 12) {
    throw new Error("relay setup operatorSetupText 형식 오류");
  }
}

export function relayRuntimeSetupPreflight(setup, nowMs = Date.now()) {
  validateRelayRuntimeSetupMetadata(setup);
  return relayTransportUxPreflight(
    {
      transportMode: setup.transportMode,
      relayEndpointUrl: setup.relayEndpointUrl,
      signedSessionTicket: setup.signedSessionTicket,
      companionIdentity: setup.companionIdentity,
      deploymentMode: setup.deploymentMode,
      operatorSetupText: setup.operatorSetupText,
    },
    nowMs,
  );
}

export function relayPrivateNetworkRuntimeSetupPreflight(setup, nowMs = Date.now()) {
  validateRelayPrivateNetworkRuntimeSetupMetadata(setup);
  return relayPrivateNetworkSetupPreflight(
    {
      transportMode: setup.transportMode,
      deploymentMode: setup.deploymentMode,
      relayEndpointUrl: setup.relayEndpointUrl,
      privateNetworkName: setup.privateNetworkName,
      signedSessionTicket: setup.signedSessionTicket,
      companionIdentity: setup.companionIdentity,
      operatorSetupText: setup.operatorSetupText,
    },
    nowMs,
  );
}

export function validateApprovalRequest(request) {
  if (!Array.isArray(request.approval_id) || request.approval_id.length === 0) {
    throw new Error("approval_id 형식 오류");
  }
  if (!Array.isArray(request.nonce) || request.nonce.length !== 32) {
    throw new Error("nonce 형식 오류");
  }
  for (const byte of [...request.approval_id, ...request.nonce]) {
    if (!Number.isInteger(byte) || byte < 0 || byte > 255) {
      throw new Error("byte array 형식 오류");
    }
  }
  if (typeof request.command_masked !== "string" || request.command_masked.length === 0) {
    throw new Error("command_masked 형식 오류");
  }
  if (typeof request.context_hash !== "string" || request.context_hash.length === 0) {
    throw new Error("context_hash 형식 오류");
  }
  if (!Number.isSafeInteger(request.expires_at) || request.expires_at <= 0) {
    throw new Error("expires_at 형식 오류");
  }
  if (!Number.isSafeInteger(request.device_epoch) || request.device_epoch < 0) {
    throw new Error("device_epoch 형식 오류");
  }
}

export function commandForPairing(payload, device) {
  const deviceId = shellToken(device.deviceId || "");
  const noise = (device.noisePubkeyHex || "").trim();
  const approval = (device.approvalPubkeyHex || "").trim();
  if (!deviceId || !/^[0-9a-f]{64}$/i.test(noise) || !/^[0-9a-f]{64}$/i.test(approval)) {
    return "-";
  }
  return [
    "ai remote pair",
    `--device-id ${deviceId}`,
    `--code ${payload.pairing_code}`,
    `--noise-pubkey-hex ${noise}`,
    `--approval-pubkey-hex ${approval}`,
  ].join(" ");
}

export function commandForApprovalVerify(request, response, deviceId) {
  validateApprovalRequest(request);
  validateApprovalResponse(response);
  const id = shellToken(deviceId || "");
  if (!id) return "-";
  return [
    "ai remote approval-verify",
    `--device-id ${id}`,
    `--request-json ${shellToken(JSON.stringify(request))}`,
    `--response-json ${shellToken(approvalResponseJson(response))}`,
  ].join(" ");
}

export function liveHelloMessage(identity) {
  const deviceId = identity?.deviceId || "";
  const noisePubkeyHex = identity?.noisePubkeyHex || "";
  const approvalPubkeyHex = identity?.approvalPubkeyHex || "";
  if (
    !/^[A-Za-z0-9._:-]+$/.test(deviceId) ||
    !/^[0-9a-f]{64}$/i.test(noisePubkeyHex) ||
    !/^[0-9a-f]{64}$/i.test(approvalPubkeyHex)
  ) {
    throw new Error("companion identity 형식 오류");
  }
  return {
    type: "hello",
    protocol_version: LIVE_TRANSPORT_PROTOCOL_VERSION,
    device_id: deviceId,
    noise_pubkey_hex: noisePubkeyHex,
    approval_pubkey_hex: approvalPubkeyHex,
  };
}

export function liveApprovalRequestMessage(request) {
  validateApprovalRequest(request);
  return { type: "approval_request", request };
}

export function liveApprovalResponseMessage(response) {
  validateApprovalResponse(response);
  return { type: "approval_response", response };
}

export function livePingMessage(nonce) {
  if (typeof nonce !== "string" || nonce.length === 0 || nonce.length > 128) {
    throw new Error("heartbeat nonce 형식 오류");
  }
  return { type: "ping", nonce };
}

export function livePongMessage(nonce) {
  if (typeof nonce !== "string" || nonce.length === 0 || nonce.length > 128) {
    throw new Error("heartbeat nonce 형식 오류");
  }
  return { type: "pong", nonce };
}

export function liveErrorMessage(message) {
  if (typeof message !== "string" || message.trim().length === 0) {
    throw new Error("error message 형식 오류");
  }
  return { type: "error", message };
}

export function liveTransportJson(message) {
  validateLiveTransportMessage(message);
  return JSON.stringify(message);
}

export function validRelaySessionId(value) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_RELAY_SESSION_ID_LENGTH &&
    /^[A-Za-z0-9._:-]+$/.test(value)
  );
}

export function validRelaySender(value) {
  return value === "daemon" || value === "companion";
}

export function validRelaySessionToken(value) {
  return (
    typeof value === "string" &&
    value.length >= MIN_RELAY_SESSION_TOKEN_LENGTH &&
    value.length <= MAX_RELAY_SESSION_TOKEN_LENGTH &&
    /^[A-Za-z0-9._:~-]+$/.test(value)
  );
}

export function validRelayDeviceId(value) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= MAX_RELAY_DEVICE_ID_LENGTH &&
    /^[A-Za-z0-9._:-]+$/.test(value)
  );
}

function validRelayPubkeyHex(value) {
  return typeof value === "string" && /^[0-9a-f]{64}$/i.test(value);
}

export function createRelaySessionTicket({
  sessionId,
  sessionToken,
  issuedAtMs,
  expiresAtMs = issuedAtMs + DEFAULT_RELAY_SESSION_TTL_MS,
  daemonPubkeyHex,
  companionDeviceId,
  companionNoisePubkeyHex,
  companionApprovalPubkeyHex,
  transport = "websocket",
}) {
  const ticket = {
    relay_protocol_version: RELAY_TRANSPORT_PROTOCOL_VERSION,
    transport,
    session_id: sessionId,
    session_token: sessionToken,
    issued_at_ms: issuedAtMs,
    expires_at_ms: expiresAtMs,
    daemon_pubkey_hex: daemonPubkeyHex,
    companion_device_id: companionDeviceId,
    companion_noise_pubkey_hex: companionNoisePubkeyHex,
    companion_approval_pubkey_hex: companionApprovalPubkeyHex,
  };
  validateRelaySessionTicket(ticket);
  return ticket;
}

export function validateRelaySessionTicket(ticket) {
  if (ticket?.relay_protocol_version !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay session protocol_version");
  }
  if (ticket.transport !== "websocket") {
    throw new Error("relay session transport 형식 오류");
  }
  if (!validRelaySessionId(ticket.session_id)) {
    throw new Error("relay session_id 형식 오류");
  }
  if (!validRelaySessionToken(ticket.session_token)) {
    throw new Error("relay session_token 형식 오류");
  }
  if (
    !Number.isSafeInteger(ticket.issued_at_ms) ||
    ticket.issued_at_ms <= 0 ||
    !Number.isSafeInteger(ticket.expires_at_ms) ||
    ticket.expires_at_ms <= ticket.issued_at_ms ||
    ticket.expires_at_ms - ticket.issued_at_ms > DEFAULT_RELAY_SESSION_TTL_MS
  ) {
    throw new Error("relay session expiry 형식 오류");
  }
  if (!validRelayPubkeyHex(ticket.daemon_pubkey_hex)) {
    throw new Error("relay daemon_pubkey_hex 형식 오류");
  }
  if (!validRelayDeviceId(ticket.companion_device_id)) {
    throw new Error("relay companion device_id 형식 오류");
  }
  if (!validRelayPubkeyHex(ticket.companion_noise_pubkey_hex)) {
    throw new Error("relay companion noise_pubkey_hex 형식 오류");
  }
  if (!validRelayPubkeyHex(ticket.companion_approval_pubkey_hex)) {
    throw new Error("relay companion approval_pubkey_hex 형식 오류");
  }
}

export function relaySessionExpiredAt(ticket, nowMs) {
  validateRelaySessionTicket(ticket);
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("relay session now_ms 형식 오류");
  }
  return nowMs >= ticket.expires_at_ms;
}

export function relaySessionConnect(ticket, peer) {
  validateRelaySessionTicket(ticket);
  const connect =
    peer === "daemon"
      ? {
          relay_protocol_version: RELAY_TRANSPORT_PROTOCOL_VERSION,
          session_id: ticket.session_id,
          peer,
          session_token: ticket.session_token,
          daemon_pubkey_hex: ticket.daemon_pubkey_hex,
        }
      : {
          relay_protocol_version: RELAY_TRANSPORT_PROTOCOL_VERSION,
          session_id: ticket.session_id,
          peer,
          session_token: ticket.session_token,
          device_id: ticket.companion_device_id,
          noise_pubkey_hex: ticket.companion_noise_pubkey_hex,
          approval_pubkey_hex: ticket.companion_approval_pubkey_hex,
        };
  validateRelaySessionConnectMetadata(connect);
  return connect;
}

export function relaySessionConnectJson(connect) {
  validateRelaySessionConnectMetadata(connect);
  return JSON.stringify(connect);
}

export function validateRelaySessionConnect(ticket, connect, nowMs) {
  validateRelaySessionTicket(ticket);
  validateRelaySessionConnectMetadata(connect);
  if (relaySessionExpiredAt(ticket, nowMs)) {
    throw new Error("relay session expired");
  }
  if (connect.session_id !== ticket.session_id) {
    throw new Error("relay session_id mismatch");
  }
  if (connect.session_token !== ticket.session_token) {
    throw new Error("relay session_token mismatch");
  }
  if (connect.peer === "daemon") {
    if (connect.daemon_pubkey_hex !== ticket.daemon_pubkey_hex) {
      throw new Error("relay daemon pubkey mismatch");
    }
    return;
  }
  if (connect.device_id !== ticket.companion_device_id) {
    throw new Error("relay companion device_id mismatch");
  }
  if (connect.noise_pubkey_hex !== ticket.companion_noise_pubkey_hex) {
    throw new Error("relay companion noise pubkey mismatch");
  }
  if (connect.approval_pubkey_hex !== ticket.companion_approval_pubkey_hex) {
    throw new Error("relay companion approval pubkey mismatch");
  }
}

export function relaySessionTicketSigningPayload(ticket) {
  validateRelaySessionTicket(ticket);
  return [
    "ai-terminal-relay-ticket-v1",
    `relay_protocol_version=${ticket.relay_protocol_version}`,
    `transport=${ticket.transport}`,
    `session_id=${ticket.session_id}`,
    `session_token=${ticket.session_token}`,
    `issued_at_ms=${ticket.issued_at_ms}`,
    `expires_at_ms=${ticket.expires_at_ms}`,
    `daemon_pubkey_hex=${ticket.daemon_pubkey_hex}`,
    `companion_device_id=${ticket.companion_device_id}`,
    `companion_noise_pubkey_hex=${ticket.companion_noise_pubkey_hex}`,
    `companion_approval_pubkey_hex=${ticket.companion_approval_pubkey_hex}`,
    "",
  ].join("\n");
}

export async function relaySessionTicketHmacSha256Hex(
  ticket,
  secret,
  webCrypto = globalThis.crypto,
) {
  validateRelaySessionTicket(ticket);
  const secretBytes = relayTicketHmacKeyBytes(secret);
  const key = await webCrypto.subtle.importKey(
    "raw",
    secretBytes,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const payload = new TextEncoder().encode(relaySessionTicketSigningPayload(ticket));
  const mac = await webCrypto.subtle.sign("HMAC", key, payload);
  return bytesToHex(new Uint8Array(mac));
}

export async function createSignedRelaySessionTicket(
  ticket,
  secret,
  webCrypto = globalThis.crypto,
) {
  const signed = {
    ticket,
    mac_alg: RELAY_TICKET_MAC_ALG_HMAC_SHA256,
    mac_hex: await relaySessionTicketHmacSha256Hex(ticket, secret, webCrypto),
  };
  validateSignedRelaySessionTicketMetadata(signed);
  return signed;
}

export async function relaySessionTicketEd25519SignatureHex(
  ticket,
  signingKeyMaterial,
  webCrypto = globalThis.crypto,
) {
  validateRelaySessionTicket(ticket);
  if (!signingKeyMaterial?.approval?.privateKey) {
    throw new Error("relay ticket ed25519 private key 없음");
  }
  const payload = new TextEncoder().encode(relaySessionTicketSigningPayload(ticket));
  const signature = await webCrypto.subtle.sign(
    { name: "Ed25519" },
    signingKeyMaterial.approval.privateKey,
    payload,
  );
  return bytesToHex(new Uint8Array(signature));
}

export async function createEd25519SignedRelaySessionTicket(
  ticket,
  signingKeyMaterial,
  options = {},
  webCrypto = globalThis.crypto,
) {
  const { keyId = "", keyVersion = 1 } = options || {};
  const signed = {
    ticket,
    mac_alg: RELAY_TICKET_MAC_ALG_ED25519,
    mac_hex: await relaySessionTicketEd25519SignatureHex(ticket, signingKeyMaterial, webCrypto),
    key_id: keyId,
    key_version: keyVersion,
  };
  validateSignedRelaySessionTicketMetadata(signed);
  return signed;
}

export function validateSignedRelaySessionTicketMetadata(signed) {
  validateRelaySessionTicket(signed?.ticket);
  if (![RELAY_TICKET_MAC_ALG_HMAC_SHA256, RELAY_TICKET_MAC_ALG_ED25519].includes(signed.mac_alg)) {
    throw new Error("relay ticket mac_alg 형식 오류");
  }
  const expectedMacHexLength = signed.mac_alg === RELAY_TICKET_MAC_ALG_ED25519 ? 128 : 64;
  if (
    typeof signed.mac_hex !== "string" ||
    !new RegExp(`^[0-9a-f]{${expectedMacHexLength}}$`, "i").test(signed.mac_hex)
  ) {
    throw new Error("relay ticket mac_hex 형식 오류");
  }
  if (signed.key_id !== undefined && !validRelayTicketKeyId(signed.key_id)) {
    throw new Error("relay ticket key_id 형식 오류");
  }
  if (signed.key_version !== undefined && !validRelayTicketKeyVersion(signed.key_version)) {
    throw new Error("relay ticket key_version 형식 오류");
  }
}

export async function validateSignedRelaySessionTicket(
  signed,
  secret,
  webCrypto = globalThis.crypto,
) {
  validateSignedRelaySessionTicketMetadata(signed);
  const expected = await relaySessionTicketHmacSha256Hex(signed.ticket, secret, webCrypto);
  if (!constantTimeHexEqual(signed.mac_hex, expected)) {
    throw new Error("relay ticket mac mismatch");
  }
  return signed.ticket;
}

export async function validateSignedRelaySessionConnect(
  signed,
  connect,
  nowMs,
  secret,
  webCrypto = globalThis.crypto,
) {
  const ticket = await validateSignedRelaySessionTicket(signed, secret, webCrypto);
  validateRelaySessionConnect(ticket, connect, nowMs);
}

export function createManagedRelayPublicVerifierKeyRegistry(entries = []) {
  if (!Array.isArray(entries)) {
    throw new Error("managed relay verifier registry entries 형식 오류");
  }
  const normalized = entries.map((entry) => {
    validateManagedRelayPublicVerifierKeyEntry(entry);
    return { ...entry };
  });
  const seen = new Set();
  for (const entry of normalized) {
    const key = managedRelayVerifierRegistryKey(entry.tenant_id, entry.key_id, entry.key_version);
    if (seen.has(key)) {
      throw new Error("managed relay verifier registry duplicate key");
    }
    seen.add(key);
  }
  const json = JSON.stringify(normalized);
  for (const prohibited of ["private_signing_key", "hmac_secret", "secret", "private_key_material"]) {
    if (json.includes(prohibited)) {
      throw new Error("managed relay verifier registry contains prohibited key material");
    }
  }
  return { entries: normalized };
}

export function lookupManagedRelayPublicVerifierKey(registry, query = {}, nowMs = Date.now()) {
  const { tenantId = "", keyId = "", keyVersion = 0 } = query || {};
  if (!validManagedRelayTenantId(tenantId)) {
    throw new Error("managed relay verifier tenant_id 형식 오류");
  }
  if (!validRelayTicketKeyId(keyId)) {
    throw new Error("managed relay verifier key_id 형식 오류");
  }
  if (!validRelayTicketKeyVersion(keyVersion)) {
    throw new Error("managed relay verifier key_version 형식 오류");
  }
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("managed relay verifier now_ms 형식 오류");
  }
  const entries = Array.isArray(registry?.entries) ? registry.entries : [];
  const entry = entries.find(
    (candidate) =>
      candidate.tenant_id === tenantId &&
      candidate.key_id === keyId &&
      candidate.key_version === keyVersion,
  );
  if (!entry) {
    throw new Error("managed relay verifier key missing");
  }
  validateManagedRelayPublicVerifierKeyEntry(entry);
  if (!["active", "rotating"].includes(entry.state)) {
    throw new Error("managed relay verifier key inactive");
  }
  if (nowMs < entry.not_before_ms || nowMs >= entry.expires_at_ms) {
    throw new Error("managed relay verifier key outside validity window");
  }
  return { ...entry };
}

export async function validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
  signed,
  registry,
  options = {},
  webCrypto = globalThis.crypto,
) {
  const { tenantId = "", nowMs = Date.now() } = options || {};
  validateSignedRelaySessionTicketMetadata(signed);
  if (signed.mac_alg !== RELAY_TICKET_MAC_ALG_ED25519) {
    throw new Error("managed relay verifier requires ed25519 ticket");
  }
  if (!validRelayTicketKeyId(signed.key_id)) {
    throw new Error("managed relay verifier signed key_id required");
  }
  if (!validRelayTicketKeyVersion(signed.key_version)) {
    throw new Error("managed relay verifier signed key_version required");
  }
  if (relaySessionExpiredAt(signed.ticket, nowMs)) {
    throw new Error("managed relay verifier ticket expired");
  }
  const verifier = lookupManagedRelayPublicVerifierKey(
    registry,
    {
      tenantId,
      keyId: signed.key_id,
      keyVersion: signed.key_version,
    },
    nowMs,
  );
  const publicKey = await webCrypto.subtle.importKey(
    "raw",
    hexToBytes(verifier.public_key_hex),
    { name: "Ed25519" },
    false,
    ["verify"],
  );
  const verified = await webCrypto.subtle.verify(
    { name: "Ed25519" },
    publicKey,
    hexToBytes(signed.mac_hex),
    new TextEncoder().encode(relaySessionTicketSigningPayload(signed.ticket)),
  );
  if (!verified) {
    throw new Error("managed relay verifier signature mismatch");
  }
  return signed.ticket;
}

export function createManagedRelayPublicVerifierKeyRegistrySnapshot(registry, options = {}) {
  const {
    snapshotId = "",
    effectiveAtMs = 0,
    previousSnapshotId,
    reason = "rotation-propagation",
  } = options || {};
  if (!validManagedRelayRegistrySnapshotId(snapshotId)) {
    throw new Error("managed relay verifier registry snapshot_id 형식 오류");
  }
  if (!Number.isSafeInteger(effectiveAtMs) || effectiveAtMs <= 0) {
    throw new Error("managed relay verifier registry effective_at_ms 형식 오류");
  }
  if (
    previousSnapshotId !== undefined &&
    !validManagedRelayRegistrySnapshotId(previousSnapshotId)
  ) {
    throw new Error("managed relay verifier registry previous_snapshot_id 형식 오류");
  }
  if (!validManagedRelayRegistrySnapshotReason(reason)) {
    throw new Error("managed relay verifier registry snapshot reason 형식 오류");
  }
  const entries = Array.isArray(registry) ? registry : registry?.entries;
  const normalizedRegistry = createManagedRelayPublicVerifierKeyRegistry(entries);
  const snapshot = {
    snapshot_id: snapshotId,
    effective_at_ms: effectiveAtMs,
    reason,
    entries: normalizedRegistry.entries,
  };
  if (previousSnapshotId !== undefined) {
    snapshot.previous_snapshot_id = previousSnapshotId;
  }
  return snapshot;
}

export function lookupManagedRelayPublicVerifierKeyFromRegistrySnapshot(
  snapshot,
  query = {},
  nowMs = Date.now(),
) {
  const normalizedSnapshot = validateManagedRelayPublicVerifierKeyRegistrySnapshot(snapshot);
  if (!Number.isSafeInteger(nowMs) || nowMs < normalizedSnapshot.effective_at_ms) {
    throw new Error("managed relay verifier registry snapshot not effective");
  }
  const verifier = lookupManagedRelayPublicVerifierKey(
    { entries: normalizedSnapshot.entries },
    query,
    nowMs,
  );
  return {
    ...verifier,
    registry_snapshot_id: normalizedSnapshot.snapshot_id,
    registry_effective_at_ms: normalizedSnapshot.effective_at_ms,
  };
}

export async function validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot(
  signed,
  snapshot,
  options = {},
  webCrypto = globalThis.crypto,
) {
  const { tenantId = "", nowMs = Date.now() } = options || {};
  validateSignedRelaySessionTicketMetadata(signed);
  const verifier = lookupManagedRelayPublicVerifierKeyFromRegistrySnapshot(
    snapshot,
    {
      tenantId,
      keyId: signed.key_id,
      keyVersion: signed.key_version,
    },
    nowMs,
  );
  const ticket = await validateManagedRelaySignedSessionTicketWithPublicVerifierRegistry(
    signed,
    { entries: snapshot.entries },
    { tenantId, nowMs },
    webCrypto,
  );
  return {
    ticket,
    auditEvent: {
      event_type: "managed-relay-session-ticket-verified",
      tenant_id: tenantId,
      key_id: signed.key_id,
      key_version: signed.key_version,
      key_state: verifier.state,
      registry_snapshot_id: verifier.registry_snapshot_id,
      registry_effective_at_ms: verifier.registry_effective_at_ms,
      decision: "accept",
      at_ms: nowMs,
    },
  };
}

export function createManagedRelayTenantSessionRegistrationQuotaState(state = {}) {
  const {
    tenantId = "",
    windowStartMs = 0,
    windowEndMs = 0,
    registrationLimit = 0,
    registrationsUsed = 0,
    billingMeter = {},
    abuseSignals = {},
  } = state || {};
  if (!validManagedRelayTenantId(tenantId)) {
    throw new Error("managed relay quota tenant_id 형식 오류");
  }
  if (!validManagedRelayQuotaWindow(windowStartMs, windowEndMs)) {
    throw new Error("managed relay quota window 형식 오류");
  }
  if (!validManagedRelayQuotaCount(registrationLimit)) {
    throw new Error("managed relay quota registration_limit 형식 오류");
  }
  if (!validManagedRelayQuotaCount(registrationsUsed)) {
    throw new Error("managed relay quota registrations_used 형식 오류");
  }
  const normalized = {
    tenant_id: tenantId,
    window_start_ms: windowStartMs,
    window_end_ms: windowEndMs,
    registration_limit: registrationLimit,
    registrations_used: registrationsUsed,
    billing_meter: {
      session_registration_count: normalizeManagedRelayQuotaCount(
        billingMeter.session_registration_count,
        registrationsUsed,
      ),
      quota_denial_count: normalizeManagedRelayQuotaCount(billingMeter.quota_denial_count, 0),
    },
    abuse_signals: {
      rate_limit_denial_count: normalizeManagedRelayQuotaCount(
        abuseSignals.rate_limit_denial_count,
        0,
      ),
      invalid_ticket_count: normalizeManagedRelayQuotaCount(abuseSignals.invalid_ticket_count, 0),
    },
  };
  assertManagedRelayQuotaMetadataHasNoSecrets(normalized, "managed relay quota state");
  return normalized;
}

export function evaluateManagedRelayTenantSessionRegistrationQuota(
  quotaState,
  registration = {},
  nowMs = Date.now(),
) {
  const state = createManagedRelayTenantSessionRegistrationQuotaState({
    tenantId: quotaState?.tenant_id,
    windowStartMs: quotaState?.window_start_ms,
    windowEndMs: quotaState?.window_end_ms,
    registrationLimit: quotaState?.registration_limit,
    registrationsUsed: quotaState?.registrations_used,
    billingMeter: quotaState?.billing_meter,
    abuseSignals: quotaState?.abuse_signals,
  });
  const request = validateManagedRelayTenantSessionRegistrationRequest(registration);
  if (request.tenant_id !== state.tenant_id) {
    throw new Error("managed relay quota tenant mismatch");
  }
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("managed relay quota now_ms 형식 오류");
  }
  const quotaRemainingBefore = Math.max(state.registration_limit - state.registrations_used, 0);
  const withinWindow = nowMs >= state.window_start_ms && nowMs < state.window_end_ms;
  const quotaAvailable = state.registrations_used < state.registration_limit;
  const decision = withinWindow && quotaAvailable ? "accept" : "reject";
  const reason = quotaDecisionReason({ withinWindow, quotaAvailable });
  const billingMeterDelta = {
    session_registration_count: decision === "accept" ? 1 : 0,
    quota_denial_count: decision === "reject" ? 1 : 0,
  };
  const abuseSignalDelta = {
    rate_limit_denial_count: 0,
    invalid_ticket_count: 0,
  };
  const auditEvent = {
    event_type: "managed-relay-session-registration-quota-decision",
    tenant_id: state.tenant_id,
    session_id: request.session_id,
    daemon_device_id: request.daemon_device_id,
    verifier_key_id: request.verifier_key_id,
    verifier_key_version: request.verifier_key_version,
    source_ip_hash: request.source_ip_hash,
    quota_scope: "tenant-session-registration",
    quota_limit: state.registration_limit,
    quota_used: state.registrations_used,
    quota_remaining_before_decision: quotaRemainingBefore,
    quota_remaining_after_decision: Math.max(
      quotaRemainingBefore - billingMeterDelta.session_registration_count,
      0,
    ),
    decision,
    reason,
    billing_meter_delta: billingMeterDelta,
    abuse_signal_delta: abuseSignalDelta,
    at_ms: nowMs,
  };
  assertManagedRelayQuotaMetadataHasNoSecrets(auditEvent, "managed relay quota audit");
  return {
    decision,
    registrationAllowed: decision === "accept",
    reason,
    auditEvent,
    billingMeterDelta,
    abuseSignalDelta,
    quotaSnapshot: {
      tenant_id: state.tenant_id,
      window_start_ms: state.window_start_ms,
      window_end_ms: state.window_end_ms,
      registration_limit: state.registration_limit,
      registrations_used: state.registrations_used,
      registrations_remaining: quotaRemainingBefore,
    },
  };
}

export function createManagedRelayActiveSessionAndByteQuotaState(state = {}) {
  const {
    tenantId = "",
    daemonDeviceId = "",
    windowStartMs = 0,
    windowEndMs = 0,
    tenantActiveSessionLimit = 0,
    tenantActiveSessions = 0,
    daemonDeviceActiveSessionLimit = 0,
    daemonDeviceActiveSessions = 0,
    relayFrameLimit = 0,
    relayFramesUsed = 0,
    relayByteLimit = 0,
    relayBytesUsed = 0,
    billingMeter = {},
    abuseSignals = {},
  } = state || {};
  if (!validManagedRelayTenantId(tenantId)) {
    throw new Error("managed relay active quota tenant_id 형식 오류");
  }
  if (!validRelayDeviceId(daemonDeviceId)) {
    throw new Error("managed relay active quota daemon_device_id 형식 오류");
  }
  if (!validManagedRelayQuotaWindow(windowStartMs, windowEndMs)) {
    throw new Error("managed relay active quota window 형식 오류");
  }
  for (const [label, value] of [
    ["tenant_active_session_limit", tenantActiveSessionLimit],
    ["tenant_active_sessions", tenantActiveSessions],
    ["daemon_device_active_session_limit", daemonDeviceActiveSessionLimit],
    ["daemon_device_active_sessions", daemonDeviceActiveSessions],
    ["relay_frame_limit", relayFrameLimit],
    ["relay_frames_used", relayFramesUsed],
    ["relay_byte_limit", relayByteLimit],
    ["relay_bytes_used", relayBytesUsed],
  ]) {
    if (!validManagedRelayQuotaCount(value)) {
      throw new Error(`managed relay active quota ${label} 형식 오류`);
    }
  }
  const normalized = {
    tenant_id: tenantId,
    daemon_device_id: daemonDeviceId,
    window_start_ms: windowStartMs,
    window_end_ms: windowEndMs,
    tenant_active_session_limit: tenantActiveSessionLimit,
    tenant_active_sessions: tenantActiveSessions,
    daemon_device_active_session_limit: daemonDeviceActiveSessionLimit,
    daemon_device_active_sessions: daemonDeviceActiveSessions,
    relay_frame_limit: relayFrameLimit,
    relay_frames_used: relayFramesUsed,
    relay_byte_limit: relayByteLimit,
    relay_bytes_used: relayBytesUsed,
    billing_meter: {
      active_session_count: normalizeManagedRelayQuotaCount(
        billingMeter.active_session_count,
        tenantActiveSessions,
      ),
      relay_frame_count: normalizeManagedRelayQuotaCount(
        billingMeter.relay_frame_count,
        relayFramesUsed,
      ),
      relay_byte_count: normalizeManagedRelayQuotaCount(
        billingMeter.relay_byte_count,
        relayBytesUsed,
      ),
      quota_denial_count: normalizeManagedRelayQuotaCount(billingMeter.quota_denial_count, 0),
    },
    abuse_signals: {
      rate_limit_denial_count: normalizeManagedRelayQuotaCount(
        abuseSignals.rate_limit_denial_count,
        0,
      ),
      invalid_ticket_count: normalizeManagedRelayQuotaCount(abuseSignals.invalid_ticket_count, 0),
    },
  };
  assertManagedRelayQuotaMetadataHasNoSecrets(normalized, "managed relay active quota state");
  return normalized;
}

export function evaluateManagedRelayActiveSessionAndByteQuota(
  quotaState,
  route = {},
  nowMs = Date.now(),
) {
  const state = createManagedRelayActiveSessionAndByteQuotaState({
    tenantId: quotaState?.tenant_id,
    daemonDeviceId: quotaState?.daemon_device_id,
    windowStartMs: quotaState?.window_start_ms,
    windowEndMs: quotaState?.window_end_ms,
    tenantActiveSessionLimit: quotaState?.tenant_active_session_limit,
    tenantActiveSessions: quotaState?.tenant_active_sessions,
    daemonDeviceActiveSessionLimit: quotaState?.daemon_device_active_session_limit,
    daemonDeviceActiveSessions: quotaState?.daemon_device_active_sessions,
    relayFrameLimit: quotaState?.relay_frame_limit,
    relayFramesUsed: quotaState?.relay_frames_used,
    relayByteLimit: quotaState?.relay_byte_limit,
    relayBytesUsed: quotaState?.relay_bytes_used,
    billingMeter: quotaState?.billing_meter,
    abuseSignals: quotaState?.abuse_signals,
  });
  const request = validateManagedRelayActiveSessionAndByteQuotaRequest(route);
  if (request.tenant_id !== state.tenant_id) {
    throw new Error("managed relay active quota tenant mismatch");
  }
  if (request.daemon_device_id !== state.daemon_device_id) {
    throw new Error("managed relay active quota daemon device mismatch");
  }
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("managed relay active quota now_ms 형식 오류");
  }
  const withinWindow = nowMs >= state.window_start_ms && nowMs < state.window_end_ms;
  const tenantActiveSessionAvailable =
    state.tenant_active_sessions < state.tenant_active_session_limit;
  const daemonDeviceActiveSessionAvailable =
    state.daemon_device_active_sessions < state.daemon_device_active_session_limit;
  const relayFrameAvailable = state.relay_frames_used < state.relay_frame_limit;
  const relayByteAvailable =
    state.relay_bytes_used + request.payload_ciphertext_bytes <= state.relay_byte_limit;
  const decision =
    withinWindow &&
    tenantActiveSessionAvailable &&
    daemonDeviceActiveSessionAvailable &&
    relayFrameAvailable &&
    relayByteAvailable
      ? "accept"
      : "reject";
  const reason = activeSessionAndByteQuotaDecisionReason({
    withinWindow,
    tenantActiveSessionAvailable,
    daemonDeviceActiveSessionAvailable,
    relayFrameAvailable,
    relayByteAvailable,
  });
  const billingMeterDelta = {
    active_session_count: decision === "accept" ? 1 : 0,
    relay_frame_count: decision === "accept" ? 1 : 0,
    relay_byte_count: decision === "accept" ? request.payload_ciphertext_bytes : 0,
    quota_denial_count: decision === "reject" ? 1 : 0,
  };
  const abuseSignalDelta = {
    rate_limit_denial_count: 0,
    invalid_ticket_count: 0,
  };
  const auditEvent = {
    event_type: "managed-relay-active-session-and-byte-quota-decision",
    tenant_id: state.tenant_id,
    session_id: request.session_id,
    daemon_device_id: request.daemon_device_id,
    verifier_key_id: request.verifier_key_id,
    verifier_key_version: request.verifier_key_version,
    frame_sequence: request.frame_sequence,
    payload_ciphertext_bytes: request.payload_ciphertext_bytes,
    quota_scope: "tenant-daemon-active-session-frame-byte",
    tenant_active_session_limit: state.tenant_active_session_limit,
    tenant_active_sessions: state.tenant_active_sessions,
    daemon_device_active_session_limit: state.daemon_device_active_session_limit,
    daemon_device_active_sessions: state.daemon_device_active_sessions,
    relay_frame_limit: state.relay_frame_limit,
    relay_frames_used: state.relay_frames_used,
    relay_byte_limit: state.relay_byte_limit,
    relay_bytes_used: state.relay_bytes_used,
    relay_bytes_after_decision: state.relay_bytes_used + billingMeterDelta.relay_byte_count,
    decision,
    reason,
    billing_meter_delta: billingMeterDelta,
    abuse_signal_delta: abuseSignalDelta,
    at_ms: nowMs,
  };
  assertManagedRelayQuotaMetadataHasNoSecrets(auditEvent, "managed relay active quota audit");
  return {
    decision,
    relayAllowed: decision === "accept",
    reason,
    auditEvent,
    billingMeterDelta,
    abuseSignalDelta,
    quotaSnapshot: {
      tenant_id: state.tenant_id,
      daemon_device_id: state.daemon_device_id,
      window_start_ms: state.window_start_ms,
      window_end_ms: state.window_end_ms,
      tenant_active_session_limit: state.tenant_active_session_limit,
      tenant_active_sessions: state.tenant_active_sessions,
      daemon_device_active_session_limit: state.daemon_device_active_session_limit,
      daemon_device_active_sessions: state.daemon_device_active_sessions,
      relay_frame_limit: state.relay_frame_limit,
      relay_frames_used: state.relay_frames_used,
      relay_byte_limit: state.relay_byte_limit,
      relay_bytes_used: state.relay_bytes_used,
    },
  };
}

export function createManagedRelayTenantAggregateUsageExport(input = {}) {
  const {
    tenantId = "",
    windowStartMs = 0,
    windowEndMs = 0,
    generatedAtMs = 0,
    planId = "managed-relay-default",
    billingMeter = {},
    abuseSignals = {},
  } = input || {};
  if (!validManagedRelayTenantId(tenantId)) {
    throw new Error("managed relay tenant usage export tenant_id 형식 오류");
  }
  if (!validManagedRelayQuotaWindow(windowStartMs, windowEndMs)) {
    throw new Error("managed relay tenant usage export window 형식 오류");
  }
  if (!Number.isSafeInteger(generatedAtMs) || generatedAtMs <= 0) {
    throw new Error("managed relay tenant usage export generated_at_ms 형식 오류");
  }
  if (!validRelayTicketKeyId(planId)) {
    throw new Error("managed relay tenant usage export plan_id 형식 오류");
  }

  const billingUsage = {
    session_registration_count: normalizeManagedRelayQuotaCount(
      billingMeter.session_registration_count,
      0,
    ),
    active_session_count: normalizeManagedRelayQuotaCount(
      billingMeter.active_session_count,
      0,
    ),
    relay_frame_count: normalizeManagedRelayQuotaCount(billingMeter.relay_frame_count, 0),
    relay_byte_count: normalizeManagedRelayQuotaCount(billingMeter.relay_byte_count, 0),
    invalid_ticket_count: normalizeManagedRelayQuotaCount(
      billingMeter.invalid_ticket_count,
      0,
    ),
    quota_denial_count: normalizeManagedRelayQuotaCount(billingMeter.quota_denial_count, 0),
  };
  const abuseSignalSummary = {
    rate_limit_denial_count: normalizeManagedRelayQuotaCount(
      abuseSignals.rate_limit_denial_count,
      0,
    ),
    invalid_ticket_count: normalizeManagedRelayQuotaCount(abuseSignals.invalid_ticket_count, 0),
    abuse_case_count: normalizeManagedRelayQuotaCount(abuseSignals.abuse_case_count, 0),
  };
  const usageExport = {
    export_version: 1,
    export_scope: "tenant-aggregate-usage",
    tenant_id: tenantId,
    plan_id: planId,
    window_start_ms: windowStartMs,
    window_end_ms: windowEndMs,
    generated_at_ms: generatedAtMs,
    payload_visibility: "payload-free",
    support_visibility: "aggregate-only",
    billing_usage: billingUsage,
    abuse_signal_summary: abuseSignalSummary,
    billing_abuse_boundary: {
      billing_usage_fields: Object.keys(billingUsage),
      abuse_signal_fields: Object.keys(abuseSignalSummary),
      abuse_signals_are_not_billing_meters: true,
    },
  };
  assertManagedRelayQuotaMetadataHasNoSecrets(input, "managed relay tenant usage export input");
  assertManagedRelayQuotaMetadataHasNoSecrets(
    usageExport,
    "managed relay tenant usage export",
  );
  return usageExport;
}

export function relayDeploymentShapeDecision() {
  return {
    ...PWA_RELAY_DEPLOYMENT_DECISION,
    knownModes: [...PWA_RELAY_DEPLOYMENT_MODES],
    deferredModes: [...PWA_RELAY_DEPLOYMENT_DECISION.deferredModes],
    guardrails: [
      "product_default_remains_live_loopback",
      "relay_ui_requires_selected_self_hosted_mode",
      "production_endpoint_requires_wss",
      "localhost_ws_is_development_only",
      "ticket_hmac_secret_stays_daemon_owned",
      "hosted_relay_prefers_public_verifier_keys",
      "self_hosted_relay_operator_trust_required",
    ],
  };
}

export function relayPrivateNetworkSetupContract() {
  return {
    ...PWA_RELAY_PRIVATE_NETWORK_SETUP_CONTRACT,
    requiredSetupFields: [...PWA_RELAY_PRIVATE_NETWORK_SETUP_CONTRACT.requiredSetupFields],
    guardrails: [...PWA_RELAY_PRIVATE_NETWORK_SETUP_CONTRACT.guardrails],
    endpointExamples: [
      "wss://relay.tailnet.example/relay",
      "wss://relay.private.example/relay",
      "ws://127.0.0.1:8080/relay",
    ],
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedOperationsPlan() {
  return {
    ...PWA_RELAY_MANAGED_OPERATIONS_PLAN,
    requiredBeforeImplementation: [
      ...PWA_RELAY_MANAGED_OPERATIONS_PLAN.requiredBeforeImplementation,
    ],
    guardrails: [...PWA_RELAY_MANAGED_OPERATIONS_PLAN.guardrails],
    operationAreas: [
      "control-plane",
      "tenant-isolation",
      "abuse-and-rate-limits",
      "support-and-incident-response",
      "retention-and-observability",
      "billing-and-quotas",
      "verifier-key-distribution",
      "payload-confidentiality",
    ],
    completedOperationContracts: [
      "control-plane-ownership",
      "tenant-isolation",
      "abuse-handling",
      "support-workflows",
      "retention-policy",
      "payload-confidentiality-plan",
      "public-verifier-key-operations",
      "billing-and-quota-policy",
    ],
    remainingOperationContracts: [],
    blockers: [],
    implementationStatus: "operations-contract-ready-runtime-still-deferred",
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedControlPlaneContract() {
  return {
    ...PWA_RELAY_MANAGED_CONTROL_PLANE_CONTRACT,
    requiredRoles: [...PWA_RELAY_MANAGED_CONTROL_PLANE_CONTRACT.requiredRoles],
    requiredContracts: [...PWA_RELAY_MANAGED_CONTROL_PLANE_CONTRACT.requiredContracts],
    prohibitedControlPlaneData: [
      ...PWA_RELAY_MANAGED_CONTROL_PLANE_CONTRACT.prohibitedControlPlaneData,
    ],
    guardrails: [...PWA_RELAY_MANAGED_CONTROL_PLANE_CONTRACT.guardrails],
    responsibilities: {
      serviceOperator: [
        "operate-relay-control-plane",
        "publish-verifier-key-policy",
        "respond-to-abuse-and-incidents",
      ],
      tenantAdmin: [
        "own-tenant-membership",
        "rotate-tenant-verifier-keys",
        "review-tenant-usage",
      ],
      daemonOwner: [
        "own-registered-device-state",
        "issue-session-tickets",
        "validate-approval-responses",
      ],
      supportOperator: [
        "use-audited-breakglass-only",
        "view-aggregate-state-only",
        "never-view-payload-json-or-secrets",
      ],
    },
    blockers: [
      "control_plane_owner_missing",
      "tenant_identity_contract_missing",
      "session_registration_contract_missing",
      "verifier_key_distribution_contract_missing",
      "support_audit_boundary_missing",
    ],
    completedFollowupContracts: [
      "abuse-retention-policy",
      "payload-confidentiality-plan",
      "public-verifier-key-operations",
      "billing-and-quota-policy",
    ],
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedAbuseRetentionPolicy() {
  return {
    ...PWA_RELAY_MANAGED_ABUSE_RETENTION_POLICY,
    rateLimitScopes: [...PWA_RELAY_MANAGED_ABUSE_RETENTION_POLICY.rateLimitScopes],
    abuseSignals: [...PWA_RELAY_MANAGED_ABUSE_RETENTION_POLICY.abuseSignals],
    deletionRequirements: [
      ...PWA_RELAY_MANAGED_ABUSE_RETENTION_POLICY.deletionRequirements,
    ],
    supportWorkflowConstraints: [
      ...PWA_RELAY_MANAGED_ABUSE_RETENTION_POLICY.supportWorkflowConstraints,
    ],
    guardrails: [...PWA_RELAY_MANAGED_ABUSE_RETENTION_POLICY.guardrails],
    retentionWindows: {
      healthAggregatesDays: 30,
      controlPlaneAuditDays: 90,
      abuseCaseMetadataDays: 180,
      supportCaseMetadataDays: 90,
      payloadJson: "not-retained",
      sessionTokens: "not-retained",
      approvalSignatures: "not-retained",
      privateKeyMaterial: "not-retained",
      hmacSecrets: "not-retained",
      fullSetupJson: "not-retained",
    },
    requiredBeforeRuntime: [
      "rate-limit-enforcement",
      "abuse-escalation-runbook",
      "tenant-deletion-workflow",
      "support-access-review",
      "audit-retention-store",
      "payload-confidentiality-plan",
    ],
    completedFollowupContracts: [
      "payload-confidentiality-plan",
    ],
    blockers: [
      "runtime_rate_limit_enforcement_missing",
      "abuse_escalation_runbook_missing",
      "tenant_deletion_workflow_missing",
      "support_access_review_missing",
    ],
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedPayloadConfidentialityPlan() {
  return {
    ...PWA_RELAY_MANAGED_PAYLOAD_CONFIDENTIALITY_PLAN,
    prohibitedManagedRelayData: [
      ...PWA_RELAY_MANAGED_PAYLOAD_CONFIDENTIALITY_PLAN.prohibitedManagedRelayData,
    ],
    allowedRelayMetadata: [
      ...PWA_RELAY_MANAGED_PAYLOAD_CONFIDENTIALITY_PLAN.allowedRelayMetadata,
    ],
    requiredBeforeRuntime: [
      ...PWA_RELAY_MANAGED_PAYLOAD_CONFIDENTIALITY_PLAN.requiredBeforeRuntime,
    ],
    guardrails: [...PWA_RELAY_MANAGED_PAYLOAD_CONFIDENTIALITY_PLAN.guardrails],
    confidentialityRequirements: [
      "encrypt-live-transport-payload-before-relay-frame",
      "relay-service-routes-opaque-ciphertext-only",
      "approval-request-command-context-remain-client-visible-only",
      "approval-response-remains-client-signed-and-opaque-to-relay",
      "support-exports-redact-ciphertext-and-metadata-identifiers",
      "no-operator-breakglass-to-decrypt-payloads",
    ],
    designDecisions: {
      selfHostedRelay: "explicit-operator-trust-is-acceptable-for-debug-setup",
      privateNetworkRelay: "explicit-operator-trust-is-acceptable-for-advanced-setup",
      managedRelay: "payload-blind-end-to-end-confidentiality-required",
      productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
    },
    implementationBlockers: [
      "e2e_payload_encryption_missing",
      "client_key_agreement_missing",
      "metadata_minimization_review_missing",
      "confidentiality_smoke_missing",
      "support_redaction_evidence_missing",
    ],
    completedFollowupContracts: [
      "public-verifier-key-operations",
      "billing-and-quota-policy",
    ],
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedVerifierKeyOperationsPolicy() {
  return {
    ...PWA_RELAY_MANAGED_VERIFIER_KEY_OPERATIONS_POLICY,
    requiredKeyStates: [
      ...PWA_RELAY_MANAGED_VERIFIER_KEY_OPERATIONS_POLICY.requiredKeyStates,
    ],
    requiredKeyOperations: [
      ...PWA_RELAY_MANAGED_VERIFIER_KEY_OPERATIONS_POLICY.requiredKeyOperations,
    ],
    prohibitedVerifierKeyData: [
      ...PWA_RELAY_MANAGED_VERIFIER_KEY_OPERATIONS_POLICY.prohibitedVerifierKeyData,
    ],
    guardrails: [...PWA_RELAY_MANAGED_VERIFIER_KEY_OPERATIONS_POLICY.guardrails],
    keyRotationRequirements: {
      overlapWindowHours: 24,
      maxActiveKeysPerTenant: 2,
      keyIdRequired: true,
      keyVersionRequired: true,
      revokedKeyRegistration: "fail-closed",
      oldKeyRetirement: "no-new-sessions-after-retirement",
    },
    distributionRequirements: [
      "publish-public-verifier-key-by-tenant-and-key-id",
      "pin-ticket-key-id-and-version",
      "validate-session-ticket-against-active-key-version",
      "remove-private-key-material-from-service-config",
      "record-key-version-audit-events",
      "propagate-revocation-before-runtime",
    ],
    trustBoundaries: {
      tenantAdmin: "owns-key-registration-rotation-and-revocation",
      daemonOwner: "issues-session-tickets-with-current-key-id-version",
      managedRelayService: "verifies-public-keys-only",
      supportOperator: "sees-key-id-version-state-only",
    },
    implementationBlockers: [
      "managed_key_registry_runtime_missing",
      "key_revocation_propagation_smoke_missing",
      "rotation_overlap_smoke_missing",
    ],
    completedFollowupContracts: [
      "billing-and-quota-policy",
    ],
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedBillingQuotaPolicy() {
  return {
    ...PWA_RELAY_MANAGED_BILLING_QUOTA_POLICY,
    requiredQuotaScopes: [
      ...PWA_RELAY_MANAGED_BILLING_QUOTA_POLICY.requiredQuotaScopes,
    ],
    meteredUsageDimensions: [
      ...PWA_RELAY_MANAGED_BILLING_QUOTA_POLICY.meteredUsageDimensions,
    ],
    prohibitedBillingData: [
      ...PWA_RELAY_MANAGED_BILLING_QUOTA_POLICY.prohibitedBillingData,
    ],
    guardrails: [...PWA_RELAY_MANAGED_BILLING_QUOTA_POLICY.guardrails],
    quotaDefaults: {
      tenantSessionRegistrationsPerHour: 1000,
      activeSessionsPerTenant: 100,
      activeSessionsPerDaemonDevice: 10,
      relayFrameBytesPerSession: 50 * 1024 * 1024,
      invalidTicketsPerTenantPerHour: 100,
      quotaExceededBehavior: "reject-new-session-or-frame",
    },
    enforcementRequirements: [
      "enforce-tenant-session-registration-quota",
      "enforce-active-session-quota",
      "enforce-frame-and-byte-quota",
      "record-quota-denial-audit-event",
      "separate-abuse-rate-limits-from-billing-meters",
      "export-tenant-aggregate-usage-without-payloads",
    ],
    retentionRequirements: [
      "usage-rollups-retained-400-days",
      "raw-control-plane-meter-events-retained-90-days",
      "quota-denial-audit-retained-90-days",
      "payload-and-secret-data-not-retained",
    ],
    trustBoundaries: {
      tenantAdmin: "reviews-tenant-usage-and-configures-plan-limits",
      serviceOperator: "enforces-aggregate-quotas-without-payload-access",
      supportOperator: "sees-tenant-aggregate-usage-only",
      managedRelayService: "meters-routing-events-and-quota-denials-only",
    },
    implementationBlockers: [
      "managed_usage_meter_runtime_missing",
      "quota_enforcement_smoke_missing",
      "tenant_usage_export_smoke_missing",
      "billing_abuse_boundary_review_missing",
    ],
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedRuntimeReadinessGate() {
  const operationsPlan = relayManagedOperationsPlan();
  const payloadPlan = relayManagedPayloadConfidentialityPlan();
  const verifierKeyPolicy = relayManagedVerifierKeyOperationsPolicy();
  const billingQuotaPolicy = relayManagedBillingQuotaPolicy();
  const abuseRetentionPolicy = relayManagedAbuseRetentionPolicy();
  const completedRuntimeEvidence = [
    "payload-blind-frame-encryption-smoke",
    "client-key-agreement-runtime-smoke",
    "metadata-minimization-review",
    "public-verifier-key-registry-runtime-smoke",
    "revocation-and-rotation-propagation-smoke",
    "tenant-session-registration-quota-smoke",
    "active-session-and-byte-quota-smoke",
    "tenant-aggregate-usage-export-smoke",
  ];
  const resolvedRuntimeBlockers = [
    "e2e_payload_encryption_missing",
    "client_key_agreement_missing",
    "metadata_minimization_review_missing",
    "confidentiality_smoke_missing",
    "managed_key_registry_runtime_missing",
    "key_revocation_propagation_smoke_missing",
    "rotation_overlap_smoke_missing",
    "quota_enforcement_smoke_missing",
    "managed_usage_meter_runtime_missing",
    "tenant_usage_export_smoke_missing",
  ];
  const auditedRuntimeBlockers = [
    ...payloadPlan.implementationBlockers,
    ...verifierKeyPolicy.implementationBlockers,
    ...billingQuotaPolicy.implementationBlockers,
    ...abuseRetentionPolicy.blockers,
  ];

  return {
    ...PWA_RELAY_MANAGED_RUNTIME_READINESS_GATE,
    requiredPlanningInputs: [
      ...PWA_RELAY_MANAGED_RUNTIME_READINESS_GATE.requiredPlanningInputs,
    ],
    requiredRuntimeEvidence: [
      ...PWA_RELAY_MANAGED_RUNTIME_READINESS_GATE.requiredRuntimeEvidence,
    ],
    guardrails: [...PWA_RELAY_MANAGED_RUNTIME_READINESS_GATE.guardrails],
    completedPlanningInputs: [...operationsPlan.completedOperationContracts],
    missingPlanningInputs: [...operationsPlan.remainingOperationContracts],
    completedRuntimeEvidence,
    remainingRuntimeEvidence: PWA_RELAY_MANAGED_RUNTIME_READINESS_GATE.requiredRuntimeEvidence.filter(
      (evidence) => !completedRuntimeEvidence.includes(evidence),
    ),
    resolvedRuntimeBlockers,
    auditedRuntimeBlockers,
    remainingRuntimeBlockers: auditedRuntimeBlockers.filter(
      (blocker) => !resolvedRuntimeBlockers.includes(blocker),
    ),
    runtimeReadinessDomains: {
      payloadConfidentiality: {
        blockers: [...payloadPlan.implementationBlockers],
        resolvedBlockers: resolvedRuntimeBlockers,
        evidence: [
          "payload-blind-frame-encryption-smoke",
          "client-key-agreement-runtime-smoke",
          "metadata-minimization-review",
        ],
        completedEvidence: [
          "payload-blind-frame-encryption-smoke",
          "client-key-agreement-runtime-smoke",
          "metadata-minimization-review",
        ],
      },
      verifierKeys: {
        blockers: [...verifierKeyPolicy.implementationBlockers],
        evidence: [
          "public-verifier-key-registry-runtime-smoke",
          "revocation-and-rotation-propagation-smoke",
        ],
        completedEvidence: [
          "public-verifier-key-registry-runtime-smoke",
          "revocation-and-rotation-propagation-smoke",
        ],
      },
      quotaAndUsage: {
        blockers: [...billingQuotaPolicy.implementationBlockers],
        evidence: [
          "tenant-session-registration-quota-smoke",
          "active-session-and-byte-quota-smoke",
          "tenant-aggregate-usage-export-smoke",
        ],
        completedEvidence: [
          "tenant-session-registration-quota-smoke",
          "active-session-and-byte-quota-smoke",
          "tenant-aggregate-usage-export-smoke",
        ],
      },
      abuseRetentionAndSupport: {
        blockers: [...abuseRetentionPolicy.blockers],
        evidence: [
          "support-redaction-and-access-review-evidence",
          "billing-abuse-boundary-review",
        ],
      },
    },
    implementationCanStart: false,
    readinessDecision: "blocked-by-runtime-evidence",
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedPayloadBlindFrameEncryptionSpike() {
  const gate = relayManagedRuntimeReadinessGate();
  return {
    ...PWA_RELAY_MANAGED_PAYLOAD_BLIND_FRAME_ENCRYPTION_SPIKE,
    completedRuntimeEvidence: [
      ...PWA_RELAY_MANAGED_PAYLOAD_BLIND_FRAME_ENCRYPTION_SPIKE.completedRuntimeEvidence,
    ],
    closedReadinessBlockers: [
      ...PWA_RELAY_MANAGED_PAYLOAD_BLIND_FRAME_ENCRYPTION_SPIKE.closedReadinessBlockers,
    ],
    guardrails: [...PWA_RELAY_MANAGED_PAYLOAD_BLIND_FRAME_ENCRYPTION_SPIKE.guardrails],
    frameEnvelope: {
      routeVisibleFields: [
        "relay_protocol_version",
        "session_id",
        "sender",
        "sequence",
        "sent_at_ms",
        "expires_at_ms",
        "payload_ciphertext_alg",
        "payload_key_scope",
        "payload_ciphertext_bytes",
      ],
      encryptedPayloadFields: [
        "payload_nonce_hex",
        "payload_ciphertext_hex",
      ],
      prohibitedManagedFrameFields: [
        "payload_json",
        "command_text",
        "context_json",
        "approval_response_payload",
        "private_key_material",
        "raw_session_token",
        "full_setup_json",
      ],
    },
    smokeEvidence: [
      "encrypted-approval-request-frame-roundtrip",
      "encrypted-approval-response-frame-roundtrip",
      "route-envelope-excludes-payload-json-and-ciphertext",
      "wrong-key-decrypt-fails-closed",
      "aad-metadata-tamper-fails-closed",
    ],
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
    implementationCanStart: false,
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedClientKeyAgreementRuntimeSmoke() {
  const gate = relayManagedRuntimeReadinessGate();
  return {
    ...PWA_RELAY_MANAGED_CLIENT_KEY_AGREEMENT_RUNTIME_SMOKE,
    completedRuntimeEvidence: [
      ...PWA_RELAY_MANAGED_CLIENT_KEY_AGREEMENT_RUNTIME_SMOKE.completedRuntimeEvidence,
    ],
    closedReadinessBlockers: [
      ...PWA_RELAY_MANAGED_CLIENT_KEY_AGREEMENT_RUNTIME_SMOKE.closedReadinessBlockers,
    ],
    guardrails: [...PWA_RELAY_MANAGED_CLIENT_KEY_AGREEMENT_RUNTIME_SMOKE.guardrails],
    keyAgreement: {
      algorithm: MANAGED_RELAY_PAYLOAD_KEY_AGREEMENT_ALG,
      sharedSecretBytes: 32,
      payloadKeyBytes: MANAGED_RELAY_PAYLOAD_KEY_BYTES,
      hkdfHash: MANAGED_RELAY_PAYLOAD_KEY_HKDF_HASH,
      hkdfInfo: MANAGED_RELAY_PAYLOAD_KEY_HKDF_INFO,
      saltFields: [
        "session_id",
      ],
      privateKeyBoundary: "daemon-and-companion-only",
      routeVisibleKeyMaterial: [
        "daemon_noise_pubkey_hex",
        "companion_noise_pubkey_hex",
      ],
      prohibitedRouteKeyMaterial: [
        "daemon_noise_private_key",
        "companion_noise_private_key",
        "payload_key_hex",
        "shared_secret_hex",
      ],
    },
    smokeEvidence: [
      "daemon-and-companion-derive-identical-session-payload-key",
      "different-session-id-derives-different-payload-key",
      "public-route-metadata-cannot-derive-payload-key",
      "derived-key-encrypts-managed-relay-frame",
      "wrong-session-derived-key-fails-decrypt",
    ],
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
    implementationCanStart: false,
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedMetadataMinimizationReview() {
  const gate = relayManagedRuntimeReadinessGate();
  return {
    ...PWA_RELAY_MANAGED_METADATA_MINIMIZATION_REVIEW,
    completedRuntimeEvidence: [
      ...PWA_RELAY_MANAGED_METADATA_MINIMIZATION_REVIEW.completedRuntimeEvidence,
    ],
    closedReadinessBlockers: [
      ...PWA_RELAY_MANAGED_METADATA_MINIMIZATION_REVIEW.closedReadinessBlockers,
    ],
    guardrails: [...PWA_RELAY_MANAGED_METADATA_MINIMIZATION_REVIEW.guardrails],
    metadataSurfaces: {
      routeEnvelope: [
        "relay_protocol_version",
        "session_id",
        "sender",
        "sequence",
        "sent_at_ms",
        "expires_at_ms",
        "payload_ciphertext_alg",
        "payload_key_scope",
        "payload_ciphertext_bytes",
      ],
      controlPlane: [
        "tenant_id",
        "session_id",
        "daemon_device_id_hash",
        "companion_device_id_hash",
        "ticket_key_id",
        "ticket_key_version",
        "session_state",
        "created_at_ms",
        "expires_at_ms",
      ],
      billingUsage: [
        "tenant_id",
        "billing_period",
        "session_registration_count",
        "active_session_count",
        "relay_frame_count",
        "relay_byte_count",
        "invalid_ticket_count",
        "quota_denial_count",
      ],
      supportView: [
        "tenant_id",
        "session_id_hash",
        "daemon_device_id_hash",
        "companion_device_id_hash",
        "aggregate_error_class",
        "quota_state",
        "key_id",
        "key_version",
        "last_seen_at_ms",
      ],
      auditEvent: [
        "tenant_id",
        "event_type",
        "session_id_hash",
        "actor_role",
        "key_id",
        "key_version",
        "occurred_at_ms",
        "aggregate_error_class",
      ],
    },
    prohibitedMetadataFields: [
      "payload_json",
      "command_text",
      "context_json",
      "approval_response_payload",
      "payload_ciphertext_hex",
      "payload_nonce_hex",
      "payload_key_hex",
      "shared_secret_hex",
      "daemon_noise_private_key",
      "companion_noise_private_key",
      "private_key_material",
      "session_token",
      "raw_session_token",
      "signed_session_ticket",
      "full_setup_json",
      "hmac_secret",
      "approval_signature",
    ],
    minimizationEvidence: [
      "route-envelope-field-allowlist-reviewed",
      "control-plane-metadata-allowlist-reviewed",
      "billing-usage-metadata-aggregate-only",
      "support-view-redacts-session-and-device-identifiers",
      "audit-events-exclude-payloads-secrets-and-raw-ciphertext",
    ],
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
    implementationCanStart: false,
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedPublicVerifierKeyRegistryRuntimeSmoke() {
  const gate = relayManagedRuntimeReadinessGate();
  return {
    ...PWA_RELAY_MANAGED_PUBLIC_VERIFIER_KEY_REGISTRY_RUNTIME_SMOKE,
    completedRuntimeEvidence: [
      ...PWA_RELAY_MANAGED_PUBLIC_VERIFIER_KEY_REGISTRY_RUNTIME_SMOKE.completedRuntimeEvidence,
    ],
    closedReadinessBlockers: [
      ...PWA_RELAY_MANAGED_PUBLIC_VERIFIER_KEY_REGISTRY_RUNTIME_SMOKE.closedReadinessBlockers,
    ],
    guardrails: [
      ...PWA_RELAY_MANAGED_PUBLIC_VERIFIER_KEY_REGISTRY_RUNTIME_SMOKE.guardrails,
    ],
    registryContract: {
      lookupFields: [
        "tenant_id",
        "key_id",
        "key_version",
      ],
      storedPublicKeyFields: [
        "tenant_id",
        "key_id",
        "key_version",
        "public_key_alg",
        "public_key_hex",
        "state",
        "not_before_ms",
        "expires_at_ms",
      ],
      acceptedKeyStates: [
        "active",
        "rotating",
      ],
      rejectedKeyStates: [
        "pending",
        "retiring",
        "revoked",
      ],
      prohibitedRegistryFields: [
        "private_signing_key",
        "hmac_secret",
        "secret",
        "private_key_material",
        "raw_session_token",
      ],
    },
    smokeEvidence: [
      "tenant-key-id-version-public-key-lookup",
      "ed25519-session-ticket-verified-with-public-key-only",
      "missing-key-id-fails-closed",
      "revoked-key-version-fails-closed",
      "tampered-ticket-signature-fails-closed",
      "registry-excludes-private-signing-keys-and-hmac-secrets",
    ],
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
    implementationCanStart: false,
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedRevocationAndRotationPropagationSmoke() {
  const gate = relayManagedRuntimeReadinessGate();
  return {
    ...PWA_RELAY_MANAGED_REVOCATION_AND_ROTATION_PROPAGATION_SMOKE,
    completedRuntimeEvidence: [
      ...PWA_RELAY_MANAGED_REVOCATION_AND_ROTATION_PROPAGATION_SMOKE.completedRuntimeEvidence,
    ],
    closedReadinessBlockers: [
      ...PWA_RELAY_MANAGED_REVOCATION_AND_ROTATION_PROPAGATION_SMOKE.closedReadinessBlockers,
    ],
    guardrails: [
      ...PWA_RELAY_MANAGED_REVOCATION_AND_ROTATION_PROPAGATION_SMOKE.guardrails,
    ],
    propagationContract: {
      snapshotFields: [
        "snapshot_id",
        "effective_at_ms",
        "previous_snapshot_id",
        "reason",
        "entries",
      ],
      acceptedDuringOverlap: [
        "active",
        "rotating",
      ],
      failClosedForNewSessions: [
        "retiring",
        "revoked",
        "missing",
        "expired",
        "not-yet-valid",
      ],
      auditFields: [
        "tenant_id",
        "key_id",
        "key_version",
        "key_state",
        "registry_snapshot_id",
        "registry_effective_at_ms",
        "decision",
        "at_ms",
      ],
    },
    smokeEvidence: [
      "active-and-rotating-key-overlap-verifies",
      "retiring-key-version-fails-closed-for-new-sessions",
      "revoked-key-version-fails-closed-after-snapshot-propagation",
      "new-active-key-version-verifies-after-rotation",
      "registry-snapshot-id-preserved-in-audit-event",
      "tenant-key-id-version-audit-metadata-preserved",
    ],
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
    implementationCanStart: false,
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedTenantSessionRegistrationQuotaSmoke() {
  const gate = relayManagedRuntimeReadinessGate();
  return {
    ...PWA_RELAY_MANAGED_TENANT_SESSION_REGISTRATION_QUOTA_SMOKE,
    completedRuntimeEvidence: [
      ...PWA_RELAY_MANAGED_TENANT_SESSION_REGISTRATION_QUOTA_SMOKE.completedRuntimeEvidence,
    ],
    closedReadinessBlockers: [
      ...PWA_RELAY_MANAGED_TENANT_SESSION_REGISTRATION_QUOTA_SMOKE.closedReadinessBlockers,
    ],
    guardrails: [
      ...PWA_RELAY_MANAGED_TENANT_SESSION_REGISTRATION_QUOTA_SMOKE.guardrails,
    ],
    quotaContract: {
      quotaStateFields: [
        "tenant_id",
        "window_start_ms",
        "window_end_ms",
        "registration_limit",
        "registrations_used",
        "billing_meter",
        "abuse_signals",
      ],
      registrationRequestFields: [
        "tenant_id",
        "session_id",
        "daemon_device_id",
        "verifier_key_id",
        "verifier_key_version",
        "source_ip_hash",
      ],
      decisionValues: [
        "accept",
        "reject",
      ],
      failClosedReasons: [
        "tenant-session-registration-quota-exceeded",
        "quota-window-not-effective",
      ],
      auditFields: [
        "tenant_id",
        "session_id",
        "daemon_device_id",
        "verifier_key_id",
        "verifier_key_version",
        "quota_scope",
        "quota_limit",
        "quota_used",
        "decision",
        "reason",
        "billing_meter_delta",
        "abuse_signal_delta",
        "at_ms",
      ],
    },
    smokeEvidence: [
      "within-limit-registration-accepted-before-session-creation",
      "tenant-registration-limit-rejects-new-session-before-registration",
      "not-yet-effective-quota-window-fails-closed",
      "quota-denial-audit-preserves-tenant-session-key-metadata",
      "quota-denial-audit-excludes-payloads-and-secrets",
      "quota-denials-increment-billing-meter-not-abuse-rate-limit",
    ],
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
    implementationCanStart: false,
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedActiveSessionAndByteQuotaSmoke() {
  const gate = relayManagedRuntimeReadinessGate();
  return {
    ...PWA_RELAY_MANAGED_ACTIVE_SESSION_AND_BYTE_QUOTA_SMOKE,
    completedRuntimeEvidence: [
      ...PWA_RELAY_MANAGED_ACTIVE_SESSION_AND_BYTE_QUOTA_SMOKE.completedRuntimeEvidence,
    ],
    closedReadinessBlockers: [
      ...PWA_RELAY_MANAGED_ACTIVE_SESSION_AND_BYTE_QUOTA_SMOKE.closedReadinessBlockers,
    ],
    guardrails: [
      ...PWA_RELAY_MANAGED_ACTIVE_SESSION_AND_BYTE_QUOTA_SMOKE.guardrails,
    ],
    quotaContract: {
      quotaStateFields: [
        "tenant_id",
        "daemon_device_id",
        "window_start_ms",
        "window_end_ms",
        "tenant_active_session_limit",
        "tenant_active_sessions",
        "daemon_device_active_session_limit",
        "daemon_device_active_sessions",
        "relay_frame_limit",
        "relay_frames_used",
        "relay_byte_limit",
        "relay_bytes_used",
        "billing_meter",
        "abuse_signals",
      ],
      routeRequestFields: [
        "tenant_id",
        "session_id",
        "daemon_device_id",
        "verifier_key_id",
        "verifier_key_version",
        "frame_sequence",
        "payload_ciphertext_bytes",
      ],
      decisionValues: [
        "accept",
        "reject",
      ],
      failClosedReasons: [
        "tenant-active-session-quota-exceeded",
        "daemon-device-active-session-quota-exceeded",
        "relay-frame-quota-exceeded",
        "relay-byte-quota-exceeded",
        "quota-window-not-effective",
      ],
      auditFields: [
        "tenant_id",
        "session_id",
        "daemon_device_id",
        "verifier_key_id",
        "verifier_key_version",
        "frame_sequence",
        "payload_ciphertext_bytes",
        "quota_scope",
        "tenant_active_session_limit",
        "tenant_active_sessions",
        "daemon_device_active_session_limit",
        "daemon_device_active_sessions",
        "relay_frame_limit",
        "relay_frames_used",
        "relay_byte_limit",
        "relay_bytes_used",
        "decision",
        "reason",
        "billing_meter_delta",
        "abuse_signal_delta",
        "at_ms",
      ],
    },
    smokeEvidence: [
      "within-limit-frame-route-accepted-before-routing",
      "tenant-active-session-limit-rejects-before-session-activation",
      "daemon-device-active-session-limit-rejects-before-session-activation",
      "relay-frame-limit-rejects-before-routing",
      "relay-byte-limit-rejects-before-routing",
      "quota-denial-audit-excludes-payloads-and-secrets",
      "accepted-routes-increment-active-frame-byte-billing-meters",
      "quota-denials-increment-billing-meter-not-abuse-rate-limit",
    ],
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
    implementationCanStart: false,
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayManagedTenantAggregateUsageExportSmoke() {
  const gate = relayManagedRuntimeReadinessGate();
  return {
    ...PWA_RELAY_MANAGED_TENANT_AGGREGATE_USAGE_EXPORT_SMOKE,
    completedRuntimeEvidence: [
      ...PWA_RELAY_MANAGED_TENANT_AGGREGATE_USAGE_EXPORT_SMOKE.completedRuntimeEvidence,
    ],
    closedReadinessBlockers: [
      ...PWA_RELAY_MANAGED_TENANT_AGGREGATE_USAGE_EXPORT_SMOKE.closedReadinessBlockers,
    ],
    guardrails: [
      ...PWA_RELAY_MANAGED_TENANT_AGGREGATE_USAGE_EXPORT_SMOKE.guardrails,
    ],
    exportContract: {
      inputFields: [
        "tenant_id",
        "window_start_ms",
        "window_end_ms",
        "generated_at_ms",
        "plan_id",
        "billing_meter",
        "abuse_signals",
      ],
      billingUsageFields: [
        "session_registration_count",
        "active_session_count",
        "relay_frame_count",
        "relay_byte_count",
        "invalid_ticket_count",
        "quota_denial_count",
      ],
      abuseSignalFields: [
        "rate_limit_denial_count",
        "invalid_ticket_count",
        "abuse_case_count",
      ],
      outputFields: [
        "export_version",
        "export_scope",
        "tenant_id",
        "plan_id",
        "window_start_ms",
        "window_end_ms",
        "generated_at_ms",
        "payload_visibility",
        "support_visibility",
        "billing_usage",
        "abuse_signal_summary",
        "billing_abuse_boundary",
      ],
      prohibitedFields: [
        "payload_json",
        "command_text",
        "context_json",
        "approval_response_payload",
        "private_key_material",
        "raw_session_token",
        "signed_session_ticket",
        "full_setup_json",
        "hmac_secret",
        "mac_hex",
      ],
    },
    smokeEvidence: [
      "tenant-aggregate-usage-export-has-session-active-frame-byte-and-quota-counters",
      "tenant-aggregate-usage-export-excludes-payloads-and-secrets",
      "billing-usage-and-abuse-signals-exported-in-separate-sections",
      "support-visibility-is-aggregate-only",
      "runtime-gate-records-tenant-usage-export-evidence",
    ],
    remainingRuntimeEvidence: gate.remainingRuntimeEvidence,
    remainingRuntimeBlockers: gate.remainingRuntimeBlockers,
    implementationCanStart: false,
    nextLocalSlice: "managed-relay-support-redaction-and-access-review-evidence",
  };
}

export function relayPrivateNetworkSetupPreflight(config = {}, nowMs = Date.now()) {
  const {
    transportMode = PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
    deploymentMode = "",
    relayEndpointUrl = "",
    privateNetworkName = "",
    signedSessionTicket = null,
    companionIdentity = null,
    operatorSetupText = "",
  } = config || {};
  const blockers = [];
  const addBlocker = (code) => {
    if (!blockers.includes(code)) {
      blockers.push(code);
    }
  };

  if (transportMode !== PWA_TRANSPORT_MODE_RELAY) {
    addBlocker("transport_mode_not_relay");
  }
  if (deploymentMode !== PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK) {
    addBlocker("private_network_deployment_mode_required");
  }
  if (!validPrivateNetworkName(privateNetworkName)) {
    addBlocker("private_network_name_invalid");
  }
  if (typeof relayEndpointUrl !== "string" || relayEndpointUrl.trim().length === 0) {
    addBlocker("relay_endpoint_url_missing");
  } else if (!validRelayWebSocketEndpointUrl(relayEndpointUrl)) {
    addBlocker("relay_endpoint_url_invalid");
  }
  if (typeof operatorSetupText !== "string" || operatorSetupText.trim().length < 12) {
    addBlocker("relay_operator_setup_text_missing");
  }

  const identityValid = validCompanionIdentity(companionIdentity);
  if (!identityValid) {
    addBlocker("companion_identity_missing");
  }

  let ticket = null;
  if (!signedSessionTicket) {
    addBlocker("relay_signed_ticket_missing");
  } else {
    try {
      validateSignedRelaySessionTicketMetadata(signedSessionTicket);
      ticket = signedSessionTicket.ticket;
      if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
        addBlocker("relay_now_ms_invalid");
      } else if (relaySessionExpiredAt(ticket, nowMs)) {
        addBlocker("relay_signed_ticket_expired");
      }
    } catch {
      addBlocker("relay_signed_ticket_invalid");
    }
  }

  if (ticket && identityValid) {
    if (
      ticket.companion_device_id !== companionIdentity.deviceId ||
      ticket.companion_noise_pubkey_hex !== companionIdentity.noisePubkeyHex ||
      ticket.companion_approval_pubkey_hex !== companionIdentity.approvalPubkeyHex
    ) {
      addBlocker("relay_ticket_identity_mismatch");
    }
  }

  const ready = blockers.length === 0;
  return {
    status: ready ? "ready" : "hidden",
    deploymentMode: PWA_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK,
    relayVisible: false,
    contractReady: ready,
    productDefault: PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
    blockers,
  };
}

export function relayTransportUxPreflight(config = {}, nowMs = Date.now()) {
  const {
    transportMode = PWA_TRANSPORT_MODE_LIVE_LOOPBACK,
    relayEndpointUrl = "",
    signedSessionTicket = null,
    companionIdentity = null,
    deploymentMode = "",
    operatorSetupText = "",
  } = config || {};
  const blockers = [];
  const addBlocker = (code) => {
    if (!blockers.includes(code)) {
      blockers.push(code);
    }
  };

  if (transportMode !== PWA_TRANSPORT_MODE_RELAY) {
    addBlocker("transport_mode_not_relay");
  }
  if (typeof relayEndpointUrl !== "string" || relayEndpointUrl.trim().length === 0) {
    addBlocker("relay_endpoint_url_missing");
  } else if (!validRelayWebSocketEndpointUrl(relayEndpointUrl)) {
    addBlocker("relay_endpoint_url_invalid");
  }
  if (typeof deploymentMode !== "string" || deploymentMode.trim().length === 0) {
    addBlocker("relay_deployment_mode_missing");
  } else if (!PWA_RELAY_DEPLOYMENT_MODES.includes(deploymentMode)) {
    addBlocker("relay_deployment_mode_invalid");
  } else if (deploymentMode !== PWA_RELAY_SELECTED_DEPLOYMENT_MODE) {
    addBlocker("relay_deployment_mode_not_selected");
  }
  if (typeof operatorSetupText !== "string" || operatorSetupText.trim().length < 12) {
    addBlocker("relay_operator_setup_text_missing");
  }

  const identityValid = validCompanionIdentity(companionIdentity);
  if (!identityValid) {
    addBlocker("companion_identity_missing");
  }

  let ticket = null;
  if (!signedSessionTicket) {
    addBlocker("relay_signed_ticket_missing");
  } else {
    try {
      validateSignedRelaySessionTicketMetadata(signedSessionTicket);
      ticket = signedSessionTicket.ticket;
      if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
        addBlocker("relay_now_ms_invalid");
      } else if (relaySessionExpiredAt(ticket, nowMs)) {
        addBlocker("relay_signed_ticket_expired");
      }
    } catch {
      addBlocker("relay_signed_ticket_invalid");
    }
  }

  if (ticket && identityValid) {
    if (
      ticket.companion_device_id !== companionIdentity.deviceId ||
      ticket.companion_noise_pubkey_hex !== companionIdentity.noisePubkeyHex ||
      ticket.companion_approval_pubkey_hex !== companionIdentity.approvalPubkeyHex
    ) {
      addBlocker("relay_ticket_identity_mismatch");
    }
  }

  const ready = blockers.length === 0;
  return {
    status: ready ? "ready" : "hidden",
    transportMode,
    deploymentMode: deploymentMode || "",
    relayVisible: ready,
    relayEnabled: ready,
    blockers,
  };
}

function validateRelaySessionConnectMetadata(connect) {
  if (connect?.relay_protocol_version !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay session protocol_version");
  }
  if (!validRelaySessionId(connect.session_id)) {
    throw new Error("relay session_id 형식 오류");
  }
  if (!validRelaySender(connect.peer)) {
    throw new Error("relay peer 형식 오류");
  }
  if (!validRelaySessionToken(connect.session_token)) {
    throw new Error("relay session_token 형식 오류");
  }
  if (connect.peer === "daemon") {
    if (!validRelayPubkeyHex(connect.daemon_pubkey_hex)) {
      throw new Error("relay daemon_pubkey_hex 형식 오류");
    }
    if (
      connect.device_id !== undefined ||
      connect.noise_pubkey_hex !== undefined ||
      connect.approval_pubkey_hex !== undefined
    ) {
      throw new Error("relay daemon connect companion field 오류");
    }
    return;
  }
  if (!validRelayDeviceId(connect.device_id)) {
    throw new Error("relay companion device_id 형식 오류");
  }
  if (!validRelayPubkeyHex(connect.noise_pubkey_hex)) {
    throw new Error("relay companion noise_pubkey_hex 형식 오류");
  }
  if (!validRelayPubkeyHex(connect.approval_pubkey_hex)) {
    throw new Error("relay companion approval_pubkey_hex 형식 오류");
  }
  if (connect.daemon_pubkey_hex !== undefined) {
    throw new Error("relay companion connect daemon field 오류");
  }
}

export function relayFrameFromLiveMessage(
  sessionId,
  sender,
  sequence,
  sentAtMs,
  expiresAtMs,
  message,
) {
  const frame = {
    relay_protocol_version: RELAY_TRANSPORT_PROTOCOL_VERSION,
    session_id: sessionId,
    sender,
    sequence,
    sent_at_ms: sentAtMs,
    expires_at_ms: expiresAtMs,
    payload_json: liveTransportJson(message),
  };
  validateRelayFrame(frame);
  return frame;
}

export function relayFrameWithDefaultExpiry(sessionId, sender, sequence, sentAtMs, message) {
  if (!Number.isSafeInteger(sentAtMs) || sentAtMs <= 0) {
    throw new Error("relay sent_at_ms 형식 오류");
  }
  return relayFrameFromLiveMessage(
    sessionId,
    sender,
    sequence,
    sentAtMs,
    sentAtMs + DEFAULT_RELAY_FRAME_TTL_MS,
    message,
  );
}

export function relayFrameJson(frame) {
  validateRelayFrame(frame);
  return JSON.stringify(frame);
}

export function parseRelayFrame(text) {
  let frame;
  try {
    frame = JSON.parse(text);
  } catch {
    throw new Error("relay frame JSON 파싱 실패");
  }
  validateRelayFrame(frame);
  return frame;
}

export function relayFramePayloadMessage(frame) {
  validateRelayFrame(frame);
  return parseLiveTransportMessage(frame.payload_json);
}

export function relayFrameRouteEnvelope(frameOrText) {
  const frame =
    typeof frameOrText === "string"
      ? parseRelayFrame(frameOrText)
      : validateRelayFrame(frameOrText) || frameOrText;
  return {
    relay_protocol_version: frame.relay_protocol_version,
    session_id: frame.session_id,
    sender: frame.sender,
    sequence: frame.sequence,
    sent_at_ms: frame.sent_at_ms,
    expires_at_ms: frame.expires_at_ms,
    payload_json_bytes: new TextEncoder().encode(frame.payload_json).length,
  };
}

export async function managedRelayEncryptedFrameFromLiveMessage(
  sessionId,
  sender,
  sequence,
  sentAtMs,
  expiresAtMs,
  message,
  payloadKeyHex,
  options = {},
) {
  validateLiveTransportMessage(message);
  const webCrypto = options.webCrypto || globalThis.crypto;
  const nonceHex = managedRelayPayloadNonceHex(options.nonceHex, webCrypto);
  const frame = {
    relay_protocol_version: RELAY_TRANSPORT_PROTOCOL_VERSION,
    session_id: sessionId,
    sender,
    sequence,
    sent_at_ms: sentAtMs,
    expires_at_ms: expiresAtMs,
    payload_ciphertext_alg: MANAGED_RELAY_PAYLOAD_CIPHERTEXT_ALG,
    payload_key_scope: MANAGED_RELAY_PAYLOAD_KEY_SCOPE,
    payload_nonce_hex: nonceHex,
    payload_ciphertext_hex: "",
  };
  validateManagedRelayEncryptedFrameMetadata(frame);
  const payloadJson = liveTransportJson(message);
  const key = await managedRelayPayloadCryptoKey(payloadKeyHex, webCrypto);
  const ciphertext = await webCrypto.subtle.encrypt(
    {
      name: "AES-GCM",
      iv: hexToBytes(nonceHex),
      additionalData: managedRelayEncryptedFrameAad(frame),
    },
    key,
    new TextEncoder().encode(payloadJson),
  );
  frame.payload_ciphertext_hex = bytesToHex(new Uint8Array(ciphertext));
  validateManagedRelayEncryptedFrame(frame);
  return frame;
}

export function managedRelayEncryptedFrameJson(frame) {
  validateManagedRelayEncryptedFrame(frame);
  return JSON.stringify(frame);
}

export function parseManagedRelayEncryptedFrame(text) {
  let frame;
  try {
    frame = JSON.parse(text);
  } catch {
    throw new Error("managed relay encrypted frame JSON 파싱 실패");
  }
  validateManagedRelayEncryptedFrame(frame);
  return frame;
}

export function managedRelayEncryptedFrameRouteEnvelope(frameOrText) {
  const frame =
    typeof frameOrText === "string"
      ? parseManagedRelayEncryptedFrame(frameOrText)
      : validateManagedRelayEncryptedFrame(frameOrText) || frameOrText;
  return {
    relay_protocol_version: frame.relay_protocol_version,
    session_id: frame.session_id,
    sender: frame.sender,
    sequence: frame.sequence,
    sent_at_ms: frame.sent_at_ms,
    expires_at_ms: frame.expires_at_ms,
    payload_ciphertext_alg: frame.payload_ciphertext_alg,
    payload_key_scope: frame.payload_key_scope,
    payload_ciphertext_bytes: hexToBytes(frame.payload_ciphertext_hex).byteLength,
  };
}

export async function managedRelayEncryptedFramePayloadMessage(
  frameOrText,
  payloadKeyHex,
  webCrypto = globalThis.crypto,
) {
  const frame =
    typeof frameOrText === "string"
      ? parseManagedRelayEncryptedFrame(frameOrText)
      : validateManagedRelayEncryptedFrame(frameOrText) || frameOrText;
  const key = await managedRelayPayloadCryptoKey(payloadKeyHex, webCrypto);
  let plaintext;
  try {
    plaintext = await webCrypto.subtle.decrypt(
      {
        name: "AES-GCM",
        iv: hexToBytes(frame.payload_nonce_hex),
        additionalData: managedRelayEncryptedFrameAad(frame),
      },
      key,
      hexToBytes(frame.payload_ciphertext_hex),
    );
  } catch {
    throw new Error("managed relay payload decrypt failed");
  }
  return parseLiveTransportMessage(new TextDecoder().decode(plaintext));
}

export function createRelayEndpoint(
  sessionId,
  sender,
  frameTtlMs = DEFAULT_RELAY_FRAME_TTL_MS,
) {
  const endpoint = {
    sessionId,
    sender,
    nextSequence: 1,
    frameTtlMs,
  };
  validateRelayEndpoint(endpoint);
  return endpoint;
}

export function validateRelayEndpoint(endpoint) {
  if (!validRelaySessionId(endpoint?.sessionId)) {
    throw new Error("relay endpoint sessionId 형식 오류");
  }
  if (!validRelaySender(endpoint.sender)) {
    throw new Error("relay endpoint sender 형식 오류");
  }
  if (!Number.isSafeInteger(endpoint.nextSequence) || endpoint.nextSequence <= 0) {
    throw new Error("relay endpoint nextSequence 형식 오류");
  }
  if (!Number.isSafeInteger(endpoint.frameTtlMs) || endpoint.frameTtlMs <= 0) {
    throw new Error("relay endpoint frameTtlMs 형식 오류");
  }
}

export function relayEndpointNextFrame(endpoint, message, nowMs = Date.now()) {
  validateRelayEndpoint(endpoint);
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("relay sent_at_ms 형식 오류");
  }
  const frame = relayFrameFromLiveMessage(
    endpoint.sessionId,
    endpoint.sender,
    endpoint.nextSequence,
    nowMs,
    nowMs + endpoint.frameTtlMs,
    message,
  );
  if (endpoint.nextSequence >= Number.MAX_SAFE_INTEGER) {
    throw new Error("relay sequence overflow");
  }
  endpoint.nextSequence += 1;
  return frame;
}

export function relayEndpointAcceptFrame(endpoint, frameOrText, nowMs = Date.now()) {
  validateRelayEndpoint(endpoint);
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0) {
    throw new Error("relay now_ms 형식 오류");
  }
  const frame =
    typeof frameOrText === "string"
      ? parseRelayFrame(frameOrText)
      : validateRelayFrame(frameOrText) || frameOrText;
  if (frame.session_id !== endpoint.sessionId) {
    throw new Error("relay session_id mismatch");
  }
  if (frame.sender === endpoint.sender) {
    throw new Error("relay sender matches endpoint");
  }
  if (nowMs >= frame.expires_at_ms) {
    return null;
  }
  return relayFramePayloadMessage(frame);
}

export function relayWebSocketConnectUrl(relayEndpointUrl, connect) {
  if (!validRelayWebSocketEndpointUrl(relayEndpointUrl)) {
    throw new Error("relay websocket endpoint URL 형식 오류");
  }
  validateRelaySessionConnectMetadata(connect);
  const url = new URL(relayEndpointUrl);
  url.searchParams.set("session_id", connect.session_id);
  url.searchParams.set("role", connect.peer);
  return url.toString();
}

export function relayEndpointLoopInitialState(connect, frameTtlMs = DEFAULT_RELAY_FRAME_TTL_MS) {
  validateRelaySessionConnectMetadata(connect);
  const endpoint = createRelayEndpoint(connect.session_id, connect.peer, frameTtlMs);
  const loop = {
    connect: { ...connect },
    connectJson: relaySessionConnectJson(connect),
    endpoint,
    webSocketUrl: "",
    connected: false,
    sentCount: 0,
    queuedCount: 0,
    receivedCount: 0,
    droppedCount: 0,
    errorCount: 0,
  };
  validateRelayEndpointLoop(loop);
  return loop;
}

export function relayCompanionEndpointLoopFromSetup(
  setup,
  frameTtlMs = DEFAULT_RELAY_FRAME_TTL_MS,
  nowMs = Date.now(),
) {
  const preflight = relayRuntimeSetupPreflight(setup, nowMs);
  if (!preflight.relayEnabled) {
    throw new Error(`relay setup not ready: ${preflight.blockers.join(",")}`);
  }
  const loop = relayEndpointLoopInitialState(setup.companionConnect, frameTtlMs);
  loop.webSocketUrl = relayWebSocketConnectUrl(setup.relayEndpointUrl, setup.companionConnect);
  loop.preflight = preflight;
  return loop;
}

export function relayPrivateNetworkCompanionEndpointLoopFromSetup(
  setup,
  frameTtlMs = DEFAULT_RELAY_FRAME_TTL_MS,
  nowMs = Date.now(),
) {
  const preflight = relayPrivateNetworkRuntimeSetupPreflight(setup, nowMs);
  if (!preflight.contractReady) {
    throw new Error(`private-network relay setup not ready: ${preflight.blockers.join(",")}`);
  }
  const loop = relayEndpointLoopInitialState(setup.companionConnect, frameTtlMs);
  loop.webSocketUrl = relayWebSocketConnectUrl(setup.relayEndpointUrl, setup.companionConnect);
  loop.preflight = preflight;
  return loop;
}

export function relayEndpointLoopConnectJson(loop) {
  validateRelayEndpointLoop(loop);
  return relaySessionConnectJson(loop.connect);
}

export function relayEndpointLoopNextFrame(loop, message, nowMs = Date.now()) {
  validateRelayEndpointLoop(loop);
  const frame = relayEndpointNextFrame(loop.endpoint, message, nowMs);
  loop.sentCount += 1;
  return {
    kind: "frame",
    frame,
    frameJson: relayFrameJson(frame),
    route: relayFrameRouteEnvelope(frame),
  };
}

export function relayEndpointLoopAcceptSocketMessage(loop, socketMessage, nowMs = Date.now()) {
  validateRelayEndpointLoop(loop);
  const message = parseRelayWebSocketEnvelope(socketMessage);
  if (message.kind === "connected") {
    if (message.session_id !== loop.connect.session_id || message.peer !== loop.connect.peer) {
      throw new Error("relay websocket connected envelope mismatch");
    }
    loop.connected = true;
    return { kind: "connected", envelope: message };
  }
  if (message.kind === "queued") {
    validateRelayRouteEnvelopeMetadata(message.route);
    if (message.route.session_id !== loop.endpoint.sessionId || message.route.sender !== loop.endpoint.sender) {
      throw new Error("relay websocket queued envelope mismatch");
    }
    loop.queuedCount += 1;
    return { kind: "queued", route: message.route };
  }
  if (message.kind === "frame") {
    if (typeof message.frame_json !== "string" || message.frame_json.length === 0) {
      throw new Error("relay websocket frame_json 형식 오류");
    }
    const route = relayFrameRouteEnvelope(message.frame_json);
    if (route.session_id !== loop.endpoint.sessionId || route.sender === loop.endpoint.sender) {
      throw new Error("relay websocket frame envelope mismatch");
    }
    const liveMessage = relayEndpointAcceptFrame(loop.endpoint, message.frame_json, nowMs);
    if (liveMessage === null) {
      loop.droppedCount += 1;
      return { kind: "dropped", route, liveMessage: null };
    }
    loop.receivedCount += 1;
    return { kind: "live_message", route, liveMessage };
  }
  if (message.kind === "error") {
    loop.errorCount += 1;
    return {
      kind: "error",
      message: typeof message.message === "string" ? message.message : "relay websocket error",
      envelope: message,
    };
  }
  throw new Error("relay websocket envelope kind 형식 오류");
}

export function relayEndpointExchange(
  daemonEndpoint,
  companionEndpoint,
  daemonMessage,
  companionReply,
  nowMs = Date.now(),
) {
  validateRelayEndpoint(daemonEndpoint);
  validateRelayEndpoint(companionEndpoint);
  validateLiveTransportMessage(daemonMessage);
  validateLiveTransportMessage(companionReply);
  if (!Number.isSafeInteger(nowMs) || nowMs <= 0 || nowMs > Number.MAX_SAFE_INTEGER - 3) {
    throw new Error("relay exchange now_ms 형식 오류");
  }
  if (daemonEndpoint.sessionId !== companionEndpoint.sessionId) {
    throw new Error("relay exchange session_id mismatch");
  }
  if (daemonEndpoint.sender !== "daemon") {
    throw new Error("relay exchange daemon endpoint sender mismatch");
  }
  if (companionEndpoint.sender !== "companion") {
    throw new Error("relay exchange companion endpoint sender mismatch");
  }

  const daemonFrame = relayEndpointNextFrame(daemonEndpoint, daemonMessage, nowMs);
  const daemonFrameJson = relayFrameJson(daemonFrame);
  const companionMessage = relayEndpointAcceptFrame(companionEndpoint, daemonFrameJson, nowMs + 1);
  if (companionMessage === null) {
    throw new Error("relay exchange daemon frame expired");
  }

  const companionFrame = relayEndpointNextFrame(companionEndpoint, companionReply, nowMs + 2);
  const companionFrameJson = relayFrameJson(companionFrame);
  const daemonReply = relayEndpointAcceptFrame(daemonEndpoint, companionFrameJson, nowMs + 3);
  if (daemonReply === null) {
    throw new Error("relay exchange companion frame expired");
  }

  return {
    daemonFrame,
    daemonFrameJson,
    companionMessage,
    companionFrame,
    companionFrameJson,
    daemonReply,
  };
}

export function validateRelayFrame(frame) {
  if (frame?.relay_protocol_version !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay protocol_version");
  }
  if (!validRelaySessionId(frame.session_id)) {
    throw new Error("relay session_id 형식 오류");
  }
  if (!validRelaySender(frame.sender)) {
    throw new Error("relay sender 형식 오류");
  }
  if (!Number.isSafeInteger(frame.sequence) || frame.sequence <= 0) {
    throw new Error("relay sequence 형식 오류");
  }
  if (!Number.isSafeInteger(frame.sent_at_ms) || frame.sent_at_ms <= 0) {
    throw new Error("relay sent_at_ms 형식 오류");
  }
  if (!Number.isSafeInteger(frame.expires_at_ms) || frame.expires_at_ms <= frame.sent_at_ms) {
    throw new Error("relay expires_at_ms 형식 오류");
  }
  if (
    typeof frame.payload_json !== "string" ||
    frame.payload_json.length === 0 ||
    new TextEncoder().encode(frame.payload_json).length > MAX_RELAY_PAYLOAD_JSON_BYTES
  ) {
    throw new Error("relay payload_json 형식 오류");
  }
}

export function validateManagedRelayEncryptedFrame(frame) {
  validateManagedRelayEncryptedFrameMetadata(frame);
  if (
    typeof frame.payload_ciphertext_hex !== "string" ||
    frame.payload_ciphertext_hex.length === 0 ||
    frame.payload_ciphertext_hex.length % 2 !== 0 ||
    !/^[0-9a-f]+$/i.test(frame.payload_ciphertext_hex) ||
    hexToBytes(frame.payload_ciphertext_hex).byteLength > MAX_MANAGED_RELAY_PAYLOAD_CIPHERTEXT_BYTES
  ) {
    throw new Error("managed relay payload_ciphertext_hex 형식 오류");
  }
}

function validateManagedRelayEncryptedFrameMetadata(frame) {
  rejectManagedRelayPlaintextFrameFields(frame);
  if (frame?.relay_protocol_version !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 managed relay protocol_version");
  }
  if (!validRelaySessionId(frame.session_id)) {
    throw new Error("managed relay session_id 형식 오류");
  }
  if (!validRelaySender(frame.sender)) {
    throw new Error("managed relay sender 형식 오류");
  }
  if (!Number.isSafeInteger(frame.sequence) || frame.sequence <= 0) {
    throw new Error("managed relay sequence 형식 오류");
  }
  if (!Number.isSafeInteger(frame.sent_at_ms) || frame.sent_at_ms <= 0) {
    throw new Error("managed relay sent_at_ms 형식 오류");
  }
  if (!Number.isSafeInteger(frame.expires_at_ms) || frame.expires_at_ms <= frame.sent_at_ms) {
    throw new Error("managed relay expires_at_ms 형식 오류");
  }
  if (frame.payload_ciphertext_alg !== MANAGED_RELAY_PAYLOAD_CIPHERTEXT_ALG) {
    throw new Error("managed relay payload_ciphertext_alg 형식 오류");
  }
  if (frame.payload_key_scope !== MANAGED_RELAY_PAYLOAD_KEY_SCOPE) {
    throw new Error("managed relay payload_key_scope 형식 오류");
  }
  if (
    typeof frame.payload_nonce_hex !== "string" ||
    !new RegExp(`^[0-9a-f]{${MANAGED_RELAY_PAYLOAD_NONCE_BYTES * 2}}$`, "i").test(
      frame.payload_nonce_hex,
    )
  ) {
    throw new Error("managed relay payload_nonce_hex 형식 오류");
  }
}

function rejectManagedRelayPlaintextFrameFields(frame) {
  if (!frame || typeof frame !== "object" || Array.isArray(frame)) {
    throw new Error("managed relay encrypted frame 형식 오류");
  }
  for (const field of [
    "payload_json",
    "command_text",
    "context_json",
    "approval_response_payload",
    "private_key_material",
    "raw_session_token",
    "full_setup_json",
  ]) {
    if (Object.prototype.hasOwnProperty.call(frame, field)) {
      throw new Error(`managed relay plaintext field not allowed: ${field}`);
    }
  }
}

function managedRelayPayloadNonceHex(nonceHex, webCrypto) {
  if (nonceHex !== undefined) {
    if (
      typeof nonceHex !== "string" ||
      !new RegExp(`^[0-9a-f]{${MANAGED_RELAY_PAYLOAD_NONCE_BYTES * 2}}$`, "i").test(nonceHex)
    ) {
      throw new Error("managed relay payload nonce 형식 오류");
    }
    return nonceHex.toLowerCase();
  }
  if (!webCrypto || typeof webCrypto.getRandomValues !== "function") {
    throw new Error("managed relay crypto unavailable");
  }
  const nonce = new Uint8Array(MANAGED_RELAY_PAYLOAD_NONCE_BYTES);
  webCrypto.getRandomValues(nonce);
  return bytesToHex(nonce);
}

async function managedRelayPayloadCryptoKey(payloadKeyHex, webCrypto) {
  if (
    typeof payloadKeyHex !== "string" ||
    !new RegExp(`^[0-9a-f]{${MANAGED_RELAY_PAYLOAD_KEY_BYTES * 2}}$`, "i").test(payloadKeyHex)
  ) {
    throw new Error("managed relay payload key 형식 오류");
  }
  if (!webCrypto?.subtle) {
    throw new Error("managed relay crypto unavailable");
  }
  return webCrypto.subtle.importKey(
    "raw",
    hexToBytes(payloadKeyHex),
    { name: "AES-GCM" },
    false,
    ["encrypt", "decrypt"],
  );
}

function managedRelayEncryptedFrameAad(frame) {
  validateManagedRelayEncryptedFrameMetadata(frame);
  return new TextEncoder().encode(
    JSON.stringify({
      relay_protocol_version: frame.relay_protocol_version,
      session_id: frame.session_id,
      sender: frame.sender,
      sequence: frame.sequence,
      sent_at_ms: frame.sent_at_ms,
      expires_at_ms: frame.expires_at_ms,
      payload_ciphertext_alg: frame.payload_ciphertext_alg,
      payload_key_scope: frame.payload_key_scope,
    }),
  );
}

export function liveEndpointUrls(baseUrl) {
  const root = new URL("/", baseUrl);
  const href = root.href.replace(/\/$/, "");
  return {
    baseUrl: href,
    healthUrl: `${href}/health`,
    eventsUrl: `${href}/events`,
    messageUrl: `${href}/message`,
  };
}

export function liveEventSourceUrl(baseUrl) {
  return liveEndpointUrls(baseUrl).eventsUrl;
}

export function liveApprovalRequestKey(request) {
  validateApprovalRequest(request);
  return `${request.approval_id.join(".")}:${request.nonce.join(".")}`;
}

export function liveApprovalQueueNext(queue, message, maxItems = 8) {
  validateLiveTransportMessage(message);
  const current = Array.isArray(queue) ? queue : [];
  if (message.type !== "approval_request") {
    return current.slice(0, maxItems);
  }
  const key = liveApprovalRequestKey(message.request);
  const withoutDuplicate = current.filter((item) => item.key !== key);
  return [{ key, request: message.request, receivedAtMs: Date.now() }, ...withoutDuplicate].slice(
    0,
    maxItems,
  );
}

export function liveMonitorInitialState(nowMs = Date.now()) {
  return {
    state: "Disconnected",
    endpoint: "",
    deviceId: "",
    pendingCount: 0,
    receivedCount: 0,
    sentCount: 0,
    approvedCount: 0,
    rejectedCount: 0,
    errorCount: 0,
    connectedAtMs: 0,
    lastHeartbeatAtMs: 0,
    lastResponseAtMs: 0,
    updatedAtMs: nowMs,
    history: [],
  };
}

export function liveMonitorNext(state, event, nowMs = Date.now(), maxHistory = 12) {
  const current = state || liveMonitorInitialState(nowMs);
  const next = { ...current, updatedAtMs: nowMs, history: [...(current.history || [])] };
  const type = event?.type || "unknown";
  const label = event?.label || type;
  if (type === "connected") {
    next.state = "Connected";
    next.endpoint = event.endpoint || next.endpoint;
    next.deviceId = event.deviceId || next.deviceId;
    next.connectedAtMs = nowMs;
  } else if (type === "disconnected") {
    next.state = "Disconnected";
    next.pendingCount = 0;
  } else if (type === "waiting") {
    next.state = "Waiting";
  } else if (type === "approval_request") {
    next.state = "Connected";
    next.pendingCount = Math.max(0, Number(event.pendingCount ?? next.pendingCount));
    next.receivedCount += 1;
  } else if (type === "approval_response") {
    next.state = "Connected";
    next.pendingCount = Math.max(0, Number(event.pendingCount ?? next.pendingCount));
    next.sentCount += 1;
    next.lastResponseAtMs = nowMs;
    if (event.approve === true) next.approvedCount += 1;
    if (event.approve === false) next.rejectedCount += 1;
  } else if (type === "ping" || type === "pong") {
    next.state = "Connected";
    next.lastHeartbeatAtMs = nowMs;
  } else if (type === "error") {
    next.errorCount += 1;
  }
  next.history = [{ type, label, atMs: nowMs }, ...next.history].slice(0, maxHistory);
  return next;
}

export function liveMessageRequest(message) {
  return {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: liveTransportJson(message),
  };
}

export async function postLiveTransportMessage(baseUrl, message, fetchImpl = globalThis.fetch) {
  if (typeof fetchImpl !== "function") {
    throw new Error("fetch 미지원");
  }
  const { messageUrl } = liveEndpointUrls(baseUrl);
  const response = await fetchImpl(messageUrl, liveMessageRequest(message));
  const reply = parseLiveTransportMessage(await response.text());
  if (!response.ok) {
    const err = new Error(reply.type === "error" ? reply.message : "live endpoint error");
    err.status = response.status;
    err.reply = reply;
    throw err;
  }
  return reply;
}

export function parseLiveTransportMessage(text) {
  let message;
  try {
    message = JSON.parse(text);
  } catch {
    throw new Error("live transport JSON 파싱 실패");
  }
  validateLiveTransportMessage(message);
  return message;
}

export function validateLiveTransportMessage(message) {
  switch (message?.type) {
    case "hello":
      if (message.protocol_version !== LIVE_TRANSPORT_PROTOCOL_VERSION) {
        throw new Error("지원하지 않는 live protocol_version");
      }
      liveHelloMessage({
        deviceId: message.device_id,
        noisePubkeyHex: message.noise_pubkey_hex,
        approvalPubkeyHex: message.approval_pubkey_hex,
      });
      return;
    case "approval_request":
      validateApprovalRequest(message.request);
      return;
    case "approval_response":
      validateApprovalResponse(message.response);
      return;
    case "ping":
    case "pong":
      if (typeof message.nonce !== "string" || message.nonce.length === 0 || message.nonce.length > 128) {
        throw new Error("heartbeat nonce 형식 오류");
      }
      return;
    case "error":
      if (typeof message.message !== "string" || message.message.trim().length === 0) {
        throw new Error("error message 형식 오류");
      }
      return;
    default:
      throw new Error("지원하지 않는 live transport message type");
  }
}

export async function generateCompanionIdentity(webCrypto = globalThis.crypto) {
  return (await generateCompanionKeyMaterial(webCrypto)).identity;
}

export async function generateCompanionKeyMaterial(webCrypto = globalThis.crypto) {
  if (!webCrypto?.subtle || typeof webCrypto.getRandomValues !== "function") {
    throw new Error("WebCrypto 미지원");
  }
  const noise = await webCrypto.subtle.generateKey({ name: "X25519" }, false, ["deriveBits"]);
  const approval = await webCrypto.subtle.generateKey({ name: "Ed25519" }, false, ["sign", "verify"]);
  const random = new Uint8Array(4);
  webCrypto.getRandomValues(random);
  const identity = {
    deviceId: `web-${bytesToHex(random)}`,
    noisePubkeyHex: await publicKeyHex(webCrypto, noise.publicKey),
    approvalPubkeyHex: await publicKeyHex(webCrypto, approval.publicKey),
  };
  return {
    identity,
    keyMaterial: { noise, approval },
  };
}

export function saveCompanionIdentity(storage, identity) {
  if (!storage) return;
  storage.setItem(COMPANION_IDENTITY_KEY, JSON.stringify(identity));
}

export function loadCompanionIdentity(storage) {
  if (!storage) return null;
  const raw = storage.getItem(COMPANION_IDENTITY_KEY);
  if (!raw) return null;
  try {
    const identity = JSON.parse(raw);
    if (
      !/^[A-Za-z0-9._:-]+$/.test(identity.deviceId || "") ||
      !/^[0-9a-f]{64}$/i.test(identity.noisePubkeyHex || "") ||
      !/^[0-9a-f]{64}$/i.test(identity.approvalPubkeyHex || "")
    ) {
      return null;
    }
    return identity;
  } catch {
    return null;
  }
}

export async function saveCompanionKeyMaterial(indexedDb, identity, keyMaterial) {
  if (!indexedDb) {
    throw new Error("IndexedDB 미지원");
  }
  const db = await openIdentityDb(indexedDb);
  try {
    await idbPut(db, COMPANION_IDENTITY_STORE, {
      id: ACTIVE_IDENTITY_ID,
      identity,
      keyMaterial,
      createdAtMs: Date.now(),
    });
  } finally {
    db.close?.();
  }
}

export async function loadCompanionKeyMaterial(indexedDb) {
  if (!indexedDb) return null;
  const db = await openIdentityDb(indexedDb);
  try {
    const record = await idbGet(db, COMPANION_IDENTITY_STORE, ACTIVE_IDENTITY_ID);
    if (!record || !loadCompanionIdentity(memoryStorageFor(record.identity))) {
      return null;
    }
    if (!record.keyMaterial?.noise?.privateKey || !record.keyMaterial?.approval?.privateKey) {
      return null;
    }
    return record;
  } finally {
    db.close?.();
  }
}

export async function signApprovalBytes(bytes, keyMaterial, webCrypto = globalThis.crypto) {
  const input = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  const signature = await webCrypto.subtle.sign(
    { name: "Ed25519" },
    keyMaterial.approval.privateKey,
    input,
  );
  return bytesToHex(new Uint8Array(signature));
}

export async function verifyApprovalBytes(bytes, signatureHex, keyMaterial, webCrypto = globalThis.crypto) {
  const input = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
  return webCrypto.subtle.verify(
    { name: "Ed25519" },
    keyMaterial.approval.publicKey,
    hexToBytes(signatureHex),
    input,
  );
}

export function approvalSigningBytes(request, approve) {
  validateApprovalRequest(request);
  const out = new Uint8Array(request.approval_id.length + request.nonce.length + 1);
  out.set(request.approval_id, 0);
  out.set(request.nonce, request.approval_id.length);
  out[out.length - 1] = approve ? 1 : 0;
  return out;
}

export async function approvalResponseForRequest(
  request,
  approve,
  keyMaterial,
  webCrypto = globalThis.crypto,
) {
  if (!keyMaterial?.approval?.privateKey) {
    throw new Error("approval private key 없음");
  }
  const signature = await webCrypto.subtle.sign(
    { name: "Ed25519" },
    keyMaterial.approval.privateKey,
    approvalSigningBytes(request, approve),
  );
  return {
    approval_id: request.approval_id,
    nonce: request.nonce,
    approve,
    sig: Array.from(new Uint8Array(signature)),
  };
}

export function approvalResponseJson(response) {
  return JSON.stringify(response);
}

export function validateApprovalResponse(response) {
  if (!Array.isArray(response.approval_id) || response.approval_id.length === 0) {
    throw new Error("approval response id 형식 오류");
  }
  if (!Array.isArray(response.nonce) || response.nonce.length !== 32) {
    throw new Error("approval response nonce 형식 오류");
  }
  if (typeof response.approve !== "boolean") {
    throw new Error("approval response decision 형식 오류");
  }
  if (!Array.isArray(response.sig) || response.sig.length !== 64) {
    throw new Error("approval response sig 형식 오류");
  }
  for (const byte of [...response.approval_id, ...response.nonce, ...response.sig]) {
    if (!Number.isInteger(byte) || byte < 0 || byte > 255) {
      throw new Error("approval response byte array 형식 오류");
    }
  }
}

export async function deriveNoiseSharedSecretHex(peerPubkeyHex, keyMaterial, webCrypto = globalThis.crypto) {
  const peerPublicKey = await webCrypto.subtle.importKey(
    "raw",
    hexToBytes(peerPubkeyHex),
    { name: "X25519" },
    false,
    [],
  );
  const bits = await webCrypto.subtle.deriveBits(
    { name: "X25519", public: peerPublicKey },
    keyMaterial.noise.privateKey,
    256,
  );
  return bytesToHex(new Uint8Array(bits));
}

export async function managedRelayDeriveSessionPayloadKeyHex(
  sessionId,
  peerNoisePubkeyHex,
  keyMaterial,
  webCrypto = globalThis.crypto,
) {
  if (!validRelaySessionId(sessionId)) {
    throw new Error("managed relay payload key session_id 형식 오류");
  }
  if (typeof peerNoisePubkeyHex !== "string" || !/^[0-9a-f]{64}$/i.test(peerNoisePubkeyHex)) {
    throw new Error("managed relay peer noise pubkey 형식 오류");
  }
  if (!keyMaterial?.noise?.privateKey) {
    throw new Error("managed relay local noise private key 없음");
  }
  if (!webCrypto?.subtle) {
    throw new Error("managed relay crypto unavailable");
  }
  const sharedSecretHex = await deriveNoiseSharedSecretHex(peerNoisePubkeyHex, keyMaterial, webCrypto);
  const baseKey = await webCrypto.subtle.importKey(
    "raw",
    hexToBytes(sharedSecretHex),
    "HKDF",
    false,
    ["deriveBits"],
  );
  const payloadKeyBits = await webCrypto.subtle.deriveBits(
    {
      name: "HKDF",
      hash: MANAGED_RELAY_PAYLOAD_KEY_HKDF_HASH,
      salt: new TextEncoder().encode(`managed-relay-session:${sessionId}`),
      info: new TextEncoder().encode(MANAGED_RELAY_PAYLOAD_KEY_HKDF_INFO),
    },
    baseKey,
    MANAGED_RELAY_PAYLOAD_KEY_BYTES * 8,
  );
  return bytesToHex(new Uint8Array(payloadKeyBits));
}

function applyIdentity(identity) {
  document.querySelector("#device-id").value = identity.deviceId;
  document.querySelector("#noise-pubkey").value = identity.noisePubkeyHex;
  document.querySelector("#approval-pubkey").value = identity.approvalPubkeyHex;
}

async function publicKeyHex(webCrypto, publicKey) {
  const raw = await webCrypto.subtle.exportKey("raw", publicKey);
  return bytesToHex(new Uint8Array(raw));
}

function bytesToHex(bytes) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function relayTicketHmacKeyBytes(secret) {
  let bytes;
  if (typeof secret === "string") {
    bytes = new TextEncoder().encode(secret);
  } else if (secret instanceof ArrayBuffer) {
    bytes = new Uint8Array(secret);
  } else if (ArrayBuffer.isView(secret)) {
    bytes = new Uint8Array(secret.buffer, secret.byteOffset, secret.byteLength);
  } else if (Array.isArray(secret)) {
    bytes = Uint8Array.from(secret);
  } else {
    throw new Error("relay ticket hmac key 형식 오류");
  }
  if (bytes.byteLength < MIN_RELAY_TICKET_HMAC_KEY_BYTES) {
    throw new Error("relay ticket hmac key too short");
  }
  return bytes;
}

function validCompanionIdentity(identity) {
  return (
    /^[A-Za-z0-9._:-]+$/.test(identity?.deviceId || "") &&
    /^[0-9a-f]{64}$/i.test(identity?.noisePubkeyHex || "") &&
    /^[0-9a-f]{64}$/i.test(identity?.approvalPubkeyHex || "")
  );
}

function validRelayTicketKeyId(value) {
  return typeof value === "string" && /^[A-Za-z0-9._:-]{1,64}$/.test(value);
}

function validRelayTicketKeyVersion(value) {
  return Number.isSafeInteger(value) && value > 0 && value <= 1_000_000;
}

function validManagedRelayTenantId(value) {
  return typeof value === "string" && /^[A-Za-z0-9._:-]{1,96}$/.test(value);
}

function managedRelayVerifierRegistryKey(tenantId, keyId, keyVersion) {
  return `${tenantId}\u0000${keyId}\u0000${keyVersion}`;
}

function validManagedRelayRegistrySnapshotId(value) {
  return typeof value === "string" && /^[A-Za-z0-9._:-]{1,96}$/.test(value);
}

function validManagedRelayRegistrySnapshotReason(value) {
  return typeof value === "string" && /^[A-Za-z0-9._:-]{1,96}$/.test(value);
}

function validManagedRelayQuotaWindow(windowStartMs, windowEndMs) {
  return (
    Number.isSafeInteger(windowStartMs) &&
    windowStartMs > 0 &&
    Number.isSafeInteger(windowEndMs) &&
    windowEndMs > windowStartMs
  );
}

function validManagedRelayQuotaCount(value) {
  return Number.isSafeInteger(value) && value >= 0;
}

function normalizeManagedRelayQuotaCount(value, fallback) {
  if (value === undefined) {
    return fallback;
  }
  if (!validManagedRelayQuotaCount(value)) {
    throw new Error("managed relay quota count 형식 오류");
  }
  return value;
}

function validManagedRelaySourceIpHash(value) {
  return typeof value === "string" && /^[0-9a-f]{16,64}$/i.test(value);
}

function validateManagedRelayTenantSessionRegistrationRequest(registration) {
  const request = {
    tenant_id: registration?.tenant_id,
    session_id: registration?.session_id,
    daemon_device_id: registration?.daemon_device_id,
    verifier_key_id: registration?.verifier_key_id,
    verifier_key_version: registration?.verifier_key_version,
    source_ip_hash: registration?.source_ip_hash,
  };
  if (!validManagedRelayTenantId(request.tenant_id)) {
    throw new Error("managed relay quota registration tenant_id 형식 오류");
  }
  if (!validRelaySessionId(request.session_id)) {
    throw new Error("managed relay quota registration session_id 형식 오류");
  }
  if (!validRelayDeviceId(request.daemon_device_id)) {
    throw new Error("managed relay quota registration daemon_device_id 형식 오류");
  }
  if (!validRelayTicketKeyId(request.verifier_key_id)) {
    throw new Error("managed relay quota registration verifier_key_id 형식 오류");
  }
  if (!validRelayTicketKeyVersion(request.verifier_key_version)) {
    throw new Error("managed relay quota registration verifier_key_version 형식 오류");
  }
  if (!validManagedRelaySourceIpHash(request.source_ip_hash)) {
    throw new Error("managed relay quota registration source_ip_hash 형식 오류");
  }
  assertManagedRelayQuotaMetadataHasNoSecrets(registration, "managed relay quota registration");
  return request;
}

function validateManagedRelayActiveSessionAndByteQuotaRequest(route) {
  const request = {
    tenant_id: route?.tenant_id,
    session_id: route?.session_id,
    daemon_device_id: route?.daemon_device_id,
    verifier_key_id: route?.verifier_key_id,
    verifier_key_version: route?.verifier_key_version,
    frame_sequence: route?.frame_sequence,
    payload_ciphertext_bytes: route?.payload_ciphertext_bytes,
  };
  if (!validManagedRelayTenantId(request.tenant_id)) {
    throw new Error("managed relay active quota route tenant_id 형식 오류");
  }
  if (!validRelaySessionId(request.session_id)) {
    throw new Error("managed relay active quota route session_id 형식 오류");
  }
  if (!validRelayDeviceId(request.daemon_device_id)) {
    throw new Error("managed relay active quota route daemon_device_id 형식 오류");
  }
  if (!validRelayTicketKeyId(request.verifier_key_id)) {
    throw new Error("managed relay active quota route verifier_key_id 형식 오류");
  }
  if (!validRelayTicketKeyVersion(request.verifier_key_version)) {
    throw new Error("managed relay active quota route verifier_key_version 형식 오류");
  }
  if (!Number.isSafeInteger(request.frame_sequence) || request.frame_sequence <= 0) {
    throw new Error("managed relay active quota route frame_sequence 형식 오류");
  }
  if (
    !Number.isSafeInteger(request.payload_ciphertext_bytes) ||
    request.payload_ciphertext_bytes <= 0
  ) {
    throw new Error("managed relay active quota route payload_ciphertext_bytes 형식 오류");
  }
  assertManagedRelayQuotaMetadataHasNoSecrets(route, "managed relay active quota route");
  return request;
}

function quotaDecisionReason({ withinWindow, quotaAvailable }) {
  if (!withinWindow) {
    return "quota-window-not-effective";
  }
  if (!quotaAvailable) {
    return "tenant-session-registration-quota-exceeded";
  }
  return "within-tenant-session-registration-quota";
}

function activeSessionAndByteQuotaDecisionReason({
  withinWindow,
  tenantActiveSessionAvailable,
  daemonDeviceActiveSessionAvailable,
  relayFrameAvailable,
  relayByteAvailable,
}) {
  if (!withinWindow) {
    return "quota-window-not-effective";
  }
  if (!tenantActiveSessionAvailable) {
    return "tenant-active-session-quota-exceeded";
  }
  if (!daemonDeviceActiveSessionAvailable) {
    return "daemon-device-active-session-quota-exceeded";
  }
  if (!relayFrameAvailable) {
    return "relay-frame-quota-exceeded";
  }
  if (!relayByteAvailable) {
    return "relay-byte-quota-exceeded";
  }
  return "within-active-session-and-byte-quota";
}

function assertManagedRelayQuotaMetadataHasNoSecrets(value, label) {
  const json = JSON.stringify(value);
  for (const prohibited of [
    "payload_json",
    "command_text",
    "context_json",
    "approval_response_payload",
    "private_key_material",
    "raw_session_token",
    "session_token",
    "signed_session_ticket",
    "full_setup_json",
    "hmac_secret",
    "secret",
    "mac_hex",
  ]) {
    if (json.includes(prohibited)) {
      throw new Error(`${label} contains prohibited payload or secret data`);
    }
  }
}

function validateManagedRelayPublicVerifierKeyRegistrySnapshot(snapshot) {
  if (!validManagedRelayRegistrySnapshotId(snapshot?.snapshot_id)) {
    throw new Error("managed relay verifier registry snapshot_id 형식 오류");
  }
  if (!Number.isSafeInteger(snapshot.effective_at_ms) || snapshot.effective_at_ms <= 0) {
    throw new Error("managed relay verifier registry effective_at_ms 형식 오류");
  }
  if (
    snapshot.previous_snapshot_id !== undefined &&
    !validManagedRelayRegistrySnapshotId(snapshot.previous_snapshot_id)
  ) {
    throw new Error("managed relay verifier registry previous_snapshot_id 형식 오류");
  }
  if (
    snapshot.reason !== undefined &&
    !validManagedRelayRegistrySnapshotReason(snapshot.reason)
  ) {
    throw new Error("managed relay verifier registry snapshot reason 형식 오류");
  }
  const registry = createManagedRelayPublicVerifierKeyRegistry(snapshot.entries);
  return {
    snapshot_id: snapshot.snapshot_id,
    effective_at_ms: snapshot.effective_at_ms,
    previous_snapshot_id: snapshot.previous_snapshot_id,
    reason: snapshot.reason,
    entries: registry.entries,
  };
}

function validateManagedRelayPublicVerifierKeyEntry(entry) {
  if (!validManagedRelayTenantId(entry?.tenant_id)) {
    throw new Error("managed relay verifier tenant_id 형식 오류");
  }
  if (!validRelayTicketKeyId(entry.key_id)) {
    throw new Error("managed relay verifier key_id 형식 오류");
  }
  if (!validRelayTicketKeyVersion(entry.key_version)) {
    throw new Error("managed relay verifier key_version 형식 오류");
  }
  if (entry.public_key_alg !== MANAGED_RELAY_PUBLIC_VERIFIER_KEY_ALG) {
    throw new Error("managed relay verifier public_key_alg 형식 오류");
  }
  if (!validRelayPubkeyHex(entry.public_key_hex)) {
    throw new Error("managed relay verifier public_key_hex 형식 오류");
  }
  if (!PWA_RELAY_MANAGED_VERIFIER_KEY_OPERATIONS_POLICY.requiredKeyStates.includes(entry.state)) {
    throw new Error("managed relay verifier state 형식 오류");
  }
  if (
    !Number.isSafeInteger(entry.not_before_ms) ||
    entry.not_before_ms <= 0 ||
    !Number.isSafeInteger(entry.expires_at_ms) ||
    entry.expires_at_ms <= entry.not_before_ms
  ) {
    throw new Error("managed relay verifier validity window 형식 오류");
  }
}

function validPrivateNetworkName(value) {
  return typeof value === "string" && /^[A-Za-z0-9._:-]{3,96}$/.test(value);
}

function validRelayWebSocketEndpointUrl(value) {
  try {
    const url = new URL(value);
    if (url.protocol === "wss:") {
      return true;
    }
    return (
      url.protocol === "ws:" &&
      (url.hostname === "127.0.0.1" || url.hostname === "localhost" || url.hostname === "[::1]")
    );
  } catch {
    return false;
  }
}

function validateRelayRouteEnvelopeMetadata(route) {
  if (route?.relay_protocol_version !== RELAY_TRANSPORT_PROTOCOL_VERSION) {
    throw new Error("지원하지 않는 relay route protocol_version");
  }
  if (!validRelaySessionId(route.session_id)) {
    throw new Error("relay route session_id 형식 오류");
  }
  if (!validRelaySender(route.sender)) {
    throw new Error("relay route sender 형식 오류");
  }
  if (!Number.isSafeInteger(route.sequence) || route.sequence <= 0) {
    throw new Error("relay route sequence 형식 오류");
  }
  if (!Number.isSafeInteger(route.sent_at_ms) || route.sent_at_ms <= 0) {
    throw new Error("relay route sent_at_ms 형식 오류");
  }
  if (!Number.isSafeInteger(route.expires_at_ms) || route.expires_at_ms <= route.sent_at_ms) {
    throw new Error("relay route expires_at_ms 형식 오류");
  }
  if (!Number.isSafeInteger(route.payload_json_bytes) || route.payload_json_bytes <= 0) {
    throw new Error("relay route payload_json_bytes 형식 오류");
  }
}

function validateRelayEndpointLoop(loop) {
  validateRelaySessionConnectMetadata(loop?.connect);
  validateRelayEndpoint(loop?.endpoint);
  if (loop.connect.session_id !== loop.endpoint.sessionId || loop.connect.peer !== loop.endpoint.sender) {
    throw new Error("relay endpoint loop connect mismatch");
  }
  for (const key of ["sentCount", "queuedCount", "receivedCount", "droppedCount", "errorCount"]) {
    if (!Number.isSafeInteger(loop[key]) || loop[key] < 0) {
      throw new Error(`relay endpoint loop ${key} 형식 오류`);
    }
  }
  if (typeof loop.connected !== "boolean") {
    throw new Error("relay endpoint loop connected 형식 오류");
  }
}

function parseRelayWebSocketEnvelope(socketMessage) {
  let message = socketMessage;
  if (typeof socketMessage === "string") {
    try {
      message = JSON.parse(socketMessage);
    } catch {
      throw new Error("relay websocket envelope JSON 파싱 실패");
    }
  }
  if (!message || typeof message !== "object" || Array.isArray(message)) {
    throw new Error("relay websocket envelope 형식 오류");
  }
  if (typeof message.kind !== "string" || message.kind.length === 0) {
    throw new Error("relay websocket envelope kind 형식 오류");
  }
  return message;
}

function rejectRelaySetupSecretFields(value, path = "$", depth = 0) {
  if (value === null || typeof value !== "object" || depth > 16) {
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((item, index) => rejectRelaySetupSecretFields(item, `${path}[${index}]`, depth + 1));
    return;
  }
  for (const [key, nested] of Object.entries(value)) {
    const normalized = key.replaceAll("_", "").toLowerCase();
    if (normalized === "secret" || normalized === "hmacsha256keys") {
      throw new Error(`relay setup secret field not allowed: ${path}.${key}`);
    }
    rejectRelaySetupSecretFields(nested, `${path}.${key}`, depth + 1);
  }
}

function constantTimeHexEqual(left, right) {
  if (
    typeof left !== "string" ||
    typeof right !== "string" ||
    !/^[0-9a-f]+$/i.test(left) ||
    !/^[0-9a-f]+$/i.test(right) ||
    left.length !== right.length
  ) {
    return false;
  }
  let diff = 0;
  const a = left.toLowerCase();
  const b = right.toLowerCase();
  for (let i = 0; i < a.length; i += 1) {
    diff |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }
  return diff === 0;
}

function hexToBytes(hex) {
  if (!/^[0-9a-f]*$/i.test(hex) || hex.length % 2 !== 0) {
    throw new Error("hex 형식 오류");
  }
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i += 1) {
    out[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

function openIdentityDb(indexedDb) {
  return new Promise((resolve, reject) => {
    const request = indexedDb.open(COMPANION_IDENTITY_DB, 1);
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains(COMPANION_IDENTITY_STORE)) {
        db.createObjectStore(COMPANION_IDENTITY_STORE, { keyPath: "id" });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error || new Error("IndexedDB open 실패"));
  });
}

function idbPut(db, storeName, value) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, "readwrite");
    tx.objectStore(storeName).put(value);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error || new Error("IndexedDB write 실패"));
    tx.onabort = () => reject(tx.error || new Error("IndexedDB write 중단"));
  });
}

function idbGet(db, storeName, key) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, "readonly");
    const request = tx.objectStore(storeName).get(key);
    request.onsuccess = () => resolve(request.result || null);
    request.onerror = () => reject(request.error || new Error("IndexedDB read 실패"));
  });
}

function memoryStorageFor(identity) {
  return {
    getItem: () => JSON.stringify(identity),
  };
}

function shellToken(value) {
  const clean = value.trim();
  if (!clean) return "";
  if (/^[A-Za-z0-9._:-]+$/.test(clean)) return clean;
  return `'${clean.replaceAll("'", "'\\''")}'`;
}

function formatExpiry(ms) {
  if (!Number.isSafeInteger(ms) || ms <= 0) return "-";
  const date = new Date(ms);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleString();
}

function formatMonitorTime(ms) {
  if (!Number.isSafeInteger(ms) || ms <= 0) return "-";
  const date = new Date(ms);
  if (Number.isNaN(date.getTime())) return "-";
  return date.toLocaleTimeString();
}

function renderPayload(payload) {
  document.querySelector("#pair-code").textContent = payload.pairing_code;
  document.querySelector("#pair-expires").textContent = formatExpiry(payload.expires_at_ms);
  document.querySelector("#pair-transport").textContent = payload.transport_addr;
  document.querySelector("#pair-key").textContent = payload.daemon_pubkey_hex;
  updateCommand(payload);
}

function renderApprovalRequest(request, source = "Manual") {
  document.querySelector("#approval-command").textContent = request.command_masked;
  document.querySelector("#approval-context").textContent = request.context_hash;
  document.querySelector("#approval-source").textContent = source;
}

function renderApprovalVerifyCommand(request, response) {
  document.querySelector("#approval-verify-command").textContent = commandForApprovalVerify(
    request,
    response,
    document.querySelector("#device-id").value,
  );
}

function updateCommand(payload) {
  const device = {
    deviceId: document.querySelector("#device-id").value,
    noisePubkeyHex: document.querySelector("#noise-pubkey").value,
    approvalPubkeyHex: document.querySelector("#approval-pubkey").value,
  };
  document.querySelector("#complete-command").textContent = commandForPairing(payload, device);
}

function setStatus(text, kind = "") {
  const el = document.querySelector("#pair-status");
  el.textContent = text;
  el.className = `status-line ${kind}`.trim();
}

function setLiveState(text, kind = "") {
  const el = document.querySelector("#live-state");
  el.textContent = text;
  el.className = kind;
}

function setLiveLastEvent(text) {
  document.querySelector("#live-last-event").textContent = text;
}

function renderLiveQueue(queue) {
  document.querySelector("#live-pending-count").textContent = String(queue.length);
  const list = document.querySelector("#live-approval-list");
  list.replaceChildren();
  if (queue.length === 0) {
    const empty = document.createElement("li");
    empty.className = "empty";
    empty.textContent = "No live approvals";
    list.append(empty);
    return;
  }
  for (const item of queue) {
    const li = document.createElement("li");
    li.textContent = `${item.request.command_masked} | ${item.request.context_hash}`;
    list.append(li);
  }
}

function renderRelayQueue(queue) {
  document.querySelector("#relay-pending-count").textContent = String(queue.length);
  const list = document.querySelector("#relay-approval-list");
  list.replaceChildren();
  if (queue.length === 0) {
    const empty = document.createElement("li");
    empty.className = "empty";
    empty.textContent = "No relay approvals";
    list.append(empty);
    return;
  }
  for (const item of queue) {
    const li = document.createElement("li");
    li.textContent = `${item.request.command_masked} | ${item.request.context_hash}`;
    list.append(li);
  }
}

function renderRelayPrivateQueue(queue) {
  document.querySelector("#relay-private-pending-count").textContent = String(queue.length);
  const list = document.querySelector("#relay-private-approval-list");
  list.replaceChildren();
  if (queue.length === 0) {
    const empty = document.createElement("li");
    empty.className = "empty";
    empty.textContent = "No private-network relay approvals";
    list.append(empty);
    return;
  }
  for (const item of queue) {
    const li = document.createElement("li");
    li.textContent = `${item.request.command_masked} | ${item.request.context_hash}`;
    list.append(li);
  }
}

function renderMonitor(monitor) {
  document.querySelector("#monitor-state").textContent = monitor.state;
  document.querySelector("#monitor-endpoint").textContent = monitor.endpoint || "-";
  document.querySelector("#monitor-device").textContent = monitor.deviceId || "-";
  document.querySelector("#monitor-pending").textContent = String(monitor.pendingCount);
  document.querySelector("#monitor-received").textContent = String(monitor.receivedCount);
  document.querySelector("#monitor-sent").textContent = String(monitor.sentCount);
  document.querySelector("#monitor-approved").textContent = String(monitor.approvedCount);
  document.querySelector("#monitor-rejected").textContent = String(monitor.rejectedCount);
  document.querySelector("#monitor-heartbeat").textContent = formatMonitorTime(monitor.lastHeartbeatAtMs);
  document.querySelector("#monitor-response").textContent = formatMonitorTime(monitor.lastResponseAtMs);

  const list = document.querySelector("#monitor-event-log");
  list.replaceChildren();
  if (!monitor.history.length) {
    const empty = document.createElement("li");
    empty.className = "empty";
    empty.textContent = "No monitor events";
    list.append(empty);
    return;
  }
  for (const event of monitor.history) {
    const li = document.createElement("li");
    li.textContent = `${formatMonitorTime(event.atMs)} | ${event.label}`;
    list.append(li);
  }
}

function relayBlockerText(code) {
  return (
    {
      transport_mode_not_relay: "transport mode is not relay",
      relay_endpoint_url_missing: "relay endpoint URL missing",
      relay_endpoint_url_invalid: "relay endpoint URL invalid",
      relay_deployment_mode_missing: "deployment mode missing",
      relay_deployment_mode_invalid: "deployment mode invalid",
      relay_deployment_mode_not_selected: "deployment mode is not self-hosted",
      private_network_deployment_mode_required: "deployment mode is not private-network",
      private_network_name_invalid: "private network name invalid",
      relay_operator_setup_text_missing: "operator setup text missing",
      companion_identity_missing: "companion identity missing",
      relay_signed_ticket_missing: "signed relay ticket missing",
      relay_signed_ticket_invalid: "signed relay ticket invalid",
      relay_signed_ticket_expired: "signed relay ticket expired",
      relay_ticket_identity_mismatch: "ticket and companion identity mismatch",
      relay_now_ms_invalid: "local clock invalid",
    }[code] || code
  );
}

function renderRelayBlockerList(selector, blockers, emptyText) {
  const list = document.querySelector(selector);
  list.replaceChildren();
  const items = blockers.length ? blockers.map(relayBlockerText) : [emptyText];
  for (const text of items) {
    const li = document.createElement("li");
    li.className = blockers.length ? "blocked" : "ready";
    li.textContent = text;
    list.append(li);
  }
}

function renderRelayBlockers(blockers, emptyText) {
  renderRelayBlockerList("#relay-blocker-list", blockers, emptyText);
}

function setRelayState(text, kind = "") {
  const el = document.querySelector("#relay-state");
  el.textContent = text;
  el.className = kind;
}

function setRelayPrivateState(text, kind = "") {
  const el = document.querySelector("#relay-private-state");
  el.textContent = text;
  el.className = kind;
}

function setRelayConnectionState(text, kind = "") {
  const el = document.querySelector("#relay-connection-state");
  el.textContent = text;
  el.className = kind;
}

function setRelayPrivateConnectionState(text, kind = "") {
  const el = document.querySelector("#relay-private-connection-state");
  el.textContent = text;
  el.className = kind;
}

function setRelayLastEvent(text) {
  document.querySelector("#relay-last-event").textContent = text;
}

function setRelayPrivateLastEvent(text) {
  document.querySelector("#relay-private-last-event").textContent = text;
}

function renderRelayRuntime(monitor) {
  setRelayConnectionState(monitor.state, monitor.state === "Connected" ? "ok" : "");
  document.querySelector("#relay-pending-count").textContent = String(monitor.pendingCount);
  document.querySelector("#relay-received-count").textContent = String(monitor.receivedCount);
  document.querySelector("#relay-sent-count").textContent = String(monitor.sentCount);
  document.querySelector("#relay-approved-count").textContent = String(monitor.approvedCount);
  document.querySelector("#relay-rejected-count").textContent = String(monitor.rejectedCount);
}

function renderRelayPrivateRuntime(monitor) {
  setRelayPrivateConnectionState(monitor.state, monitor.state === "Connected" ? "ok" : "");
  document.querySelector("#relay-private-pending-count").textContent = String(monitor.pendingCount);
  document.querySelector("#relay-private-received-count").textContent = String(monitor.receivedCount);
  document.querySelector("#relay-private-sent-count").textContent = String(monitor.sentCount);
  document.querySelector("#relay-private-approved-count").textContent = String(monitor.approvedCount);
  document.querySelector("#relay-private-rejected-count").textContent = String(monitor.rejectedCount);
}

function renderRelaySetup(setup = null, preflight = null) {
  const ticket = setup?.signedSessionTicket?.ticket || null;
  const keyId = setup?.signedSessionTicket?.key_id || "-";
  const blockers = preflight?.blockers || [];
  const ready = Boolean(setup && preflight?.status === "ready" && blockers.length === 0);

  setRelayState(setup ? (ready ? "Ready" : "Blocked") : "No setup", setup ? (ready ? "ok" : "error") : "");
  document.querySelector("#relay-default-mode").textContent = PWA_TRANSPORT_MODE_LIVE_LOOPBACK;
  document.querySelector("#relay-endpoint").textContent = setup?.relayEndpointUrl || "-";
  document.querySelector("#relay-deployment").textContent = setup?.deploymentMode || "-";
  document.querySelector("#relay-device").textContent = setup?.companionIdentity?.deviceId || "-";
  document.querySelector("#relay-session").textContent = ticket?.session_id || "-";
  document.querySelector("#relay-expires").textContent = formatExpiry(ticket?.expires_at_ms || 0);
  document.querySelector("#relay-ticket-key").textContent = keyId;
  document.querySelector("#relay-companion-connect").textContent = setup
    ? relaySessionConnectJson(setup.companionConnect)
    : "-";
  document.querySelector("#relay-daemon-connect").textContent = setup
    ? relaySessionConnectJson(setup.daemonConnect)
    : "-";
  renderRelayBlockers(blockers, setup ? "Relay setup ready" : "No relay setup loaded");
}

function renderRelaySetupError(message) {
  renderRelaySetup();
  setRelayState("Invalid", "error");
  renderRelayBlockers([message], "");
}

function renderRelayPrivateNetworkSetup(setup = null, preflight = null) {
  const ticket = setup?.signedSessionTicket?.ticket || null;
  const blockers = preflight?.blockers || [];
  const ready = Boolean(setup && preflight?.status === "ready" && blockers.length === 0);

  setRelayPrivateState(setup ? (ready ? "Ready" : "Blocked") : "No setup", setup ? (ready ? "ok" : "error") : "");
  document.querySelector("#relay-private-default-mode").textContent = PWA_TRANSPORT_MODE_LIVE_LOOPBACK;
  document.querySelector("#relay-private-network").textContent = setup?.privateNetworkName || "-";
  document.querySelector("#relay-private-endpoint").textContent = setup?.relayEndpointUrl || "-";
  document.querySelector("#relay-private-deployment").textContent = setup?.deploymentMode || "-";
  document.querySelector("#relay-private-device").textContent = setup?.companionIdentity?.deviceId || "-";
  document.querySelector("#relay-private-session").textContent = ticket?.session_id || "-";
  document.querySelector("#relay-private-expires").textContent = formatExpiry(ticket?.expires_at_ms || 0);
  document.querySelector("#relay-private-companion-connect").textContent = setup
    ? relaySessionConnectJson(setup.companionConnect)
    : "-";
  document.querySelector("#relay-private-daemon-connect").textContent = setup
    ? relaySessionConnectJson(setup.daemonConnect)
    : "-";
  renderRelayBlockerList(
    "#relay-private-blocker-list",
    blockers,
    setup ? "Private-network relay setup ready" : "No private-network setup loaded",
  );
}

function renderRelayPrivateNetworkSetupError(message) {
  renderRelayPrivateNetworkSetup();
  setRelayPrivateState("Invalid", "error");
  renderRelayBlockerList("#relay-private-blocker-list", [message], "");
}

function init() {
  const input = document.querySelector("#payload-input");
  const approvalInput = document.querySelector("#approval-input");
  const parse = document.querySelector("#parse-button");
  const clear = document.querySelector("#clear-button");
  const identity = document.querySelector("#identity-button");
  const approvalParse = document.querySelector("#approval-parse-button");
  const approveButton = document.querySelector("#approve-button");
  const rejectButton = document.querySelector("#reject-button");
  const copyResponse = document.querySelector("#copy-response-button");
  const copyVerify = document.querySelector("#copy-verify-button");
  const liveEndpointInput = document.querySelector("#live-endpoint");
  const liveConnectButton = document.querySelector("#live-connect-button");
  const liveDisconnectButton = document.querySelector("#live-disconnect-button");
  const relaySetupInput = document.querySelector("#relay-setup-input");
  const relaySetupLoadButton = document.querySelector("#relay-setup-load-button");
  const relaySetupClearButton = document.querySelector("#relay-setup-clear-button");
  const relayPrivateSetupInput = document.querySelector("#relay-private-setup-input");
  const relayPrivateLoadButton = document.querySelector("#relay-private-load-button");
  const relayPrivateClearButton = document.querySelector("#relay-private-clear-button");
  const relayPrivateConnectButton = document.querySelector("#relay-private-connect-button");
  const relayPrivateDisconnectButton = document.querySelector("#relay-private-disconnect-button");
  const relayConnectButton = document.querySelector("#relay-connect-button");
  const relayDisconnectButton = document.querySelector("#relay-disconnect-button");
  let activePayload = null;
  let activeApprovalRequest = null;
  let activeApprovalResponse = null;
  let activeApprovalTransport = "manual";
  let activeRelaySetup = null;
  let activeRelayPrivateSetup = null;
  let activeRelayLoop = null;
  let activeRelayPrivateLoop = null;
  let activeKeyMaterial = null;
  let liveBaseUrl = "";
  let liveEventSource = null;
  let liveApprovalQueue = [];
  let relaySocket = null;
  let relayApprovalQueue = [];
  let relayPrivateSocket = null;
  let relayPrivateApprovalQueue = [];
  let liveMonitor = liveMonitorInitialState();
  let relayMonitor = liveMonitorInitialState();
  let relayPrivateMonitor = liveMonitorInitialState();
  renderMonitor(liveMonitor);
  renderRelaySetup();
  renderRelayPrivateNetworkSetup();
  renderRelayQueue(relayApprovalQueue);
  renderRelayRuntime(relayMonitor);
  renderRelayPrivateQueue(relayPrivateApprovalQueue);
  renderRelayPrivateRuntime(relayPrivateMonitor);

  function updateMonitor(event) {
    liveMonitor = liveMonitorNext(liveMonitor, event);
    renderMonitor(liveMonitor);
  }

  function updateRelayMonitor(event) {
    relayMonitor = liveMonitorNext(relayMonitor, event);
    renderRelayRuntime(relayMonitor);
  }

  function updateRelayPrivateMonitor(event) {
    relayPrivateMonitor = liveMonitorNext(relayPrivateMonitor, event);
    renderRelayPrivateRuntime(relayPrivateMonitor);
  }

  async function loadActiveIdentityAndKeys() {
    const savedIdentity = loadCompanionIdentity(window.localStorage);
    const record = await loadCompanionKeyMaterial(window.indexedDB);
    const identity = savedIdentity || record?.identity || null;
    if (!identity) {
      throw new Error("Companion identity 없음");
    }
    if (record?.identity?.deviceId === identity.deviceId) {
      activeKeyMaterial = record.keyMaterial;
    }
    if (!activeKeyMaterial) {
      throw new Error("approval private key 없음");
    }
    applyIdentity(identity);
    return identity;
  }

  function closeLiveEvents(stateText = "Disconnected") {
    liveEventSource?.close();
    liveEventSource = null;
    liveBaseUrl = "";
    setLiveState(stateText);
    liveConnectButton.disabled = false;
    liveDisconnectButton.disabled = true;
    updateMonitor({ type: stateText === "Waiting" ? "waiting" : "disconnected", label: stateText });
  }

  function closeRelaySocket(stateText = "Disconnected") {
    if (relaySocket) {
      const socket = relaySocket;
      relaySocket = null;
      try {
        if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
          socket.close();
        }
      } catch {
        // Best-effort UI cleanup only.
      }
    }
    activeRelayLoop = null;
    relayConnectButton.disabled = !(activeRelaySetup && relayRuntimeSetupPreflight(activeRelaySetup).relayEnabled);
    relayDisconnectButton.disabled = true;
    setRelayConnectionState(stateText);
    updateRelayMonitor({ type: stateText === "Waiting" ? "waiting" : "disconnected", label: stateText });
  }

  function closeRelayPrivateSocket(stateText = "Disconnected") {
    if (relayPrivateSocket) {
      const socket = relayPrivateSocket;
      relayPrivateSocket = null;
      try {
        if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
          socket.close();
        }
      } catch {
        // Best-effort UI cleanup only.
      }
    }
    activeRelayPrivateLoop = null;
    relayPrivateConnectButton.disabled = !(
      activeRelayPrivateSetup &&
      relayPrivateNetworkRuntimeSetupPreflight(activeRelayPrivateSetup).contractReady
    );
    relayPrivateDisconnectButton.disabled = true;
    setRelayPrivateConnectionState(stateText);
    updateRelayPrivateMonitor({ type: stateText === "Waiting" ? "waiting" : "disconnected", label: stateText });
  }

  function relaySetupMatchesIdentity(setup, identity) {
    return (
      setup?.companionIdentity?.deviceId === identity?.deviceId &&
      setup?.companionIdentity?.noisePubkeyHex === identity?.noisePubkeyHex &&
      setup?.companionIdentity?.approvalPubkeyHex === identity?.approvalPubkeyHex
    );
  }

  function handleLiveEventData(data) {
    const message = parseLiveTransportMessage(data);
    setLiveLastEvent(message.type);
    if (message.type === "approval_request") {
      liveApprovalQueue = liveApprovalQueueNext(liveApprovalQueue, message);
      activeApprovalRequest = message.request;
      activeApprovalResponse = null;
      activeApprovalTransport = "live";
      approvalInput.value = JSON.stringify(activeApprovalRequest, null, 2);
      document.querySelector("#approval-response").textContent = "-";
      document.querySelector("#approval-verify-command").textContent = "-";
      renderApprovalRequest(activeApprovalRequest, "Live");
      renderLiveQueue(liveApprovalQueue);
      updateMonitor({
        type: "approval_request",
        label: `approval_request ${message.request.command_masked}`,
        pendingCount: liveApprovalQueue.length,
      });
      setStatus("Live 승인 요청 수신됨", "ok");
      return;
    }
    if (message.type === "ping") {
      setLiveState("Connected", "ok");
      updateMonitor({ type: "ping", label: `ping ${message.nonce}` });
    }
  }

  function handleRelayLiveMessage(message) {
    setRelayLastEvent(message.type);
    if (message.type === "approval_request") {
      relayApprovalQueue = liveApprovalQueueNext(relayApprovalQueue, message);
      activeApprovalRequest = message.request;
      activeApprovalResponse = null;
      activeApprovalTransport = "relay";
      approvalInput.value = JSON.stringify(activeApprovalRequest, null, 2);
      document.querySelector("#approval-response").textContent = "-";
      document.querySelector("#approval-verify-command").textContent = "-";
      renderApprovalRequest(activeApprovalRequest, "Relay");
      renderRelayQueue(relayApprovalQueue);
      updateRelayMonitor({
        type: "approval_request",
        label: `relay approval_request ${message.request.command_masked}`,
        pendingCount: relayApprovalQueue.length,
      });
      setStatus("Relay 승인 요청 수신됨", "ok");
      return;
    }
    if (message.type === "ping") {
      setRelayConnectionState("Connected", "ok");
      updateRelayMonitor({ type: "ping", label: `relay ping ${message.nonce}` });
    }
  }

  function handleRelaySocketMessage(data) {
    if (!activeRelayLoop) {
      throw new Error("relay endpoint loop 없음");
    }
    const result = relayEndpointLoopAcceptSocketMessage(activeRelayLoop, data);
    if (result.kind === "connected") {
      setRelayConnectionState("Connected", "ok");
      setRelayLastEvent("connected");
      relayConnectButton.disabled = true;
      relayDisconnectButton.disabled = false;
      updateRelayMonitor({
        type: "connected",
        label: "relay connected",
        endpoint: activeRelaySetup?.relayEndpointUrl || "",
        deviceId: activeRelaySetup?.companionIdentity?.deviceId || "",
      });
      setStatus("Relay companion 연결됨", "ok");
      return;
    }
    if (result.kind === "queued") {
      setRelayLastEvent("queued");
      return;
    }
    if (result.kind === "live_message") {
      handleRelayLiveMessage(result.liveMessage);
      return;
    }
    if (result.kind === "dropped") {
      setRelayLastEvent("dropped");
      updateRelayMonitor({ type: "error", label: "relay frame dropped" });
      return;
    }
    if (result.kind === "error") {
      throw new Error(result.message);
    }
  }

  function handleRelayPrivateLiveMessage(message) {
    setRelayPrivateLastEvent(message.type);
    if (message.type === "approval_request") {
      relayPrivateApprovalQueue = liveApprovalQueueNext(relayPrivateApprovalQueue, message);
      activeApprovalRequest = message.request;
      activeApprovalResponse = null;
      activeApprovalTransport = "relay-private";
      approvalInput.value = JSON.stringify(activeApprovalRequest, null, 2);
      document.querySelector("#approval-response").textContent = "-";
      document.querySelector("#approval-verify-command").textContent = "-";
      renderApprovalRequest(activeApprovalRequest, "Private Relay");
      renderRelayPrivateQueue(relayPrivateApprovalQueue);
      updateRelayPrivateMonitor({
        type: "approval_request",
        label: `private relay approval_request ${message.request.command_masked}`,
        pendingCount: relayPrivateApprovalQueue.length,
      });
      setStatus("Private-network relay 승인 요청 수신됨", "ok");
      return;
    }
    if (message.type === "ping") {
      setRelayPrivateConnectionState("Connected", "ok");
      updateRelayPrivateMonitor({ type: "ping", label: `private relay ping ${message.nonce}` });
    }
  }

  function handleRelayPrivateSocketMessage(data) {
    if (!activeRelayPrivateLoop) {
      throw new Error("private-network relay endpoint loop 없음");
    }
    const result = relayEndpointLoopAcceptSocketMessage(activeRelayPrivateLoop, data);
    if (result.kind === "connected") {
      setRelayPrivateConnectionState("Connected", "ok");
      setRelayPrivateLastEvent("connected");
      relayPrivateConnectButton.disabled = true;
      relayPrivateDisconnectButton.disabled = false;
      updateRelayPrivateMonitor({
        type: "connected",
        label: "private relay connected",
        endpoint: activeRelayPrivateSetup?.relayEndpointUrl || "",
        deviceId: activeRelayPrivateSetup?.companionIdentity?.deviceId || "",
      });
      setStatus("Private-network relay companion 연결됨", "ok");
      return;
    }
    if (result.kind === "queued") {
      setRelayPrivateLastEvent("queued");
      return;
    }
    if (result.kind === "live_message") {
      handleRelayPrivateLiveMessage(result.liveMessage);
      return;
    }
    if (result.kind === "dropped") {
      setRelayPrivateLastEvent("dropped");
      updateRelayPrivateMonitor({ type: "error", label: "private relay frame dropped" });
      return;
    }
    if (result.kind === "error") {
      throw new Error(result.message);
    }
  }

  function openLiveEvents() {
    if (typeof window.EventSource !== "function") {
      throw new Error("EventSource 미지원");
    }
    liveEventSource?.close();
    liveEventSource = new window.EventSource(liveEventSourceUrl(liveBaseUrl));
    liveEventSource.onopen = () => setLiveState("Connected", "ok");
    liveEventSource.onmessage = (event) => {
      try {
        handleLiveEventData(event.data);
      } catch (err) {
        updateMonitor({ type: "error", label: err.message });
        setStatus(err.message, "error");
      }
    };
    liveEventSource.onerror = () => {
      if (liveBaseUrl) {
        setLiveState("Waiting");
        updateMonitor({ type: "waiting", label: "EventSource waiting" });
      }
    };
  }

  async function connectRelay() {
    relayConnectButton.disabled = true;
    try {
      if (!activeRelaySetup) {
        activeRelaySetup = parseRelayRuntimeSetupInput(relaySetupInput.value);
        relaySetupInput.value = JSON.stringify(activeRelaySetup, null, 2);
      }
      const preflight = relayRuntimeSetupPreflight(activeRelaySetup);
      renderRelaySetup(activeRelaySetup, preflight);
      if (!preflight.relayEnabled) {
        throw new Error(`relay setup blocked: ${preflight.blockers.join(",")}`);
      }
      const identity = await loadActiveIdentityAndKeys();
      if (!relaySetupMatchesIdentity(activeRelaySetup, identity)) {
        throw new Error("relay setup companion identity mismatch");
      }
      activeRelayLoop = relayCompanionEndpointLoopFromSetup(activeRelaySetup);
      const socket = new WebSocket(activeRelayLoop.webSocketUrl);
      relaySocket = socket;
      setRelayConnectionState("Connecting");
      setRelayLastEvent("connecting");
      socket.addEventListener("open", () => {
        try {
          socket.send(relayEndpointLoopConnectJson(activeRelayLoop));
        } catch (err) {
          setStatus(err.message, "error");
          closeRelaySocket("Disconnected");
        }
      });
      socket.addEventListener("message", (event) => {
        try {
          handleRelaySocketMessage(event.data);
        } catch (err) {
          updateRelayMonitor({ type: "error", label: err.message });
          setStatus(err.message, "error");
        }
      });
      socket.addEventListener("error", () => {
        updateRelayMonitor({ type: "error", label: "relay websocket error" });
        setStatus("Relay websocket 오류", "error");
      });
      socket.addEventListener("close", () => {
        if (relaySocket === socket) {
          relaySocket = null;
          activeRelayLoop = null;
          relayConnectButton.disabled = !(activeRelaySetup && relayRuntimeSetupPreflight(activeRelaySetup).relayEnabled);
          relayDisconnectButton.disabled = true;
          setRelayConnectionState("Disconnected");
          setRelayLastEvent("closed");
        }
      });
    } catch (err) {
      closeRelaySocket("Disconnected");
      updateRelayMonitor({ type: "error", label: err.message });
      setStatus(err.message, "error");
    } finally {
      relayConnectButton.disabled = Boolean(relaySocket);
    }
  }

  async function connectRelayPrivateNetwork() {
    relayPrivateConnectButton.disabled = true;
    try {
      if (!activeRelayPrivateSetup) {
        activeRelayPrivateSetup = parseRelayPrivateNetworkRuntimeSetupInput(relayPrivateSetupInput.value);
        relayPrivateSetupInput.value = JSON.stringify(activeRelayPrivateSetup, null, 2);
      }
      const preflight = relayPrivateNetworkRuntimeSetupPreflight(activeRelayPrivateSetup);
      renderRelayPrivateNetworkSetup(activeRelayPrivateSetup, preflight);
      if (!preflight.contractReady) {
        throw new Error(`private-network relay setup blocked: ${preflight.blockers.join(",")}`);
      }
      const identity = await loadActiveIdentityAndKeys();
      if (!relaySetupMatchesIdentity(activeRelayPrivateSetup, identity)) {
        throw new Error("private-network relay setup companion identity mismatch");
      }
      activeRelayPrivateLoop = relayPrivateNetworkCompanionEndpointLoopFromSetup(activeRelayPrivateSetup);
      const socket = new WebSocket(activeRelayPrivateLoop.webSocketUrl);
      relayPrivateSocket = socket;
      setRelayPrivateConnectionState("Connecting");
      setRelayPrivateLastEvent("connecting");
      socket.addEventListener("open", () => {
        try {
          socket.send(relayEndpointLoopConnectJson(activeRelayPrivateLoop));
        } catch (err) {
          setStatus(err.message, "error");
          closeRelayPrivateSocket("Disconnected");
        }
      });
      socket.addEventListener("message", (event) => {
        try {
          handleRelayPrivateSocketMessage(event.data);
        } catch (err) {
          updateRelayPrivateMonitor({ type: "error", label: err.message });
          setStatus(err.message, "error");
        }
      });
      socket.addEventListener("error", () => {
        updateRelayPrivateMonitor({ type: "error", label: "private relay websocket error" });
        setStatus("Private-network relay websocket 오류", "error");
      });
      socket.addEventListener("close", () => {
        if (relayPrivateSocket === socket) {
          relayPrivateSocket = null;
          activeRelayPrivateLoop = null;
          relayPrivateConnectButton.disabled = !(
            activeRelayPrivateSetup &&
            relayPrivateNetworkRuntimeSetupPreflight(activeRelayPrivateSetup).contractReady
          );
          relayPrivateDisconnectButton.disabled = true;
          setRelayPrivateConnectionState("Disconnected");
          setRelayPrivateLastEvent("closed");
        }
      });
    } catch (err) {
      closeRelayPrivateSocket("Disconnected");
      updateRelayPrivateMonitor({ type: "error", label: err.message });
      setStatus(err.message, "error");
    } finally {
      relayPrivateConnectButton.disabled = relayPrivateSocket
        ? true
        : !(
            activeRelayPrivateSetup &&
            relayPrivateNetworkRuntimeSetupPreflight(activeRelayPrivateSetup).contractReady
          );
    }
  }

  async function connectLive() {
    liveConnectButton.disabled = true;
    try {
      const urls = liveEndpointUrls(liveEndpointInput.value.trim());
      liveBaseUrl = urls.baseUrl;
      liveEndpointInput.value = liveBaseUrl;
      const identity = await loadActiveIdentityAndKeys();
      await postLiveTransportMessage(liveBaseUrl, liveHelloMessage(identity));
      openLiveEvents();
      setLiveState("Connected", "ok");
      setLiveLastEvent("hello");
      setStatus("Live companion 연결됨", "ok");
      updateMonitor({
        type: "connected",
        label: "hello",
        endpoint: liveBaseUrl,
        deviceId: identity.deviceId,
      });
      liveDisconnectButton.disabled = false;
    } catch (err) {
      closeLiveEvents("Disconnected");
      updateMonitor({ type: "error", label: err.message });
      setStatus(err.message, "error");
    } finally {
      liveConnectButton.disabled = Boolean(liveBaseUrl);
    }
  }

  function parseInput() {
    try {
      activePayload = parsePairingInput(input.value, window.location.search);
      input.value = JSON.stringify(activePayload, null, 2);
      renderPayload(activePayload);
      setStatus("페어링 payload 확인됨", "ok");
    } catch (err) {
      activePayload = null;
      setStatus(err.message, "error");
    }
  }

  function loadRelaySetup() {
    try {
      closeRelaySocket("Disconnected");
      relayApprovalQueue = [];
      relayMonitor = liveMonitorInitialState();
      activeRelaySetup = parseRelayRuntimeSetupInput(relaySetupInput.value);
      relaySetupInput.value = JSON.stringify(activeRelaySetup, null, 2);
      const preflight = relayRuntimeSetupPreflight(activeRelaySetup);
      renderRelaySetup(activeRelaySetup, preflight);
      renderRelayQueue(relayApprovalQueue);
      renderRelayRuntime(relayMonitor);
      relayConnectButton.disabled = !preflight.relayEnabled;
      setStatus(preflight.relayEnabled ? "Relay setup 확인됨" : "Relay setup blocked", preflight.relayEnabled ? "ok" : "error");
    } catch (err) {
      activeRelaySetup = null;
      relayConnectButton.disabled = true;
      renderRelaySetupError(err.message);
      setStatus(err.message, "error");
    }
  }

  function loadRelayPrivateNetworkSetup() {
    try {
      closeRelayPrivateSocket("Disconnected");
      relayPrivateApprovalQueue = [];
      relayPrivateMonitor = liveMonitorInitialState();
      activeRelayPrivateSetup = parseRelayPrivateNetworkRuntimeSetupInput(relayPrivateSetupInput.value);
      relayPrivateSetupInput.value = JSON.stringify(activeRelayPrivateSetup, null, 2);
      const preflight = relayPrivateNetworkRuntimeSetupPreflight(activeRelayPrivateSetup);
      renderRelayPrivateNetworkSetup(activeRelayPrivateSetup, preflight);
      renderRelayPrivateQueue(relayPrivateApprovalQueue);
      renderRelayPrivateRuntime(relayPrivateMonitor);
      relayPrivateConnectButton.disabled = !preflight.contractReady;
      setStatus(
        preflight.contractReady ? "Private-network relay setup 확인됨" : "Private-network relay setup blocked",
        preflight.contractReady ? "ok" : "error",
      );
    } catch (err) {
      activeRelayPrivateSetup = null;
      relayPrivateConnectButton.disabled = true;
      renderRelayPrivateNetworkSetupError(err.message);
      setStatus(err.message, "error");
    }
  }

  parse.addEventListener("click", parseInput);
  clear.addEventListener("click", () => {
    input.value = "";
    activePayload = null;
    setStatus("페어링 payload 대기");
    renderPayload({
      pairing_code: "-",
      expires_at_ms: 0,
      transport_addr: "-",
      daemon_pubkey_hex: "-",
    });
  });
  relaySetupLoadButton.addEventListener("click", loadRelaySetup);
  relaySetupClearButton.addEventListener("click", () => {
    closeRelaySocket("Disconnected");
    relaySetupInput.value = "";
    activeRelaySetup = null;
    relayApprovalQueue = [];
    relayMonitor = liveMonitorInitialState();
    renderRelaySetup();
    renderRelayQueue(relayApprovalQueue);
    renderRelayRuntime(relayMonitor);
    setStatus("Relay setup 대기");
  });
  relayPrivateLoadButton.addEventListener("click", loadRelayPrivateNetworkSetup);
  relayPrivateClearButton.addEventListener("click", () => {
    closeRelayPrivateSocket("Disconnected");
    relayPrivateSetupInput.value = "";
    activeRelayPrivateSetup = null;
    relayPrivateApprovalQueue = [];
    relayPrivateMonitor = liveMonitorInitialState();
    renderRelayPrivateNetworkSetup();
    renderRelayPrivateQueue(relayPrivateApprovalQueue);
    renderRelayPrivateRuntime(relayPrivateMonitor);
    relayPrivateConnectButton.disabled = true;
    setStatus("Private-network relay setup 대기");
  });
  for (const id of ["device-id", "noise-pubkey", "approval-pubkey"]) {
    document.querySelector(`#${id}`).addEventListener("input", () => {
      if (activePayload) updateCommand(activePayload);
      if (activeApprovalRequest && activeApprovalResponse) {
        renderApprovalVerifyCommand(activeApprovalRequest, activeApprovalResponse);
      }
    });
  }
  identity.addEventListener("click", async () => {
    identity.disabled = true;
    try {
      const generated = await generateCompanionKeyMaterial();
      await saveCompanionKeyMaterial(window.indexedDB, generated.identity, generated.keyMaterial);
      saveCompanionIdentity(window.localStorage, generated.identity);
      applyIdentity(generated.identity);
      activeKeyMaterial = generated.keyMaterial;
      if (activePayload) updateCommand(activePayload);
      setStatus("Companion identity 생성됨", "ok");
    } catch (err) {
      setStatus(err.message, "error");
    } finally {
      identity.disabled = false;
    }
  });
  function parseApprovalRequest() {
    try {
      activeApprovalRequest = parseApprovalInput(approvalInput.value);
      activeApprovalResponse = null;
      activeApprovalTransport = "manual";
      approvalInput.value = JSON.stringify(activeApprovalRequest, null, 2);
      renderApprovalRequest(activeApprovalRequest, "Manual");
      document.querySelector("#approval-verify-command").textContent = "-";
      setStatus("승인 요청 확인됨", "ok");
    } catch (err) {
      activeApprovalRequest = null;
      activeApprovalResponse = null;
      setStatus(err.message, "error");
    }
  }
  async function signApprovalDecision(approve) {
    try {
      if (!activeApprovalRequest) {
        activeApprovalRequest = parseApprovalInput(approvalInput.value);
        renderApprovalRequest(activeApprovalRequest);
      }
      if (!activeKeyMaterial) {
        const record = await loadCompanionKeyMaterial(window.indexedDB);
        activeKeyMaterial = record?.keyMaterial || null;
      }
      const response = await approvalResponseForRequest(activeApprovalRequest, approve, activeKeyMaterial);
      activeApprovalResponse = response;
      document.querySelector("#approval-response").textContent = approvalResponseJson(response);
      renderApprovalVerifyCommand(activeApprovalRequest, response);
      if (
        activeApprovalTransport === "relay" &&
        relaySocket?.readyState === WebSocket.OPEN &&
        activeRelayLoop?.connected
      ) {
        const out = relayEndpointLoopNextFrame(activeRelayLoop, liveApprovalResponseMessage(response));
        relaySocket.send(out.frameJson);
        const sentKey = liveApprovalRequestKey(activeApprovalRequest);
        relayApprovalQueue = relayApprovalQueue.filter((item) => item.key !== sentKey);
        renderRelayQueue(relayApprovalQueue);
        setRelayLastEvent("approval_response");
        updateRelayMonitor({
          type: "approval_response",
          label: approve ? "relay approval_response approve" : "relay approval_response reject",
          approve,
          pendingCount: relayApprovalQueue.length,
        });
        setStatus(approve ? "Relay 승인 응답 전송됨" : "Relay 거부 응답 전송됨", "ok");
      } else if (
        activeApprovalTransport === "relay-private" &&
        relayPrivateSocket?.readyState === WebSocket.OPEN &&
        activeRelayPrivateLoop?.connected
      ) {
        const out = relayEndpointLoopNextFrame(activeRelayPrivateLoop, liveApprovalResponseMessage(response));
        relayPrivateSocket.send(out.frameJson);
        const sentKey = liveApprovalRequestKey(activeApprovalRequest);
        relayPrivateApprovalQueue = relayPrivateApprovalQueue.filter((item) => item.key !== sentKey);
        renderRelayPrivateQueue(relayPrivateApprovalQueue);
        setRelayPrivateLastEvent("approval_response");
        updateRelayPrivateMonitor({
          type: "approval_response",
          label: approve ? "private relay approval_response approve" : "private relay approval_response reject",
          approve,
          pendingCount: relayPrivateApprovalQueue.length,
        });
        setStatus(
          approve ? "Private-network relay 승인 응답 전송됨" : "Private-network relay 거부 응답 전송됨",
          "ok",
        );
      } else if (activeApprovalTransport === "live" && liveBaseUrl) {
        await postLiveTransportMessage(liveBaseUrl, liveApprovalResponseMessage(response));
        const sentKey = liveApprovalRequestKey(activeApprovalRequest);
        liveApprovalQueue = liveApprovalQueue.filter((item) => item.key !== sentKey);
        renderLiveQueue(liveApprovalQueue);
        setLiveLastEvent("approval_response");
        updateMonitor({
          type: "approval_response",
          label: approve ? "approval_response approve" : "approval_response reject",
          approve,
          pendingCount: liveApprovalQueue.length,
        });
        setStatus(approve ? "Live 승인 응답 전송됨" : "Live 거부 응답 전송됨", "ok");
      } else {
        setStatus(approve ? "승인 응답 서명됨" : "거부 응답 서명됨", "ok");
      }
    } catch (err) {
      updateMonitor({ type: "error", label: err.message });
      updateRelayMonitor({ type: "error", label: err.message });
      setStatus(err.message, "error");
    }
  }
  liveConnectButton.addEventListener("click", connectLive);
  liveDisconnectButton.addEventListener("click", () => {
    closeLiveEvents("Disconnected");
    setStatus("Live companion 연결 해제됨");
  });
  relayConnectButton.addEventListener("click", connectRelay);
  relayDisconnectButton.addEventListener("click", () => {
    closeRelaySocket("Disconnected");
    setStatus("Relay companion 연결 해제됨");
  });
  relayPrivateConnectButton.addEventListener("click", connectRelayPrivateNetwork);
  relayPrivateDisconnectButton.addEventListener("click", () => {
    closeRelayPrivateSocket("Disconnected");
    setStatus("Private-network relay companion 연결 해제됨");
  });
  approvalParse.addEventListener("click", parseApprovalRequest);
  approveButton.addEventListener("click", () => signApprovalDecision(true));
  rejectButton.addEventListener("click", () => signApprovalDecision(false));
  copyResponse.addEventListener("click", async () => {
    const text = document.querySelector("#approval-response").textContent;
    if (!text || text === "-") {
      setStatus("복사할 승인 응답 없음", "error");
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      setStatus("승인 응답 복사됨", "ok");
    } catch {
      setStatus("클립보드 복사 실패", "error");
    }
  });
  copyVerify.addEventListener("click", async () => {
    const text = document.querySelector("#approval-verify-command").textContent;
    if (!text || text === "-") {
      setStatus("복사할 검증 명령 없음", "error");
      return;
    }
    try {
      await navigator.clipboard.writeText(text);
      setStatus("검증 명령 복사됨", "ok");
    } catch {
      setStatus("클립보드 복사 실패", "error");
    }
  });
  for (const tab of document.querySelectorAll(".tab")) {
    tab.addEventListener("click", () => {
      document.querySelectorAll(".tab").forEach((item) => item.classList.remove("active"));
      tab.classList.add("active");
      const mode = tab.dataset.mode;
      const target =
        mode === "approve"
          ? document.querySelector(".approval-section")
          : mode === "monitor"
            ? document.querySelector(".monitor-section")
            : mode === "relay"
              ? document.querySelector(".relay-section")
              : document.querySelector("#detail-title");
      target?.scrollIntoView({ block: "start", behavior: "smooth" });
    });
  }
  const savedIdentity = loadCompanionIdentity(window.localStorage);
  if (savedIdentity) {
    loadCompanionKeyMaterial(window.indexedDB)
      .then((record) => {
        if (record?.identity?.deviceId === savedIdentity.deviceId) {
          applyIdentity(savedIdentity);
          activeKeyMaterial = record.keyMaterial;
          if (activePayload) updateCommand(activePayload);
          if (activeApprovalRequest && activeApprovalResponse) {
            renderApprovalVerifyCommand(activeApprovalRequest, activeApprovalResponse);
          }
          setStatus("Companion identity 복원됨", "ok");
        }
      })
      .catch(() => {});
  }
  if (window.location.search.includes("payload=")) {
    parseInput();
  }
  if (window.location.search.includes("approval=") || window.location.search.includes("request=")) {
    try {
      activeApprovalRequest = parseApprovalInput("", window.location.search);
      activeApprovalResponse = null;
      approvalInput.value = JSON.stringify(activeApprovalRequest, null, 2);
      renderApprovalRequest(activeApprovalRequest);
      document.querySelector("#approval-verify-command").textContent = "-";
      setStatus("승인 요청 확인됨", "ok");
    } catch (err) {
      setStatus(err.message, "error");
    }
  }
  if ("serviceWorker" in navigator) {
    navigator.serviceWorker.register("./sw.js").catch(() => {});
  }
}

if (typeof document !== "undefined") {
  init();
}
