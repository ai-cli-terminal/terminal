# 2026-07-02 RA/PWA Relay WebSocket Bridge Auth Enforcement

## Purpose

The WebSocket relay bridge smoke now needs to enforce the ticket/connect
contract before any relay frame can be routed. This slice turns the previously
shared Rust/PWA session contract into local bridge evidence: sockets must
authenticate as daemon or companion before joining a relay session.

## Scope

- Add a smoke-only `POST /sessions` endpoint to register WebSocket relay
  session tickets.
- Require the first WebSocket text message to be a daemon or companion connect
  message that matches the registered ticket.
- Add authenticated peers to the session connection set only after the connect
  message passes session id, session token, role, key/device, and expiry checks.
- Reject unauthenticated frame messages before frame routing.
- Reject bad-token connect messages before frame routing.
- Preserve the existing frame invariants: duplicate sequence rejection, expired
  frame dropping, endpoint-only payload decoding, and no `payload_json` in route
  envelopes.

## Non-Goals

- No hosted relay deployment.
- No persistent daemon-side ticket issuer storage yet.
- No product default change away from `live-loopback`.
- No operator-visible relay setup UX.

## Evidence Shape

The WebSocket smoke now proves:

- `acceptedConnects=8`: daemon and companion for the happy-path,
  expired-frame, old rotation, and new rotation sessions.
- `rejectedConnects=4`: one expired-ticket connect, one old-token reconnect
  attempt against the new session, one unauthenticated frame sent before
  connect, and one bad-token companion connect.
- `registeredTickets=7` and `rejectedTickets=2`: signed tickets are required;
  unsigned and bad-MAC tickets are rejected at registration.
- `acceptedFrames=5`, `deliveredFrames=4`, `expiredFrames=1`,
  `rejectedFrames=1`: relay-frame behavior remains stable while adding
  rotation/reconnect coverage.
- `openedConnections=12` and `closedConnections=12`: all authenticated and
  rejected sockets are closed by the end of the smoke.

Evidence path:

```text
artifacts/ra-pwa-relay-websocket-bridge/ra-pwa-relay-websocket-bridge.json
```

## Progress

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-signed-ticket-prototype.md`
added the HMAC wrapper and updated this smoke so ticket registration requires a
valid `hmac-sha256` MAC.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-session-rotation-reconnect.md`
added expired-ticket connect rejection, rotated old/new session reconnect
delivery, stale-session frame isolation, and old-token rejection evidence.

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

## Follow-Up

1. Add persistent relay secret storage and key id migration for daemon-owned
   self-hosted relay HMAC keys.
2. Add visible self-hosted relay setup UI after endpoint, ticket, identity, and
   operator copy are ready.
3. Revisit managed relay and private-network/Tailscale only after separate
   deployment and support evidence exists.

## Verification

```powershell
npm run smoke:pwa-relay-websocket-bridge
npm run check:pwa-relay-transport-decision
```
