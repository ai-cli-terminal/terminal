# 2026-07-04 RA/PWA Relay Daemon Runtime Loop

## Purpose

Close the next local Relay/M2 gap from the 2026-07-02 handoff: bind the
daemon-side `decide_with_remote_relay_bridge` helper to the actual daemon
runtime path behind explicit `ai remote daemon --transport relay`.

## Status

Completed in this slice. Explicit relay daemon startup now feeds setup-derived
runtime state into a relay bridge instead of falling back to the live-loopback
listener. The product default remains `live-loopback`.

## Remaining Work Priority

| Priority | Work | Completion Criteria | Blockers / Notes |
|---|---|---|---|
| P1 | PWA Relay approve/reject evidence | Visible PWA Relay tab approves and rejects through setup-derived endpoint loop against a running daemon/relay bridge | Depends on runtime loop binding |
| P2 | Self-hosted relay deployment runbook | Operator runbook covers hosted relay, ticket registration, reconnect, observability, and failure evidence | Do not make relay user-selectable before this |
| External P1 | Release follow-up closeout | Windows MSI native evidence, Android signing secret names, and F-Droid build/buildserver evidence are ready | Requires external host/secrets/buildserver |

## Scope

- Keep `live-loopback` as the product default and active mode.
- Add a daemon runtime relay state selected only by explicit
  `--transport relay`.
- Use existing relay setup JSON and daemon connect metadata as the runtime
  source of truth.
- Reuse `decide_with_remote_relay_bridge` and `finish_remote_gate_response` so
  relay approvals retain nonce, registered-device signature, and context drift
  validation.
- Add deterministic tests for runtime selection and relay-backed daemon
  decisions.

## Non-Goals

- Do not add hosted relay infrastructure.
- Do not make relay the default or broadly user-selectable product transport.
- Do not close browser/operator evidence in this slice unless the runtime loop
  is already green.
- Do not change release tags or public assets.

## Implementation Plan

1. Introduce a `RemoteDaemonBridge` selection inside `daemon.rs` with live
   listener and relay variants.
2. Add a relay runtime bridge object that owns the setup-derived daemon relay
   endpoint state.
3. Route `RemoteDaemonState::decide_with_arm` through the relay bridge when the
   daemon was started with `--transport relay`.
4. Update `main.rs` startup so relay setup issuance feeds the relay runtime
   state instead of only printing setup JSON.
5. Add focused Rust tests for relay daemon runtime decisions.
6. Re-run PWA relay smoke, focused Rust relay tests, and `git diff --check`.

## Work Added

- Added a live-vs-relay bridge selection inside `DaemonRuntime`.
- Added `CompanionRelayDaemonRuntime`, which owns setup-derived daemon relay
  endpoint state and registers the signed session ticket with a local
  self-hosted relay bridge.
- Added a small dependency-free WebSocket client for `ws://localhost` relay
  evidence. `wss://` remains a clear runtime error until hosted/TLS evidence is
  added.
- Updated `ai remote daemon --transport relay --relay-endpoint-url <url>` so it
  registers the ticket and serves gate decisions through `serve_with_remote_relay`.
- Added relay daemon runtime tests, including an actual local WebSocket
  handshake/frame roundtrip.

## Guardrails

- Relay runtime failures must fail closed with a clear gate block reason.
- Local Low/Medium and Critical decisions should not contact relay.
- Relay responses must be `ApprovalResponse`; `Error` or unexpected message
  types block the command.
- The setup JSON must not expose relay ticket HMAC key material.

## Verification

```powershell
npm run test:pwa
npm run smoke:pwa-relay-websocket-bridge
npm run check:pwa-relay-transport-decision
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo fmt'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_gate_relay'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote relay_daemon_runtime'
git diff --check
```

Latest run:

- `npm run test:pwa` — `PWA_COMPANION_TEST_OK`
- `npm run smoke:pwa-relay-websocket-bridge` —
  `RA_PWA_RELAY_WEBSOCKET_BRIDGE_OK`
- `npm run check:pwa-relay-transport-decision` —
  `RA_PWA_RELAY_TRANSPORT_DECISION_OK`
- `cargo test --features remote` — 409 lib tests, 41 CLI tests, integration
  tests, intercept e2e, version sync, and doc tests all passed
- `cargo clippy --all-targets --features remote -- -D warnings` — passed
- `git diff --check` — passed
