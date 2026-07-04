# 2026-07-04 RA/PWA Relay Observability Retention Evidence

## Purpose

Close the hosted observability and retention policy blocker for the current
self-hosted Relay/M2 service artifact.

## Status

Completed in this slice. Hosted relay is still blocked by failure-mode evidence
matching or exceeding local bridge smoke.

## Scope

- Add production-safe aggregate observability metadata to relay service health.
- Record a retention policy for sessions, tickets, queued frames, payloads,
  setup JSON, approval signatures, and private key material.
- Keep observability free of `payload_json`, session tokens, setup JSON, HMAC
  secrets, approval signatures, command text, and private signing material.
- Update service smoke and hosted-readiness gates so the next local blocker is
  hosted failure-mode evidence.

## Non-Goals

- Do not add external metrics backend integration in this slice.
- Do not log payloads or per-session sensitive values.
- Do not make relay the product default.
- Do not close hosted failure-mode evidence.

## Work Added

- Added aggregate-only `observability` metadata to `GET /health` in
  `scripts/relay-self-hosted-service.mjs`.
- Updated `npm run smoke:pwa-relay-service-artifact` to assert retention policy
  fields and error classes without payload or private key leakage.
- Updated runbook, deploy recipe, hosted-readiness gate, handoff, history, and
  remaining-work priority.

## Remaining Blocker

- Hosted failure-mode evidence matching or exceeding local bridge smoke.

## Verification

```powershell
npm run smoke:pwa-relay-service-artifact
npm run check:pwa-relay-hosted-readiness
npm run check:pwa-relay-deployment-runbook
npm run test:pwa
git diff --check
```
