# 2026-07-05 RA/PWA Relay Managed Runtime Operator Setup Approval Response Endpoint Delivery Evidence

## Purpose

Prove the managed operator setup approval response can leave the PWA through an
explicit operator-started managed endpoint path without changing product
defaults or exposing managed relay secrets.

## Status

Completed in this slice. Endpoint delivery is ready only when the approval
response delivery boundary is ready, the operator-started endpoint is present,
manual managed connect was requested, and the client-held session payload key is
available. The delivered response is wrapped as a managed encrypted frame, the
relay-visible route contains only envelope metadata, and manual signed-response
copy remains available as fallback.

## Scope

- Add `managedRelayRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence()`.
- Add `relayManagedRuntimeOperatorSetupApprovalResponseEndpointDeliveryEvidence()`.
- Add `npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence`.
- Prove a signed approval response can be encrypted into a managed frame and
  delivered back to the daemon boundary.
- Keep endpoint auto-start disabled, public bind disabled, and `live-loopback`
  as product default.
- Update next-mode planning so the next local slice is browser evidence for the
  endpoint delivery path.

## Non-Goals

- Do not make managed relay the product default.
- Do not auto-start a managed endpoint.
- Do not enable public bind.
- Do not render capability envelope JSON, signed session tickets, raw session
  tokens, payload material, payload keys, ciphertext, private key material,
  operator setup text, support contact metadata, or raw identifiers.
- Do not remove the manual signed-response copy fallback.

## Work Added

- Added an async endpoint delivery evidence helper that requires a ready
  delivery boundary, explicit operator endpoint readiness, manual connect, a
  valid session id, and a client-held payload key.
- Added summary evidence for explicit endpoint delivery, encrypted-frame routing,
  manual copy fallback, and no auto-start/public-bind behavior.
- Added unit coverage for endpoint delivery ready/blocked states and route
  visibility.
- Added an npm check script that writes sanitized evidence under `artifacts/`.
- Updated the relay runbook, next-mode planning, handoff, and history.

## Boundary Contract

The endpoint delivery path is ready only when:

- the approval response delivery boundary is ready;
- the operator-started endpoint is ready;
- manual managed connect was requested;
- endpoint auto-start is false;
- public bind is false;
- the response is wrapped in a managed encrypted frame;
- route-visible evidence excludes payload JSON, ciphertext, payload keys,
  shared secrets, approval response payloads, and private key material;
- the daemon boundary receives an `approval_response` message;
- manual signed-response copy remains available.

## Next Slice

Managed relay runtime operator setup approval response endpoint browser
evidence:

- show the endpoint delivery path from browser/operator evidence;
- keep copy/verify fallback visible in the existing approval panel;
- avoid rendering payload keys, ciphertext, capability envelopes, raw tokens,
  payload material, private key material, or raw identifiers;
- keep endpoint auto-start disabled and public bind off.

## Verification

```powershell
npm run test:pwa
npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence
npm run check:pwa-relay-next-mode-planning
npm run check:pwa-relay-deployment-runbook
git diff --check
```
