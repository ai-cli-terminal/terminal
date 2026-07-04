# 2026-07-04 RA/PWA Relay Daemon WSS Runtime Support

## Purpose

Close the next local Relay/M2 implementation slice: let the daemon relay runtime
use hosted `wss://` endpoints when the existing `tls` feature is enabled, while
keeping localhost `ws://` evidence and product defaults unchanged.

## Status

Completed in this slice. The daemon relay runtime now parses hosted `wss://`
endpoints, registers tickets over HTTPS/TLS, and performs the WebSocket upgrade
over TLS when built with `remote,tls`. Builds without `tls` still fail closed
with a clear feature requirement.

## Scope

- Extend daemon relay endpoint parsing from localhost-only `ws://` to
  scheme-aware `ws://` and `wss://`.
- Keep public `ws://` blocked; only localhost `ws://` remains valid.
- Use HTTPS over TLS for signed ticket registration when the endpoint is
  `wss://`.
- Use TLS plus WebSocket upgrade for daemon relay sockets when the endpoint is
  `wss://`.
- Keep `wss://` fail-closed with a clear `tls` feature requirement in builds
  that do not enable `tls`.
- Update hosted-readiness evidence so daemon WSS runtime support is no longer
  the next local blocker.

## Non-Goals

- Do not add a production relay service artifact.
- Do not change relay ticket signing or key distribution.
- Do not add payload encryption.
- Do not make relay the default transport or broadly user-selectable.
- Do not close hosted observability or hosted failure-mode evidence.

## Guardrails

- `live-loopback` remains the product default.
- Local smoke evidence must keep using localhost `ws://`.
- Hosted relay endpoints must use `wss://`.
- Relay still forwards frames only; daemon-side approval validation remains the
  authority.

## Work Added

- Added scheme-aware daemon relay endpoint parsing for `ws://` and `wss://`.
- Kept public `ws://` blocked to localhost-only evidence.
- Added a small `RelayStream` abstraction so registration POST and WebSocket
  frames can use either plain TCP or TLS.
- Added `remote,tls` WSS support through the existing `tokio-rustls` and
  `webpki-roots` dependencies.
- Added a focused Rust parser/guardrail test.
- Updated hosted-readiness and deployment-runbook checks to track WSS runtime as
  ready in `remote,tls` builds.

## Remaining Hosted Blockers

- Hosted failure-mode evidence matching or exceeding local bridge smoke.

The production relay service artifact and Ed25519 public-key verifier support
were closed by later slices. The payload trust decision was also closed by a
later slice for explicit self-hosted relay use, followed by observability and
retention evidence.

## Verification

```powershell
npm run check:pwa-relay-hosted-readiness
npm run check:pwa-relay-deployment-runbook
npm run smoke:pwa-relay-websocket-bridge
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote relay_daemon_runtime'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote,tls relay_daemon_runtime'
git diff --check
```
