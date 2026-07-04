# 2026-07-04 RA/PWA Relay Managed Billing Abuse Boundary Review

## Purpose

Prove managed Relay/M2 billing usage meters and abuse signals stay separately
reviewed, and that support evidence cannot become a billing source.

## Status

Completed in this slice as runtime readiness evidence. The managed runtime
readiness gate is green and `implementationCanStart=true`, while
`selectedRuntime` remains `deferred` and the product default remains
`live-loopback`.

## Scope

- Add a billing/abuse boundary review summary for managed relay readiness.
- Add a helper that normalizes billing usage and abuse signal summaries while
  rejecting cross-classified fields.
- Prove support redaction/access evidence is not used as a billing source.
- Prove tenant aggregate usage export remains aggregate-only and payload-free.
- Close `billing-abuse-boundary-review`,
  `billing_abuse_boundary_review_missing`,
  `runtime_rate_limit_enforcement_missing`,
  `abuse_escalation_runbook_missing`, and
  `tenant_deletion_workflow_missing`.
- Keep `live-loopback` as the product default.
- Keep managed relay selected runtime deferred until the runtime implementation
  plan explicitly changes exposure.

## Non-Goals

- Do not implement the managed relay runtime service in this slice.
- Do not expose managed relay in the PWA.
- Do not change the product default away from `live-loopback`.
- Do not treat support access evidence, abuse case metadata, or tenant deletion
  workflow metadata as billing meters.

## Work Added

- Added `PWA_RELAY_MANAGED_BILLING_ABUSE_BOUNDARY_REVIEW` to `pwa/app.mjs`.
- Added `createManagedRelayBillingAbuseBoundaryReview()` to build a
  payload-free boundary review from billing usage, abuse signals, tenant usage
  export, and redacted support view inputs.
- Added `relayManagedBillingAbuseBoundaryReview()` to expose the readiness
  contract, guardrails, evidence checks, and next local slice.
- Added `npm run check:pwa-relay-managed-billing-abuse-boundary-review`.
- Updated PWA tests for billing/abuse field separation, support-evidence
  exclusion from billing, tenant aggregate export boundaries, runtime gate
  completion, and next-slice pointers.
- Updated `relayManagedRuntimeReadinessGate()` so all managed runtime evidence
  is complete, remaining evidence/blockers are empty, and the next local slice
  is `managed-relay-runtime-implementation-plan`.

## Boundary Contract

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

Review decisions:

- billing usage is control-plane metering;
- abuse signals are case-review inputs, not billing meters;
- support evidence is not a billing source;
- tenant usage export is aggregate-only;
- tenant deletion workflow is reviewed;
- abuse escalation runbook is reviewed.

## Evidence

The check and PWA tests prove:

- billing usage and abuse signals remain separate sections;
- billing usage rejects support and abuse-only fields;
- abuse signals reject billing-only fields;
- support evidence remains aggregate-only, redacted, and payload-free;
- tenant aggregate usage export remains aggregate-only and payload-free;
- runtime gate records `billing-abuse-boundary-review`;
- managed runtime readiness gate is green.

## Gate Result

`relayManagedRuntimeReadinessGate()` now reports:

- `gateStatus=runtime-evidence-green`
- `implementationDecision=managed-runtime-implementation-can-start`
- `implementationCanStart=true`
- `readinessDecision=ready-for-managed-runtime-implementation`
- `remainingRuntimeEvidence=[]`
- `remainingRuntimeBlockers=[]`

## Next Slice

Managed relay runtime implementation plan:

- plan the managed runtime service boundary without changing the product
  default;
- keep `selectedRuntime` deferred until the implementation plan explicitly
  changes exposure;
- preserve payload-blind, public-verifier, quota, support, and billing/abuse
  boundaries.

## Verification

```powershell
npm run check:pwa-relay-managed-billing-abuse-boundary-review
npm run check:pwa-relay-managed-runtime-readiness-gate
npm run check:pwa-relay-managed-support-redaction-access-review-evidence
npm run check:pwa-relay-managed-tenant-aggregate-usage-export-smoke
npm run check:pwa-relay-next-mode-planning
npm run test:pwa
git diff --check
```
