# 2026-07-04 RA/PWA Relay Public-Key Ticket Signing

## Purpose

Close the next local Relay/M2 production-readiness slice by removing the hosted
relay service requirement for shared daemon HMAC secret material.

## Status

Completed in this slice. Hosted relay is still blocked by payload
confidentiality or an explicit relay-operator trust decision, hosted
observability, and hosted failure-mode evidence.

## Scope

- Add Ed25519 public-key verifier support to the self-hosted relay service
  artifact.
- Keep existing HMAC verifier support as a legacy/local compatibility path.
- Add service smoke coverage that registers and routes with an Ed25519 signed
  relay session ticket while the service only receives public verifier key
  material.
- Update deploy recipe and hosted-readiness gates so the next local blocker
  becomes payload confidentiality or explicit relay-operator trust decision.

## Non-Goals

- Do not make relay the product default.
- Do not remove HMAC compatibility in this slice.
- Do not solve payload confidentiality.
- Do not claim hosted observability or hosted failure-mode evidence complete.

## Guardrails

- Public hosted relay still requires `wss://`.
- Relay service must not receive private signing keys.
- Health evidence must not expose payloads, session tokens, setup JSON, HMAC
  secrets, private keys, or full public verifier records.

## Work Added

- Added Ed25519 public-key verifier support to
  `scripts/relay-self-hosted-service.mjs`.
- Kept HMAC verifier support as a legacy/local compatibility path.
- Updated `npm run smoke:pwa-relay-service-artifact` to register an Ed25519
  signed ticket against a service configured only with public verifier key
  material.
- Updated PWA signed-ticket metadata validation to accept Ed25519 signature
  wrappers.
- Updated deploy/runbook/readiness docs and gates so the next local blocker is
  payload confidentiality or an explicit relay-operator trust decision.

## Remaining Blockers

- Payload confidentiality or explicit relay-operator trust decision.
- Hosted observability and retention policy evidence.
- Hosted failure-mode evidence matching or exceeding local bridge smoke.

## Verification

```powershell
npm run smoke:pwa-relay-service-artifact
npm run check:pwa-relay-hosted-readiness
npm run check:pwa-relay-deployment-runbook
npm run smoke:pwa-relay-websocket-bridge
npm run test:pwa
git diff --check
```
