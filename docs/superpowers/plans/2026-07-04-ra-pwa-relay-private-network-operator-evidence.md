# 2026-07-04 RA/PWA Relay Private-Network Operator Evidence

## Purpose

Record operator evidence for explicit private-network Relay/M2 setup after the
runtime guardrails are in place.

## Status

Completed in this slice. The follow-up visible import path, connection
controls, approval flow evidence, runbook closeout, and managed operations
planning slices are also complete, and managed verifier-key operations policy
is complete; the next local implementation slice is managed relay billing/quota
policy.

## Scope

- Capture `ai remote relay-setup --relay-deployment-mode private-network`
  output from an isolated CLI registry.
- Persist the emitted private-network setup JSON as an artifact.
- Verify the setup JSON with PWA private-network runtime metadata validation.
- Verify PWA private-network runtime preflight is ready while relay UI remains
  hidden.
- Exercise a setup-derived daemon/companion relay frame roundtrip.
- Prove public `ws://` private-network relay setup is still blocked.
- Keep `live-loopback` as the product default and managed relay deferred.

## Non-Goals

- Do not make private-network relay the product default.
- Do not add managed relay operations.
- Do not expose private-network setup through the self-hosted Relay tab in this
  slice.
- Do not claim payload confidentiality from a relay operator.

## Work Added

- Added `npm run smoke:pwa-relay-private-network-operator-evidence`.
- Added an isolated WSL CLI smoke that pairs a deterministic companion identity,
  emits private-network setup JSON, imports it through PWA helpers, and records
  frame roundtrip evidence.
- Wrote artifacts under
  `artifacts/ra-pwa-relay-private-network-operator-evidence/`.

## Follow-Up

Private-network relay visible import path, connection controls, and approval
flow evidence are complete:

- add an explicit advanced private-network import path in the PWA without
  changing `live-loopback` default;
- keep self-hosted visible setup behavior unchanged;
- show private-network setup status from `privateNetworkName` and endpoint;
- add explicit private-network connect/disconnect controls and browser connect
  evidence;
- capture private-network approve/reject browser evidence and daemon-side
  response delivery;
- keep managed relay deferred.

Managed relay abuse retention policy, payload confidentiality plan, and
verifier-key operations policy are complete. The next slice is managed relay
billing/quota policy.

## Verification

```powershell
npm run smoke:pwa-relay-private-network-operator-evidence
npm run check:pwa-relay-private-network-runtime-guardrails
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
