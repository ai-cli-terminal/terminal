# 2026-07-04 RA/PWA Relay Managed Runtime Operator Setup Connection Controls

## Purpose

Add explicit Managed Relay/M2 operator setup connection controls after browser
evidence proved the setup import surface.

## Status

Completed in this slice as manual request/cancel controls. The PWA enables the
request control only after a ready managed setup import, updates connection
state when the operator requests managed connect, and keeps the control
status-only: no WebSocket is created, no endpoint is started, and public bind
remains off.

## Scope

- Add `managedRelayRuntimeOperatorSetupConnectionControls()`.
- Add `relayManagedRuntimeOperatorSetupConnectionControls()`.
- Add Managed Relay request/cancel connection controls to the PWA Relay tab.
- Keep the controls disabled until a valid operator setup payload is imported.
- Replace the original setup JSON with the existing hidden metadata marker after
  import.
- Capture browser evidence for ready, requested, cancelled, and mobile states.
- Move the next local slice to managed operator setup session handshake.

## Non-Goals

- Do not connect to a managed relay endpoint.
- Do not create a WebSocket from the request control.
- Do not mint or render signed tickets, raw tokens, payload material, key
  material, support contact metadata, or raw identifiers.
- Do not change `live-loopback` as the product default.
- Do not enable public bind or endpoint auto-start.

## Work Added

- Added the managed setup connection controls contract in `pwa/app.mjs`.
- Added PWA request/cancel buttons and managed connection status fields.
- Added unit coverage for ready, manual-requested, and expired setup control
  states.
- Added `npm run smoke:pwa-relay-managed-runtime-operator-setup-connection-controls`.
- Updated next-mode planning so the next local slice is
  `managed-relay-runtime-operator-setup-session-handshake`.

## Control Boundary

The request control is intentionally a status boundary, not a runtime transport
handshake:

- before import: request and cancel are disabled;
- after ready import: request is enabled and cancel remains disabled;
- after request: connection state becomes `Manual connect requested`, request is
  disabled, and cancel is enabled;
- after cancel: connection state returns to `Ready`;
- request does not create a WebSocket;
- request does not start an endpoint;
- request does not enable public bind;
- the visible setup surface remains sanitized metadata only.

## Next Slice

Managed relay runtime operator setup session handshake:

- define how a browser turns a ready operator setup import and manual request
  into a session capability;
- keep signed tickets, raw tokens, payloads, and private key material out of
  visible PWA surfaces;
- keep handshake failure modes fail-closed;
- preserve `live-loopback` rollback, explicit opt-in, disabled endpoint
  auto-start, and public bind off.

## Verification

```powershell
npm run smoke:pwa-relay-managed-runtime-operator-setup-connection-controls
npm run smoke:pwa-relay-managed-runtime-operator-setup-browser-evidence
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
