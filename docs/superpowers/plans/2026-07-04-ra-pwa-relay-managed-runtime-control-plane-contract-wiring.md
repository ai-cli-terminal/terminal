# 2026-07-04 RA/PWA Relay Managed Runtime Control-Plane Contract Wiring

## Purpose

Wire the managed Relay/M2 runtime control-plane contract on top of the service
scaffold without opening a public endpoint, exposing managed relay in the PWA,
or changing the product default.

## Status

Completed in this slice as contract-wiring evidence. The managed runtime
readiness gate remains green and `implementationCanStart=true`, but
`selectedRuntime` remains `deferred`, `runtimeDefault` remains `not-selected`,
endpoint mode remains `disabled`, public bind remains off, and the product
default remains `live-loopback`.

## Scope

- Add a control-plane wiring helper and summary for the managed runtime.
- Wire tenant identity, session registration, public verifier-key lookup, quota
  preflight, and payload-free audit event contracts.
- Keep route frame handling disabled until encrypted frame routing.
- Keep PWA exposure disabled until the later managed runtime exposure gate.
- Move the next local slice to
  `managed-relay-runtime-encrypted-frame-routing`.

## Non-Goals

- Do not bind a managed relay endpoint.
- Do not route encrypted relay frames yet.
- Do not integrate quota/metering into frame routing yet.
- Do not expose managed relay in the PWA.
- Do not change `live-loopback` as the product default.

## Work Added

- Added `PWA_RELAY_MANAGED_RUNTIME_CONTROL_PLANE_CONTRACT_WIRING` to
  `pwa/app.mjs`.
- Added `createManagedRelayRuntimeControlPlaneContractWiring()` to produce the
  internal control-plane contract with endpoint/PWA exposure still disabled.
- Added `relayManagedRuntimeControlPlaneContractWiring()` to expose startup
  contract, wired contracts, health surface, evidence checks, remaining phases,
  and next local slice.
- Added `npm run check:pwa-relay-managed-runtime-control-plane-contract-wiring`.
- Updated PWA tests, managed relay checks, and next-mode planning so the next
  local slice is `managed-relay-runtime-encrypted-frame-routing`.

## Wired Contracts

The control-plane wiring covers:

- tenant identity
- session registration
- public verifier-key lookup
- quota preflight before registration
- payload-free control-plane audit event

The session registration contract accepts only tenant/session/device/key/source
metadata and returns registration, session, quota, verifier, and audit
decisions. Public endpoints remain disabled.

## Boundary

The wiring keeps:

- `selectedRuntime=deferred`
- `runtimeDefault=not-selected`
- `endpoint_mode=disabled`
- `public_bind_enabled=false`
- `pwa_exposure=disabled`
- `route_runtime=not-wired`
- `control_plane_runtime=tenant-session-registration-contract-wired`
- rollback transport `live-loopback`

The health/control-plane surface remains aggregate-only and payload-free.
Payload JSON, command/context content, approval payloads, payload keys, shared
secrets, private key material, raw session tokens, full setup JSON, HMAC
secrets, and MAC material remain prohibited from actual runtime-visible
surfaces.

## Next Slice

Managed relay runtime encrypted frame routing:

- route only opaque encrypted relay frames;
- enforce the route-visible field allowlist;
- keep control-plane metadata and frame payload boundaries separate;
- keep `selectedRuntime` deferred and PWA exposure disabled;
- preserve live-loopback rollback.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-control-plane-contract-wiring
npm run check:pwa-relay-managed-runtime-service-scaffold
npm run check:pwa-relay-managed-runtime-implementation-plan
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-billing-abuse-boundary-review
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
