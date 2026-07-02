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
- 2026-07-02: PWA relay HTTP bridge smoke slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-http-bridge-smoke.md`.
- 2026-07-02: PWA relay WebSocket bridge smoke slice documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-websocket-bridge-smoke.md`.
- 2026-07-02: relay transport shape decision documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-transport-shape-decision.md`.
  WebSocket is the first relay prototype substrate; HTTP polling remains a
  fallback/diagnostics candidate; `live-loopback` remains the product default.
- 2026-07-02: WebSocket relay auth/session contract documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-websocket-auth-session.md`.
  Rust and PWA helpers validate WebSocket session tickets and daemon/companion
  connect messages against session token, peer identity, keys, and expiry.
- 2026-07-02: WebSocket bridge auth enforcement documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-websocket-bridge-auth.md`.
  The local WebSocket bridge smoke now registers tickets, requires daemon and
  companion connect messages before joining a session, and rejects
  unauthenticated/bad-token sockets before frame routing.
- 2026-07-02: signed relay ticket prototype documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-signed-ticket-prototype.md`.
  Rust and PWA helpers produce the same canonical `hmac-sha256` ticket wrapper,
  and the WebSocket bridge smoke now rejects unsigned or bad-MAC tickets at
  registration.
- 2026-07-02: relay session rotation/reconnect evidence documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-session-rotation-reconnect.md`.
  The WebSocket bridge smoke now rejects expired-ticket connects, reconnects
  through a rotated session id/token, rejects old-token reconnect attempts, and
  isolates old-session frames from the new endpoint.
- 2026-07-02: daemon-side ticket issuer/key rotation policy documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-daemon-ticket-issuer-policy.md`.
  Rust `CompanionRelayTicketIssuer` now signs with an active HMAC key, verifies
  against a bounded active+previous keyring, and rejects short, duplicate, or
  over-retained key state.
- 2026-07-02: PWA relay UX preflight documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-ux-preflight.md`.
  The PWA now keeps relay hidden until relay mode, endpoint URL, signed ticket,
  matching companion identity, deployment mode, and operator setup text are all
  present.
- 2026-07-02: relay deployment shape decision documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-deployment-shape-decision.md`.
  The first deployable shape is self-hosted WebSocket relay; managed relay and
  private-network/Tailscale remain deferred candidates.
- 2026-07-02: relay secret keyring migration documented in
  `docs/superpowers/plans/2026-07-02-ra-pwa-relay-secret-keyring-migration.md`.
  Rust now has persistent self-hosted relay HMAC keyring records, optional
  signed ticket key ids, and legacy no-key-id ticket validation fallback.

Next slices:

1. Wire daemon runtime ticket issuance to the persisted self-hosted relay
   keyring.
2. Visible self-hosted relay setup UI after endpoint, ticket, identity, and
   operator copy are ready.
3. Revisit managed relay and private-network/Tailscale only after separate
   deployment and support evidence exists.

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
