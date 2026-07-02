# 2026-07-02 RA/PWA Relay Session Rotation Reconnect

## Purpose

The WebSocket bridge smoke already requires signed tickets and authenticated
connect messages. This slice adds evidence for the next boundary before relay
can become product-facing: expired tickets must fail connect, and a companion
must be able to reconnect through a newly issued session id/token without
accepting stale-session frames or old tokens.

## Scope

- Extend the browser-native WebSocket bridge smoke with an expired signed
  ticket connect attempt.
- Register an old signed session ticket, connect daemon and companion peers,
  and deliver one frame.
- Close the old sockets, register a new signed ticket with a different
  `session_id` and `session_token`, reconnect both peers, and deliver a new
  frame.
- Prove the new endpoint rejects an old-session frame at endpoint decode.
- Prove the old session token cannot authenticate against the new session.
- Update the relay transport decision check so aggregate evidence must include
  the new rotation/reconnect assertions.

## Non-Goals

- No hosted relay deployment.
- No daemon-side persistent ticket issuer storage yet.
- No multi-key relay ticket verification or HMAC key rotation yet.
- No product default change away from `live-loopback`.
- No operator-visible relay setup UX.

## Evidence Shape

The WebSocket smoke now proves:

- `expiredTicketConnectRejected=true`: a signed but already-expired ticket can
  be registered by the smoke issuer boundary, but its connect message is
  rejected before session join.
- `rotationReconnected=true`: daemon and companion reconnect using a newly
  signed session after the old sockets close.
- `rotationReconnectDelivered=true`: the new session delivers a post-rotation
  relay frame.
- `rotationOldTokenRejected=true`: the old session token cannot authenticate
  against the new session.
- `rotationOldFrameIsolated=true`: a frame from the old session is rejected by
  the new endpoint as a session mismatch.
- `acceptedConnects=8`, `rejectedConnects=4`, `registeredTickets=7`,
  `openedConnections=12`, and `closedConnections=12`.
- `acceptedFrames=5`, `deliveredFrames=4`, `expiredFrames=1`, and
  `rejectedFrames=1`.

Evidence path:

```text
artifacts/ra-pwa-relay-websocket-bridge/ra-pwa-relay-websocket-bridge.json
```

## Follow-Up

1. Add daemon-side ticket issuer state and HMAC key rotation policy when a
   hosted relay shape is chosen.
2. Decide relay setup UX only after deployment mode is chosen.
3. Keep `live-loopback` as the product default until hosted deployment,
   daemon integration, and operator UX evidence exist.

## Verification

```powershell
npm run smoke:pwa-relay-websocket-bridge
npm run check:pwa-relay-transport-decision
npm run test:pwa
```
