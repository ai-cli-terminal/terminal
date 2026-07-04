# 2026-07-04 RA/PWA Relay Managed Payload Confidentiality Plan

## Purpose

Define the managed Relay/M2 payload confidentiality and operator trust
boundaries before any managed relay runtime implementation.

## Status

Completed in this slice. Managed relay remains deferred until verifier-key
operations and billing/quota policies are specified.

## Scope

- Add a repeatable managed payload confidentiality check.
- Decide that managed relay cannot rely on the self-hosted/private-network
  explicit operator-trust model.
- Require payload-blind managed relay behavior before runtime implementation.
- Define prohibited managed relay data and allowed routing metadata.
- Define required end-to-end payload confidentiality evidence before runtime.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement payload encryption in this slice.
- Do not implement a managed relay runtime or control plane.
- Do not expose managed relay in the PWA.
- Do not change self-hosted or private-network relay trust decisions.
- Do not claim managed relay production readiness.

## Work Added

- Added `relayManagedPayloadConfidentialityPlan()` to `pwa/app.mjs`.
- Added `npm run check:pwa-relay-managed-payload-confidentiality-plan`.
- Added PWA tests for prohibited payload data, allowed routing metadata,
  required confidentiality evidence, operator trust boundaries, and guardrails.
- Updated next-mode planning and managed relay follow-up pointers to
  `managed-relay-verifier-key-operations-policy`.

## Confidentiality Boundaries

- Managed relay must be payload-blind before runtime implementation.
- Relay operators must not be able to read payload JSON, command text, context
  JSON, approval response payloads, session tokens, private key material, HMAC
  secrets, or full setup JSON.
- Allowed relay metadata is limited to routing and aggregate diagnostics:
  tenant id, session id, hashed daemon/companion identifiers, frame sequence,
  frame expiry, ticket key id, and aggregate error class.
- Payload keys must remain client-held by daemon/companion boundaries.
- Support workflows may inspect aggregate state only and must not gain
  breakglass access to decrypt payloads.

## Runtime Requirements

- frame payload end-to-end encryption;
- envelope metadata minimization;
- client-held payload keys;
- key rotation and revocation;
- confidentiality regression evidence;
- support payload redaction.

## Next Slice

Managed relay verifier-key operations policy:

- define managed verifier-key ownership and rotation;
- define public verifier-key distribution and revocation;
- define key id/version evidence before managed runtime implementation;
- keep managed relay deferred until verifier-key operations are green.

## Verification

```powershell
npm run check:pwa-relay-managed-payload-confidentiality-plan
npm run check:pwa-relay-managed-abuse-retention-policy
npm run check:pwa-relay-managed-operations-planning
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
