# 2026-07-04 RA/PWA Relay Managed Payload-Blind Frame Encryption Spike

## Purpose

Define and verify the managed Relay/M2 encrypted frame envelope shape before
any managed relay runtime implementation starts. The managed relay must route
opaque ciphertext and routing metadata only.

## Status

Completed in this slice as a spike. Managed relay runtime remains deferred,
and implementation cannot start until the remaining runtime readiness evidence
is green.

## Scope

- Add a repeatable payload-blind frame encryption smoke check.
- Add managed relay AES-GCM encrypted frame helpers for request and response
  live transport messages.
- Require a client-held payload key outside the managed relay route boundary.
- Keep route-visible state limited to routing metadata, payload algorithm,
  key scope, and ciphertext byte count.
- Prove encrypted frame JSON excludes payload JSON, command text, context
  hashes, approval response payloads, private key material, raw session tokens,
  and full setup JSON.
- Close the payload-blind encryption evidence item at spike level while keeping
  managed relay deferred.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime service.
- Do not expose managed relay in the PWA.
- Do not implement client key agreement runtime in this slice.
- Do not make the relay service responsible for payload decryption keys.
- Do not close metadata minimization, key registry, quota enforcement, usage
  export, support access, billing, or abuse runtime evidence.
- Do not claim managed relay production readiness.

## Work Added

- Added `relayManagedPayloadBlindFrameEncryptionSpike()` to `pwa/app.mjs`.
- Added managed encrypted frame helpers:
  `managedRelayEncryptedFrameFromLiveMessage()`,
  `managedRelayEncryptedFrameJson()`,
  `parseManagedRelayEncryptedFrame()`,
  `managedRelayEncryptedFrameRouteEnvelope()`,
  `managedRelayEncryptedFramePayloadMessage()`, and
  `validateManagedRelayEncryptedFrame()`.
- Added `npm run check:pwa-relay-managed-payload-blind-frame-encryption-spike`.
- Updated `relayManagedRuntimeReadinessGate()` so
  `payload-blind-frame-encryption-smoke` is completed and
  `e2e_payload_encryption_missing` / `confidentiality_smoke_missing` are
  resolved at spike level.
- Updated managed relay next-mode pointers to
  `managed-relay-client-key-agreement-runtime-smoke`.

## Envelope Boundary

Route-visible fields:

- `session_id`
- `sender`
- `sequence`
- `sent_at_ms`
- `expires_at_ms`
- `payload_ciphertext_alg`
- `payload_key_scope`
- `payload_ciphertext_bytes`

Encrypted frame fields carried end to end:

- `payload_nonce_hex`
- `payload_ciphertext_hex`

Prohibited managed frame fields:

- `payload_json`
- `command_text`
- `context_json`
- `approval_response_payload`
- `private_key_material`
- `raw_session_token`
- `full_setup_json`

## Evidence

The spike check and PWA tests prove:

- approval request and response frames round-trip through AES-GCM encryption;
- encrypted frame JSON does not contain plaintext command, context, payload, or
  approval response material;
- route envelopes expose byte counts but not plaintext or ciphertext bytes;
- wrong payload keys fail closed;
- route metadata tampering fails closed through AES-GCM AAD binding;
- plaintext managed frame fields are rejected before routing.

## Remaining Runtime Evidence

Managed relay implementation remains blocked on:

- `client-key-agreement-runtime-smoke`
- `metadata-minimization-review`
- `public-verifier-key-registry-runtime-smoke`
- `revocation-and-rotation-propagation-smoke`
- `tenant-session-registration-quota-smoke`
- `active-session-and-byte-quota-smoke`
- `tenant-aggregate-usage-export-smoke`
- `support-redaction-and-access-review-evidence`
- `billing-abuse-boundary-review`

## Next Slice

Managed relay client key agreement runtime smoke:

- derive or transport client-held payload keys across daemon/companion
  boundaries without exposing them to the managed relay route;
- prove encrypted frames can use session-specific client-held keys;
- keep route-visible metadata payload-blind;
- keep managed relay deferred until the full readiness gate is green.

## Verification

```powershell
npm run check:pwa-relay-managed-payload-blind-frame-encryption-spike
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
