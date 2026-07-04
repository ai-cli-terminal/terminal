# 2026-07-04 RA/PWA Relay Managed Metadata Minimization Review

## Purpose

Review and prove the managed Relay/M2 metadata boundary after the
payload-blind encrypted frame spike and client-held key agreement smoke.
Managed relay may route and operate on allowlisted route/control/billing/
support/audit metadata only.

## Status

Completed in this slice as a review. The follow-up public verifier-key
registry runtime smoke is also complete. Managed relay runtime remains
deferred, and implementation cannot start until the remaining runtime
readiness evidence is green.

## Scope

- Add a repeatable managed metadata minimization review check.
- Add a public PWA contract for route, control-plane, billing, support, and
  audit metadata allowlists.
- Prove the actual managed encrypted frame route envelope matches the route
  metadata allowlist.
- Prove support, billing, control-plane, and audit metadata exclude raw
  ciphertext, payload keys, shared secrets, private keys, command text, and
  context data.
- Close `metadata-minimization-review` and
  `metadata_minimization_review_missing` at review level.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime service.
- Do not expose managed relay in the PWA.
- Do not implement revocation and rotation propagation runtime smoke.
- Do not close quota enforcement, usage export, support access, billing, or
  abuse runtime evidence.
- Do not claim managed relay production readiness.

## Work Added

- Added `PWA_RELAY_MANAGED_METADATA_MINIMIZATION_REVIEW` to `pwa/app.mjs`.
- Added `relayManagedMetadataMinimizationReview()` to `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-metadata-minimization-review`.
- Updated PWA tests for metadata review status, allowlists, prohibited fields,
  runtime gate completion, and next-slice pointers.
- Updated `relayManagedRuntimeReadinessGate()` so
  `metadata-minimization-review` is completed and
  `metadata_minimization_review_missing` is resolved at review level.
- Updated managed relay next-mode pointers to
  `managed-relay-revocation-and-rotation-propagation-smoke` after the public
  verifier-key registry runtime smoke completed.

## Metadata Boundary

Route envelope metadata is limited to:

- `relay_protocol_version`
- `session_id`
- `sender`
- `sequence`
- `sent_at_ms`
- `expires_at_ms`
- `payload_ciphertext_alg`
- `payload_key_scope`
- `payload_ciphertext_bytes`

Control-plane metadata is limited to tenant/session/device hashes, ticket key
id/version, session state, and lifecycle timestamps.

Billing metadata is aggregate-only: billing period, registration count, active
session count, relay frame count, relay byte count, invalid ticket count, and
quota denial count.

Support metadata uses hashed identifiers and aggregate error/quota/key/version
state only. Audit metadata records event type, actor role, key id/version,
timestamp, and aggregate error class.

Prohibited managed metadata fields include payload JSON, command text, context
JSON, approval response payloads, raw ciphertext/nonce hex, payload key hex,
shared secret hex, Noise private keys, private key material, session tokens,
signed session tickets, full setup JSON, HMAC secrets, and approval signatures.

## Evidence

The review check and PWA tests prove:

- actual route envelope keys match the route allowlist exactly;
- route metadata exposes ciphertext byte count without raw ciphertext bytes;
- control-plane, billing, support, and audit metadata keys match allowlists;
- metadata JSON excludes command text, context data, payload keys, raw
  ciphertext, nonces, shared secrets, and daemon/companion public keys where
  only hashes are allowed;
- encrypted frame JSON excludes command/context plaintext, payload keys, and
  shared secrets;
- managed runtime remains deferred.

## Remaining Runtime Evidence

Managed relay implementation remains blocked on:

- `revocation-and-rotation-propagation-smoke`
- `tenant-session-registration-quota-smoke`
- `active-session-and-byte-quota-smoke`
- `tenant-aggregate-usage-export-smoke`
- `support-redaction-and-access-review-evidence`
- `billing-abuse-boundary-review`

## Next Slice

Managed relay revocation and rotation propagation smoke:

- prove active and rotating verifier key versions can overlap during rotation;
- prove revoked and retiring key versions fail closed for new session
  registration;
- prove registry snapshot changes propagate to ticket verification decisions;
- preserve key id/version audit metadata;
- keep managed relay deferred until the full readiness gate is green.

## Verification

```powershell
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-managed-public-verifier-key-registry-runtime-smoke
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-client-key-agreement-runtime-smoke
npm run check:pwa-relay-managed-payload-blind-frame-encryption-spike
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
