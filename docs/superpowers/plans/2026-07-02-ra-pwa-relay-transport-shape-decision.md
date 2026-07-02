# 2026-07-02 RA/PWA Relay Transport Shape Decision

## Purpose

The relay track now has matching HTTP polling and WebSocket bridge evidence.
This slice records the first prototype transport decision and adds a repeatable
check that refreshes both bridge smokes before writing aggregate evidence.

## Decision

Use **WebSocket** as the first relay prototype substrate.

Keep **HTTP polling** as a fallback/diagnostics candidate, not the primary
prototype path.

Keep the product default as **`live-loopback`**. This decision does not make
`relay` or `websocket` selectable product transports.

## Why WebSocket First

- Both HTTP and WebSocket smokes prove the same relay invariants: route metadata
  only at the bridge, opaque `payload_json`, duplicate sender sequence rejection,
  expired-frame dropping, and endpoint-only approval payload decode.
- WebSocket is browser-native full-duplex, so pending daemon frames can be pushed
  to the companion without a polling loop.
- The live approval model is naturally bidirectional: daemon request to
  companion, companion response to daemon, plus heartbeat/error envelopes.
- The HTTP bridge remains valuable as a simpler fallback and diagnostic harness,
  especially for environments where persistent browser sockets are unreliable.

## Guardrails

- Do not switch `active_product_mode()` away from `live-loopback`.
- Do not mark `relay` or `websocket` as ready in the transport catalog.
- Do not bypass device registry, signed approval validation, nonce/replay,
  expiry, context hash, or explicit peer identity validation.
- Do not expose operator-visible relay setup until signed tickets, deployment,
  and reconnect/rotation boundaries are documented and tested.

## Work Added

- Add `npm run check:pwa-relay-transport-decision`.
- Refresh `npm run smoke:pwa-relay-http-bridge` and
  `npm run smoke:pwa-relay-websocket-bridge` through the check command.
- Assert both evidence files prove matching relay invariants.
- Assert the WebSocket evidence requires the ticket/connect handshake before
  routing relay frames.
- Assert WebSocket ticket registration rejects unsigned and bad-MAC tickets.
- Assert WebSocket evidence rejects expired-ticket connects, reconnects with a
  rotated session token, rejects old tokens, and isolates old-session frames.
- Write aggregate decision evidence under
  `artifacts/ra-pwa-relay-transport-decision/`.

## Follow-Up

1. Add visible self-hosted relay setup UI after endpoint, ticket, identity, and
   operator copy are ready.
2. Add browser evidence that the runtime setup bundle drives relay UX preflight
   to ready without changing the product default.
3. Revisit managed relay and private-network/Tailscale only after separate
   deployment and support evidence exists.

## Progress

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-daemon-ticket-issuer-policy.md`
added the daemon-side issuer/key rotation policy and Rust
`CompanionRelayTicketIssuer` helper while keeping the product default on
`live-loopback`.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-ux-preflight.md` added the PWA
relay visibility preflight and evidence command. Relay remains hidden until all
readiness inputs are present.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-deployment-shape-decision.md`
selected self-hosted WebSocket relay as the first deployable shape while keeping
managed relay and private-network/Tailscale deferred.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-secret-keyring-migration.md`
added persistent daemon-owned relay ticket keyring records and optional signed
ticket key ids.

## Verification

```powershell
npm run check:pwa-relay-transport-decision
npm run check:pwa-relay-deployment-decision
npm run smoke:pwa-relay-browser-parity
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
```
