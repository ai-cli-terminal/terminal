# 2026-07-02 RA/PWA Relay Browser Parity Smoke

## Purpose

Relay frame, endpoint, exchange, and route envelope helpers now exist in the PWA
module. Before adding a relay process or WebSocket bridge, run those helpers in
a real browser module runtime so browser import, `TextEncoder`, frame JSON, and
endpoint payload decode all stay aligned.

This is a browser parity smoke, not a relay product transport. It keeps
`live-loopback` as the default and does not make `relay` selectable.

## Scope

- Add `npm run smoke:pwa-relay-browser-parity`.
- Serve the static PWA from localhost and open it with Playwright/Chromium.
- Import `app.mjs` in the browser context and run a relay approval
  request/response exchange.
- Verify route envelopes expose metadata and payload byte length without
  returning `payload_json`.
- Verify invalid payload JSON is accepted at relay metadata mapping but rejected
  at endpoint decode.
- Write repeatable evidence under `artifacts/ra-pwa-relay-browser-parity/`.

## Non-Goals

- No daemon connection.
- No relay server or WebSocket bridge.
- No hosted relay decision.
- No PWA UI controls for relay mode.
- No selectable `relay` product transport.

## Verification

```powershell
npm run smoke:pwa-relay-browser-parity
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```

## Progress

- 2026-07-02: implemented and committed as
  `1969fb1 test(pwa): add relay browser parity smoke`.
- 2026-07-02: next slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-http-bridge-smoke.md`.
