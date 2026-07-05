# 2026-07-05 RA/PWA Relay Managed Runtime Operator Setup Approval Response Delivery Boundary

## Purpose

Close the managed operator setup approval response delivery boundary before any
managed endpoint delivery path is enabled.

## Status

Completed in this slice. Managed approval responses remain
`manual-signed-response-copy-only`; the response and verify command stay in the
existing approval panel; the managed setup surface does not show response
payloads; and managed endpoint delivery remains blocked until a separate
endpoint-delivery evidence slice verifies it.

## Scope

- Add `managedRelayRuntimeOperatorSetupApprovalResponseDeliveryBoundary()`.
- Add `relayManagedRuntimeOperatorSetupApprovalResponseDeliveryBoundary()`.
- Require a ready managed approval flow and a valid signed approval response
  before the delivery boundary is ready.
- Keep copy and verify controls visible only through the existing approval
  panel.
- Update next-mode planning so the next local slice is managed approval
  response endpoint delivery evidence.
- Update the relay runbook and handoff/history docs.

## Non-Goals

- Do not deliver managed approval responses over WebSocket or any managed
  endpoint.
- Do not start a managed endpoint or enable public bind.
- Do not render capability envelope JSON, signed session tickets, raw session
  tokens, payload material, private key material, operator setup text, support
  contact metadata, or raw identifiers.
- Do not change `live-loopback` as the product default.

## Work Added

- Added a manual-copy-only approval response delivery boundary helper in
  `pwa/app.mjs`.
- Added summary evidence for selectors, guardrails, completed implementation
  evidence, and next slice selection.
- Added unit coverage for valid signed response delivery, blocked delivery when
  approval flow is not ready, copy/verify controls, hidden managed setup
  response, and no network/endpoint/public-bind side effects.
- Updated next-mode planning evidence to include the delivery boundary and move
  the follow-up to endpoint delivery evidence.
- Updated the relay runbook evidence map.

## Boundary Contract

The delivery boundary is ready only when:

- managed approval flow is ready;
- the signed approval response validates;
- response `approval_id` and `nonce` match the approval request;
- delivery remains `manual-signed-response-copy-only`;
- copy and verify controls remain in the existing approval panel;
- the managed setup surface does not show the approval response;
- no WebSocket, endpoint start, public bind, ticket/token/payload/key material,
  or capability envelope is exposed.

## Next Slice

Managed relay runtime operator setup approval response endpoint delivery
evidence:

- verify an explicit managed endpoint delivery path before network delivery is
  allowed;
- keep `live-loopback` as the product default;
- keep managed relay explicit opt-in with endpoint auto-start disabled and
  public bind off;
- preserve the manual signed-response copy fallback until endpoint delivery is
  proven.

## Verification

```powershell
npm run test:pwa
npm run check:pwa-relay-next-mode-planning
npm run check:pwa-relay-deployment-runbook
npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-flow-evidence
git diff --check
```
