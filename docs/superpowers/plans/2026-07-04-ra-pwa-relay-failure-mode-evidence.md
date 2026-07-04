# 2026-07-04 RA/PWA Relay Failure-Mode Evidence

## Purpose

Close the final self-hosted Relay/M2 hosted-readiness blocker by recording
repeatable failure-mode evidence for the relay service artifact.

## Status

Completed in this slice. The explicit self-hosted relay readiness gate is green
without changing the product default from `live-loopback`.

## Scope

- Extend `npm run smoke:pwa-relay-service-artifact` to cover fail-closed ticket,
  connect, frame, and expiry behavior.
- Keep `npm run smoke:pwa-relay-websocket-bridge` as the local bridge/session
  rotation and reconnect evidence source.
- Update hosted-readiness and deployment-runbook gates so no local self-hosted
  Relay/M2 hosted-readiness blockers remain.

## Evidence Covered

- Unsigned ticket rejection.
- Bad-MAC ticket rejection.
- Expired ticket registration rejection.
- Missing ticket connect rejection.
- Bad session token connect rejection.
- Wrong role connect rejection.
- Wrong sender frame rejection.
- Duplicate sequence rejection.
- Expired frame drop.
- Existing local bridge reconnect/session isolation evidence through
  `npm run smoke:pwa-relay-websocket-bridge`.

## Guardrails

- `live-loopback` remains the product default.
- Relay remains explicit setup/debug path.
- Public/staging relay endpoints still require `wss://`.
- Managed or untrusted relay infrastructure still requires separate planning
  and evidence.

## Verification

```powershell
npm run smoke:pwa-relay-service-artifact
npm run smoke:pwa-relay-websocket-bridge
npm run check:pwa-relay-hosted-readiness
npm run check:pwa-relay-deployment-runbook
npm run test:pwa
git diff --check
```
