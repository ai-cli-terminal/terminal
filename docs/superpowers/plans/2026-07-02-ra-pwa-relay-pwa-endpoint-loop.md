# 2026-07-02 RA/PWA Relay PWA Endpoint Loop

## Purpose

The PWA can now see and validate a self-hosted relay setup bundle. This slice
turns the validated companion connect JSON into a browser-side relay endpoint
loop that can authenticate to a WebSocket relay, receive live companion
messages, and send replies as relay frames.

## Scope

- Add PWA helpers that derive a WebSocket connect URL from relay endpoint URL
  plus validated daemon/companion connect JSON.
- Add endpoint-loop state helpers for connect JSON, outgoing relay frames,
  queued acknowledgements, incoming relay frames, dropped expired frames, and
  bridge error envelopes.
- Add a `relayCompanionEndpointLoopFromSetup()` helper that only builds the
  companion loop after runtime setup preflight is ready.
- Extend PWA unit coverage for URL construction, connect envelope validation,
  route metadata checks, live payload decode, reply frame generation, and
  mismatch rejection.
- Extend the browser WebSocket bridge smoke so a setup-derived PWA companion
  loop authenticates with the local relay bridge, receives an approval request,
  sends an approval response, and proves route envelopes still do not expose
  `payload_json`.

## Guardrails

- Product default remains `live-loopback`.
- This is still helper/evidence work; the visible PWA does not switch operator
  approval traffic to relay automatically.
- The relay bridge remains a routing layer. Live approval payload validation is
  still performed at the daemon/PWA endpoint helpers.
- Runtime setup remains self-hosted WebSocket only. Managed and private-network
  modes stay deferred.

## Work Added

- Added `relayWebSocketConnectUrl`, `relayEndpointLoopInitialState`,
  `relayCompanionEndpointLoopFromSetup`, `relayEndpointLoopConnectJson`,
  `relayEndpointLoopNextFrame`, and `relayEndpointLoopAcceptSocketMessage` to
  `pwa/app.mjs`.
- Updated `pwa/app.test.mjs` with deterministic loop roundtrip coverage.
- Extended `scripts/smoke-pwa-relay-websocket-bridge.mjs` to exercise the PWA
  endpoint loop over a real browser WebSocket and local authenticated relay
  bridge.
- Tightened `scripts/check-pwa-relay-transport-decision.mjs` so transport
  evidence requires the PWA endpoint loop result.

## Follow-Up

1. Add daemon-side relay transport runtime integration behind an explicit
   transport selection, keeping `live-loopback` as the default.
2. Add browser/operator evidence for approve/reject over the relay transport
   before exposing relay as a selectable production workflow.
3. Revisit managed relay and private-network/Tailscale only after deployment,
   support, and security evidence exists.

## Verification

```powershell
npm run test:pwa
npm run smoke:pwa-relay-websocket-bridge
npm run check:pwa-relay-transport-decision
```
