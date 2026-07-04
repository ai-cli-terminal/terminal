# 2026-07-04 RA/PWA Relay Managed Runtime Support and Abuse Operations Integration

## Purpose

Wire managed Relay/M2 runtime support and abuse operation contracts after quota
and metering, without opening a public endpoint, exposing managed relay in the
PWA, or changing the product default.

## Status

Completed in this slice as support and abuse operations integration evidence.
The managed runtime remains implementation-ready, but `selectedRuntime` remains
`deferred`, `runtimeDefault` remains `not-selected`, endpoint mode remains
`disabled`, public bind remains off, PWA exposure remains disabled, and the
product default remains `live-loopback`.

## Scope

- Add a support/abuse operations integration helper and summary for the managed
  runtime.
- Wire support redaction/access review into runtime-visible support operation
  contracts.
- Wire billing/abuse boundary review into runtime-visible abuse operation
  contracts.
- Keep support views aggregate-only, redacted, tenant-admin-approved, and
  time-bounded.
- Keep abuse operation counters separate from billing source data.
- Keep payload ciphertext hex, nonce hex, payload keys, plaintext payloads,
  command text, context data, approval payloads, raw session tokens, signed
  tickets, and raw device/support identifiers out of operations surfaces.
- Move the next local slice to `managed-relay-runtime-pwa-exposure-gate`.

## Non-Goals

- Do not bind a managed relay endpoint.
- Do not expose managed relay in the PWA.
- Do not change `live-loopback` as the product default.
- Do not turn managed relay into the selected runtime.
- Do not add PWA copy/setup text changes yet.

## Work Added

- Added `PWA_RELAY_MANAGED_RUNTIME_SUPPORT_AND_ABUSE_OPERATIONS_INTEGRATION` to
  `pwa/app.mjs`.
- Added `createManagedRelayRuntimeSupportAndAbuseOperationsIntegration()` to
  produce the runtime support/abuse operation contract with endpoint/PWA exposure
  still disabled.
- Added `relayManagedRuntimeSupportAndAbuseOperationsIntegration()` to expose
  startup contract, support operations contract, abuse operations contract,
  health surface, evidence checks, remaining phases, and next local slice.
- Added
  `npm run check:pwa-relay-managed-runtime-support-and-abuse-operations-integration`.
- Updated managed relay next-slice planning from
  `managed-relay-runtime-support-and-abuse-operations-integration` to
  `managed-relay-runtime-pwa-exposure-gate`.
- Updated PWA tests and managed relay checks to assert the support/abuse runtime
  operation boundary.

## Operations Boundary

The support/abuse operations boundary keeps:

- `selectedRuntime=deferred`
- `runtimeDefault=not-selected`
- `endpoint_mode=disabled`
- `public_bind_enabled=false`
- `pwa_exposure=disabled`
- `control_plane_runtime=tenant-session-registration-contract-wired`
- `route_runtime=encrypted-frame-routing-wired`
- `quota_runtime=active-session-frame-byte-metering-wired`
- `support_runtime=support-redaction-access-review-wired`
- `abuse_runtime=billing-abuse-boundary-review-wired`
- rollback transport `live-loopback`

Support views are available only as aggregate, redacted support surfaces with
hashed identifiers and audited access windows. Abuse operations consume counters
such as rate-limit denials, invalid tickets, and abuse case counts, but these
counters are not billing source data and cannot be reclassified into billing
meters.

## Next Slice

Managed relay runtime PWA exposure gate:

- add the final exposure gate summary and check;
- keep managed runtime opt-in only;
- update PWA setup/copy text only after the exposure gate passes;
- preserve live-loopback rollback and product default;
- continue to block public bind unless a separate deployment decision changes it.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-support-and-abuse-operations-integration
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
