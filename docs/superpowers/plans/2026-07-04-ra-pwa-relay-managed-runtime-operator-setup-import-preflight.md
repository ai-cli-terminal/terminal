# 2026-07-04 RA/PWA Relay Managed Runtime Operator Setup Import Preflight

## Purpose

Add the Managed Relay/M2 operator setup import preflight after the
operator-issued setup contract was defined.

## Status

Completed in this slice as parser, validator, PWA status-only import path, and
repeatable check. Managed setup import does not start an endpoint, does not
enable public bind, does not change the product default away from
`live-loopback`, and does not render the original setup JSON after import.

## Scope

- Add a parser for operator-issued managed setup payloads.
- Validate the setup payload against the managed operator setup contract.
- Require `wss://` endpoints, unexpired validity windows, and hash-only
  identifiers.
- Reject unknown fields and prohibited fields such as signed tickets, raw
  session tokens, payloads, key material, and raw identifiers.
- Add a PWA Managed Relay import/status block.
- Render only a sanitized metadata summary after import.
- Keep connect disabled in this preflight slice.
- Move the next local slice to browser evidence for the managed setup import
  path.

## Non-Goals

- Do not connect to a managed relay endpoint.
- Do not add managed relay connect/disconnect controls.
- Do not render full setup JSON, signed tickets, raw tokens, payload
  ciphertext, payload keys, HMAC material, or raw device/session identifiers.
- Do not change `live-loopback` as the product default.

## Work Added

- Added `parseManagedRelayRuntimeOperatorSetupInput()`.
- Added `validateManagedRelayRuntimeOperatorSetupMetadata()`.
- Added `managedRelayRuntimeOperatorSetupImportPreflight()`.
- Added `relayManagedRuntimeOperatorSetupImportPreflight()`.
- Added Managed Relay setup import/status UI in the PWA Relay tab.
- Added `npm run check:pwa-relay-managed-runtime-operator-setup-import-preflight`.
- Updated next-mode planning so the next local slice is
  `managed-relay-runtime-operator-setup-browser-evidence`.

## Import Boundary

The import preflight accepts the operator setup contract payload shape:

- `setup_version = 1`
- `deployment_mode = managed`
- `relay_endpoint_url` with `wss://`
- tenant id
- hashed session, daemon-device, and companion-device identifiers
- verifier key id/version
- issued/expires timestamps and optional `not_before_ms`
- operator setup text
- `rollback_transport = live-loopback`

After import, the PWA shows only:

- endpoint URL
- tenant
- session, daemon, and companion hashes
- verifier key id/version
- expiry
- manual activation state
- rollback transport

The PWA does not render the original JSON after import.

## Next Slice

Managed relay runtime operator setup browser evidence:

- capture desktop and mobile evidence for the new Managed Relay import/status
  block;
- confirm no horizontal overflow on mobile;
- confirm the original setup JSON and prohibited tokens are not visible after
  import;
- keep endpoint auto-start disabled, public bind off, and connect controls out
  of scope.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-operator-setup-import-preflight
npm run check:pwa-relay-managed-runtime-operator-setup-contract
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
