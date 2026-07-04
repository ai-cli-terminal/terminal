# 2026-07-04 RA/PWA Relay Payload Trust Decision

## Purpose

Close the next local Relay/M2 production-readiness blocker by recording an
explicit relay-operator trust decision for the current self-hosted relay shape.

## Status

Completed in this slice. Payload confidentiality is not implemented; instead,
the current self-hosted relay mode is bounded to operators who explicitly trust
the relay process as transport infrastructure. After the follow-up
observability retention slice, hosted relay is still blocked by failure-mode
evidence.

## Decision

The current Relay/M2 self-hosted mode does not provide end-to-end payload
confidentiality from the relay operator. Relay frames contain `payload_json`,
so the relay process can observe approval transport payloads while forwarding
them.

This is acceptable only for the explicit self-hosted setup/debug path where the
operator controls and trusts the relay service. It is not a blanket decision for
managed relay, untrusted relay infrastructure, or changing the product default.

## Bounds

- `live-loopback` remains the product default.
- Relay remains explicit setup/debug path until hosted failure-mode evidence is
  green.
- Public/staging relay endpoints still require `wss://`.
- Relay service logs, health, metrics, and evidence must not include
  `payload_json`, session tokens, setup JSON, HMAC secrets, approval
  signatures, command text beyond already masked fields, or private key
  material.
- The relay remains transport-only and must not validate approvals or decide
  command safety.
- Payload encryption remains the required path for future untrusted or managed
  relay infrastructure.

## Work Added

- Added a Relay Operator Trust Decision section to the self-hosted relay
  runbook.
- Updated hosted-readiness and deployment-runbook gates so
  payloadConfidentiality is ready through an explicit relay-operator trust
  decision.
- Updated handoff, history, and remaining-work priority. A follow-up slice
  closed hosted observability and retention policy evidence.

## Remaining Blockers

- Hosted failure-mode evidence matching or exceeding local bridge smoke.

## Verification

```powershell
npm run check:pwa-relay-hosted-readiness
npm run check:pwa-relay-deployment-runbook
npm run check:pwa-relay-deployment-decision
git diff --check
```
