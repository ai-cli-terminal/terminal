# 2026-07-04 RA/PWA Relay Managed Control-Plane Contract

## Purpose

Define managed Relay/M2 control-plane ownership, tenant/session boundaries, and
audit constraints before any managed runtime implementation.

## Status

Completed in this slice. The follow-up abuse retention policy, payload
confidentiality plan, and verifier-key operations policy slices are also
complete. Managed relay remains deferred until the billing/quota policy is
specified.

## Scope

- Add a repeatable managed control-plane contract check.
- Define required managed relay roles and responsibility boundaries.
- Define tenant/session isolation constraints.
- Define operator-visible state and audit boundaries.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay service or control plane.
- Do not expose managed relay in the PWA.
- Do not weaken self-hosted or private-network evidence boundaries.
- Do not permit control-plane access to payload JSON or secrets.

## Work Added

- Added `relayManagedControlPlaneContract()` to `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-control-plane-contract`.
- Added PWA tests for managed control-plane roles, contracts, prohibited data,
  responsibilities, and guardrails.

## Contract Boundaries

- Required roles: service operator, tenant admin, daemon owner, support
  operator.
- Required contracts: tenant identity, session registration, verifier-key
  distribution, quota/rate limit, support access, audit retention.
- Prohibited control-plane data: `payload_json`, session tokens, approval
  signatures, private key material, HMAC secrets, and full setup JSON.
- Operator-visible state stays limited to aggregate health and control-plane
  events.

## Next Slice

Managed relay abuse and retention policy is complete:

- define abuse handling and rate-limit policy;
- define retention windows and deletion requirements;
- define support workflow constraints;
- keep managed relay deferred until these policies are green.

Managed relay payload confidentiality plan and verifier-key operations policy
are complete. The next slice is managed relay billing/quota policy.

## Verification

```powershell
npm run check:pwa-relay-managed-control-plane-contract
npm run check:pwa-relay-managed-abuse-retention-policy
npm run check:pwa-relay-managed-payload-confidentiality-plan
npm run check:pwa-relay-managed-verifier-key-operations-policy
npm run check:pwa-relay-managed-operations-planning
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
