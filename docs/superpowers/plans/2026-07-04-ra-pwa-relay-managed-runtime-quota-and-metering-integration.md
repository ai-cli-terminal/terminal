# 2026-07-04 RA/PWA Relay Managed Runtime Quota and Metering Integration

## Purpose

Wire active-session, frame, and byte quota checks into the managed Relay/M2
runtime route boundary without opening a public endpoint, exposing managed relay
in the PWA, or changing the product default.

## Status

Completed in this slice as quota-and-metering integration evidence. The managed
runtime readiness gate remains green and `implementationCanStart=true`, but
`selectedRuntime` remains `deferred`, `runtimeDefault` remains `not-selected`,
endpoint mode remains `disabled`, public bind remains off, PWA exposure remains
disabled, and the product default remains `live-loopback`.

## Scope

- Add a quota/metering integration helper and summary for the managed runtime.
- Route only after encrypted-frame validation and quota acceptance.
- Enforce tenant active-session, daemon active-session, relay frame, and relay
  byte quota before encrypted frame delivery.
- Fail closed before delivery on quota denial.
- Record aggregate billing meter deltas and keep abuse signal deltas separate.
- Keep payload ciphertext hex, nonce hex, payload keys, plaintext payloads,
  command text, context data, and approval payloads out of metering surfaces.
- Move the next local slice to
  `managed-relay-runtime-support-and-abuse-operations-integration`.

## Non-Goals

- Do not bind a managed relay endpoint.
- Do not add support/abuse runtime operations yet.
- Do not expose managed relay in the PWA.
- Do not change `live-loopback` as the product default.

## Work Added

- Added `PWA_RELAY_MANAGED_RUNTIME_QUOTA_AND_METERING_INTEGRATION` to
  `pwa/app.mjs`.
- Added `createManagedRelayRuntimeQuotaAndMeteringIntegration()` to produce the
  quota/metering contract with endpoint/PWA exposure still disabled.
- Added `routeManagedRelayRuntimeQuotaMeteredFrame()` to validate encrypted
  frames, evaluate quota before delivery, return accepted metered routes, and
  reject quota failures before delivery.
- Added `relayManagedRuntimeQuotaAndMeteringIntegration()` to expose startup
  contract, quota/metering contract, health surface, evidence checks, remaining
  phases, and next local slice.
- Added `npm run check:pwa-relay-managed-runtime-quota-and-metering-integration`.
- Updated PWA tests, managed relay checks, and next-mode planning so the next
  local slice is
  `managed-relay-runtime-support-and-abuse-operations-integration`.

## Metering Boundary

The quota/metering boundary keeps:

- `selectedRuntime=deferred`
- `runtimeDefault=not-selected`
- `endpoint_mode=disabled`
- `public_bind_enabled=false`
- `pwa_exposure=disabled`
- `control_plane_runtime=tenant-session-registration-contract-wired`
- `route_runtime=encrypted-frame-routing-wired`
- `quota_runtime=active-session-frame-byte-metering-wired`
- rollback transport `live-loopback`

Quota decisions happen at `before-encrypted-frame-delivery`. Accepted routes
increment aggregate active-session/frame/byte counters. Rejected routes increment
quota denial counters and deliver no encrypted frame. Billing meter deltas and
abuse signal deltas stay separate, and metering-visible fields remain
payload-free.

## Next Slice

Managed relay runtime support and abuse operations integration:

- wire support redaction/access evidence into runtime-visible support views;
- keep support views aggregate-only and approval/audit bounded;
- wire abuse-operation counters without making them billing source data;
- keep `selectedRuntime` deferred and PWA exposure disabled;
- preserve live-loopback rollback.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-quota-and-metering-integration
npm run check:pwa-relay-managed-runtime-encrypted-frame-routing
npm run check:pwa-relay-managed-runtime-control-plane-contract-wiring
npm run check:pwa-relay-managed-runtime-service-scaffold
npm run check:pwa-relay-managed-runtime-implementation-plan
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
