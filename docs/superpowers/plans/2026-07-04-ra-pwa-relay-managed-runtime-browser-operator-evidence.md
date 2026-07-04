# 2026-07-04 RA/PWA Relay Managed Runtime Browser Operator Evidence

## Purpose

Capture browser/operator evidence for the managed Relay/M2 PWA exposure surface
after the explicit opt-in PWA exposure gate passed.

## Status

Completed in this slice as browser and operator evidence. The PWA now has
automated desktop and mobile evidence that the Managed Relay panel is visible,
explicit opt-in only, keeps `live-loopback` as product default, keeps public bind
off, keeps endpoint auto-start disabled, and exposes no payloads, secrets, raw
session tokens, signed tickets, key material, or raw identifiers.

## Scope

- Add a browser/operator evidence summary for the managed runtime.
- Capture a desktop screenshot of the PWA Relay tab with the Managed Relay panel.
- Capture a mobile screenshot and assert no horizontal overflow.
- Assert the visible operator text keeps managed relay explicit opt-in only.
- Assert the visible PWA body does not expose prohibited payload, ticket, token,
  key, support, or device identifier fields.
- Move the next local slice to managed relay operator setup contract work.

## Non-Goals

- Do not start a managed relay endpoint.
- Do not connect to a production managed relay service.
- Do not change the product default away from `live-loopback`.
- Do not enable public bind or endpoint auto-start.
- Do not add tenant/customer control-plane UI beyond the current setup/copy
  exposure surface.

## Work Added

- Added `PWA_RELAY_MANAGED_RUNTIME_BROWSER_OPERATOR_EVIDENCE` to `pwa/app.mjs`.
- Added `relayManagedRuntimeBrowserOperatorEvidence()` to expose the evidence
  summary, required selectors, screenshots, guardrails, and next local slice.
- Added `npm run smoke:pwa-relay-managed-runtime-browser-operator-evidence`.
- Added Playwright evidence capture for the Managed Relay panel on desktop and
  mobile.

## Evidence Boundary

The browser evidence asserts:

- `Product default` shows `live-loopback`
- `Exposure` shows `explicit-opt-in`
- `Endpoint` shows `operator-setup-required`
- `Public bind` shows `off`
- `Auto start` shows `off`
- `Rollback` shows `live-loopback`
- setup copy says operator-issued setup is required

The visible PWA body must not contain:

- payload JSON or plaintext command/context data
- payload ciphertext hex, nonce hex, payload keys, or shared secrets
- raw session tokens or signed tickets
- HMAC secrets or MAC material
- raw support actor, session, daemon-device, or companion-device identifiers

## Next Slice

Managed relay runtime operator setup contract:

- define the operator-issued setup payload boundary for managed relay;
- keep endpoint activation explicit and operator-owned;
- preserve live-loopback rollback;
- keep browser evidence as a required regression check.

## Verification

```powershell
npm run smoke:pwa-relay-managed-runtime-browser-operator-evidence
npm run check:pwa-relay-managed-runtime-pwa-exposure-gate
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
