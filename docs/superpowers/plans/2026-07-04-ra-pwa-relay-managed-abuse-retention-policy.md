# 2026-07-04 RA/PWA Relay Managed Abuse Retention Policy

## Purpose

Define the managed Relay/M2 abuse handling, rate-limit, retention, deletion,
and support workflow policy before any managed relay runtime implementation.

## Status

Completed in this slice. The follow-up payload confidentiality plan,
verifier-key operations policy, and billing/quota policy are also complete.
Managed relay remains deferred until the runtime readiness gate is green.

## Scope

- Add a repeatable managed abuse/retention policy check.
- Define tenant-scoped rate-limit and abuse signal boundaries.
- Define retention windows for aggregate health, audit, abuse case, and support
  case metadata.
- Define deletion requirements for tenant/session metadata and verifier-key
  revocation.
- Define support workflow constraints that exclude payload JSON and secrets.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime or control plane.
- Do not expose managed relay in the PWA.
- Do not retain payload JSON, session tokens, approval signatures, private key
  material, HMAC secrets, or full setup JSON.
- Do not replace self-hosted or private-network relay evidence.

## Work Added

- Added `relayManagedAbuseRetentionPolicy()` to `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-abuse-retention-policy`.
- Added PWA tests for rate-limit scopes, abuse signals, retention windows,
  deletion requirements, support workflow constraints, and guardrails.
- The follow-up payload confidentiality plan, verifier-key operations policy,
  and billing/quota policy later moved the managed relay pointer to
  `managed-relay-runtime-readiness-gate`.

## Policy Boundaries

- Rate limits are scoped by tenant, daemon device, session, source IP, and
  verifier key.
- Abuse signals include invalid tickets, failed session registration, duplicate
  or replayed frame sequences, expired frame drops, and quota exhaustion.
- Aggregate health is retained for 30 days; control-plane audit is retained for
  90 days; abuse case metadata is retained for 180 days; support case metadata
  is retained for 90 days.
- Payload JSON, session tokens, approval signatures, private key material, HMAC
  secrets, and full setup JSON are not retained.
- Support access requires tenant-admin approval, auditability, aggregate-only
  state, and time-bounded breakglass.

## Next Slice

Managed relay payload confidentiality plan, verifier-key operations policy, and
billing/quota policy are complete:

- decide whether managed relay can be payload-blind or must remain an explicit
  operator-trust deployment;
- define the operator trust boundary for managed relay;
- define end-to-end payload confidentiality requirements before managed runtime
  implementation;
- keep managed relay deferred until payload confidentiality, key operations,
  and billing/quota policies are green.

The next slice is managed relay runtime readiness gate.

## Verification

```powershell
npm run check:pwa-relay-managed-abuse-retention-policy
npm run check:pwa-relay-managed-payload-confidentiality-plan
npm run check:pwa-relay-managed-verifier-key-operations-policy
npm run check:pwa-relay-managed-billing-quota-policy
npm run check:pwa-relay-managed-control-plane-contract
npm run check:pwa-relay-managed-operations-planning
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
