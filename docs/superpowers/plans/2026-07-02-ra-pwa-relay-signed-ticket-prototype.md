# 2026-07-02 RA/PWA Relay Signed Ticket Prototype

## Purpose

The WebSocket relay bridge already requires a session ticket and connect
message before routing frames. This slice makes the ticket fail-closed as an
issued artifact: a bridge accepts only tickets wrapped with an `hmac-sha256`
MAC over a canonical ticket payload.

## Scope

- Add a Rust `CompanionRelaySignedSessionTicket` wrapper.
- Define the canonical ticket signing payload with stable field order and
  newline separators.
- Add Rust HMAC-SHA256 creation and verification helpers under the `remote`
  feature.
- Add PWA WebCrypto helpers that create and verify the same signed ticket.
- Tighten the local WebSocket bridge smoke so `/sessions` accepts only signed
  tickets.
- Prove unsigned tickets and bad-MAC tickets are rejected before connect or
  frame routing.

## Non-Goals

- No hosted relay deployment.
- No persistent daemon ticket issuer storage yet.
- No key rotation or multi-key verification yet.
- No product default change away from `live-loopback`.
- No operator-visible relay setup UX.

## Signed Shape

The signed wrapper is:

```json
{
  "ticket": {
    "relay_protocol_version": 1,
    "transport": "websocket",
    "session_id": "relay-ws-session-1",
    "session_token": "token_...",
    "issued_at_ms": 1000,
    "expires_at_ms": 2000,
    "daemon_pubkey_hex": "...",
    "companion_device_id": "web-...",
    "companion_noise_pubkey_hex": "...",
    "companion_approval_pubkey_hex": "..."
  },
  "mac_alg": "hmac-sha256",
  "mac_hex": "..."
}
```

The MAC payload begins with `ai-terminal-relay-ticket-v1` and then one
`key=value` line per ticket field. The field order is fixed in Rust, PWA, and
the bridge smoke.

## Evidence Shape

The WebSocket smoke now proves:

- `registeredTickets=4`: every happy-path and expired-frame test session uses a
  signed ticket.
- `rejectedTickets=2`: one unsigned ticket and one bad-MAC ticket are rejected
  at registration.
- `acceptedConnects=4` and `rejectedConnects=2`: connect auth still gates all
  WebSocket peers.
- `acceptedFrames=3`, `deliveredFrames=2`, `expiredFrames=1`,
  `rejectedFrames=1`: relay-frame behavior remains unchanged after signing.

Evidence path:

```text
artifacts/ra-pwa-relay-websocket-bridge/ra-pwa-relay-websocket-bridge.json
```

## Follow-Up

1. Add relay session rotation and reconnect evidence.
2. Add daemon-side ticket issuer state and key rotation policy when a hosted
   relay shape is chosen.
3. Decide relay setup UX only after deployment mode is chosen.

## Verification

```powershell
npm run test:pwa
npm run smoke:pwa-relay-websocket-bridge
npm run check:pwa-relay-transport-decision
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
```
