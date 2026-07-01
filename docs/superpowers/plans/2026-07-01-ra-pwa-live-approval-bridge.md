# RA/PWA Live Approval Bridge — 2026-07-01

> P2c implementation note for connecting the loopback browser endpoint to the
> existing remote gate approval path. The goal is to reuse the current
> fail-closed validation path, not to create a second approval engine.

## Problem

The live loopback endpoint can accept `hello`, return `pong`, and expose an
SSE-readable endpoint. Before this slice, it still did not participate in gate
decisions:

1. `decide_with_remote_listener` sent a `DeviceListenerRequest`.
2. The native `device.sock` listener consumed that request.
3. The PWA endpoint was separate and could not wake the waiting gate request.

That meant the browser endpoint was real transport surface, but not yet the live
approval path.

## Choice

Reuse the existing `DeviceListenerRequest` queue boundary.

The browser endpoint now owns a `DeviceListenerHandle`:

- the gate sends the same `DeviceListenerRequest` it already used for
  `device.sock`;
- `GET /events` waits for that request and emits an `approval_request` envelope
  as Server-Sent Events;
- `POST /message` accepts an `approval_response` envelope, matches it to the
  pending approval id and nonce, and sends it back through the original
  `response_tx`;
- `decide_with_remote_listener` continues to call `finish_remote_gate_response`,
  so nonce, signature, expiry, epoch, and context-drift checks stay in one place.

## Flow

```text
shell hook
  -> gate.sock
    -> decide_with_remote_listener
      -> DeviceListenerRequest
        -> live endpoint pending slot
          -> GET /events => approval_request
          <- POST /message approval_response
      <- response_tx
    -> finish_remote_gate_response
  <- allow/block
```

## Public Surface

`ai remote daemon` in a remote build now uses the live loopback endpoint as the
approval listener it passes to `serve_with_remote`.

The native `device.sock` helpers remain in code and tests as the lower-level
Noise substrate, but the daemon's PWA path no longer requires a browser to open
that socket.

Endpoint behavior added in this slice:

| Request | Behavior |
|---|---|
| valid `POST /message` `hello` | Marks the registered companion as connected. |
| `GET /events` after `hello` and pending gate request | Emits `approval_request`. |
| `GET /events` after `hello` with no pending request | Emits heartbeat `ping`. |
| `GET /events` before `hello` | Returns HTTP 409 with typed `error`. |
| matching `POST /message` `approval_response` | Wakes the gate waiter and returns `pong`. |
| mismatched `approval_response` | Returns HTTP 409 and keeps the pending approval available. |

## Completed In This Slice

- [x] Live endpoint owns a `DeviceListenerHandle`.
- [x] Valid `hello` establishes the connected registered device.
- [x] `/events` emits pending `approval_request` envelopes from the gate queue.
- [x] `/message` accepts matching `approval_response` envelopes and wakes the
      existing gate waiter.
- [x] Mismatched responses fail closed without dropping the pending approval.
- [x] Rust tests cover live gate allow roundtrip and mismatched response recovery.

## Remaining Work

The next slice is P3 UI and evidence:

1. Add PWA connected/disconnected status around the printed live endpoint URL.
2. Use `EventSource` on `/events` to render incoming `approval_request` items.
3. Send approve/reject responses through `postLiveTransportMessage`.
4. Run an end-to-end smoke with `ai remote daemon`, `ai remote arm --allow-high`,
   a High-risk command, and a browser/PWA approval.
5. Decide whether to keep `device.sock` as a fallback runtime path, expose a
   daemon flag for native-vs-PWA approval listeners, or move native device socket
   behind a later transport mode.
