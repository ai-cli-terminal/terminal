# RA/PWA Live PWA UX — 2026-07-01

> P3 implementation note for turning the static companion page into a live
> approval client for the local daemon endpoint.

## Problem

The daemon can now expose a loopback endpoint and bridge live
`approval_request` / `approval_response` messages into the existing remote gate
path. The PWA still behaves like a manual JSON tool:

1. the operator pastes an approval request;
2. the page signs a response;
3. the operator copies that response or a verify command by hand.

That proves the cryptographic boundary, but it is not yet a companion workflow.

## Scope

This slice keeps the daemon transport contract unchanged and updates only the
static PWA plus tests.

In scope:

- accept the printed daemon live endpoint URL in the PWA;
- send a typed `hello` using the stored/generated companion identity;
- open `EventSource` on `/events`;
- render incoming `approval_request` messages as the active pending approval;
- sign approve/reject with the existing non-extractable approval key;
- POST a typed `approval_response` through `/message`;
- keep the manual paste/copy flow working as a fallback.

Out of scope:

- relay/Tailscale transport;
- native `device.sock` fallback flags;
- full browser-driven daemon e2e evidence. That belongs to P4 after this UI
  path is available.

## Flow

```text
operator opens PWA
  -> enters PWA live endpoint printed by `ai remote daemon`
  -> Connect sends `hello`
  -> EventSource `/events`
     -> ping updates connected state
     -> approval_request fills the Approve panel
  -> Approve / Reject signs with IndexedDB key material
  -> POST `/message` approval_response
  -> daemon gate waiter resumes existing validation path
```

## Acceptance

- [x] PWA shows live endpoint input and connected/disconnected state.
- [x] Connect sends `hello` and opens `/events`.
- [x] Incoming `approval_request` populates the approval panel without manual
      JSON paste.
- [x] Approve/reject sends `approval_response` through `postLiveTransportMessage`
      when live mode is connected.
- [x] Manual approval paste/copy still works without a live endpoint.
- [x] `node pwa/app.test.mjs` covers the new live UI helpers.

## Remaining After This Slice

P4 should run and record end-to-end evidence:

1. start `ai remote daemon --device-id <id>`;
2. open the PWA and connect to the printed live endpoint;
3. run `ai remote arm --allow-high`;
4. trigger a High-risk command;
5. approve and reject from the PWA;
6. record evidence paths in `docs/HISTORY.md` and `docs/HANDOFF.md`.
