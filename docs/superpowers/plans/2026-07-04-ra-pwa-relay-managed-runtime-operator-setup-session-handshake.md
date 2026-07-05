# 2026-07-04 RA/PWA Relay Managed Runtime Operator Setup Session Handshake

## Purpose

Add the Managed Relay/M2 operator setup session handshake after manual
connection controls.

## Status

Completed in this slice as a metadata-only session capability handshake. The
PWA requires a ready managed setup import and a manual connect request before
starting the handshake, then displays only a capability handle and transcript
hash. It does not render the capability envelope JSON, signed tickets, raw
tokens, payload material, or private key material.

## Scope

- Add `managedRelayRuntimeOperatorSetupSessionHandshakePayload()`.
- Add `managedRelayRuntimeOperatorSetupSessionHandshake()`.
- Add `relayManagedRuntimeOperatorSetupSessionHandshake()`.
- Add PWA start/reset handshake controls.
- Render handshake state, capability handle, and transcript hash only.
- Capture desktop/mobile browser evidence for the handshake surface.
- Move the next local slice to managed operator setup approval-flow evidence.

## Non-Goals

- Do not connect to a managed relay endpoint.
- Do not create a WebSocket during the handshake.
- Do not start an endpoint or enable public bind.
- Do not render capability envelope JSON, signed session tickets, raw session
  tokens, payloads, private key material, support contact metadata, or raw
  identifiers.
- Do not change `live-loopback` as the product default.

## Work Added

- Added the managed setup session handshake contract in `pwa/app.mjs`.
- Added PWA start/reset handshake buttons and handshake status fields.
- Added unit coverage for blocked and ready handshake states.
- Added `npm run smoke:pwa-relay-managed-runtime-operator-setup-session-handshake`.
- Updated next-mode planning so the next local slice is
  `managed-relay-runtime-operator-setup-approval-flow-evidence`.

## Handshake Boundary

The session handshake is intentionally metadata-only:

- before import: handshake controls are disabled;
- after ready import: handshake waits for manual connect request;
- after manual connect request: start handshake is enabled;
- after handshake: the PWA shows `Handshake ready`, a `managed-cap:*` handle,
  and a `sha256:*` transcript hash;
- the capability envelope exists only as an internal evidence object;
- signed tickets, raw tokens, payloads, and private key material are not
  rendered;
- no WebSocket is created;
- no endpoint is started;
- public bind remains off.

## Next Slice

Managed relay runtime operator setup approval-flow evidence:

- prove the approval surface can use the session capability boundary;
- keep approval payloads and responses outside visible managed relay setup
  surfaces except for the existing approval UI;
- keep raw credentials, signed tickets, tokens, payload material, and private
  key material hidden;
- preserve `live-loopback` rollback, explicit opt-in, disabled endpoint
  auto-start, and public bind off.

## Verification

```powershell
npm run smoke:pwa-relay-managed-runtime-operator-setup-session-handshake
npm run smoke:pwa-relay-managed-runtime-operator-setup-connection-controls
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
