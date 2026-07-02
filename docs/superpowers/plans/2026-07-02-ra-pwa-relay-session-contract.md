# 2026-07-02 RA/PWA Relay Session Contract

## Purpose

The transport catalog exists and keeps `live-loopback` as the active product
mode. The next Relay/M2 slice is to freeze the session contract before adding a
relay process or browser WebSocket path.

This slice should make the relay boundary explicit:

- the relay may route by session metadata;
- endpoints still validate `CompanionTransportMsg`;
- approval signatures, nonce consumption, expiry, context hash, device registry,
  and hello identity checks remain endpoint responsibilities;
- planned relay code must not make `relay` user-selectable yet.

## Threat Model

The relay is a routing component, not an approval authority.

| Risk | Required behavior |
|---|---|
| Relay observes traffic | Keep relay metadata minimal: session id, sender, sequence, timestamps, payload JSON bytes. Do not require relay logic to inspect approval semantics. |
| Replay or duplicate delivery | Require monotonically usable sequence metadata and endpoint nonce/expiry validation. The first slice records sequence shape; replay enforcement belongs to the harness/session store slice. |
| Cross-session injection | Every relayed frame carries a validated session id. The harness must only forward within a session bucket. |
| Companion spoofing | Relay metadata is not identity. Endpoints must continue to require `hello` and registry public-key matching. |
| Expired approval response | Relay frame expiry is an outer freshness guard; approval request `expires_at` and nonce consumption remain authoritative. |

## Contract

The first code contract is a relay frame wrapper around the existing companion
transport message JSON:

```text
CompanionRelayFrame
  relay_protocol_version = 1
  session_id             = stable ASCII routing id
  sender                 = daemon | companion
  sequence               = positive u64
  sent_at_ms             = positive unix epoch ms
  expires_at_ms          = greater than sent_at_ms
  payload_json           = existing CompanionTransportMsg JSON
```

The wrapper intentionally stores `payload_json` as JSON text. A relay can
validate metadata and byte limits without parsing approval semantics, while the
daemon/PWA endpoints can decode the payload as `CompanionTransportMsg`.

## Implementation Slice

1. Add `CompanionRelayPeer` and `CompanionRelayFrame` to `remote_transport`.
2. Add metadata validation: session id, sequence, sent/expiry times, and payload
   size.
3. Add endpoint helper validation by decoding `payload_json` back into
   `CompanionTransportMsg`.
4. Add tests proving relay frames preserve existing messages, reject bad
   metadata, and keep invalid payload handling at the endpoint decode step.

## Progress

- 2026-07-02: implemented in `770e754 feat(remote): define relay session frame`.
- 2026-07-02: next slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-local-harness.md`.

## Non-Goals

- No relay server, WebSocket bridge, or deployment target.
- No runtime `--transport relay` flag.
- No change to live browser smoke or active transport id.
- No weakening of PWA private key storage or approval validation.

## Verification

```powershell
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
npm run test:pwa
```
