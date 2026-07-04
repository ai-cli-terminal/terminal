# 2026-07-04 RA/PWA Relay Private-Network Connection Controls

## Purpose

Add explicit PWA connect/disconnect controls for private-network Relay/M2 setup
without changing the self-hosted relay controls or `live-loopback` default.

## Status

Completed in this slice. The follow-up approval flow evidence slice is also
complete; the next local implementation slice is private-network relay runbook
closeout.

## Scope

- Add private-network connect/disconnect buttons in the PWA Relay tab.
- Build the browser companion WebSocket loop from private-network setup JSON.
- Keep self-hosted relay setup/connect behavior unchanged.
- Route private-network relay connection status separately from self-hosted
  relay status.
- Capture browser evidence that private-network setup connects over a
  setup-derived endpoint loop.

## Non-Goals

- Do not make private-network relay the product default.
- Do not change managed relay operations.
- Do not remove the self-hosted relay controls or evidence.

## Work Added

- Added `relayPrivateNetworkCompanionEndpointLoopFromSetup()`.
- Added private-network connect/disconnect buttons and private runtime status in
  the PWA Relay tab.
- Routed private-network relay approval requests/responses through a separate
  `relay-private` transport state.
- Added `npm run smoke:pwa-relay-private-network-connection-controls` with
  desktop/mobile browser evidence against the relay service artifact.

## Follow-Up

Private-network relay approval flow evidence is complete:

- drive a private-network relay approval request from the daemon side;
- send approve/reject responses through the private relay socket;
- confirm self-hosted relay approval evidence remains unchanged;
- keep `live-loopback` as the product default and managed relay deferred.

The next slice is private-network relay runbook closeout.

## Verification

```powershell
npm run smoke:pwa-relay-private-network-connection-controls
npm run smoke:pwa-relay-private-network-visible-import
npm run smoke:pwa-relay-setup-ui
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
