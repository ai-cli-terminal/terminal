# 2026-07-04 RA/PWA Relay Managed Billing/Quota Policy

## Purpose

Define managed Relay/M2 billing, quota, usage metering, and tenant usage
boundaries before any managed relay runtime implementation.

## Status

Completed in this slice. The follow-up runtime readiness gate, payload-blind
frame encryption spike, client key agreement runtime smoke, metadata
minimization review, public verifier-key registry runtime smoke, and
revocation/rotation propagation smoke plus tenant session registration quota
smoke are also complete, and managed relay remains deferred until runtime
evidence is green.

## Scope

- Add a repeatable managed billing/quota policy check.
- Define tenant, daemon-device, session, verifier-key, and source-IP quota
  scopes.
- Define billable usage dimensions for session registration, active sessions,
  relay frames, relay bytes, invalid tickets, and quota denials.
- Define fail-closed quota enforcement before managed runtime.
- Keep billing records payload-blind and secret-free.
- Keep abuse/rate-limit signals separate from billing meters.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement managed usage metering runtime in this slice.
- Do not implement payment provider integration.
- Do not expose managed relay in the PWA.
- Do not store payload JSON, command text, context JSON, approval payloads,
  private key material, raw session tokens, or full setup JSON in billing
  records.
- Do not claim managed relay production readiness.

## Work Added

- Added `relayManagedBillingQuotaPolicy()` to `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-billing-quota-policy`.
- Added PWA tests for quota scopes, metered usage dimensions, prohibited
  billing data, quota defaults, enforcement, retention, and trust boundaries.
- Updated managed relay follow-up pointers toward the runtime readiness gate.
  Later payload-blind frame encryption, client key agreement, metadata
  minimization, public verifier registry, and revocation/rotation propagation
  slices plus tenant session registration quota smoke and active session and
  byte quota smoke moved the pointer to
  `managed-relay-support-redaction-and-access-review-evidence`.

## Policy Boundaries

- Quotas are scoped by tenant, daemon device, session, verifier key, and source
  IP.
- Usage meters count control-plane and routing metadata only.
- Billing records exclude payload JSON, command text, context JSON, approval
  response payloads, private key material, raw session tokens, and full setup
  JSON.
- Quota exhaustion rejects new session registration or frame routing before
  managed runtime can be considered ready.
- Tenant-admin views are aggregate usage and plan-limit oriented.
- Support views are aggregate-only and cannot access payload or secret data.

## Follow-up

Managed relay runtime readiness gate, payload-blind frame encryption spike,
client key agreement runtime smoke, metadata minimization review, public
verifier-key registry runtime smoke, revocation/rotation propagation smoke, and
tenant session registration quota smoke are complete. Active session and byte
quota smoke plus tenant aggregate usage export smoke are also complete. The
next slice is managed relay support redaction and access review evidence:

- prove support views remain aggregate-only and redacted;
- require tenant-admin approval or equivalent audited support access boundary;
- preserve tenant/session/key/quota metadata without payloads or secrets;
- keep `live-loopback` as product default;
- keep managed relay deferred until the readiness gate evidence is green.

## Verification

```powershell
npm run check:pwa-relay-managed-billing-quota-policy
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-payload-blind-frame-encryption-spike
npm run check:pwa-relay-managed-client-key-agreement-runtime-smoke
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-managed-public-verifier-key-registry-runtime-smoke
npm run check:pwa-relay-managed-verifier-key-operations-policy
npm run check:pwa-relay-managed-payload-confidentiality-plan
npm run check:pwa-relay-managed-operations-planning
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
