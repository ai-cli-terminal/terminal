# RA/PWA Live Loopback Endpoint — 2026-07-01

> P2b implementation note for the browser-compatible local companion endpoint.
> The previous slice defined the shared JSON envelope. This slice gives the PWA
> a real daemon endpoint to talk to without adding a WebSocket dependency yet.

## Problem

The PWA can build `hello`, `ping`, `approval_request`, `approval_response`, and
`error` envelopes, but there was no browser-readable daemon endpoint. The only
live-ish path was still the native `device.sock` Noise listener, which a browser
cannot open directly.

## Choice

Use a small loopback HTTP endpoint first:

- `GET /health` returns endpoint metadata.
- `GET /events` returns a minimal Server-Sent Events stream with a typed
  transport envelope.
- `POST /message` accepts one `CompanionTransportMsg` JSON envelope and returns
  one `CompanionTransportMsg` JSON envelope.

This keeps the first browser endpoint dependency-free. It also leaves the future
WebSocket or relay shape open because all payloads still use the same transport
envelope from `session.rs` and `pwa/app.mjs`.

## Public Surface

Daemon startup with `--features remote` now starts and prints:

```text
PWA live endpoint  : http://127.0.0.1:<port>
PWA message endpoint: http://127.0.0.1:<port>/message
PWA events endpoint : http://127.0.0.1:<port>/events
```

Endpoint behavior:

| Request | Behavior |
|---|---|
| `GET /health` | JSON descriptor with `protocol_version`, `base_url`, `message_url`, and `events_url`. |
| `GET /events` | Minimal `text/event-stream` response containing a `ping` envelope. |
| `POST /message` with `ping` | Returns matching `pong`. |
| `POST /message` with `hello` | Checks selected/registered device id plus Noise and approval public keys, then returns `pong`. |
| `POST /message` with malformed JSON/envelope | Returns HTTP 400 with an `error` envelope. |
| unknown path | Returns HTTP 404 with an `error` envelope. |
| incomplete request body | Returns HTTP 408 with an `error` envelope after the endpoint timeout. |

## Implementation

- Rust endpoint: `daemon::spawn_companion_live_endpoint`.
- Rust envelope parser: `session::parse_companion_transport_json`.
- PWA endpoint helpers:
  - `liveEndpointUrls(baseUrl)`
  - `liveMessageRequest(message)`
  - `postLiveTransportMessage(baseUrl, message, fetchImpl)`

The endpoint always binds `127.0.0.1:0`, so the OS selects an available local
port. It emits CORS headers because the static PWA may be served from another
local origin during development.

## Completed In This Slice

- [x] Daemon starts a browser-compatible loopback endpoint in remote builds.
- [x] `ai remote daemon` prints the live base, message, and events URLs.
- [x] Browser/PWA helper code can POST typed live transport messages.
- [x] Rust tests cover descriptor, SSE framing, ping/pong, registered `hello`,
      malformed JSON, unknown route, and incomplete-body timeout.
- [x] Node PWA tests cover endpoint URL normalization, POST request construction,
      success parsing, and error propagation.

## Remaining Work

The follow-up slice attached live approval bridge state. Details:
`docs/superpowers/plans/2026-07-01-ra-pwa-live-approval-bridge.md`.

The remaining work is PWA UI/evidence: connect from the browser, render incoming
approvals, post signed approve/reject responses, and capture an end-to-end smoke.
