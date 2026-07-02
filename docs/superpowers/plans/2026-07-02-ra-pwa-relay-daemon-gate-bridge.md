# 2026-07-02 RA/PWA Relay Daemon Gate Bridge

## Purpose

`ai remote daemon --transport relay` can now issue setup JSON, and the PWA can
use that setup to exchange relay frames. This slice adds the daemon-side gate
bridge boundary that turns a `RemoteApprovalPlan` into a relay
`ApprovalRequest` message and folds the returned relay `ApprovalResponse`
through the existing nonce, registered-device, signature, and context-drift
validation path.

## Scope

- Add a daemon helper for relay-backed remote approval decisions.
- Reuse `plan_remote_gate` for local-vs-remote gate planning.
- Send only `CompanionTransportMsg::ApprovalRequest` into the relay bridge
  closure.
- Accept only `CompanionTransportMsg::ApprovalResponse` back from the relay
  bridge closure.
- Reuse `finish_remote_gate_response` so relay approvals have the same
  fail-closed semantics as the live listener.
- Add deterministic loopback relay tests for approve, reject, local no-bridge,
  and invalid response type behavior.

## Guardrails

- Product default remains `live-loopback`.
- No Rust WebSocket client dependency is added in this slice.
- The helper is a daemon-side contract boundary. A later runtime slice can bind
  it to a real WebSocket relay loop.
- Relay transport still does not become a production workflow until
  browser/operator approve and reject evidence exists.

## Work Added

- Add `decide_with_remote_relay_bridge` to `src/daemon.rs`.
- Add a small context-hash helper shared by live listener and relay bridge
  decisions.
- Add relay loopback tests that send daemon request frames, simulate companion
  responses, and verify final gate replies.

## Follow-Up

1. Bind `decide_with_remote_relay_bridge` to an actual daemon relay runtime loop
   behind `--transport relay`.
2. Add browser/operator approve and reject evidence with the visible PWA relay
   endpoint loop.
3. Decide whether relay can graduate from setup/debug workflow to selectable
   transport only after hosted deployment and operations evidence.

## Verification

```powershell
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo fmt'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_gate_relay'
```
