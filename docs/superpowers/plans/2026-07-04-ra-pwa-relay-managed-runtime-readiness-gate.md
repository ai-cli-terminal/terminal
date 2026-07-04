# 2026-07-04 RA/PWA Relay Managed Runtime Readiness Gate

## Purpose

Audit the remaining managed Relay/M2 runtime blockers and define the minimum
evidence required before managed relay implementation can start.

## Status

Completed in this slice as a readiness gate. Managed relay runtime remains
deferred, and implementation cannot start until the gate is green.

## Scope

- Add a repeatable managed runtime readiness gate check.
- Confirm all managed planning inputs are complete: control-plane ownership,
  tenant isolation, abuse handling, support workflows, retention policy,
  payload confidentiality, public verifier-key operations, and billing/quota
  policy.
- Aggregate the remaining runtime blockers from payload confidentiality,
  verifier-key operations, billing/quota, abuse retention, and support review.
- Define the minimum runtime evidence required before managed relay
  implementation starts.
- Keep `live-loopback` as the product default.
- Keep managed relay hidden and deferred until runtime evidence is green.

## Non-Goals

- Do not implement the managed relay runtime in this slice.
- Do not expose managed relay in the PWA.
- Do not select managed relay as the product default.
- Do not weaken payload-blind, public-verifier-key-only, quota, billing,
  support, or retention boundaries to make the gate green.
- Do not treat planning completion as production readiness.

## Work Added

- Added `relayManagedRuntimeReadinessGate()` to `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-runtime-readiness-gate`.
- Added PWA tests for gate status, implementation lock, completed planning
  inputs, runtime evidence, blocker audit, domain evidence, and next slice.
- Updated managed relay follow-up pointers to
  `managed-relay-payload-blind-frame-encryption-spike`.

## Runtime Blocker Audit

The gate carries forward these blocker groups:

- Payload confidentiality: end-to-end payload encryption, client key agreement,
  metadata minimization, confidentiality smoke evidence, and support redaction.
- Verifier keys: managed key registry runtime, revocation propagation, and
  rotation overlap evidence.
- Quota and usage: managed usage meter runtime, quota enforcement smoke,
  tenant usage export smoke, and billing/abuse boundary review.
- Abuse retention and support: runtime rate-limit enforcement, abuse escalation
  runbook, tenant deletion workflow, and support access review.

## Minimum Green Evidence

Managed relay implementation stays blocked until these evidence items exist:

- `payload-blind-frame-encryption-smoke`
- `client-key-agreement-runtime-smoke`
- `metadata-minimization-review`
- `public-verifier-key-registry-runtime-smoke`
- `revocation-and-rotation-propagation-smoke`
- `tenant-session-registration-quota-smoke`
- `active-session-and-byte-quota-smoke`
- `tenant-aggregate-usage-export-smoke`
- `support-redaction-and-access-review-evidence`
- `billing-abuse-boundary-review`

## Gate Decision

The readiness gate status is `blocked-until-runtime-evidence`.
`implementationCanStart` is `false`. Managed relay remains deferred, and
runtime implementation cannot start until the gate is green.

## Next Slice

Managed relay payload-blind frame encryption spike:

- define the client-held payload key and frame envelope shape;
- prove the relay routes opaque ciphertext only;
- add smoke evidence for encrypted request/response frames;
- keep managed relay deferred until the runtime readiness gate evidence becomes
  green.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-billing-quota-policy
npm run check:pwa-relay-managed-verifier-key-operations-policy
npm run check:pwa-relay-managed-payload-confidentiality-plan
npm run check:pwa-relay-managed-operations-planning
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
