# 2026-07-02 RA/PWA Relay M2 Transport Kickoff

## Purpose

RA/PWA local live loopback is now the product default and has browser/operator
evidence. The next local-progress track is Relay/M2: allowing an approved
companion to participate when it is not on the same localhost browser loopback.

This kickoff keeps the current release follow-up blockers separate. Windows MSI,
real Android signing secrets, and F-Droid build/buildserver evidence still need
external environments and remain P1 release follow-up work.

## Current Baseline

- `ai remote daemon` owns the gate socket and starts the browser-compatible
  live loopback endpoint.
- PWA live companion uses `hello`, `approval_request`, `approval_response`,
  `ping`, `pong`, and `error` envelopes through `CompanionTransportMsg`.
- P4b browser/operator smoke proves pair, daemon connect, approve, reject, and
  monitor counters on the live loopback path.
- Native `device.sock` remains an internal/test substrate and future fallback
  candidate, not a user-facing transport flag.

## Non-Goals

- Do not change the default product transport away from `live-loopback`.
- Do not expose a `--transport device-sock` runtime flag yet.
- Do not weaken PWA private-key storage by making private keys exportable.
- Do not treat relay as permission to bypass device registry, signed approval,
  nonce, expiry, context hash, or explicit `hello` identity validation.

## Transport Catalog Decision

Before adding a relay server or WebSocket bridge, the code should have a small
transport catalog with stable ids:

| ID | Status | Role |
|---|---|---|
| `live-loopback` | ready | Product default, browser on same host |
| `device-sock` | internal | Unix socket substrate for tests/future fallback |
| `relay` | planned | M2 remote companion path through a relay service |
| `tailscale` | planned | Direct/private-network candidate |
| `websocket` | planned | Browser-friendly relay/session transport candidate |

The catalog is documentation and status plumbing only. It must not make planned
modes selectable before their security and evidence gates exist.

## First Slice

1. Add a Rust transport catalog with stable ids and readiness metadata.
2. Make daemon startup output derive `PWA transport mode` from that catalog.
3. Add `ai remote transport` as a read-only status command.
4. Keep `npm run smoke:pwa-live-browser-evidence` compatible by preserving the
   exact active mode id `live-loopback`.
5. Update handoff and remaining-work docs so P1 external blockers and local
   Relay/M2 progress are distinct.

## Follow-Up Slices

Progress:

- 2026-07-02: first slice committed as `906da34 feat(remote): catalog companion transports`.
- 2026-07-02: next slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-session-contract.md`.
- 2026-07-02: local harness slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-local-harness.md`.
- 2026-07-02: endpoint adapter slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-endpoint-adapter.md`.
- 2026-07-02: PWA frame parity slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-pwa-frame-parity.md`.
- 2026-07-02: PWA endpoint helper slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-pwa-endpoint-helper.md`.
- 2026-07-02: PWA relay exchange smoke slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-exchange-smoke.md`.
- 2026-07-02: relay threat model and envelope mapping slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-threat-model-envelope.md`.
- 2026-07-02: PWA relay browser parity smoke slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-browser-parity-smoke.md`.

Next slices:

1. Relay process or WebSocket bridge: forward route envelopes and frame JSON
   while preserving endpoint-only approval validation.
2. PWA relay transport UX preflight: keep relay hidden until the process/bridge
   has evidence, then decide what operator-visible setup text is needed.
3. Deployment decision: self-hosted relay, Tailscale/private-network direct
   mode, or managed relay.

## Verification

Minimum checks for the first slice:

```powershell
npm run test:pwa
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features "storage tls remote"'
```

Release follow-up status should remain blocked on this host until external
evidence is supplied:

```powershell
npm run status:release-followup
```
