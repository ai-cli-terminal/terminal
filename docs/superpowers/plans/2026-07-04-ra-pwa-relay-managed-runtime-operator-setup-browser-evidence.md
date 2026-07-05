# 2026-07-04 RA/PWA Relay Managed Runtime Operator Setup Browser Evidence

## Purpose

Capture browser evidence for the Managed Relay/M2 operator setup import path
before adding any managed connection controls.

## Status

Completed in this slice as a browser evidence contract and repeatable smoke.
The PWA imports an operator-issued managed setup payload, hides the original
JSON, renders only a sanitized metadata summary, captures desktop/mobile
screenshots, and confirms there is no mobile horizontal overflow.

## Scope

- Add `relayManagedRuntimeOperatorSetupBrowserEvidence()`.
- Define required desktop/mobile screenshot artifacts for the operator setup
  import path.
- Verify the existing Managed Relay setup UI renders the ready import state in
  a browser.
- Confirm the visible surface excludes setup field names, signed tickets,
  tokens, payload fields, key material, raw identifiers, operator setup text,
  and support contact metadata.
- Keep endpoint auto-start disabled, public bind off, and connect controls out
  of scope.
- Move the next local slice to managed relay operator setup connection
  controls.

## Non-Goals

- Do not connect to a managed relay endpoint.
- Do not add managed relay connect/disconnect controls.
- Do not render full setup JSON, signed tickets, raw tokens, payload material,
  key material, support contact metadata, or raw identifiers.
- Do not change `live-loopback` as the product default.

## Work Added

- Added the managed operator setup browser evidence summary in `pwa/app.mjs`.
- Added PWA tests for required selectors, screenshots, expected visible copy,
  prohibited visible tokens, evidence checks, and next slice selection.
- Added `npm run smoke:pwa-relay-managed-runtime-operator-setup-browser-evidence`.
- Updated next-mode planning so the next local slice is
  `managed-relay-runtime-operator-setup-connection-controls`.

## Evidence Boundary

The smoke opens the PWA Relay tab in Chromium, loads a valid managed setup
payload, and verifies:

- import state is `Ready`;
- product default remains `live-loopback`;
- endpoint mode remains `operator-setup-required`;
- public bind remains off;
- endpoint auto-start remains off;
- setup input is replaced with `Managed setup imported (metadata hidden)`;
- sanitized summary shows endpoint URL, tenant, hashed identifiers, verifier,
  expiry, manual activation, and rollback only;
- original setup JSON, setup field names, operator setup text, support contact,
  tickets, tokens, payloads, key material, and raw identifiers are not visible;
- desktop and mobile screenshots are captured;
- mobile layout has no horizontal overflow.

## Next Slice

Managed relay runtime operator setup connection controls:

- add explicit managed connect/disconnect controls after a ready import;
- keep activation manual and user-driven;
- keep endpoint auto-start disabled and public bind off;
- preserve the sanitized setup surface and `live-loopback` rollback.

## Verification

```powershell
npm run smoke:pwa-relay-managed-runtime-operator-setup-browser-evidence
npm run check:pwa-relay-managed-runtime-operator-setup-import-preflight
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
