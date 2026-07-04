# 2026-07-04 RA/PWA Relay Managed Operations Planning

## Purpose

Define the managed Relay/M2 operations requirements that must be green before
any managed relay implementation starts.

## Status

Completed in this slice. The follow-up control-plane contract, abuse retention
policy, and payload confidentiality plan slices are also complete. Managed
relay remains deferred until verifier-key operations and billing/quota policies
are specified.

## Scope

- Add a repeatable managed relay operations planning check.
- Record required pre-implementation areas for managed relay.
- Keep `live-loopback` as the product default.
- Keep self-hosted readiness and private-network evidence as separate explicit
  paths.
- Keep managed runtime implementation blocked until operations contracts exist.

## Non-Goals

- Do not implement a managed relay control plane.
- Do not expose managed relay in the PWA.
- Do not change self-hosted or private-network relay behavior.
- Do not change `live-loopback` product default.

## Work Added

- Added `relayManagedOperationsPlan()` to `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-operations-planning`.
- Added PWA tests for managed relay planning guardrails.
- Updated next-mode planning to point at the managed relay control-plane
  contract.

## Required Before Managed Implementation

- control-plane ownership;
- tenant isolation;
- abuse handling;
- support workflows;
- retention policy;
- billing and quota policy;
- public verifier-key operations;
- payload confidentiality plan.

## Follow-Up

Managed relay control-plane contract is complete:

- define owner/responsibility boundaries;
- define tenant and session isolation requirements;
- define operator-visible state and audit boundaries;
- keep managed relay deferred until this contract is green.

Managed relay abuse retention policy and payload confidentiality plan are
complete. The next slice is managed relay verifier-key operations policy.

## Verification

```powershell
npm run check:pwa-relay-managed-operations-planning
npm run check:pwa-relay-managed-abuse-retention-policy
npm run check:pwa-relay-managed-payload-confidentiality-plan
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
