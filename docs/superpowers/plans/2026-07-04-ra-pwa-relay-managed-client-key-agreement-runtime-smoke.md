# 2026-07-04 RA/PWA Relay Managed Client Key Agreement Runtime Smoke

## Purpose

Prove daemon and companion endpoints can derive the same session-specific,
client-held managed relay payload key before any managed relay runtime
implementation starts.

## Status

Completed in this slice as a runtime smoke. The follow-up metadata
minimization review and public verifier-key registry runtime smoke are also
complete. Managed relay runtime remains deferred, and implementation cannot
start until the remaining runtime readiness evidence is green.

## Scope

- Add a repeatable managed client key agreement runtime smoke check.
- Derive a 32-byte AES payload key from the existing X25519 daemon/companion
  shared secret through HKDF-SHA-256.
- Bind the derived payload key to `session_id`.
- Prove daemon and companion derive the same payload key for a session.
- Prove a different session id derives a different payload key.
- Prove route-visible public key metadata cannot derive the payload key.
- Use the derived key with managed payload-blind request and response frames.
- Keep payload keys, shared secrets, and private keys outside managed relay
  route-visible state.
- Close `client-key-agreement-runtime-smoke` and
  `client_key_agreement_missing` at smoke level.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime service.
- Do not expose managed relay in the PWA.
- Do not complete the metadata minimization review in this slice; it is covered
  by the follow-up metadata minimization review.
- Do not close key registry, quota enforcement, usage export, support access,
  billing, or abuse runtime evidence.
- Do not claim managed relay production readiness.

## Work Added

- Added `managedRelayDeriveSessionPayloadKeyHex()` to `pwa/app.mjs`.
- Added `relayManagedClientKeyAgreementRuntimeSmoke()` to `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-client-key-agreement-runtime-smoke`.
- Updated PWA tests so managed encrypted frames use session-derived payload
  keys instead of a fixed test key.
- Updated `relayManagedRuntimeReadinessGate()` so
  `client-key-agreement-runtime-smoke` is completed and
  `client_key_agreement_missing` is resolved at smoke level.
- Updated managed relay next-mode pointers to
  `managed-relay-revocation-and-rotation-propagation-smoke` after the
  follow-up metadata minimization review and public verifier registry smoke
  completed.

## Key Agreement Boundary

- Algorithm: X25519 shared secret plus HKDF-SHA-256.
- HKDF info: `ai-terminal-managed-relay-payload-key-v1`.
- HKDF salt fields: `session_id`.
- Output key length: 32 bytes.
- Private key boundary: daemon and companion only.
- Route-visible key metadata: `session_id`, `daemon_noise_pubkey_hex`, and
  `companion_noise_pubkey_hex`.
- Prohibited route key material: `daemon_noise_private_key`,
  `companion_noise_private_key`, `payload_key_hex`, and `shared_secret_hex`.

## Evidence

The smoke check and PWA tests prove:

- daemon and companion derive identical session payload keys;
- different session ids derive different payload keys;
- public route metadata without private key material fails closed;
- derived keys encrypt and decrypt managed payload-blind request/response
  frames;
- wrong-session derived keys fail decrypt;
- encrypted frame JSON and route envelopes exclude command text, context hash,
  payload keys, shared secrets, private keys, and ciphertext bytes.

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
npm run check:pwa-relay-managed-client-key-agreement-runtime-smoke
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-managed-public-verifier-key-registry-runtime-smoke
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-payload-blind-frame-encryption-spike
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
