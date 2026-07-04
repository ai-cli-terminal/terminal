# 2026-07-04 RA/PWA Relay Hosted WSS Readiness Gate

## Purpose

Start the Relay hosted/WSS production-readiness work by adding a repeatable
status gate that records what is ready, what is still blocked, and the next
local implementation slice.

## Status

Completed in the readiness-gate slice, then updated after daemon WSS runtime
support, the production relay service artifact, and Ed25519 public-key ticket
verification landed. It was updated again after the explicit relay-operator
trust decision and observability retention evidence. Hosted relay is still
blocked, but daemon `wss://` runtime, the relay service artifact, public
verifier key support, the self-hosted payload trust decision, and aggregate
observability/retention evidence are ready.

## Scope

- Verify the deployment decision still selects self-hosted WebSocket relay while
  keeping `live-loopback` as product default.
- Verify the PWA setup/UX path accepts a `wss://` self-hosted relay endpoint.
- Verify the daemon relay runtime has `wss://` support in `remote,tls` builds
  while non-`tls` builds fail closed.
- Verify the production relay service artifact, smoke, and deploy recipe exist.
- Verify the relay service supports Ed25519 public verifier keys for signed
  session tickets.
- Verify the current self-hosted payload visibility is covered by an explicit
  relay-operator trust decision.
- Verify aggregate-only observability and retention policy evidence is present.
- Verify the deployment runbook still lists hosted production blockers.
- Emit JSON evidence under `artifacts/ra-pwa-relay-hosted-readiness/`.

## Non-Goals

- Do not add payload encryption or mark relay-operator trust as resolved.
- Do not make relay the product default or broadly user-selectable.

## Work Added

- Added `scripts/check-pwa-relay-hosted-readiness.mjs`.
- Added `npm run check:pwa-relay-hosted-readiness`.
- Updated the self-hosted relay runbook, HISTORY, HANDOFF, and remaining-work
  priority so the next local task is payload confidentiality or explicit
  relay-operator trust decision.
- Updated the gate after the public-key ticket slice so verifier key
  distribution is ready with Ed25519 public verifier keys.
- Updated the gate after the payload trust decision slice so payload
  confidentiality is ready through an explicit relay-operator trust decision.
- Updated the gate after the observability retention slice so hosted
  observability is ready with aggregate health and retention policy evidence.

## Findings

- PWA Relay setup can accept `wss://` self-hosted setup metadata.
- Daemon relay runtime supports hosted `wss://` endpoints in `remote,tls`
  builds and keeps public `ws://` blocked.
- The repository has a production-oriented service artifact and deploy recipe.
- The relay service can verify Ed25519 signed tickets using only public verifier
  key material.
- The current self-hosted relay shape has an explicit relay-operator trust
  decision instead of claiming end-to-end payload confidentiality.
- The relay service exposes aggregate-only health observability and retention
  policy evidence.
- Hosted production remains blocked by hosted failure-mode evidence.

## Verification

```powershell
npm run check:pwa-relay-hosted-readiness
npm run check:pwa-relay-deployment-runbook
npm run check:pwa-relay-deployment-decision
npm run test:pwa
git diff --check
```
