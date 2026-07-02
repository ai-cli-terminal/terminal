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
- No hosted relay deployment.
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

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-session-rotation-reconnect.md`
extended the bridge smoke with expired-ticket connect rejection, old/new
session token rotation, reconnect delivery, and stale-session frame isolation.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-daemon-ticket-issuer-policy.md`
added the daemon-side issuer/key rotation policy and Rust helper for active-key
issuance plus bounded active+previous key verification.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-ux-preflight.md` added the PWA
visibility preflight so relay remains hidden until all readiness inputs are
present.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-deployment-shape-decision.md`
selected self-hosted WebSocket relay as the first deployable shape.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-secret-keyring-migration.md`
added persistent daemon-owned relay ticket keyring records and optional signed
ticket key ids.

## Follow-Up

1. Wire daemon runtime ticket issuance to the persisted self-hosted relay
   keyring.
2. Add visible self-hosted relay setup UI after endpoint, ticket, identity, and
   operator copy are ready.
3. Revisit managed relay and private-network/Tailscale only after separate
   deployment and support evidence exists.

## Verification

```powershell
npm run test:pwa
npm run smoke:pwa-relay-websocket-bridge
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
```
