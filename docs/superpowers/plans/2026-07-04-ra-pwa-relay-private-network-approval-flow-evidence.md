# 2026-07-04 RA/PWA Relay Private-Network Approval Flow Evidence

## Purpose

Capture browser evidence that private-network Relay/M2 approval requests and
approve/reject responses work over the setup-derived private relay WebSocket
loop.

## Status

Completed in this slice. The follow-up runbook closeout and managed operations
planning slices are also complete; the next local implementation slice is
managed relay verifier-key operations policy.

## Scope

- Drive a private-network approval request from a daemon-side WebSocket client.
- Verify the PWA receives the request as `Private Relay`.
- Send approve and reject responses through the private relay socket.
- Verify the daemon-side WebSocket receives both approval responses.
- Keep self-hosted relay approval behavior unchanged.
- Keep `live-loopback` as the product default and managed relay deferred.

## Non-Goals

- Do not make private-network relay the product default.
- Do not change managed relay operations.
- Do not replace the self-hosted approval evidence.

## Work Added

- Added `npm run smoke:pwa-relay-private-network-approval-flow-evidence`.
- Started the existing relay service artifact as a private-network endpoint.
- Connected the PWA as private-network companion and a Node WebSocket as daemon.
- Drove approve and reject requests through the private relay socket and
  verified both daemon-side responses.
- Captured desktop/mobile browser evidence.

## Follow-Up

Private-network relay runbook closeout is complete:

- update operator/runbook docs with the private-network evidence map;
- list setup, connection, and approval-flow smoke commands together;
- keep self-hosted readiness and `live-loopback` default explicit;
- keep managed relay deferred.

Managed relay abuse retention policy and payload confidentiality plan are
complete. The next slice is managed relay verifier-key operations policy.

## Verification

```powershell
npm run smoke:pwa-relay-private-network-approval-flow-evidence
npm run smoke:pwa-relay-private-network-connection-controls
npm run smoke:pwa-relay-setup-ui
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
