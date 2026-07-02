# 2026-07-02 RA/PWA Relay Local Harness

## Purpose

The relay session frame contract is now typed. The next Relay/M2 slice is a
local harness that routes those frames between daemon and companion without
turning relay code into an approval authority.

This is still not a network relay. It is the smallest executable substrate for
the future loopback relay process and browser WebSocket smoke.

## Scope

- Add an in-memory relay harness for `CompanionRelayFrame`.
- Route frames by `session_id` and recipient direction.
- Reject non-monotonic sender sequence numbers within a session.
- Drop expired frames during dequeue.
- Keep payloads opaque to the harness. Endpoint helpers still decode
  `CompanionTransportMsg`.

## Non-Goals

- No daemon command or runtime transport switch.
- No WebSocket, HTTP, Tailscale, or hosted relay.
- No user-selectable `relay` transport mode.
- No changes to live loopback browser evidence.

## Design

The harness owns a map of session buckets:

```text
session_id
  daemon -> companion queue
  companion -> daemon queue
  last daemon sequence
  last companion sequence
```

Enqueue validates relay frame metadata and requires each sender's sequence to
increase strictly within the session. Dequeue chooses the opposite-direction
queue for the requested recipient and skips expired frames.

The harness never parses `payload_json`. Tests use `payload_message()` at the
endpoint boundary to prove the existing companion transport payload survives the
relay unchanged.

## Verification

```powershell
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
npm run test:pwa
```
