# 2026-07-04 RA/PWA Relay Managed Runtime PWA Exposure Gate

## Purpose

Close the managed Relay/M2 runtime PWA exposure gate after service scaffold,
control-plane wiring, encrypted routing, quota/metering, support evidence, and
billing/abuse operation checks are green.

## Status

Completed in this slice as an explicit PWA exposure gate. Managed relay becomes
PWA-visible only as an explicit opt-in setup path. The product default remains
`live-loopback`, `runtimeDefault` remains `not-selected`, public bind remains
off, and endpoint auto-start remains disabled.

## Scope

- Add a managed runtime PWA exposure gate helper and summary.
- Require the completed support/abuse operations integration before exposure.
- Publish only setup/copy/status text into the PWA surface.
- Keep managed relay out of the product default path.
- Keep public bind and endpoint auto-start disabled.
- Preserve rollback to `live-loopback`.
- Move the next local slice to browser/operator evidence for the newly visible
  managed relay surface.

## Non-Goals

- Do not make managed relay the default transport.
- Do not start a managed relay endpoint from the PWA.
- Do not bind public managed relay routes.
- Do not expose payload JSON, plaintext command/context, approval payloads,
  raw session tokens, signed tickets, key material, HMAC secrets, MAC material,
  raw device identifiers, or raw support actor identifiers.
- Do not implement a production managed relay control plane beyond the existing
  contract and runtime evidence.

## Work Added

- Added `PWA_RELAY_MANAGED_RUNTIME_PWA_EXPOSURE_GATE` to `pwa/app.mjs`.
- Added `createManagedRelayRuntimePwaExposureGate()` to build the PWA-visible
  managed relay setup/copy contract.
- Added `relayManagedRuntimePwaExposureGate()` to expose the gate summary,
  startup boundary, PWA copy, visible fields, prohibited fields, evidence
  checks, and next local slice.
- Added PWA markup/rendering for a managed relay exposure panel.
- Added
  `npm run check:pwa-relay-managed-runtime-pwa-exposure-gate`.

## Exposure Boundary

The PWA-visible managed relay boundary keeps:

- `productDefault=live-loopback`
- `selectedRuntime=explicit-opt-in-managed`
- `runtimeDefault=not-selected`
- `pwa_exposure=explicit-opt-in`
- `endpoint_mode=operator-setup-required`
- `public_bind_enabled=false`
- `endpoint_auto_start=false`
- rollback transport `live-loopback`

The PWA surface may show only setup/copy/status metadata. It must not contain
payload bodies, ciphertext hex, nonce hex, key material, signed tickets, raw
tokens, raw device identifiers, raw support actor identifiers, or support/abuse
internal fields.

## Next Slice

Managed relay runtime browser/operator evidence:

- capture browser evidence that the managed relay panel is visible;
- verify the panel keeps managed relay opt-in and not default;
- verify no public bind or endpoint auto-start is implied by the UI;
- keep the existing live-loopback rollback path visible.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-pwa-exposure-gate
npm run check:pwa-relay-managed-runtime-support-and-abuse-operations-integration
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
