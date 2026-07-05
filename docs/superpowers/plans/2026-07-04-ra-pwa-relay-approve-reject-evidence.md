# 2026-07-04 RA/PWA Relay Approve Reject Evidence

## Purpose

Close the next local Relay/M2 evidence gap: make the visible PWA `Relay` tab
connect to a setup-derived self-hosted WebSocket relay endpoint, receive High
approval requests, and send signed approve/reject responses over relay frames.

## Status

Completed in this slice. The visible PWA `Relay` tab can load daemon-issued
runtime setup JSON, connect as the companion over the setup-derived WebSocket
relay endpoint, receive High approval requests, and send signed approve/reject
responses back to a running relay daemon.

## Scope

- Keep `live-loopback` as the product default.
- Extend the Relay tab beyond setup readiness into a connection/operator
  surface.
- Reuse the existing setup-derived `relayCompanionEndpointLoopFromSetup` helper
  for WebSocket URL, connect JSON, frame encoding, and incoming frame decode.
- Reuse existing approval signing and monitor counters so relay evidence records
  received, sent, approved, rejected, and pending counts.
- Add a browser smoke that captures approve and reject evidence through the
  visible Relay tab.

## Non-Goals

- Do not make relay the default transport.
- Do not add hosted relay infrastructure or TLS deployment.
- Do not expose relay HMAC key material in PWA setup, DOM, screenshots, or
  evidence artifacts.
- Do not change release tags or public release assets.

## Implementation Plan

1. Add Relay tab controls for connect/disconnect and visible runtime counters.
2. Create a small browser-side relay WebSocket session wrapper around the
   existing endpoint-loop helpers.
3. Feed incoming relay `approval_request` messages into the existing approval
   request panel and queue.
4. On Approve/Reject, when relay is connected, sign with the stored companion
   approval key and send an `approval_response` relay frame.
5. Add a Playwright smoke for setup load, relay connect, approve path, reject
   path, screenshots, and JSON evidence.
6. Update HISTORY/HANDOFF/remaining priority after verification.

## Evidence Target

The browser smoke should write an artifact under
`artifacts/ra-pwa-relay-approve-reject-evidence/` with:

- PWA URL and relay endpoint URL.
- Browser executable path.
- Screenshots for connected, approve-pending, reject-pending, and final states.
- Relay counters showing `received=2`, `sent=2`, `approved=1`, `rejected=1`,
  and `pending=0`.
- Approve command exit code `0` and reject command non-zero when the smoke is
  wired to the Rust daemon; if the smoke uses a daemon harness, it must still
  prove both relay request/response directions through the visible PWA UI.

## Work Added

- Added visible Relay tab connect/disconnect controls, runtime counters, and a
  relay approval queue.
- Wired the PWA Relay tab to `relayCompanionEndpointLoopFromSetup` and browser
  `WebSocket` so it authenticates with daemon-issued companion connect JSON.
- Routed incoming relay `approval_request` messages into the existing approval
  panel and signed approve/reject responses back as relay frames.
- Added `npm run smoke:pwa-relay-approve-reject-evidence`, which starts an
  isolated WSL daemon with `--transport relay`, a WSL-local stdlib Python relay
  bridge, and a Playwright/Chrome PWA operator session.
- Captured evidence at
  `artifacts/ra-pwa-relay-approve-reject-evidence/ra-pwa-relay-approve-reject-evidence.json`.

## Verification

```powershell
npm run test:pwa
npm run smoke:pwa-relay-approve-reject-evidence
npm run smoke:pwa-relay-websocket-bridge
git diff --check
```

Latest run:

- `npm run test:pwa` — `PWA_COMPANION_TEST_OK`
- `npm run smoke:pwa-relay-approve-reject-evidence` —
  `RA_PWA_RELAY_APPROVE_REJECT_EVIDENCE_OK`
- `npm run smoke:pwa-relay-setup-ui` — `RA_PWA_RELAY_SETUP_UI_OK`
- `npm run smoke:pwa-relay-websocket-bridge` —
  `RA_PWA_RELAY_WEBSOCKET_BRIDGE_OK`
- Evidence counters: `received=2`, `sent=2`, `approved=1`, `rejected=1`,
  `pending=0`; approve exit code `0`, reject exit code `1`.
