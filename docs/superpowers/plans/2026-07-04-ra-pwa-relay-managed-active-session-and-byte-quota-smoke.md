# 2026-07-04 RA/PWA Relay Managed Active Session And Byte Quota Smoke

## Purpose

Prove the managed Relay/M2 quota preflight rejects new active sessions or relay
frames before activation/routing when tenant, daemon-device, frame, or byte
quotas are exhausted.

## Status

Completed in this slice as a runtime smoke. Managed relay runtime remains
deferred, and implementation cannot start until the remaining runtime readiness
evidence is green.

## Scope

- Add a repeatable managed active session and byte quota smoke check.
- Add a normalized quota state helper for tenant active sessions, daemon-device
  active sessions, relay frame counts, relay byte counts, billing meters, and
  abuse signals.
- Add a relay route quota evaluator that runs before session activation or
  frame routing.
- Prove within-limit route attempts are accepted before routing.
- Prove tenant active-session limits reject before session activation.
- Prove daemon-device active-session limits reject before session activation.
- Prove relay frame and byte limits reject before frame routing.
- Preserve tenant, session, daemon device, verifier key, frame sequence, byte
  count, quota, decision, billing, and abuse audit metadata.
- Keep usage billing meter deltas separate from abuse/rate-limit signals.
- Close `active-session-and-byte-quota-smoke` and
  `managed_usage_meter_runtime_missing` at smoke level.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime service.
- Do not expose managed relay in the PWA.
- Do not implement tenant aggregate usage export in this slice.
- Do not close support access review or billing/abuse boundary evidence.
- Do not claim managed relay production readiness.

## Work Added

- Added `PWA_RELAY_MANAGED_ACTIVE_SESSION_AND_BYTE_QUOTA_SMOKE` to
  `pwa/app.mjs`.
- Added `createManagedRelayActiveSessionAndByteQuotaState()` to normalize
  tenant/daemon active-session counters, relay frame/byte counters, billing
  meters, and abuse signals.
- Added `evaluateManagedRelayActiveSessionAndByteQuota()` to return an
  accept/reject preflight decision, audit event, billing delta, abuse-signal
  delta, and quota snapshot.
- Added `relayManagedActiveSessionAndByteQuotaSmoke()` to expose the smoke
  contract and remaining runtime evidence.
- Added `npm run check:pwa-relay-managed-active-session-and-byte-quota-smoke`.
- Updated PWA tests for within-limit acceptance, tenant/daemon active-session
  rejection, frame/byte rejection, not-yet-effective windows, audit metadata
  preservation, payload/secret exclusion, billing/abuse separation, runtime
  gate completion, and next-slice pointers.
- Updated `relayManagedRuntimeReadinessGate()` so
  `active-session-and-byte-quota-smoke` is completed and
  `managed_usage_meter_runtime_missing` is resolved at smoke level.
- Updated managed relay next-mode pointers to
  `managed-relay-support-redaction-and-access-review-evidence`.

## Quota Boundary

Quota state fields:

- `tenant_id`
- `daemon_device_id`
- `window_start_ms`
- `window_end_ms`
- `tenant_active_session_limit`
- `tenant_active_sessions`
- `daemon_device_active_session_limit`
- `daemon_device_active_sessions`
- `relay_frame_limit`
- `relay_frames_used`
- `relay_byte_limit`
- `relay_bytes_used`
- `billing_meter`
- `abuse_signals`

Route request fields:

- `tenant_id`
- `session_id`
- `daemon_device_id`
- `verifier_key_id`
- `verifier_key_version`
- `frame_sequence`
- `payload_ciphertext_bytes`

Decision values:

- `accept`
- `reject`

Fail-closed reasons:

- `tenant-active-session-quota-exceeded`
- `daemon-device-active-session-quota-exceeded`
- `relay-frame-quota-exceeded`
- `relay-byte-quota-exceeded`
- `quota-window-not-effective`

Quota decision audit events preserve:

- `tenant_id`
- `session_id`
- `daemon_device_id`
- `verifier_key_id`
- `verifier_key_version`
- `frame_sequence`
- `payload_ciphertext_bytes`
- `quota_scope`
- active-session limits and counts
- relay frame and byte limits and counts
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

- within-limit frame routing is accepted before routing;
- exhausted tenant active-session quota rejects before session activation;
- exhausted daemon-device active-session quota rejects before session
  activation;
- exhausted relay frame quota rejects before routing;
- exhausted relay byte quota rejects before routing;
- ineffective quota windows reject fail-closed;
- quota-denial audit excludes payloads and secrets;
- accepted routes increment active-session, frame, and byte billing meters;
- quota denials increment billing meter deltas, not abuse/rate-limit signals;
- managed runtime remains deferred.

## Remaining Runtime Evidence

Managed relay implementation remains blocked on:

- `support-redaction-and-access-review-evidence`
- `billing-abuse-boundary-review`

## Next Slice

Managed relay support redaction and access review evidence:

- prove support views remain aggregate-only and redacted;
- require tenant-admin approval or equivalent audited support access boundary;
- preserve tenant/session/key/quota metadata without payloads or secrets;
- keep managed relay deferred until the full readiness gate is green.

## Verification

```powershell
npm run check:pwa-relay-managed-active-session-and-byte-quota-smoke
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-tenant-session-registration-quota-smoke
npm run check:pwa-relay-managed-billing-quota-policy
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
