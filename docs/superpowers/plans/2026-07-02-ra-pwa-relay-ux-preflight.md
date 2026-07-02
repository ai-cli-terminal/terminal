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
- Require a known deployment mode and non-empty operator setup text.
- Add `npm run check:pwa-relay-ux-preflight` to write deterministic evidence.

## Non-Goals

- No visible relay setup UI.
- No product default change away from `live-loopback`.
- No hosted relay deployment.
- No persistent relay secret storage or key id migration.
- No PWA access to relay HMAC secrets.

## Evidence Shape

The PWA check proves:

- Default state is `hidden` and includes blockers for non-relay transport mode,
  missing endpoint URL, missing signed ticket, and missing identity.
- Relay mode with missing inputs remains `hidden`.
- A ready relay shape becomes `ready` only when endpoint, signed ticket,
  matching identity, deployment mode, and setup text are all present.
- Expired tickets keep relay hidden.
- Invalid endpoint URL, unknown deployment mode, short setup text, and identity
  mismatch keep relay hidden.

Evidence path:

```text
artifacts/ra-pwa-relay-ux-preflight/ra-pwa-relay-ux-preflight.json
```

## Follow-Up

1. Decide deployment shape: self-hosted relay, private-network/Tailscale direct
   mode, or managed relay.
2. Add persistent relay secret storage and key id migration after deployment
   shape is chosen.
3. Add visible relay setup UI only after deployment mode and operator copy are
   ready.

## Verification

```powershell
npm run test:pwa
npm run check:pwa-relay-ux-preflight
npm run check:pwa-relay-transport-decision
```
