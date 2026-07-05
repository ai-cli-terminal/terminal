# 2026-07-04 RA/PWA Relay Self-Hosted Deployment Runbook

## Purpose

Close the next local Relay/M2 documentation gap by writing an operator runbook
for self-hosted WebSocket relay deployment, readiness checks, observability,
failure-mode evidence, and rollback.

## Status

Completed in this slice. The runbook documents what is ready for local/staging
evidence, what remains blocked for hosted production, and which guardrails must
stay in place before relay can become user-selectable.

## Scope

- Keep `live-loopback` as the product default.
- Document the compatible self-hosted relay service contract.
- Document local staging and manual staging procedures.
- Document ticket registration, connect authentication, reconnect/failure
  evidence, observability, rollback, and completion criteria.
- Call out production blockers: daemon WSS runtime support, production relay
  artifact, verifier-key distribution, and payload confidentiality or a recorded
  trust decision.
- Add a repeatable runbook contract check.

## Non-Goals

- Do not implement a production relay service.
- Do not add daemon `wss://` runtime support in this slice.
- Do not change relay from explicit setup/debug path to product default.
- Do not close payload confidentiality as solved.
- Do not change release tags or public assets.

## Work Added

- Added `docs/relay-self-hosted-runbook.md`.
- Added `scripts/check-pwa-relay-deployment-runbook.mjs`.
- Added `npm run check:pwa-relay-deployment-runbook`.
- Updated remaining-work priority, HISTORY, and HANDOFF to move the next local
  Relay/M2 blocker to hosted/WSS relay production readiness.

## Guardrails

- Public hosted relay must be `wss://`; current daemon runtime supports only
  localhost `ws://` evidence and fails closed for `wss://`.
- A relay process must route frames only and must not validate approval
  signatures or decide command safety.
- Relay logs and metrics must not include `payload_json`, session tokens, HMAC
  secrets, setup JSON, approval signatures, or private key material.
- Product default remains `live-loopback`.

## Verification

```powershell
npm run check:pwa-relay-deployment-runbook
npm run check:pwa-relay-deployment-decision
npm run smoke:pwa-relay-websocket-bridge
git diff --check
```
