# 2026-07-04 RA/PWA Relay Managed Runtime Encrypted Frame Routing

## Purpose

Wire encrypted frame routing into the managed Relay/M2 runtime boundary without
opening a public endpoint, exposing managed relay in the PWA, or changing the
product default.

## Status

Completed in this slice as encrypted-routing evidence. The managed runtime
readiness gate remains green and `implementationCanStart=true`, but
`selectedRuntime` remains `deferred`, `runtimeDefault` remains `not-selected`,
endpoint mode remains `disabled`, public bind remains off, PWA exposure remains
disabled, and the product default remains `live-loopback`.

## Scope

- Add an encrypted frame routing helper and summary for the managed runtime.
- Route only validated managed encrypted relay frames.
- Expose only the route-visible allowlist: protocol version, session id,
  sender, sequence, timing, ciphertext algorithm/scope, and ciphertext byte
  count.
- Keep ciphertext hex, nonce hex, payload keys, plaintext payloads, command
  text, context data, and approval payloads out of route-visible surfaces.
- Fail closed before route for expired frames and plaintext frame fields.
- Move the next local slice to
  `managed-relay-runtime-quota-and-metering-integration`.

## Non-Goals

- Do not bind a managed relay endpoint.
- Do not integrate quota/metering counters yet.
- Do not add support/abuse runtime operations yet.
- Do not expose managed relay in the PWA.
- Do not change `live-loopback` as the product default.

## Work Added

- Added `PWA_RELAY_MANAGED_RUNTIME_ENCRYPTED_FRAME_ROUTING` to `pwa/app.mjs`.
- Added `createManagedRelayRuntimeEncryptedFrameRouting()` to produce the
  routing contract with endpoint/PWA exposure still disabled.
- Added `routeManagedRelayRuntimeEncryptedFrame()` to validate encrypted frames,
  reject expired/plaintext frames, and return a payload-free route decision.
- Added `relayManagedRuntimeEncryptedFrameRouting()` to expose startup contract,
  route contract, health surface, evidence checks, remaining phases, and next
  local slice.
- Added `npm run check:pwa-relay-managed-runtime-encrypted-frame-routing`.
- Updated PWA tests, managed relay checks, and next-mode planning so the next
  local slice is `managed-relay-runtime-quota-and-metering-integration`.

## Route Boundary

The routing boundary keeps:

- `selectedRuntime=deferred`
- `runtimeDefault=not-selected`
- `endpoint_mode=disabled`
- `public_bind_enabled=false`
- `pwa_exposure=disabled`
- `control_plane_runtime=tenant-session-registration-contract-wired`
- `route_runtime=encrypted-frame-routing-wired`
- rollback transport `live-loopback`

The route decision returns only allowlisted routing metadata. The encrypted
frame itself can be forwarded internally, but `payload_ciphertext_hex`,
`payload_nonce_hex`, payload keys, plaintext payloads, command/context content,
approval payloads, raw session tokens, setup JSON, HMAC secrets, and MAC
material remain prohibited from route-visible surfaces.

## Next Slice

Managed relay runtime quota and metering integration:

- enforce active session, frame, and byte quotas before route;
- record aggregate frame/byte counters without payloads or secrets;
- keep billing usage and abuse signals separate;
- keep `selectedRuntime` deferred and PWA exposure disabled;
- preserve live-loopback rollback.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-encrypted-frame-routing
npm run check:pwa-relay-managed-runtime-control-plane-contract-wiring
npm run check:pwa-relay-managed-runtime-service-scaffold
npm run check:pwa-relay-managed-runtime-implementation-plan
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
