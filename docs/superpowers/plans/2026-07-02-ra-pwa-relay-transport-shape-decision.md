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
- Do not expose operator-visible relay setup until auth/session/deployment
  boundaries are documented and tested.

## Work Added

- Add `npm run check:pwa-relay-transport-decision`.
- Refresh `npm run smoke:pwa-relay-http-bridge` and
  `npm run smoke:pwa-relay-websocket-bridge` through the check command.
- Assert both evidence files prove matching relay invariants.
- Write aggregate decision evidence under
  `artifacts/ra-pwa-relay-transport-decision/`.

## Follow-Up

1. Define the relay auth/session handshake around the WebSocket prototype.
2. Add a PWA relay UX preflight that keeps relay hidden until endpoint URL,
   session, auth, and deployment text are ready.
3. Decide deployment shape: self-hosted relay, private-network/Tailscale direct
   mode, or managed relay.

## Verification

```powershell
npm run check:pwa-relay-transport-decision
npm run smoke:pwa-relay-browser-parity
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
```
