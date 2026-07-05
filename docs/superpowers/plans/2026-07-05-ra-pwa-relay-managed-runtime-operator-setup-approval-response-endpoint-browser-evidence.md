# 2026-07-05 RA/PWA Relay Managed Runtime Operator Setup Approval Response Endpoint Browser Evidence

## Purpose

Capture browser/operator evidence that the managed operator setup approval
response endpoint path is visible only as an explicit operator action, while
manual signed-response copy remains available and managed relay secrets stay
out of the PWA surface.

## Status

Completed in this slice. The PWA enables endpoint delivery only after a managed
approval request is loaded and a signed approval response exists. The browser
surface shows endpoint delivery state, encrypted route status, daemon receipt
status, and manual copy fallback. It does not render route envelopes, payload
keys, ciphertext, approval response payload material, or private key material.

## Scope

- Add `relayManagedRuntimeOperatorSetupApprovalResponseEndpointBrowserEvidence()`.
- Add browser-visible endpoint delivery controls and status fields to the
  Managed Relay setup panel.
- Add `npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-browser-evidence`.
- Capture desktop pending/delivered screenshots plus mobile delivered evidence.
- Keep manual copy/verify controls in the existing approval panel.
- Keep endpoint auto-start disabled, public bind disabled, and `live-loopback`
  as product default.
- Update next-mode planning so the next local slice is daemon bridge evidence
  for the managed endpoint response path.

## Non-Goals

- Do not make managed relay the product default.
- Do not auto-start a managed endpoint.
- Do not enable public bind.
- Do not render route envelopes, payload JSON, payload keys, ciphertext, shared
  secrets, approval response payload material, private key material, operator
  setup text, support contact metadata, or raw identifiers.
- Do not remove the manual signed-response copy fallback.
- Do not replace daemon-side approval verification in this slice.

## Work Added

- Added a browser evidence summary contract for endpoint delivery UI proof.
- Added `Deliver endpoint response` to the managed approval flow, enabled only
  after a signed managed approval response exists.
- Added browser-visible endpoint delivery state, route status, copy fallback,
  and daemon receipt fields.
- Reused the endpoint delivery evidence helper from the browser action while
  keeping the client-held payload key in memory only.
- Added unit coverage for the browser evidence contract.
- Added a Playwright smoke that captures desktop and mobile evidence and checks
  prohibited visible tokens.
- Updated the relay runbook, next-mode planning, handoff, and history.

## Browser Boundary

The browser evidence path is ready only when:

- managed setup import is ready;
- manual managed connect was requested;
- session handshake is ready;
- managed approval request is loaded into the existing approval panel;
- a signed approval response exists;
- the operator explicitly clicks the endpoint delivery control;
- the browser shows `Endpoint delivery ready`;
- the browser shows `encrypted frame routed` and `response received`;
- manual signed-response copy and verify command remain visible;
- route envelopes, payload keys, ciphertext, and private key material are not
  visible in the managed setup surface.

## Next Slice

Managed relay runtime operator setup approval response daemon bridge evidence:

- consume the managed endpoint response through the existing daemon approval
  verification boundary;
- keep managed route envelopes, payload keys, ciphertext, raw tokens, and
  private key material out of logs and user-visible output;
- preserve manual signed-response copy as fallback;
- keep endpoint auto-start disabled and public bind off.

## Verification

```powershell
npm run test:pwa
npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-browser-evidence
npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence
npm run check:pwa-relay-next-mode-planning
npm run check:pwa-relay-deployment-runbook
git diff --check
```
