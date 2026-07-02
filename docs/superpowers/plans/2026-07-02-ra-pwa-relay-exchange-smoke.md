# 2026-07-02 RA/PWA Relay Exchange Smoke

## Purpose

PWA relay frame helpers and endpoint helpers now exist independently. This
slice proves they can run a deterministic daemon-to-companion-to-daemon exchange
using only relay frame JSON, without a WebSocket server or browser UI wiring.

The goal is to keep the future relay/browser smoke small: connection code should
only move frame JSON while endpoint helpers own session, sender, sequence, TTL,
and live transport payload validation.

## Scope

- Add a pure PWA relay exchange helper for one request/reply roundtrip.
- Use daemon and companion relay endpoints for the same session.
- Serialize both directions through relay frame JSON before accept/decode.
- Preserve endpoint sequence increments for both peers.
- Reject mismatched sessions and wrong endpoint roles before sequence mutation.
- Treat expired in-exchange frames as failed exchanges.

## Non-Goals

- No relay server.
- No WebSocket or EventSource transport.
- No PWA UI changes.
- No selectable `relay` product transport.

## Verification

```powershell
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```

## Progress

- 2026-07-02: implemented and committed as
  `c3feade feat(pwa): add relay exchange smoke`.
- 2026-07-02: next slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-threat-model-envelope.md`.
