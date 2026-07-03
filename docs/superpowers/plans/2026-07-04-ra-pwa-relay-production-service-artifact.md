# 2026-07-04 RA/PWA Relay Production Service Artifact

## Purpose

Close the next local Relay/M2 production-readiness slice by adding a
production-oriented self-hosted relay service artifact, deploy recipe, and local
smoke evidence.

## Status

Completed in this slice. Hosted production relay is still blocked by key
distribution/signing, payload confidentiality or explicit trust decision,
hosted observability, and hosted failure-mode evidence.

## Scope

- Add an executable self-hosted relay service entrypoint.
- Keep the service transport-only: it registers tickets, authenticates peers,
  routes frames, and exposes health counters.
- Add a deploy recipe for running the service behind a TLS/WSS reverse proxy.
- Add a local smoke that proves signed ticket registration, daemon/companion
  WebSocket authentication, bidirectional frame routing, and no payload/secret
  exposure in route acknowledgements or health evidence.
- Update readiness docs and gates so the next local blocker moves to verifier
  key distribution or public-key ticket signing.

## Non-Goals

- Do not make relay the product default.
- Do not expose public `ws://`.
- Do not solve verifier-key distribution as production-ready.
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

## Verification

```powershell
npm run smoke:pwa-relay-service-artifact
npm run check:pwa-relay-hosted-readiness
npm run check:pwa-relay-deployment-runbook
npm run smoke:pwa-relay-websocket-bridge
npm run test:pwa
git diff --check
```
