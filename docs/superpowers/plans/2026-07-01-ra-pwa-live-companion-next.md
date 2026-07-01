# RA/PWA Live Companion Next Work — 2026-07-01

> This is the live next-work document for finishing the local remote-approval
> companion path after v0.3.3. It narrows the broad RA/PWA backlog into
> small slices that can be implemented and verified independently.

## Current State

The desktop daemon already has the core RA path:

- `ai remote daemon` starts a local gate socket and a daemon-owned `device.sock`.
- `ai remote pair` creates a pairing code, persists the daemon Noise key, and can
  register a device in `remote-devices.json`.
- High-risk commands with `ai remote arm --allow-high` can be promoted to a
  registered-device approval request.
- The PWA can parse pair/approval payloads, generate WebCrypto identity keys,
  sign approve/reject responses, and produce `ai remote approval-verify` commands.
- The queue-backed device listener now uses per-request response channels and
  accept timeouts, so a timed-out approval does not poison the next request.

The main remaining product gap is that the companion is still a manual handoff
flow. A real browser/phone session does not yet stay connected to the daemon,
receive approval requests live, and push the signed response back automatically.

## Invariants

- Remote approval remains fail-closed for timeout, invalid signature, replay,
  stale context hash, missing device, or ambiguous device selection.
- PWA private keys stay non-extractable in IndexedDB. Only public keys and signed
  approval responses leave the browser.
- Desktop daemon remains local-first. Relay/Tailscale/WebSocket transport can be
  added later, but the first live path should work on localhost or local network.
- Existing one-device flows must continue to work without new flags.

## Priority Slices

### P0 — Multi-device selection floor

Problem: `plan_remote_gate()` currently relies on `DeviceRegistry::single_device()`.
That keeps early flows simple, but pairing a second device makes daemon approvals
fail as ambiguous.

Acceptance:

- [x] `ai remote daemon` accepts an optional `--device-id <id>`.
- [x] Without `--device-id`, one registered device still works.
- [x] Without `--device-id`, zero or multiple registered devices fail closed with
  an actionable message.
- [x] With `--device-id`, the daemon uses that registered device for gate
  approval planning.
- [x] Tests cover selected-device planning and ambiguous multi-device failure.

### P1 — Device registry operator surface

Problem: users need a supported way to inspect registered device ids before
starting the daemon with a selected device.

Acceptance:

- [x] `ai remote devices` lists registered device ids, epochs, paired timestamps,
  and public key fingerprints/hex values.
- [x] Empty registry prints a clear empty state.
- [x] The command does not reveal private key material.

### P2 — Live local companion transport

Problem: the PWA can sign responses, but it does not yet connect to the daemon and
wait for approval requests.

Contract detail: `docs/superpowers/plans/2026-07-01-ra-pwa-live-transport-contract.md`.
Loopback endpoint detail:
`docs/superpowers/plans/2026-07-01-ra-pwa-live-loopback-endpoint.md`.
Approval bridge detail:
`docs/superpowers/plans/2026-07-01-ra-pwa-live-approval-bridge.md`.

Acceptance:

- [x] Define a local transport contract for browser companion connections.
- [x] Serve or document the daemon endpoint the PWA should connect to.
- [x] Browser transport can receive an approval request without manual JSON copy.
- [x] Browser transport can send a signed approve/reject response back to the daemon.
- [x] Rust/PWA tests cover the shared transport envelope shape and malformed
  message rejection.
- [x] Rust tests cover network transport framing and fail-closed timeout behavior.
- [x] Rust tests cover live gate request -> SSE `approval_request` -> HTTP
  `approval_response` -> existing gate validation.

### P3 — PWA live approval UX

Problem: manual approval JSON flow is useful for smoke tests but not a companion
experience.

Acceptance:

- [ ] PWA shows paired/connected/disconnected state.
- [ ] PWA renders incoming approval requests as queue items.
- [ ] Approve/reject UI actions send signed responses through the live transport.
- [x] `node pwa/app.test.mjs` covers the transport message and loopback POST helpers.

### P4 — End-to-end evidence

Problem: current evidence proves pieces, not a full phone/browser roundtrip.

Acceptance:

- [ ] Run a local browser/PWA pairing smoke.
- [ ] Run a High opt-in command through daemon -> companion -> signed approval ->
  gate allow/block.
- [ ] Record evidence paths in `docs/HISTORY.md` and `docs/HANDOFF.md`.

## First Implementation Choice

P0/P1/P2 are complete at the transport/backend level: contract, loopback
endpoint, and gate approval bridge are wired. The next implementation choice is
P3: make the static PWA use the printed live endpoint, render incoming approvals,
and send approve/reject responses through the live transport.

## Validation Commands

Use WSL for Rust validation:

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo fmt --all -- --check'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo test --features remote remote_gate'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo test --features remote cli_parses_remote_daemon'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo clippy --features remote -- -D warnings'
node pwa/app.test.mjs
git diff --check
```

## Documentation Updates

After each completed slice:

- Update this file's checkboxes.
- Add a short top entry to `docs/HISTORY.md`.
- Update `docs/HANDOFF.md` next-work status.
- If CLI surface changes, update `README.md` or `docs/INSTALL.md` only when the
  command becomes user-facing enough for release docs.
