# 2026-07-04 RA/PWA Relay Managed Revocation and Rotation Propagation Smoke

## Purpose

Prove the managed Relay/M2 verifier-key registry can propagate rotation and
revocation state through registry snapshots before any managed relay runtime
implementation starts.

## Status

Completed in this slice as a runtime smoke. Managed relay runtime remains
deferred, and implementation cannot start until the remaining runtime readiness
evidence is green.

## Scope

- Add a repeatable managed revocation and rotation propagation smoke check.
- Add public verifier-key registry snapshot helpers.
- Prove old active and new rotating key versions overlap during rotation.
- Prove retiring and revoked old key versions fail closed for new session
  ticket verification after snapshot propagation.
- Prove the new active key version continues to verify after the old version is
  revoked.
- Preserve tenant, key id, key version, key state, snapshot id, and snapshot
  effective time in audit metadata.
- Close `revocation-and-rotation-propagation-smoke`,
  `key_revocation_propagation_smoke_missing`, and
  `rotation_overlap_smoke_missing` at smoke level.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime service.
- Do not expose managed relay in the PWA.
- Do not implement tenant registration quota enforcement in this slice.
- Do not close active session quota, byte quota, usage export, support access,
  billing, or abuse runtime evidence.
- Do not claim managed relay production readiness.

## Work Added

- Added `PWA_RELAY_MANAGED_REVOCATION_AND_ROTATION_PROPAGATION_SMOKE` to
  `pwa/app.mjs`.
- Added `relayManagedRevocationAndRotationPropagationSmoke()` to
  `pwa/app.mjs`.
- Added verifier registry snapshot helpers:
  `createManagedRelayPublicVerifierKeyRegistrySnapshot()`,
  `lookupManagedRelayPublicVerifierKeyFromRegistrySnapshot()`, and
  `validateManagedRelaySignedSessionTicketWithPublicVerifierRegistrySnapshot()`.
- Added `npm run check:pwa-relay-managed-revocation-and-rotation-propagation-smoke`.
- Updated PWA tests for active/rotating overlap, retiring/revoked fail-closed
  behavior, post-rotation active verification, audit metadata, runtime gate
  completion, and next-slice pointers.
- Updated `relayManagedRuntimeReadinessGate()` so
  `revocation-and-rotation-propagation-smoke` is completed and
  `key_revocation_propagation_smoke_missing` /
  `rotation_overlap_smoke_missing` are resolved at smoke level.
- Updated managed relay next-mode pointers to
  `managed-relay-tenant-aggregate-usage-export-smoke`.

## Propagation Boundary

Registry snapshot fields:

- `snapshot_id`
- `effective_at_ms`
- `previous_snapshot_id`
- `reason`
- `entries`

During overlap, `active` and `rotating` key states can verify new session
tickets. For new sessions, `retiring`, `revoked`, missing, expired, and
not-yet-valid key states fail closed.

Accepted validation audit events preserve:

- `tenant_id`
- `key_id`
- `key_version`
- `key_state`
- `registry_snapshot_id`
- `registry_effective_at_ms`
- `decision`
- `at_ms`

## Evidence

The smoke check and PWA tests prove:

- an old active key and a new rotating key both verify during the overlap
  snapshot;
- retiring old key versions fail closed for new sessions;
- revoked old key versions fail closed after the propagated snapshot;
- the new key version verifies as active after revocation of the old version;
- accepted validations include tenant/key/snapshot audit metadata;
- registry snapshots exclude private signing keys, HMAC secrets, raw session
  tokens, and signed ticket MAC material;
- managed runtime remains deferred.

## Remaining Runtime Evidence

Managed relay implementation remains blocked on:

- `tenant-aggregate-usage-export-smoke`
- `support-redaction-and-access-review-evidence`
- `billing-abuse-boundary-review`

## Next Slice

Managed relay tenant aggregate usage export smoke:

- export tenant aggregate usage counters without payloads or secrets;
- include active session, relay frame, relay byte, and quota denial meters;
- preserve billing/abuse boundary metadata for review;
- keep managed relay deferred until the full readiness gate is green.

## Verification

```powershell
npm run check:pwa-relay-managed-revocation-and-rotation-propagation-smoke
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-public-verifier-key-registry-runtime-smoke
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-managed-client-key-agreement-runtime-smoke
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
