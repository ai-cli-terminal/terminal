# 2026-07-04 RA/PWA Relay Private-Network Runtime Guardrails

## Purpose

Add daemon/runtime guardrails for explicit private-network Relay/M2 setup after
the PWA setup contract is defined.

## Status

Completed in this slice. The follow-up operator evidence slice is also
complete; the next local implementation slice is private-network relay visible
import path.

## Scope

- Add daemon CLI parsing for `--relay-deployment-mode private-network`.
- Require `--private-network-name` for private-network relay setup.
- Keep `self-hosted` as the default relay deployment mode.
- Keep `live-loopback` as the product default.
- Keep managed relay deferred.
- Emit `privateNetworkName` in setup JSON only for private-network setup.
- Reject public `ws://` endpoints before setup JSON emission.
- Add PWA private-network runtime setup preflight.
- Add a repeatable guardrails check.

## Non-Goals

- Do not make private-network relay the product default.
- Do not introduce managed relay operations.
- Do not expose private-network setup through the self-hosted visible setup UI.
- Do not add a new relay service protocol.

## Work Added

- Added daemon and `relay-setup` CLI fields for relay deployment mode and
  private-network name.
- Added Rust setup validation for self-hosted vs private-network deployment
  boundaries.
- Added optional `privateNetworkName` setup JSON emission for private-network
  setup only.
- Added PWA private-network runtime setup validation and preflight helper.
- Added `npm run check:pwa-relay-private-network-runtime-guardrails`.

## Follow-Up

Private-network relay operator evidence is complete:

- capture CLI-emitted private-network setup JSON evidence;
- verify PWA private-network runtime preflight import from that setup JSON;
- exercise a local/private-network relay bridge roundtrip using the same setup
  boundary;
- keep `live-loopback` default and managed relay deferred.

The next slice is private-network relay visible import path.

## Verification

```powershell
npm run check:pwa-relay-private-network-runtime-guardrails
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote relay_private_network_runtime_setup_emits_guarded_contract'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote daemon_transport_selection_validates_relay_inputs'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote cli_parses_remote_daemon_relay_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote cli_parses_remote_relay_setup'
git diff --check
```
