# 2026-07-05 RA/PWA Relay Managed Runtime Operator Setup Approval Response Daemon Bridge Evidence

## Purpose

Prove the managed operator setup approval response that was delivered through
the explicit endpoint path can be consumed through the existing daemon approval
verification boundary without exposing managed relay route, key, ciphertext, or
token material.

## Status

Completed in this slice. Daemon bridge evidence is ready only after endpoint
delivery evidence reports daemon receipt, the delivered response matches the
approval request, the approval response signature verifies with the registered
approval public key boundary, and the current context hash matches the approval
request context hash.

## Scope

- Add `managedRelayRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence()`.
- Add `relayManagedRuntimeOperatorSetupApprovalResponseDaemonBridgeEvidence()`.
- Add `npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-daemon-bridge-evidence`.
- Prove endpoint-delivered approval responses can enter the existing daemon
  approval validation boundary.
- Keep manual signed-response copy as fallback.
- Keep endpoint auto-start disabled, public bind disabled, and `live-loopback`
  as product default.
- Update next-mode planning so the next local slice is managed operator setup
  production closeout.

## Non-Goals

- Do not make managed relay the product default.
- Do not auto-start a managed endpoint.
- Do not enable public bind.
- Do not replace Rust approval validation.
- Do not log or render route envelopes, payload keys, ciphertext, shared
  secrets, raw session tokens, approval response payload material, or private
  key material in daemon bridge evidence.

## Work Added

- Added an async daemon bridge evidence helper that requires ready delivery
  boundary evidence, ready endpoint delivery evidence, daemon receipt, response
  request match, signature verification, and context hash continuity.
- Added a summary evidence contract that names the existing daemon boundaries:
  `decide_with_remote_relay_bridge`, `finish_remote_gate_response`,
  `approval::validate`, and `ai remote approval-verify`.
- Added unit coverage for ready, missing approval key, and context mismatch
  states.
- Added an npm check script that writes sanitized evidence under `artifacts/`.
- Updated the relay runbook, next-mode planning, handoff, and history.

## Boundary Contract

The daemon bridge path is ready only when:

- endpoint browser evidence is complete;
- endpoint delivery evidence is ready;
- daemon receipt of an `approval_response` is present;
- the delivered response matches the original approval request id and nonce;
- the signed approval response verifies through the approval public key
  boundary;
- the current context hash matches the approval request context hash;
- manual signed-response copy remains available as fallback;
- daemon bridge evidence excludes route envelopes, payload keys, ciphertext,
  raw tokens, approval response payload material, and private key material.

## Next Slice

Managed relay runtime operator setup production closeout:

- link the operator setup contract, import preflight, browser evidence,
  connection controls, session handshake, approval flow, runbook closeout,
  delivery boundary, endpoint delivery, endpoint browser, and daemon bridge
  evidence;
- keep managed relay explicit opt-in only;
- keep endpoint auto-start disabled and public bind off;
- keep `live-loopback` as product default.

## Verification

```powershell
npm run test:pwa
npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-daemon-bridge-evidence
npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence
npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-browser-evidence
npm run check:pwa-relay-next-mode-planning
npm run check:pwa-relay-deployment-runbook
git diff --check
```
