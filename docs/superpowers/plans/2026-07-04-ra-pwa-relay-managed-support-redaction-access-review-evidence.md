# 2026-07-04 RA/PWA Relay Managed Support Redaction Access Review Evidence

## Purpose

Prove managed Relay/M2 support workflows expose only aggregate, redacted state
and require audited tenant-admin approval before any support access is
available.

## Status

Completed in this slice as runtime readiness evidence. Managed relay runtime remains deferred; the later billing/abuse boundary review has made the readiness gate green.

## Scope

- Add repeatable support redaction/access review evidence.
- Add a helper that builds a redacted support view from hashed identifiers and
  aggregate billing/abuse counters only.
- Require tenant-admin approval id, support case id, support actor hash, and a
  time-bounded access window.
- Prove raw session/device/support actor identifiers are rejected.
- Prove payloads, command text, context data, raw session tokens, signed
  tickets, HMAC secrets, and MAC material are excluded.
- Close `support-redaction-and-access-review-evidence`,
  `support_audit_boundary_missing`, `support_access_review_missing`, and
  `support_redaction_evidence_missing` at evidence level.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime service.
- Do not expose managed relay in the PWA.
- Do not close billing/abuse boundary review evidence.
- Do not claim managed relay production readiness.

## Work Added

- Added `PWA_RELAY_MANAGED_SUPPORT_REDACTION_AND_ACCESS_REVIEW_EVIDENCE` to
  `pwa/app.mjs`.
- Added `createManagedRelaySupportRedactionAccessReview()` to normalize a
  redacted, aggregate-only support view and audited access-review event.
- Added `relayManagedSupportRedactionAndAccessReviewEvidence()` to expose the
  evidence contract and remaining runtime evidence.
- Added `npm run check:pwa-relay-managed-support-redaction-access-review-evidence`.
- Updated PWA tests for hashed support identifiers, tenant-admin approval,
  time-bounded access, raw identifier rejection, payload/secret exclusion,
  runtime gate completion, and next-slice pointers.
- Updated `relayManagedRuntimeReadinessGate()` so
  `support-redaction-and-access-review-evidence` is completed and support
  access/redaction blockers are resolved at evidence level.
- Updated managed relay next-mode pointers to
  `managed-relay-runtime-implementation-plan`.

## Support Boundary

Input fields:

- `tenant_id`
- `support_case_id`
- `support_actor_id_hash`
- `tenant_admin_approval_id`
- `access_approved_at_ms`
- `access_expires_at_ms`
- `generated_at_ms`
- `session_id_hash`
- `daemon_device_id_hash`
- `companion_device_id_hash`
- `aggregate_error_class`
- `quota_state`
- `key_id`
- `key_version`
- `billing_usage`
- `abuse_signals`

Output fields:

- `tenant_id`
- `support_case_id`
- `support_actor_id_hash`
- `tenant_admin_approval_id`
- `access_window_start_ms`
- `access_window_end_ms`
- `generated_at_ms`
- `support_visibility`
- `redaction_state`
- `payload_visibility`
- `session_id_hash`
- `daemon_device_id_hash`
- `companion_device_id_hash`
- `aggregate_error_class`
- `quota_state`
- `key_id`
- `key_version`
- `billing_usage_summary`
- `abuse_signal_summary`
- `access_review_audit`

The support view must not include raw session id, daemon device id, companion
device id, support actor id, payload JSON, command text, context JSON, approval
payloads, private key material, raw session tokens, signed session tickets,
full setup JSON, HMAC secrets, or signed ticket MAC material.

## Evidence

The check and PWA tests prove:

- support views are aggregate-only;
- support identifiers are hashed;
- tenant-admin approval is recorded;
- support access is time-bounded and audited;
- raw identifiers are rejected;
- support views exclude payloads and secrets;
- runtime gate records support redaction/access review evidence;
- managed runtime remains deferred.

## Remaining Runtime Evidence

No managed runtime readiness evidence remains after the billing/abuse boundary review.

## Next Slice

Managed relay runtime implementation plan:

- plan the managed runtime service boundary without changing the product default;
- keep `selectedRuntime` deferred until the implementation plan explicitly changes exposure;
- preserve payload-blind, public-verifier, quota, support, and billing/abuse boundaries.

## Verification

```powershell
npm run check:pwa-relay-managed-support-redaction-access-review-evidence
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-tenant-aggregate-usage-export-smoke
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
