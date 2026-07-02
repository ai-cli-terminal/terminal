# 2026-07-02 RA/PWA Relay UX Preflight

## Purpose

Relay has local WebSocket evidence, signed tickets, reconnect coverage, and a
daemon issuer policy. This slice keeps relay non-product-facing in the PWA until
all operator-facing prerequisites are present: relay mode, endpoint URL, signed
session ticket, matching companion identity, deployment mode, and setup text.

## Scope

- Add a PWA `relayTransportUxPreflight` helper.
- Keep the default PWA state hidden while `live-loopback` remains the product
  transport.
- Require a WebSocket relay endpoint URL (`wss://`, or localhost `ws://` for
  local smoke/development).
- Require a signed relay session ticket with valid metadata and non-expired
  session timestamps.
- Require the signed ticket to match the active companion device id, Noise
  public key, and approval public key.
- Require the selected self-hosted deployment mode and non-empty operator setup
  text.
- Add `npm run check:pwa-relay-ux-preflight` to write deterministic evidence.

## Non-Goals

- No visible relay setup UI.
- No product default change away from `live-loopback`.
- No hosted relay deployment.
- No PWA access to relay HMAC secrets.

## Evidence Shape

The PWA check proves:

- Default state is `hidden` and includes blockers for non-relay transport mode,
  missing endpoint URL, missing signed ticket, and missing identity.
- Relay mode with missing inputs remains `hidden`.
- A ready relay shape becomes `ready` only when endpoint, signed ticket,
  matching identity, selected self-hosted deployment mode, and setup text are all
  present.
- Expired tickets keep relay hidden.
- Deferred deployment modes such as `managed` keep relay hidden with
  `relay_deployment_mode_not_selected`.
- Invalid endpoint URL, unknown deployment mode, short setup text, and identity
  mismatch keep relay hidden.

Evidence path:

```text
artifacts/ra-pwa-relay-ux-preflight/ra-pwa-relay-ux-preflight.json
```

## Follow-Up

1. Add visible self-hosted relay setup UI only after endpoint, ticket, identity,
   and operator copy are ready.
2. Add browser evidence that the runtime setup bundle drives relay UX preflight
   to ready without changing the product default.
3. Revisit managed relay and private-network/Tailscale only after separate
   deployment and support evidence exists.

## Progress

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-secret-keyring-migration.md`
added optional signed ticket key-id metadata validation on the PWA side while
keeping relay HMAC secrets daemon-only.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-daemon-runtime-issuer.md`
added daemon/runtime setup issuance for the endpoint, signed ticket, matching
identity, and operator setup text consumed by this preflight.

## Verification

```powershell
npm run test:pwa
npm run check:pwa-relay-deployment-decision
npm run check:pwa-relay-ux-preflight
npm run check:pwa-relay-transport-decision
```
