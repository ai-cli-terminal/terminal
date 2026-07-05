# 2026-07-04 RA/PWA Relay Managed Tenant Aggregate Usage Export Smoke

## Purpose

Prove the managed Relay/M2 tenant usage export is aggregate-only and excludes
payloads, secrets, raw tickets, and command/context data before any managed
runtime implementation starts.

## Status

Completed in this slice as a runtime smoke. Managed relay runtime remains deferred; the later billing/abuse boundary review has made the readiness gate green.

## Scope

- Add a repeatable managed tenant aggregate usage export smoke check.
- Add a tenant usage export helper that normalizes billing usage counters and
  abuse signal summaries into separate sections.
- Export session registration, active session, relay frame, relay byte, invalid
  ticket, and quota denial billing counters.
- Export rate-limit denial, invalid ticket, and abuse case signal summaries
  separately from billing usage.
- Prove support visibility remains aggregate-only.
- Prove payloads, command text, context data, raw session tokens, signed
  tickets, HMAC secrets, and MAC material are excluded.
- Close `tenant-aggregate-usage-export-smoke` and
  `tenant_usage_export_smoke_missing` at smoke level.
- Keep `live-loopback` as the product default.

## Non-Goals

- Do not implement a managed relay runtime service.
- Do not expose managed relay in the PWA.
- Do not implement support redaction/access review in this slice.
- Do not close billing/abuse boundary review evidence.
- Do not claim managed relay production readiness.

## Work Added

- Added `PWA_RELAY_MANAGED_TENANT_AGGREGATE_USAGE_EXPORT_SMOKE` to
  `pwa/app.mjs`.
- Added `createManagedRelayTenantAggregateUsageExport()` to normalize a
  tenant-scoped usage export with payload-free billing and abuse sections.
- Added `relayManagedTenantAggregateUsageExportSmoke()` to expose the smoke
  contract and remaining runtime evidence.
- Added `npm run check:pwa-relay-managed-tenant-aggregate-usage-export-smoke`.
- Updated PWA tests for export fields, billing/abuse separation, aggregate-only
  support visibility, payload/secret exclusion, runtime gate completion, and
  next-slice pointers.
- Updated `relayManagedRuntimeReadinessGate()` so
  `tenant-aggregate-usage-export-smoke` is completed and
  `tenant_usage_export_smoke_missing` is resolved at smoke level.
- Updated managed relay next-mode pointers to
  `managed-relay-runtime-implementation-plan`.

## Export Boundary

Input fields:

- `tenant_id`
- `window_start_ms`
- `window_end_ms`
- `generated_at_ms`
- `plan_id`
- `billing_meter`
- `abuse_signals`

Billing usage fields:

- `session_registration_count`
- `active_session_count`
- `relay_frame_count`
- `relay_byte_count`
- `invalid_ticket_count`
- `quota_denial_count`

Abuse signal fields:

- `rate_limit_denial_count`
- `invalid_ticket_count`
- `abuse_case_count`

Output fields:

- `export_version`
- `export_scope`
- `tenant_id`
- `plan_id`
- `window_start_ms`
- `window_end_ms`
- `generated_at_ms`
- `payload_visibility`
- `support_visibility`
- `billing_usage`
- `abuse_signal_summary`
- `billing_abuse_boundary`

The export must not include payload JSON, command text, context JSON, approval
payloads, private key material, raw session tokens, signed session tickets,
full setup JSON, HMAC secrets, or signed ticket MAC material.

## Evidence

The smoke check and PWA tests prove:

- tenant aggregate usage export includes session registration, active session,
  relay frame, relay byte, invalid ticket, and quota denial counters;
- tenant aggregate usage export excludes payloads and secrets;
- billing usage and abuse signals are exported in separate sections;
- support visibility is aggregate-only;
- runtime gate records tenant usage export evidence;
- managed runtime remains deferred.

## Remaining Runtime Evidence

No managed runtime readiness evidence remains after the billing/abuse boundary review.

## Next Slice

Managed relay runtime implementation plan:

- plan the managed runtime service boundary without changing the product default;
- keep `selectedRuntime` deferred until the implementation plan explicitly changes exposure;
- preserve payload-blind, public-verifier, quota, support, and billing/abuse boundaries.

## Verification

```powershell
npm run check:pwa-relay-managed-tenant-aggregate-usage-export-smoke
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-active-session-and-byte-quota-smoke
npm run check:pwa-relay-managed-billing-quota-policy
npm run check:pwa-relay-managed-metadata-minimization-review
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
