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
- No persistent HMAC key storage yet.
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

## Progress

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-daemon-ticket-issuer-policy.md`
added the daemon-side issuer/key rotation policy and Rust helper for active-key
issuance plus bounded active+previous key verification.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-ux-preflight.md` added the PWA
visibility preflight so relay remains hidden until all readiness inputs are
present.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-deployment-shape-decision.md`
selected self-hosted WebSocket relay as the first deployable shape.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-secret-keyring-migration.md`
added persistent daemon-owned relay ticket keyring records and optional signed
ticket key ids.

## Follow-Up

1. Wire daemon runtime ticket issuance to the persisted self-hosted relay
   keyring.
2. Keep `live-loopback` as the product default until deployment, daemon
   integration, and operator UX evidence exist.
3. Add visible self-hosted relay setup UI after endpoint, ticket, identity, and
   operator copy are ready.

## Verification

```powershell
npm run smoke:pwa-relay-websocket-bridge
npm run check:pwa-relay-transport-decision
npm run test:pwa
```
