# 2026-07-04 RA/PWA Relay Managed Tenant Session Registration Quota Smoke

## Purpose

Prove the managed Relay/M2 tenant session registration quota preflight rejects
new sessions before registration when a tenant quota is exhausted or the quota
window is not effective.

## Status

Completed in this slice as a runtime smoke. Managed relay runtime remains
deferred, and implementation cannot start until the remaining runtime readiness
evidence is green.

## Scope

- Add a repeatable managed tenant session registration quota smoke check.
- Add a normalized tenant quota state helper.
- Add a registration quota evaluator that runs before session creation.
- Prove within-limit registrations are accepted before session creation.
- Prove exhausted tenant registration quotas reject new sessions fail-closed.
- Prove not-yet-effective quota windows reject new sessions fail-closed.
- Preserve tenant, session, daemon device, verifier key, quota, and decision
  audit metadata.
- Keep quota-denial billing meter deltas separate from abuse/rate-limit
  signals.
- Close `tenant-session-registration-quota-smoke` and
  `quota_enforcement_smoke_missing` at smoke level.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime service.
- Do not expose managed relay in the PWA.
- Do not implement active session ceilings or frame/byte quotas in this slice.
- Do not implement tenant aggregate usage export.
- Do not close support access review or billing/abuse boundary evidence.
- Do not claim managed relay production readiness.

## Work Added

- Added `PWA_RELAY_MANAGED_TENANT_SESSION_REGISTRATION_QUOTA_SMOKE` to
  `pwa/app.mjs`.
- Added `createManagedRelayTenantSessionRegistrationQuotaState()` to normalize
  tenant quota windows, registration limits, billing meters, and abuse signals.
- Added `evaluateManagedRelayTenantSessionRegistrationQuota()` to return an
  accept/reject preflight decision, audit event, billing delta, abuse-signal
  delta, and quota snapshot.
- Added `relayManagedTenantSessionRegistrationQuotaSmoke()` to expose the smoke
  contract and remaining runtime evidence.
- Added `npm run check:pwa-relay-managed-tenant-session-registration-quota-smoke`.
- Updated PWA tests for accept, exhausted quota rejection, not-yet-effective
  window rejection, audit metadata preservation, payload/secret exclusion,
  billing/abuse separation, runtime gate completion, and next-slice pointers.
- Updated `relayManagedRuntimeReadinessGate()` so
  `tenant-session-registration-quota-smoke` is completed and
  `quota_enforcement_smoke_missing` is resolved at smoke level.
- Updated managed relay next-mode pointers to
  `managed-relay-billing-abuse-boundary-review`.

## Quota Boundary

Quota state fields:

- `tenant_id`
- `window_start_ms`
- `window_end_ms`
- `registration_limit`
- `registrations_used`
- `billing_meter`
- `abuse_signals`

Registration request fields:

- `tenant_id`
- `session_id`
- `daemon_device_id`
- `verifier_key_id`
- `verifier_key_version`
- `source_ip_hash`

Decision values:

- `accept`
- `reject`

Fail-closed reasons:

- `tenant-session-registration-quota-exceeded`
- `quota-window-not-effective`

Quota decision audit events preserve:

- `tenant_id`
- `session_id`
- `daemon_device_id`
- `verifier_key_id`
- `verifier_key_version`
- `source_ip_hash`
- `quota_scope`
- `quota_limit`
- `quota_used`
- `quota_remaining_before_decision`
- `quota_remaining_after_decision`
- `decision`
- `reason`
- `billing_meter_delta`
- `abuse_signal_delta`
- `at_ms`

The audit event must not include payload JSON, command text, context JSON,
approval payloads, private key material, raw session tokens, signed session
tickets, full setup JSON, HMAC secrets, or signed ticket MAC material.

## Evidence

The smoke check and PWA tests prove:

- within-limit registration is accepted before session creation;
- exhausted tenant registration quota rejects new sessions before registration;
- a quota window that is not yet effective rejects new sessions fail-closed;
- quota-denial audit preserves tenant/session/device/key/quota metadata;
- quota-denial audit excludes payloads and secrets;
- quota denials increment billing meter deltas, not abuse/rate-limit signals;
- managed runtime remains deferred.

## Remaining Runtime Evidence

Managed relay implementation remains blocked on:

- `billing-abuse-boundary-review`

## Next Slice

Managed relay billing/abuse boundary review:

- prove billing usage meters and abuse signals remain separately reviewed;
- confirm support/redaction evidence cannot be reclassified as billing data;
- keep aggregate-only tenant usage exports without payloads or secrets;
- keep managed relay deferred until the full readiness gate is green.

## Verification

```powershell
npm run check:pwa-relay-managed-tenant-session-registration-quota-smoke
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-billing-quota-policy
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
