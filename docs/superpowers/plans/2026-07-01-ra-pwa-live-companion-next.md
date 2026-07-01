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

The PWA now has the first live companion path: it can connect to the daemon's
loopback endpoint, receive approval requests through `/events`, and POST signed
approval responses back through `/message`. The main remaining product gap is
full local browser/phone evidence and any follow-up transport-mode decision.

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

Detail: `docs/superpowers/plans/2026-07-01-ra-pwa-live-pwa-ux.md`.

Acceptance:

- [x] PWA shows paired/connected/disconnected state.
- [x] PWA renders incoming approval requests as queue items.
- [x] Approve/reject UI actions send signed responses through the live transport.
- [x] `node pwa/app.test.mjs` covers the transport message, loopback POST, and
  live queue helpers.

### P4 — End-to-end evidence

Problem: current evidence proves pieces, not a full phone/browser roundtrip.

Evidence detail: `docs/superpowers/plans/2026-07-01-ra-pwa-live-e2e-evidence.md`.

Acceptance:

- [x] Add repeatable PWA live evidence harness for PWA helpers, live selectors,
  and Rust endpoint/bridge tests.
- [x] Record P4a evidence path in `docs/HISTORY.md` and `docs/HANDOFF.md`.
- [x] Run a local browser/PWA pairing smoke.
- [x] Run a High opt-in command through daemon -> companion -> signed approval ->
  gate allow/block.
- [x] Record P4b browser/operator evidence paths in `docs/HISTORY.md` and
  `docs/HANDOFF.md`.

## First Implementation Choice

P0/P1/P2/P3/P4 are complete for the local live companion path. P4b now has a
repeatable browser smoke with daemon, PWA session, and High-risk gate
approval/denial. The next implementation choice is the transport mode decision:
keep native `device.sock` as fallback/flag or document live loopback as the
default product path.

## Validation Commands

Use WSL for Rust validation:

```bash
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo fmt --all -- --check'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo test --features remote remote_gate'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo test --features remote cli_parses_remote_daemon'
MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo clippy --features remote -- -D warnings'
node pwa/app.test.mjs
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-approval.ps1
git diff --check
```

## Documentation Updates

After each completed slice:

- Update this file's checkboxes.
- Add a short top entry to `docs/HISTORY.md`.
- Update `docs/HANDOFF.md` next-work status.
- If CLI surface changes, update `README.md` or `docs/INSTALL.md` only when the
  command becomes user-facing enough for release docs.
