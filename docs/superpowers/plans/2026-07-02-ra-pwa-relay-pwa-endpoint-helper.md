# 2026-07-02 RA/PWA Relay PWA Endpoint Helper

## Purpose

The PWA can now validate and build relay frame JSON. The next browser-facing
slice is a small endpoint helper that owns companion-side sequence state and
accepts inbound relay frames for the same session.

This keeps the future WebSocket smoke simple: WebSocket code can move relay
frame JSON strings, while endpoint helpers continue to work with existing
`CompanionTransportMsg` objects.

## Scope

- Add PWA relay endpoint state with `sessionId`, `sender`, `nextSequence`, and
  `frameTtlMs`.
- Add outbound frame creation that increments sequence only after a valid frame
  is built.
- Add inbound frame acceptance from object or JSON text.
- Reject wrong-session and self-sent inbound frames.
- Drop expired inbound frames.
- Decode payloads only through the existing live transport message validator.

## Non-Goals

- No WebSocket connection code.
- No relay server.
- No PWA UI changes.
- No selectable `relay` transport mode.

## Verification

```powershell
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```
