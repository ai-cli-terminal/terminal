# 2026-07-04 RA/PWA Relay Managed Runtime Service Scaffold

## Purpose

Add the first managed Relay/M2 runtime service scaffold without changing the
product default or exposing managed relay in the PWA.

## Status

Completed in this slice as scaffold evidence. The managed runtime readiness gate
remains green and the implementation plan is complete, but `selectedRuntime`
remains `deferred`, `runtimeDefault` remains `not-selected`, public bind remains
disabled, and the product default remains `live-loopback`.

## Scope

- Add a managed runtime service scaffold helper and summary.
- Prove the scaffold starts without public listener or PWA exposure.
- Keep route and session registration handlers disabled until later wiring
  slices.
- Keep the health surface aggregate-only and payload-free.
- Move the next local slice to
  `managed-relay-runtime-control-plane-contract-wiring`.

## Non-Goals

- Do not bind a managed relay endpoint.
- Do not wire session registration, frame routing, quota metering, or support
  operations into a running service.
- Do not expose managed relay in the PWA.
- Do not change `live-loopback` as the product default.

## Work Added

- Added `PWA_RELAY_MANAGED_RUNTIME_SERVICE_SCAFFOLD` to `pwa/app.mjs`.
- Added `createManagedRelayRuntimeServiceScaffold()` to produce a scaffold
  config/health surface with endpoint mode disabled and public bind off.
- Added `relayManagedRuntimeServiceScaffold()` to expose startup contract,
  health surface, evidence checks, remaining implementation phases, and next
  local slice.
- Added `npm run check:pwa-relay-managed-runtime-service-scaffold`.
- Updated PWA tests and next-mode planning checks so the next local slice is
  `managed-relay-runtime-control-plane-contract-wiring`.

## Scaffold Contract

The scaffold starts with:

- `selectedRuntime=deferred`
- `runtimeDefault=not-selected`
- `endpoint_mode=disabled`
- `public_bind_enabled=false`
- `pwa_exposure=disabled`
- `route_runtime=not-wired`
- `control_plane_runtime=not-wired`
- rollback transport `live-loopback`

## Health Surface

Allowed health fields are aggregate-only:

- `service_state`
- `pwa_exposure`
- `endpoint_mode`
- `tenant_count`
- `active_session_count`
- `relay_frame_count`
- `relay_byte_count`
- `quota_denial_count`
- `payload_visibility`
- `support_visibility`

Payloads, command/context content, approval payloads, payload keys, shared
secrets, private key material, raw session tokens, full setup JSON, HMAC
secrets, and MAC material remain prohibited from the runtime health/config
surface.

## Next Slice

Follow-up status: Managed relay runtime control-plane contract wiring and
encrypted frame routing are now complete. Current next slice is managed relay
runtime quota and metering integration:

- enforce active session, frame, and byte quotas before route;
- record aggregate frame/byte counters without payloads or secrets;
- keep `selectedRuntime` deferred and PWA exposure disabled;
- preserve live-loopback rollback.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-service-scaffold
npm run check:pwa-relay-managed-runtime-implementation-plan
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
