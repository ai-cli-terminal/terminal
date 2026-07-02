# 2026-07-02 RA/PWA Relay Threat Model and Envelope Mapping

## Purpose

Relay/M2 now has a frame contract, local harness, endpoint adapters, and PWA
parity. Before adding any relay process or browser transport, freeze the threat
boundary and the exact envelope that relay code may route on.

The relay is a transport component. It is not an approval authority and must not
decide whether a command is safe, whether a device is registered, or whether an
approval response is valid.

## Threat Model

| Area | Decision |
|---|---|
| Relay authority | Route frames only. Approval validation remains daemon-side through device registry, signed response, nonce, expiry, and context hash. |
| Relay-visible metadata | `relay_protocol_version`, `session_id`, `sender`, `sequence`, `sent_at_ms`, `expires_at_ms`, and payload byte length. |
| Payload handling | Relay code treats `payload_json` as bytes. Endpoint code decodes it as `CompanionTransportMsg`. |
| Payload confidentiality | Current frame JSON is not confidential from a relay operator. Hosted relay is blocked until payload encryption or an equivalent trust decision is made. |
| Auth tokens | Future relay admission tokens may authorize session access only. They must not replace pairing, registry lookup, or approval signatures. |
| Replay/expiry | Relay rejects non-increasing sender sequence in a session and drops expired frames; daemon endpoint still enforces approval nonce/expiry. |
| Session ids | Session ids are bounded ASCII routing labels, not authentication secrets. |
| Heartbeats | `ping`/`pong` remain endpoint payload messages. Relay may transport them but does not interpret liveness semantics. |
| Failure modes | Unknown protocol version, bad session id, invalid sender, bad sequence/timestamps, oversize payload, expired frame, and wrong-session endpoint receipt fail closed. |

## Scope

- Add Rust and PWA route envelope helpers that expose routing metadata without
  returning `payload_json`.
- Keep invalid payload JSON acceptable at the relay metadata boundary and
  rejected only at endpoint payload decode.
- Document the hosted relay blocker around payload confidentiality.
- Keep `live-loopback` as the active product transport.

## Non-Goals

- No relay server.
- No WebSocket browser transport.
- No relay auth token implementation.
- No payload encryption implementation.
- No selectable `relay` transport mode.

## Verification

```powershell
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```
