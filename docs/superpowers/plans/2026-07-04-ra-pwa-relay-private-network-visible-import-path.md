# 2026-07-04 RA/PWA Relay Private-Network Visible Import Path

## Purpose

Add an explicit PWA visible import/status path for private-network Relay/M2 setup
without changing the self-hosted relay setup path or `live-loopback` default.

## Status

Completed in this slice. The follow-up connection controls slice is also
complete; the next local implementation slice is private-network relay approval
flow evidence.

## Scope

- Add a separate private-network runtime setup input in the Relay tab.
- Parse private-network setup JSON with a dedicated validator.
- Show private-network status, network name, endpoint, deployment, device,
  session, expiry, and blockers.
- Keep self-hosted runtime setup loading and connect controls unchanged.
- Keep private-network relay visible path status-only in this slice.
- Keep `live-loopback` as the product default and managed relay deferred.
- Add desktop and mobile browser evidence for the visible import path.

## Non-Goals

- Do not make private-network relay the product default.
- Do not enable private-network relay connect/approval controls in this slice.
- Do not change self-hosted visible setup behavior.
- Do not add managed relay operations.

## Work Added

- Added `parseRelayPrivateNetworkRuntimeSetupInput()`.
- Added a separate Private Network import/status block to the PWA Relay tab.
- Added `npm run smoke:pwa-relay-private-network-visible-import`.
- Updated next-mode planning to point at connection controls; the follow-up
  connection controls slice later moved the next pointer to approval flow
  evidence.

## Follow-Up

Private-network relay connection controls is complete:

- add explicit private-network connect/disconnect controls;
- build a setup-derived private-network companion endpoint loop;
- capture browser connect evidence;
- keep self-hosted relay setup and `live-loopback` default unchanged.

The next slice is private-network relay approval flow evidence.

## Verification

```powershell
npm run smoke:pwa-relay-private-network-visible-import
npm run smoke:pwa-relay-setup-ui
npm run check:pwa-relay-private-network-runtime-guardrails
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
