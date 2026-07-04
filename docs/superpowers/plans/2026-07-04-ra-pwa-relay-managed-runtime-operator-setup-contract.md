# 2026-07-04 RA/PWA Relay Managed Runtime Operator Setup Contract

## Purpose

Define the operator-issued setup payload boundary for Managed Relay/M2 after
browser/operator evidence confirmed the PWA surface is explicit opt-in only.

## Status

Completed in this slice as a code-backed setup contract and regression check.
Managed Relay still does not auto-start an endpoint, does not enable public
bind, does not change the product default away from `live-loopback`, and does
not expose signed tickets, raw tokens, payloads, key material, support actor
ids, or raw device/session identifiers in the setup contract surface.

## Scope

- Add a managed runtime operator setup contract summary.
- Add a versioned operator setup contract factory.
- Require a `wss://` managed relay endpoint.
- Require metadata-only setup fields with hashed identifiers.
- Require manual operator connect after setup import.
- Keep endpoint auto-start disabled and public bind off.
- Move the next local slice to managed relay operator setup import preflight.

## Non-Goals

- Do not import a managed relay setup payload into the PWA in this slice.
- Do not start or connect to a managed relay endpoint.
- Do not add signed tickets, session tokens, payload ciphertext, payload keys,
  HMAC material, or raw identifiers to the PWA setup surface.
- Do not change the product default from `live-loopback`.

## Work Added

- Added `PWA_RELAY_MANAGED_RUNTIME_OPERATOR_SETUP_CONTRACT` to `pwa/app.mjs`.
- Added `createManagedRelayRuntimeOperatorSetupContract()` to build and validate
  the setup contract.
- Added `relayManagedRuntimeOperatorSetupContract()` to expose the summary,
  guardrails, evidence checks, and next local slice.
- Added `npm run check:pwa-relay-managed-runtime-operator-setup-contract`.
- Updated next-mode planning so the next local slice is
  `managed-relay-runtime-operator-setup-import-preflight`.

## Contract Boundary

The setup contract allows only metadata needed to prepare an explicit managed
relay setup import:

- setup version and deployment mode;
- `wss://` relay endpoint URL;
- tenant id;
- hashed session, daemon-device, and companion-device identifiers;
- verifier key id/version;
- issued/expires timestamps;
- operator setup text;
- `live-loopback` rollback transport.

The setup contract explicitly prohibits:

- payload JSON or approval command/context data;
- payload ciphertext, nonce, payload keys, shared secrets, private key material,
  HMAC secrets, or MAC material;
- raw session tokens or signed session tickets;
- full setup JSON rendering;
- raw support actor, session, daemon-device, or companion-device identifiers.

## Next Slice

Managed relay runtime operator setup import preflight:

- parse an operator-issued setup payload without rendering full JSON;
- keep endpoint activation manual;
- keep public bind and endpoint auto-start disabled;
- keep browser evidence and this setup contract as required regressions.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-operator-setup-contract
npm run smoke:pwa-relay-managed-runtime-browser-operator-evidence
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
