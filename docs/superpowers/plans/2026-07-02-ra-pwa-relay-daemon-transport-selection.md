# 2026-07-02 RA/PWA Relay Daemon Transport Selection

## Purpose

The PWA endpoint loop can now authenticate to a self-hosted WebSocket relay from
a daemon-issued setup bundle. This slice gives `ai remote daemon` an explicit
transport selection surface so relay setup can be requested from the daemon
runtime without changing the product default away from `live-loopback`.

## Scope

- Add `ai remote daemon --transport <mode>` with `live-loopback` as the default.
- Add `--relay-endpoint-url` and `--relay-ttl-seconds` for explicit
  `--transport relay` startup.
- Validate daemon transport selection before the gate daemon starts.
- Reject relay endpoint input for `live-loopback`.
- Reject planned non-runtime modes (`device-sock`, `tailscale`, `websocket`) as
  user-selectable daemon transports for now.
- When relay is explicitly requested, issue a daemon/keyring-backed self-hosted
  relay setup JSON at daemon startup and print it for PWA import.

## Guardrails

- `PWA transport mode` remains `live-loopback`; existing browser evidence that
  depends on the local live endpoint should keep passing.
- Relay startup prints `setup-issued (gate bridge pending)` because gate
  approval traffic still uses the live listener in this slice.
- Relay mode still requires a selected registered device, daemon public key, and
  daemon-owned relay ticket keyring.
- No WebSocket client dependency is added to Rust in this slice.

## Work Added

- Extended `RemoteAction::Daemon` parsing with `--transport`,
  `--relay-endpoint-url`, and `--relay-ttl-seconds`.
- Added daemon transport selection validation for live-loopback and relay.
- Wired explicit relay daemon startup to issue the same self-hosted runtime
  setup bundle as `ai remote relay-setup`.
- Added CLI parse coverage and relay selection validation tests.

## Follow-Up

1. Add the daemon-side relay gate bridge loop that sends `ApprovalRequestMsg`
   as relay frames and waits for matching `ApprovalResponseMsg` frames.
2. Add browser/operator approve and reject evidence using the explicit relay
   daemon startup path.
3. Only after that evidence, decide whether relay can become user-selectable
   beyond setup/debug workflows.

## Verification

```powershell
cargo fmt
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote cli_parses_remote_daemon'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote daemon_transport_selection_validates_relay_inputs'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal-default; cargo test cli_parses_remote_daemon'
```
