# 2026-07-02 RA/PWA Relay WebSocket Auth Session

## Purpose

WebSocket is now the first relay prototype substrate, but the bridge still needs
a fail-closed session contract before it can become product-facing. This slice
adds the shared Rust/PWA ticket and connect-message validation boundary for that
future WebSocket relay.

## Scope

- Add a WebSocket-only relay session ticket contract.
- Bind a session ticket to:
  - `session_id`
  - `session_token`
  - daemon public key
  - companion device id
  - companion Noise public key
  - companion approval public key
  - issued/expires timestamps
- Add daemon and companion connect-message helpers.
- Validate connect messages fail closed on token, session, peer, key, device,
  or expiry mismatch.
- Keep `relay` and `websocket` transport modes planned, not product-ready.

## Non-Goals

- No hosted relay implementation.
- No HMAC/signed relay ticket yet.
- No product default change away from `live-loopback`.
- No operator-visible relay setup UX.

## Added Contract

The bridge-facing auth shape is:

```json
{
  "relay_protocol_version": 1,
  "transport": "websocket",
  "session_id": "relay-ws-session-1",
  "session_token": "token_...",
  "issued_at_ms": 11000,
  "expires_at_ms": 311000,
  "daemon_pubkey_hex": "...",
  "companion_device_id": "web-...",
  "companion_noise_pubkey_hex": "...",
  "companion_approval_pubkey_hex": "..."
}
```

Daemon connect messages must present the same `session_id`, `session_token`,
and `daemon_pubkey_hex`. Companion connect messages must present the same
`session_id`, `session_token`, `device_id`, `noise_pubkey_hex`, and
`approval_pubkey_hex`.

## Progress

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-websocket-bridge-auth.md`
wired this ticket/connect contract into the local WebSocket bridge smoke. The
bridge now rejects unauthenticated frame messages and bad-token connect messages
before frame routing.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-signed-ticket-prototype.md`
wrapped tickets with an `hmac-sha256` MAC and updated the bridge smoke to reject
unsigned or bad-MAC tickets at registration.

## Follow-Up

1. Add relay session rotation evidence and reconnect behavior.
2. Add daemon-side ticket issuer state and key rotation policy when deployment
   shape is chosen.
3. Decide relay setup UX only after deployment mode is chosen.

## Verification

```powershell
npm run test:pwa
npm run smoke:pwa-relay-websocket-bridge
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
```
