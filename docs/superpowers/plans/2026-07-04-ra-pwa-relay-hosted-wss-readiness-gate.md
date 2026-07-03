# 2026-07-04 RA/PWA Relay Hosted WSS Readiness Gate

## Purpose

Start the Relay hosted/WSS production-readiness work by adding a repeatable
status gate that records what is ready, what is still blocked, and the next
local implementation slice.

## Status

Completed in the readiness-gate slice, then updated after daemon WSS runtime
support and the production relay service artifact landed. Hosted relay is still
blocked, but daemon `wss://` runtime and the relay service artifact are ready.

## Scope

- Verify the deployment decision still selects self-hosted WebSocket relay while
  keeping `live-loopback` as product default.
- Verify the PWA setup/UX path accepts a `wss://` self-hosted relay endpoint.
- Verify the daemon relay runtime has `wss://` support in `remote,tls` builds
  while non-`tls` builds fail closed.
- Verify the production relay service artifact, smoke, and deploy recipe exist.
- Verify the deployment runbook still lists hosted production blockers.
- Emit JSON evidence under `artifacts/ra-pwa-relay-hosted-readiness/`.

## Non-Goals

- Do not implement daemon `wss://` runtime support in this slice.
- Do not add a production relay service artifact.
- Do not define verifier-key distribution or replace HMAC tickets with
  public-key signing.
- Do not add payload encryption or mark relay-operator trust as resolved.
- Do not make relay the product default or broadly user-selectable.

## Work Added

- Added `scripts/check-pwa-relay-hosted-readiness.mjs`.
- Added `npm run check:pwa-relay-hosted-readiness`.
- Updated the self-hosted relay runbook, HISTORY, HANDOFF, and remaining-work
  priority so the next local task is verifier-key distribution or public-key
  ticket signing.

## Findings

- PWA Relay setup can accept `wss://` self-hosted setup metadata.
- Daemon relay runtime supports hosted `wss://` endpoints in `remote,tls`
  builds and keeps public `ws://` blocked.
- The repository has a production-oriented service artifact and deploy recipe.
- Hosted production remains blocked by verifier-key distribution or public-key
  ticket signing, payload confidentiality or explicit trust decision, hosted
  observability, and hosted failure-mode evidence.

## Verification

```powershell
npm run check:pwa-relay-hosted-readiness
npm run check:pwa-relay-deployment-runbook
npm run check:pwa-relay-deployment-decision
npm run test:pwa
git diff --check
```
