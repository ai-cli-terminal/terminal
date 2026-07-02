# 2026-07-02 RA/PWA Relay Endpoint Adapter

## Purpose

The local relay harness can route validated frames, but daemon and companion
callers still need a small endpoint-facing API that works with
`CompanionTransportMsg` instead of manually managing relay frame sequence and
expiry metadata.

This slice adds that endpoint adapter while keeping relay mode non-selectable.

## Scope

- Add a relay endpoint helper for one session and one peer.
- Auto-assign per-sender sequence numbers.
- Apply a bounded frame TTL.
- Send existing `CompanionTransportMsg` values through `CompanionRelayLoopback`.
- Receive and decode existing `CompanionTransportMsg` values at the endpoint
  boundary.

## Non-Goals

- No WebSocket or HTTP relay server.
- No PWA UI changes.
- No runtime `--transport relay` flag.
- No changes to `live-loopback` product default or evidence smoke.

## Design

```text
CompanionRelayEndpoint
  session_id
  peer = daemon | companion
  next_sequence
  frame_ttl_ms

send_message(relay, now_ms, message)
  -> CompanionRelayFrame::from_message(...)
  -> relay.enqueue(...)

recv_message(relay, now_ms)
  -> relay.dequeue(session_id, peer, now_ms)
  -> frame.payload_message()
```

The endpoint adapter is deliberately small. It owns only client-side sequence
state and frame freshness defaults; it does not authorize approvals, select
devices, or parse approval semantics beyond the existing companion transport
message validation.

## Verification

```powershell
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
npm run test:pwa
```

## Progress

- 2026-07-02: implemented in `2f90cc2 feat(remote): add relay endpoint adapter`.
- 2026-07-02: next slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-pwa-frame-parity.md`.
