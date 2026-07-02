# 2026-07-02 RA/PWA Relay PWA Frame Parity

## Purpose

Rust now has a relay frame, loopback harness, and endpoint adapter. Before a
browser/WebSocket smoke can be meaningful, the PWA needs the same relay frame
JSON contract so it can wrap and unwrap existing `CompanionTransportMsg`
payloads without inventing a second browser-only shape.

## Scope

- Add PWA relay frame constants and validators matching the Rust contract.
- Build relay frames from existing live companion transport messages.
- Parse relay frames from JSON text.
- Decode relay frame payloads only at the endpoint boundary.
- Add PWA tests that prove valid metadata, invalid metadata rejection, payload
  opacity, and approval payload preservation.

## Non-Goals

- No WebSocket connection UI.
- No hosted relay or daemon relay server.
- No change to `live-loopback` default behavior.
- No private key export or approval validation weakening.

## Contract

The PWA uses the same frame shape as Rust:

```json
{
  "relay_protocol_version": 1,
  "session_id": "session-a",
  "sender": "companion",
  "sequence": 1,
  "sent_at_ms": 100,
  "expires_at_ms": 30100,
  "payload_json": "{\"type\":\"ping\",\"nonce\":\"p1\"}"
}
```

Relay frame validation checks routing metadata and payload size. It does not
parse approval semantics. Endpoint decode uses the existing
`parseLiveTransportMessage` validator.

## Verification

```powershell
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```
