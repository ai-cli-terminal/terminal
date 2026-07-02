# 2026-07-02 RA/PWA Relay HTTP Bridge Smoke

## Purpose

The PWA relay helpers have browser parity evidence, but frames have not yet
crossed a process boundary. This slice adds a dependency-free local HTTP bridge
smoke that forwards relay frame JSON by route metadata while leaving payload
decode to the endpoints.

This is still not a product relay transport. It is a repeatable bridge harness
before deciding whether the real browser substrate should be WebSocket, another
HTTP shape, or a different relay deployment.

## Scope

- Add `npm run smoke:pwa-relay-http-bridge`.
- Start a local static PWA server and a local relay bridge server.
- Let the browser PWA module POST daemon and companion relay frame JSON through
  the bridge.
- Have the bridge validate only routing metadata, sequence, expiry, and payload
  size without decoding `payload_json`.
- Verify duplicate sender sequence rejection and expired-frame dropping.
- Write evidence under `artifacts/ra-pwa-relay-http-bridge/`.

## Non-Goals

- No product daemon integration.
- No hosted relay or deployment decision.
- No relay auth token implementation.
- No WebSocket implementation yet.
- No selectable `relay` product transport.

## Verification

```powershell
npm run smoke:pwa-relay-http-bridge
npm run smoke:pwa-relay-browser-parity
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```
