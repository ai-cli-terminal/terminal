# 2026-07-04 RA/PWA Relay Private-Network Runbook Closeout

## Purpose

Close the private-network Relay/M2 local evidence chain by adding the setup,
import, connection, and approval-flow evidence map to the relay operator
runbook.

## Status

Completed in this slice. Private-network relay remains an explicit advanced
setup path; managed relay remains deferred to separate operations planning.

## Scope

- Add a private-network evidence map to `docs/relay-self-hosted-runbook.md`.
- Link the private-network setup, visible import, connection, and approval-flow
  smoke commands together.
- Keep explicit self-hosted relay readiness and `live-loopback` product default
  language intact.
- Update runbook checks so the private-network evidence map stays present.

## Non-Goals

- Do not make private-network relay the product default.
- Do not start managed relay implementation.
- Do not replace self-hosted relay readiness evidence.

## Work Added

- Added runbook guidance for private-network advanced setup evidence.
- Updated `npm run check:pwa-relay-deployment-runbook`.
- Updated handoff/history/remaining-work pointers.

## Follow-Up

Managed relay operations planning is complete:

- define control-plane ownership and tenant isolation requirements;
- define abuse handling, support, and retention operations;
- keep `live-loopback` as product default until those requirements are green.

Managed relay control-plane contract and abuse retention policy are also
complete, and the payload confidentiality plan is complete too. The next slice
is managed relay verifier-key operations policy.

## Verification

```powershell
npm run check:pwa-relay-deployment-runbook
npm run check:pwa-relay-next-mode-planning
npm run smoke:pwa-relay-private-network-approval-flow-evidence
npm run test:pwa
git diff --check
```
