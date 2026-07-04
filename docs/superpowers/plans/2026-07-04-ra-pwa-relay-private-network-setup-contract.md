# 2026-07-04 RA/PWA Relay Private-Network Setup Contract

## Purpose

Define the private-network Relay/M2 setup contract after explicit self-hosted
relay readiness is green.

## Status

Completed in this slice. The follow-up runtime guardrails and operator evidence
slices are also complete; the next local implementation slice is private-network
relay visible import path.

## Scope

- Add a PWA-side private-network relay setup contract.
- Add a private-network setup preflight helper that accepts `wss://` endpoints
  and localhost `ws://` development endpoints while rejecting public `ws://`.
- Require private-network name, signed relay ticket, companion identity, relay
  transport mode, private-network deployment mode, and operator setup text.
- Keep `live-loopback` as the product default.
- Keep managed relay deferred.
- Add a repeatable contract check.

## Non-Goals

- Do not implement private-network daemon runtime selection in this slice.
- Do not make private-network relay visible as a product default path.
- Do not add managed relay operations.
- Do not weaken public `ws://` guardrails.

## Work Added

- Added `relayPrivateNetworkSetupContract()` and
  `relayPrivateNetworkSetupPreflight()` to `pwa/app.mjs`.
- Added PWA tests for private-network ready, localhost-development, public
  `ws://` rejection, and contract guardrails.
- Added `npm run check:pwa-relay-private-network-contract`.

## Follow-Up

Private-network relay runtime guardrails is complete:

- daemon-side mode parsing and startup guardrails;
- endpoint policy for private-network `wss://` and localhost development;
- setup JSON emission boundary for private-network mode;
- evidence that `live-loopback` remains default and public `ws://` remains
  blocked.

The next slice is private-network relay visible import path.

## Verification

```powershell
npm run check:pwa-relay-private-network-contract
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
