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
- No daemon-side ticket issuer yet.
- No product default change away from `live-loopback`.
- No operator-visible relay setup UX.

## Evidence Shape

The WebSocket smoke now proves:

- `acceptedConnects=4`: daemon and companion for the happy-path session, plus
  daemon and companion for the expired-frame session.
- `rejectedConnects=2`: one unauthenticated frame sent before connect, and one
  bad-token companion connect.
- `registeredTickets=4` and `rejectedTickets=2`: signed tickets are required;
  unsigned and bad-MAC tickets are rejected at registration.
- `acceptedFrames=3`, `deliveredFrames=2`, `expiredFrames=1`,
  `rejectedFrames=1`: unchanged relay-frame behavior after authentication.
- `openedConnections=6` and `closedConnections=6`: all authenticated and
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

## Follow-Up

1. Add relay session rotation and reconnect evidence.
2. Add daemon-side ticket issuer state and key rotation policy when deployment
   shape is chosen.
3. Decide how the PWA should present relay setup once deployment mode is chosen.

## Verification

```powershell
npm run smoke:pwa-relay-websocket-bridge
npm run check:pwa-relay-transport-decision
```
