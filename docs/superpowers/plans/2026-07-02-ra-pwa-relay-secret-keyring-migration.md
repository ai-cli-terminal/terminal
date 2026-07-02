# 2026-07-02 RA/PWA Relay Secret Keyring Migration

## Purpose

The first deployable relay shape is self-hosted WebSocket relay. The daemon now
needs a persistent relay ticket HMAC keyring so tickets can survive daemon
restart and key rotation can be explicit instead of an in-memory test-only
policy.

## Scope

- Add a daemon-owned relay ticket keyring record.
- Persist the record as `remote-relay-ticket-keys.json` under the config
  directory.
- Keep HMAC secrets daemon-owned. PWA and relay bridge code receive signed
  tickets, never the HMAC secret.
- Add optional non-secret `key_id` to signed relay ticket wrappers.
- Preserve legacy no-key-id tickets by validating them against the bounded
  active+previous verifier keyring.
- Sign new tickets with the active key id when the issuer is created from the
  persistent keyring.

## Record Shape

```json
{
  "version": 1,
  "active_key_id": "relay-active-2",
  "hmac_sha256_keys": [
    {
      "key_id": "relay-active-2",
      "secret": [1, 2, 3],
      "created_at_ms": 2000
    },
    {
      "key_id": "relay-active-1",
      "secret": [4, 5, 6],
      "created_at_ms": 1000,
      "retired_at_ms": 2000
    }
  ]
}
```

The record is intentionally local daemon state. It is not logged, rendered in
PWA UI, or included in relay evidence JSON.

## Guardrails

- Reject short HMAC secrets, duplicate key ids, duplicate secret bytes, missing
  active keys, retired active keys, and previous keys without `retired_at_ms`.
- Keep verifier key retention bounded by
  `MAX_COMPANION_RELAY_TICKET_HMAC_VERIFY_KEYS`.
- Keep existing no-key-id ticket verification working during migration.
- Reject unknown keyed tickets fail-closed instead of falling back to all keys.
- Do not expose relay setup UI in this slice.

## Work Added

- Add `CompanionRelayTicketKeyringRecord` and
  `CompanionRelayTicketHmacKeyRecord`.
- Add load/save/load-or-create helpers for `remote-relay-ticket-keys.json`.
- Add keyring-backed `CompanionRelayTicketIssuer` construction.
- Add optional signed ticket `key_id` metadata.
- Add Rust tests for persistence, rotation, legacy ticket migration, and bad
  persistent state.
- Add PWA metadata validation for optional signed ticket `key_id`.

## Follow-Up

1. Wire the daemon relay ticket issuer to the persisted self-hosted keyring.
2. Add visible self-hosted relay setup UI after endpoint, ticket, identity, and
   operator copy are ready.
3. Revisit managed relay and private-network/Tailscale only after separate
   deployment and support evidence exists.

## Verification

```powershell
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```
