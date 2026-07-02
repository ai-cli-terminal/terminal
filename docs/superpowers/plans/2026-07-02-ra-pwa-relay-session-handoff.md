# 2026-07-02 RA/PWA Relay Session Handoff

## Status

This handoff closes the 2026-07-02 Relay/M2 working session.

- Working branch: `develop`
- PR target: `main`
- Code closeout commit before this document: `d936596 feat(remote): add relay gate bridge`
- Product default transport: `live-loopback`
- Relay state: setup/debug path only until runtime-loop binding and browser/operator evidence are complete

## Completed Scope

- Cataloged companion transports and kept the active product mode on `live-loopback`.
- Added relay session frame contracts, local relay harnesses, endpoint adapters, and PWA parity helpers.
- Selected WebSocket as the first relay prototype substrate, with HTTP polling retained as fallback/diagnostics.
- Added signed session tickets, rotation/reconnect evidence, persisted relay keyring records, and daemon runtime setup issuance.
- Added visible PWA Relay setup UI plus setup-derived endpoint-loop helpers.
- Added `ai remote daemon --transport relay --relay-endpoint-url <url>` setup validation/issuance while keeping the effective approval bridge on `live-loopback`.
- Added daemon-side `decide_with_remote_relay_bridge` so relay-returned `ApprovalResponse` messages reuse existing nonce, signature, registered-device, and context-drift validation.

## Verification Run

Latest focused verification from this session:

```powershell
npm run smoke:pwa-relay-websocket-bridge
npm run check:pwa-relay-transport-decision
npm run smoke:pwa-relay-setup-ui
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo fmt'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_gate_relay'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_gate_listener'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote companion_live_bridge'
git diff --check
```

Artifacts are written under `artifacts/` and remain uncommitted evidence.

## Next Work Priority

1. Bind `decide_with_remote_relay_bridge` to the actual daemon relay runtime loop behind explicit `--transport relay`.
2. Capture browser/operator approve and reject evidence through the visible PWA Relay tab and setup-derived endpoint loop.
3. Add a self-hosted WebSocket relay deployment runbook and runtime evidence before considering relay selectable outside debug/setup workflow.
4. Decide whether relay graduates from hidden setup/debug to user-selectable transport only after hosted deploy, observability, reconnect, and failure-mode evidence are green.
5. Keep external release follow-up separate: Windows MSI native Rust/MSVC/WiX evidence, real GitHub Android signing secret names, and F-Droid build/buildserver evidence remain blocked by external environment requirements.

## Resume Checklist

1. Confirm branch and remote state:

   ```powershell
   git status --short --branch
   git log --oneline -8
   gh pr status
   ```

2. Re-run the focused relay checks before code changes:

   ```powershell
   npm run smoke:pwa-relay-websocket-bridge
   npm run check:pwa-relay-transport-decision
   wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_gate_relay'
   ```

3. Start with the daemon runtime loop binding. Do not make relay the product default until the browser/operator approve and reject evidence exists.
