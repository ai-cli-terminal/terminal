# 2026-07-05 RA/PWA Relay Managed Runtime Operator Setup Runbook Closeout

## Purpose

Close the Managed Relay/M2 operator setup local evidence chain by adding the
managed setup, import, browser, connection, handshake, and approval-flow
evidence map to the relay runbook.

## Status

Completed in this slice. Managed relay remains an explicit opt-in setup path;
the product default remains `live-loopback`; endpoint auto-start remains
disabled; public bind remains off; and managed approval response delivery stays
manual-only until a separate delivery-boundary slice verifies network delivery.

## Scope

- Add `relayManagedRuntimeOperatorSetupRunbookCloseout()`.
- Add a Managed Relay Operator Setup Evidence Map to
  `docs/relay-self-hosted-runbook.md`.
- Link the managed operator setup contract, import preflight, browser evidence,
  connection controls, session handshake, and approval-flow evidence commands.
- Update `npm run check:pwa-relay-deployment-runbook` so the managed evidence
  map remains present.
- Update next-mode planning so the next local slice is managed approval response
  delivery boundary.

## Non-Goals

- Do not connect to a managed relay endpoint.
- Do not deliver managed approval responses over the network.
- Do not start an endpoint or enable public bind.
- Do not render capability envelope JSON, signed session tickets, raw session
  tokens, payload material, private key material, operator setup text, support
  contact metadata, or raw identifiers.
- Do not change `live-loopback` as the product default.

## Work Added

- Added a runbook closeout summary helper in `pwa/app.mjs`.
- Added unit coverage for the runbook closeout evidence map and next slice.
- Added the Managed Relay Operator Setup Evidence Map to the relay runbook.
- Strengthened the deployment runbook check to require all managed operator
  setup evidence commands.
- Updated next-mode planning output.

## Evidence Map

The managed operator setup runbook now links:

- `npm run check:pwa-relay-managed-runtime-operator-setup-contract`
- `npm run check:pwa-relay-managed-runtime-operator-setup-import-preflight`
- `npm run smoke:pwa-relay-managed-runtime-operator-setup-browser-evidence`
- `npm run smoke:pwa-relay-managed-runtime-operator-setup-connection-controls`
- `npm run smoke:pwa-relay-managed-runtime-operator-setup-session-handshake`
- `npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-flow-evidence`

The closeout boundary keeps managed endpoint delivery out of scope and preserves
manual signed-response copy for approval responses.

## Next Slice

Managed relay runtime operator setup approval response delivery boundary:

- define how a signed approval response can leave the PWA after approval-flow
  evidence;
- keep manual signed-response copy as the current safe baseline;
- avoid rendering raw credentials, capability envelopes, payload material, or
  private key material;
- keep endpoint auto-start disabled and public bind off until a delivery path is
  explicitly verified.

## Verification

```powershell
npm run check:pwa-relay-deployment-runbook
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
