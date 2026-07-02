# 2026-07-02 RA/PWA Relay Visible Setup UI

## Purpose

The daemon can issue a self-hosted relay setup bundle. This slice makes that
bundle visible and verifiable in the static PWA without changing the product
default from `live-loopback`.

## Scope

- Add a PWA `Relay` tab with runtime setup JSON input.
- Parse the camelCase `ai remote relay-setup` bundle in the browser.
- Validate relay protocol version, selected self-hosted deployment mode,
  WebSocket endpoint policy, signed ticket metadata, daemon/companion connect
  JSON, companion identity binding, and operator setup text.
- Reject obvious secret/keyring fields such as `secret` or
  `hmac_sha256_keys`.
- Render ready/blocked state, endpoint, selected deployment, device, session,
  expiry, active ticket key id, and daemon/companion connect JSON.
- Add browser evidence that a valid setup bundle drives
  `relayRuntimeSetupPreflight` to ready while the UI still records
  `live-loopback` as the product default.

## Guardrails

- The PWA never accepts or displays HMAC keyring secrets.
- Expired tickets remain parseable but blocked by preflight.
- The visible relay UI is a setup/readiness surface only; it does not switch
  runtime approval traffic away from local live loopback.
- Managed relay and private-network/Tailscale modes remain deferred.

## Work Added

- Added `parseRelayRuntimeSetupInput`, `validateRelayRuntimeSetupMetadata`, and
  `relayRuntimeSetupPreflight` to `pwa/app.mjs`.
- Added a visible `Relay` panel to `pwa/index.html` and matching CSS.
- Added PWA unit coverage for setup JSON parsing, URL payload parsing, secret
  field rejection, connect mismatch rejection, ready preflight, and expired
  preflight blocking.
- Added `scripts/smoke-pwa-relay-setup-ui.mjs` and
  `npm run smoke:pwa-relay-setup-ui` for real-browser UI evidence.

## Follow-Up

1. Wire the PWA companion relay endpoint loop to a self-hosted WebSocket relay
   using the validated companion connect JSON.
2. Add daemon-side relay transport runtime integration behind an explicit
   transport selection while keeping `live-loopback` as the default until
   browser/operator evidence covers approve/reject over relay.
3. Revisit managed relay and private-network/Tailscale only after deployment,
   support, and security evidence exists.

## Verification

```powershell
npm run test:pwa
npm run smoke:pwa-relay-setup-ui
npm run check:pwa-relay-ux-preflight
```
