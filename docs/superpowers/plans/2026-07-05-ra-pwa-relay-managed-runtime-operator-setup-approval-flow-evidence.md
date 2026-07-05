# 2026-07-05 RA/PWA Relay Managed Runtime Operator Setup Approval Flow Evidence

## Purpose

Add browser evidence that the Managed Relay/M2 operator setup approval flow can
use the session capability boundary after the managed setup handshake.

## Status

Completed in this slice as manual-only approval flow evidence. The PWA requires
a ready managed setup import, manual connect request, and session handshake
before it can load a managed approval request into the existing Approve panel.
The managed setup surface continues to show only sanitized setup metadata, the
capability handle, transcript hash, approval source, and approval context hash.

## Scope

- Add `managedRelayRuntimeOperatorSetupApprovalRequest()`.
- Add `managedRelayRuntimeOperatorSetupApprovalFlowEvidenceFromHandshake()`.
- Add `managedRelayRuntimeOperatorSetupApprovalFlowEvidence()`.
- Add `relayManagedRuntimeOperatorSetupApprovalFlowEvidence()`.
- Add PWA Managed Relay approval evidence state and load control.
- Reuse the existing Approve panel for approve/reject signing.
- Capture desktop/mobile browser evidence for the managed approval flow.
- Move the next local slice to managed operator setup runbook closeout.

## Non-Goals

- Do not connect to a managed relay endpoint.
- Do not create a WebSocket during approval evidence.
- Do not deliver approval responses over the network.
- Do not start an endpoint or enable public bind.
- Do not render capability envelope JSON, signed session tickets, raw session
  tokens, payload material, private key material, operator setup text, support
  contact metadata, or raw identifiers in the managed setup surface.
- Do not change `live-loopback` as the product default.

## Work Added

- Added the managed approval flow evidence contract in `pwa/app.mjs`.
- Added a Managed Relay `Load managed approval` control that is enabled only
  after a ready session handshake.
- Added managed approval state/source/context fields.
- Loaded a capability-bound approval request into the existing approval panel.
- Added unit coverage for blocked and ready approval-flow states.
- Added `npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-flow-evidence`.
- Updated next-mode planning so the next local slice is
  `managed-relay-runtime-operator-setup-runbook-closeout`.

## Approval Flow Boundary

The approval flow is intentionally manual-only at this stage:

- before handshake: managed approval loading is disabled;
- after handshake: managed approval loading is enabled;
- after loading: the PWA shows `Approval request ready`, source
  `Managed Relay`, and a `sha256:*` approval context hash;
- the existing Approve panel shows the masked command and context hash;
- approve and reject responses are signed by the existing approval key path;
- the managed setup panel does not render approval payloads or responses;
- no WebSocket is created;
- no endpoint is started;
- public bind remains off.

## Next Slice

Managed relay runtime operator setup runbook closeout:

- add a managed operator setup evidence map to the relay runbook;
- link setup contract, import preflight, browser evidence, connection controls,
  session handshake, and approval-flow evidence commands;
- keep `live-loopback` rollback, explicit opt-in, disabled endpoint auto-start,
  and public bind off;
- keep managed approval response delivery separate from network delivery until
  a later explicit delivery-boundary slice.

## Verification

```powershell
npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-flow-evidence
npm run smoke:pwa-relay-managed-runtime-operator-setup-session-handshake
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
