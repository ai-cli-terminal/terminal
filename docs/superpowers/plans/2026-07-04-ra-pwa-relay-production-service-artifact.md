# 2026-07-04 RA/PWA Relay Production Service Artifact

## Purpose

Close the next local Relay/M2 production-readiness slice by adding a
production-oriented self-hosted relay service artifact, deploy recipe, and local
smoke evidence.

## Status

Completed in this slice. After the follow-up public-key ticket verification,
payload trust decision, and observability retention slices, hosted production
relay is green for the explicit self-hosted setup/debug path after the
failure-mode evidence slice.

## Scope

- Add an executable self-hosted relay service entrypoint.
- Keep the service transport-only: it registers tickets, authenticates peers,
  routes frames, and exposes health counters.
- Add a deploy recipe for running the service behind a TLS/WSS reverse proxy.
- Add a local smoke that proves signed ticket registration, daemon/companion
  WebSocket authentication, bidirectional frame routing, and no payload/secret
  exposure in route acknowledgements or health evidence.
- Update readiness docs and gates; the follow-up public-key ticket slice moves
  the next local blocker to payload confidentiality or relay-operator trust.

## Non-Goals

- Do not make relay the product default.
- Do not expose public `ws://`.
- Do not add payload encryption.
- Do not claim hosted observability or hosted failure-mode evidence complete.

## Work Added

- Added `scripts/relay-self-hosted-service.mjs`.
- Added `npm run relay:self-hosted`.
- Added `docs/relay-self-hosted-deploy.md`.
- Added `scripts/smoke-pwa-relay-service-artifact.mjs`.
- Added `npm run smoke:pwa-relay-service-artifact`.
- Updated deployment runbook, hosted-readiness check, HISTORY, HANDOFF, and
  remaining-work priority.
- Follow-up public-key ticket verification added Ed25519 public verifier support
  while keeping HMAC as a legacy/local compatibility path.
- Follow-up payload trust decision records the current self-hosted operator
  trust boundary instead of claiming end-to-end payload confidentiality.
- Follow-up observability retention evidence added aggregate-only health and
  memory-only retention policy.
- Follow-up failure-mode evidence closed explicit self-hosted relay readiness.

## Verification

```powershell
npm run smoke:pwa-relay-service-artifact
npm run check:pwa-relay-hosted-readiness
npm run check:pwa-relay-deployment-runbook
npm run smoke:pwa-relay-websocket-bridge
npm run test:pwa
git diff --check
```
