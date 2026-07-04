# 2026-07-04 RA/PWA Relay Managed Public Verifier-Key Registry Runtime Smoke

## Purpose

Prove the managed Relay/M2 verifier-key registry can resolve tenant/key-id/key
version public verifier keys and validate Ed25519 session tickets without
private signing keys or HMAC secrets in the managed relay boundary.

## Status

Completed in this slice as a runtime smoke. Managed relay runtime remains
deferred. Revocation/rotation propagation smoke is complete, and
implementation cannot start until the remaining runtime readiness evidence is
green.

## Scope

- Add a repeatable managed public verifier-key registry runtime smoke check.
- Add a public PWA contract for tenant/key-id/key-version public verifier
  lookup.
- Sign managed relay session tickets with Ed25519 test key material.
- Verify signed tickets through a public-key-only managed registry.
- Prove missing keys, revoked keys, tampered tickets, and HMAC tickets fail
  closed.
- Prove registry JSON excludes private signing keys, HMAC secrets, raw session
  tokens, and signed ticket MAC material.
- Close `public-verifier-key-registry-runtime-smoke` and
  `managed_key_registry_runtime_missing` at smoke level.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime service.
- Do not expose managed relay in the PWA.
- Do not implement revocation and rotation propagation in this slice.
- Do not close quota enforcement, usage export, support access, billing, or
  abuse runtime evidence.
- Do not claim managed relay production readiness.

## Work Added

- Added `PWA_RELAY_MANAGED_PUBLIC_VERIFIER_KEY_REGISTRY_RUNTIME_SMOKE` to
  `pwa/app.mjs`.
- Added `relayManagedPublicVerifierKeyRegistryRuntimeSmoke()` to
  `pwa/app.mjs`.
- Added Ed25519 ticket signing and public verifier registry helpers to
  `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-public-verifier-key-registry-runtime-smoke`.
- Updated PWA tests for registry lookup, public-key-only verification,
  fail-closed cases, runtime gate completion, and next-slice pointers.
- Updated `relayManagedRuntimeReadinessGate()` so
  `public-verifier-key-registry-runtime-smoke` is completed and
  `managed_key_registry_runtime_missing` is resolved at smoke level.
- Updated managed relay next-mode pointers to
  `managed-relay-tenant-aggregate-usage-export-smoke`.

## Registry Boundary

Lookup fields:

- `tenant_id`
- `key_id`
- `key_version`

Stored public verifier-key fields:

- `tenant_id`
- `key_id`
- `key_version`
- `public_key_alg`
- `public_key_hex`
- `state`
- `not_before_ms`
- `expires_at_ms`

Accepted key states are `active` and `rotating`. Pending, retiring, revoked,
missing, expired, and not-yet-valid keys fail closed for new session ticket
verification.

Prohibited registry fields include private signing keys, HMAC secrets, generic
secret fields, private key material, raw session tokens, and ticket MAC values.

## Evidence

The smoke check and PWA tests prove:

- managed relay can resolve a tenant/key-id/key-version public verifier key;
- an Ed25519 signed session ticket verifies with public key material only;
- missing key id/version lookup fails closed;
- revoked key versions fail closed;
- tampered tickets fail with signature mismatch;
- HMAC signed tickets are rejected by the public verifier registry path;
- registry JSON excludes private signing keys, HMAC secrets, raw session
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
npm run check:pwa-relay-managed-public-verifier-key-registry-runtime-smoke
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-managed-client-key-agreement-runtime-smoke
npm run check:pwa-relay-managed-payload-blind-frame-encryption-spike
npm run check:pwa-relay-managed-verifier-key-operations-policy
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
