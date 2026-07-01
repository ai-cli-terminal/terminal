# RA/PWA Live Transport Contract — 2026-07-01

> This document defines the first live companion transport slice. It is a
> contract document, not the full network server. The goal is to make Rust and
> the browser agree on the JSON envelope before adding WebSocket/SSE plumbing.

## Problem

The current PWA can sign approval responses, but the handoff is manual:

1. The daemon or CLI emits an approval request JSON or URL.
2. The user pastes or opens it in the PWA.
3. The PWA signs a response.
4. The user copies the response back to the terminal.

That is useful for smoke tests, but it is not a live companion. Before adding a
browser connection, both sides need a small, typed envelope for connection
identity, approval requests, approval responses, and heartbeat/error messages.

## Scope

This slice defines and tests the JSON envelope shared by:

- Rust `session.rs`
- Static PWA `pwa/app.mjs`
- Future local browser transport endpoint

It deliberately does not open a network listener yet.

## Envelope

All transport messages are JSON objects with a `type` string.

### `hello`

Sent by the browser companion after opening a live connection.

```json
{
  "type": "hello",
  "protocol_version": 1,
  "device_id": "web-1234abcd",
  "noise_pubkey_hex": "<32-byte hex public key>",
  "approval_pubkey_hex": "<32-byte hex public key>"
}
```

Validation:

- `protocol_version` must be `1`.
- `device_id` must be non-empty and contain only `A-Z a-z 0-9 . _ : -`.
- public keys must be 64 hex characters.
- no private key material is allowed.

### `approval_request`

Sent by the daemon to the browser when a High opt-in command needs approval.

```json
{
  "type": "approval_request",
  "request": {
    "approval_id": [97, 112, 112, 114],
    "nonce": [0, 1, 2],
    "command_masked": "chmod -R 777 .",
    "context_hash": "ctx",
    "expires_at": 1782804241,
    "device_epoch": 1
  }
}
```

The nested `request` is the existing `ApprovalRequestMsg` shape.

### `approval_response`

Sent by the browser to the daemon after approve/reject.

```json
{
  "type": "approval_response",
  "response": {
    "approval_id": [97, 112, 112, 114],
    "nonce": [0, 1, 2],
    "approve": true,
    "sig": [0, 1, 2]
  }
}
```

The nested `response` is the existing `ApprovalResponseMsg` shape. The daemon
still validates nonce, signature, epoch, expiry, and context hash before allowing
the command.

### `ping` / `pong`

Connection liveness messages. They carry an opaque short `nonce` string.

### `error`

Diagnostic message for connection setup failures. It is not authoritative for
gate decisions; the daemon still fails closed on timeout or malformed messages.

## Invariants

- Malformed messages are rejected before reaching gate decision logic.
- Browser code can construct all envelope messages without exposing private keys.
- Existing manual approval request/response JSON remains unchanged.
- Future transport code should use these envelopes over a local WebSocket or
  equivalent browser-compatible local channel.

## Completed In This Slice

- [x] Rust envelope type and JSON parse/validation helpers.
- [x] PWA envelope constructors and parser/validator.
- [x] Rust tests for accepted and rejected envelope shapes.
- [x] Node PWA tests for accepted and rejected envelope shapes.

## Follow-up Slice

P2b added a dependency-free loopback HTTP/SSE endpoint. Details:
`docs/superpowers/plans/2026-07-01-ra-pwa-live-loopback-endpoint.md`.

P2c added the live approval bridge. Details:
`docs/superpowers/plans/2026-07-01-ra-pwa-live-approval-bridge.md`.

The next slice should make the static PWA use this backend path from the UI.
