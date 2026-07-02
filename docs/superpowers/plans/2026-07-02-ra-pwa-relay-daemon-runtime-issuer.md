# 2026-07-02 RA/PWA Relay Daemon Runtime Issuer

## Purpose

The relay keyring is now persistent. This slice wires daemon/runtime ticket
issuance to that persisted self-hosted keyring so the next PWA UI slice can
consume a ready setup bundle without ever seeing the HMAC secret.

## Scope

- Load or create `remote-relay-ticket-keys.json` when `ai remote daemon` starts.
- Print only the relay keyring path and active non-secret key id from daemon
  startup output.
- Add `ai remote relay-setup --relay-endpoint-url <url>` to issue a
  self-hosted WebSocket relay setup JSON for a selected registered device.
- Generate fresh relay session id/token values for runtime setup issuance.
- Bind signed ticket metadata to daemon pubkey, companion device id, companion
  Noise pubkey, companion approval pubkey, issued/expiry timestamps, and active
  key id.
- Include daemon/companion connect JSON and companion identity in the setup
  bundle, but never include HMAC keyring secrets.
- Serialize the outer setup bundle in PWA-ready camelCase
  (`signedSessionTicket`, `relayEndpointUrl`, `companionIdentity`,
  `operatorSetupText`) while preserving the existing signed ticket wire shape.

## Guardrails

- Production relay endpoints must use `wss://`.
- Local `ws://` is accepted only for localhost development/smoke endpoints.
- Product default remains `live-loopback`.
- Relay setup issuance requires a registered device and valid daemon key.
- Ticket TTL remains bounded by `DEFAULT_COMPANION_RELAY_SESSION_TTL_MS`.
- The setup JSON must not include `secret` or `hmac_sha256_keys`.

## Work Added

- Add `CompanionRelaySelfHostedRuntimeSetup` and companion identity setup
  records.
- Add `issue_self_hosted_relay_runtime_setup()` backed by
  `CompanionRelayTicketKeyringRecord::issuer()`.
- Add Rust tests for keyring-backed setup issuance, connect validation, secret
  non-exposure, PWA-ready setup JSON, endpoint policy, TTL bounds, and identity
  validation.
- Add `ai remote relay-setup` parser/runtime command.
- Make `ai remote daemon` initialize the persisted relay keyring issuer at
  startup and print its active key id.

## Follow-Up

1. Add visible self-hosted relay setup UI that can ingest the setup bundle,
   endpoint URL, signed ticket, companion identity, and operator setup text.
2. Add browser evidence for the setup bundle feeding `relayTransportUxPreflight`
   into a ready state while `live-loopback` remains the product default.
3. Revisit managed relay and private-network/Tailscale only after separate
   deployment and support evidence exists.

## Verification

```powershell
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote relay_self_hosted_runtime_setup --lib'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote cli_parses_remote_relay_setup --bin ai'
```
