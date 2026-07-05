# 2026-07-04 RA/PWA Relay Managed Verifier-Key Operations Policy

## Purpose

Define managed Relay/M2 verifier-key ownership, public-key distribution,
rotation, and revocation policy before any managed relay runtime
implementation.

## Status

Completed in this slice. The follow-up billing/quota policy, runtime readiness
gate, payload-blind frame encryption spike, client key agreement runtime smoke,
metadata minimization review, public verifier-key registry runtime smoke, and
revocation/rotation propagation smoke plus tenant session registration quota
smoke are also complete. Managed relay remains deferred until runtime evidence
is green.

## Scope

- Add a repeatable managed verifier-key operations policy check.
- Define that managed relay receives public verifier keys only.
- Keep private signing keys outside the managed relay service boundary.
- Require key id and key version on tickets and audit events.
- Define rotation overlap, retirement, and revocation behavior.
- Keep support/audit visibility limited to key id/version state.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime or control plane.
- Do not place private signing keys in the managed relay service.
- Do not implement billing or quota policy in this slice.
- Do not expose managed relay in the PWA.
- Do not claim managed relay production readiness.

## Work Added

- Added `relayManagedVerifierKeyOperationsPolicy()` to `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-verifier-key-operations-policy`.
- Added PWA tests for key ownership, public verifier-key distribution, private
  signing-key boundaries, rotation, revocation, audit, and guardrails.
- Updated managed relay follow-up pointers. The later billing/quota policy and
  runtime readiness gate plus payload-blind frame encryption spike moved the
  pointer to `managed-relay-runtime-implementation-plan` after
  the client key agreement runtime smoke, metadata minimization review, and
  public verifier registry smoke and revocation/rotation propagation smoke
  plus tenant session registration quota smoke completed.

## Policy Boundaries

- Key owner: tenant admin owns key registration, rotation, and revocation;
  daemon owners issue tickets with the current key id/version.
- Public distribution: managed relay stores and verifies public verifier keys
  only.
- Prohibited data: private signing keys, HMAC secrets, raw session tokens,
  payload JSON, and approval signature payloads must not enter the managed
  relay service.
- Rotation: key rotation requires an overlap window and explicit retirement of
  old key versions.
- Revocation: revoked key ids fail closed for new session registration.
- Audit: support and operator workflows see key id/version events without
  private key material.

## Next Slice

Managed relay billing/quota policy, runtime readiness gate, payload-blind
frame encryption spike, client key agreement runtime smoke, metadata
minimization review, public verifier-key registry runtime smoke, and
revocation/rotation propagation smoke plus tenant session registration quota
smoke are complete. Active session and byte quota smoke plus tenant aggregate
usage export smoke are also complete. The next slice is managed relay support
redaction and access review evidence:

- prove support views remain aggregate-only and redacted;
- require tenant-admin approval or equivalent audited support access boundary;
- preserve tenant/session/key/quota metadata without payloads or secrets;
- keep managed relay deferred until runtime readiness gate evidence is green.

## Verification

```powershell
npm run check:pwa-relay-managed-verifier-key-operations-policy
npm run check:pwa-relay-managed-billing-quota-policy
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-payload-blind-frame-encryption-spike
npm run check:pwa-relay-managed-client-key-agreement-runtime-smoke
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-managed-public-verifier-key-registry-runtime-smoke
npm run check:pwa-relay-managed-payload-confidentiality-plan
npm run check:pwa-relay-managed-operations-planning
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
