# 2026-07-02 RA/PWA Relay WebSocket Bridge Smoke

## Purpose

The HTTP bridge smoke proves relay frame JSON can cross a local process-like
boundary, but it uses request/response polling. This slice adds a WebSocket
bridge candidate smoke so the relay prototype has browser-native full-duplex
evidence before selecting a product transport shape.

This is still not a product relay transport. It is a candidate harness that
reuses the existing relay frame contract and keeps approval payload validation
at daemon/companion endpoints.

## Scope

- Add `npm run smoke:pwa-relay-websocket-bridge`.
- Start a local static PWA server and a dependency-free local WebSocket bridge.
- Connect browser PWA relay endpoints as `daemon` and `companion` WebSocket
  peers.
- Route relay frame JSON by visible route metadata while keeping
  `payload_json` opaque to the bridge.
- Verify duplicate sender sequence rejection and expired-frame dropping.
- Write evidence under `artifacts/ra-pwa-relay-websocket-bridge/`.

## Non-Goals

- No product daemon integration.
- No hosted relay deployment.
- No relay auth token implementation.
- No default transport change away from `live-loopback`.
- No operator-visible relay UX yet.

## Decision Note

This smoke closes the immediate "WebSocket candidate" question at the harness
level. The next decision should compare HTTP polling versus WebSocket for the
first relay prototype using operational requirements: connection durability,
mobile browser background behavior, server complexity, auth/session renewal,
and observability.

## Verification

```powershell
npm run smoke:pwa-relay-websocket-bridge
npm run smoke:pwa-relay-http-bridge
npm run smoke:pwa-relay-browser-parity
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```
