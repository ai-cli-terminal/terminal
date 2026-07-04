# 2026-07-04 RA/PWA Relay Managed Runtime Implementation Plan

## Purpose

Define the managed Relay/M2 runtime implementation boundary before adding any
managed service scaffold or PWA exposure.

## Status

Completed in this slice as a planning and regression gate. The managed runtime
readiness gate is green and `implementationCanStart=true`, but
`selectedRuntime` remains `deferred`, `runtimeDefault` remains `not-selected`,
and the product default remains `live-loopback`.

## Scope

- Add a managed runtime implementation plan summary.
- Carry forward the green runtime readiness gate result.
- Define the managed service boundary, implementation phases, exposure gates,
  and regression checks.
- Move the next local slice to `managed-relay-runtime-service-scaffold`.
- Keep PWA managed runtime exposure deferred until a later scaffold and
  exposure-gate slice explicitly changes it.

## Non-Goals

- Do not implement the managed relay runtime service in this slice.
- Do not expose managed relay in the PWA.
- Do not change the product default away from `live-loopback`.
- Do not allow managed relay visibility into plaintext payloads, command text,
  context data, approval payloads, private key material, raw session tokens, or
  setup payloads.

## Work Added

- Added `PWA_RELAY_MANAGED_RUNTIME_IMPLEMENTATION_PLAN` to `pwa/app.mjs`.
- Added `relayManagedRuntimeImplementationPlan()` to expose the service
  boundary, implementation phases, exposure gates, regression checks, and next
  local slice.
- Added `npm run check:pwa-relay-managed-runtime-implementation-plan`.
- Updated managed runtime readiness, billing/abuse, and next-mode planning
  checks so the next local slice is `managed-relay-runtime-service-scaffold`.
- Updated PWA tests to assert the plan keeps managed runtime deferred while
  allowing scaffold implementation to start.

## Service Boundary

The plan keeps route-visible managed relay fields limited to routing metadata:

- `relay_protocol_version`
- `session_id`
- `sender`
- `sequence`
- `sent_at_ms`
- `expires_at_ms`
- `payload_ciphertext_alg`
- `payload_key_scope`
- `payload_ciphertext_bytes`

The managed runtime remains barred from plaintext payloads, command/context
content, approval payloads, payload keys, shared secrets, private key material,
raw session tokens, full setup JSON, HMAC secrets, and MAC material.

## Implementation Phases

1. `managed-runtime-service-scaffold`
2. `managed-runtime-control-plane-contract-wiring`
3. `managed-runtime-encrypted-frame-routing`
4. `managed-runtime-quota-and-metering-integration`
5. `managed-runtime-support-and-abuse-operations-integration`
6. `managed-runtime-pwa-exposure-gate`

## Exposure Gates

- `readiness_gate_green`
- `managed_runtime_contract_check_passed`
- `payload_blind_frame_routing_smoke_passed`
- `quota_metering_integration_smoke_passed`
- `support_billing_abuse_regression_passed`
- `pwa_copy_and_setup_text_updated`
- `live_loopback_rollback_documented`

## Next Slice

Managed relay runtime service scaffold:

- add the service scaffold without PWA exposure;
- keep `selectedRuntime` deferred;
- preserve payload-blind frame routing, public verifier-key lookup,
  quota/metering, support redaction, and billing/abuse boundaries;
- document rollback to `live-loopback`.

## Verification

```powershell
npm run check:pwa-relay-managed-runtime-implementation-plan
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-billing-abuse-boundary-review
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
