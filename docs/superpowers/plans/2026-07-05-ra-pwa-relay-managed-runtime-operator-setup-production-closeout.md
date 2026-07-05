# 2026-07-05 RA/PWA Relay Managed Runtime Operator Setup Production Closeout

## Purpose

Close the local managed relay runtime operator setup evidence chain after daemon
bridge evidence. This slice proves the managed operator setup path is complete
as an explicit opt-in path while `live-loopback` remains the product default.

## Status

Completed in this slice. The closeout links the managed operator setup contract,
import preflight, browser evidence, connection controls, session handshake,
approval flow, runbook closeout, approval response delivery boundary, endpoint
delivery evidence, endpoint browser evidence, and daemon bridge evidence.

## Scope

- Add `relayManagedRuntimeOperatorSetupProductionCloseout()`.
- Add `npm run check:pwa-relay-managed-runtime-operator-setup-production-closeout`.
- Update next-mode planning so the next project slice is external release
  follow-up evidence closeout.
- Update the relay runbook, handoff, history, troubleshooting, and remaining
  work priority documents.
- Keep endpoint auto-start disabled, public bind disabled, and `live-loopback`
  as product default.

## Non-Goals

- Do not make managed relay the product default.
- Do not auto-start a managed endpoint.
- Do not enable public bind.
- Do not replace Rust approval validation.
- Do not close external release follow-up blockers without MSI, Android
  signing, and F-Droid build/buildserver evidence.

## Work Added

- Added a production closeout summary contract in `pwa/app.mjs`.
- Added unit coverage for the complete local managed operator setup evidence
  chain.
- Added an npm check script that writes sanitized closeout evidence under
  `artifacts/`.
- Updated `docs/TROUBLESHOOTING.md` with Relay/M2 managed evidence failure
  modes and verification commands.
- Refreshed `docs/superpowers/plans/2026-07-01-remaining-work-priority.md` so
  external release blockers are the next project priority after this local
  Relay/M2 closeout.

## Boundary Contract

Production closeout is ready only when:

- daemon bridge evidence is complete;
- all local managed operator setup evidence names are present in the closeout
  chain;
- the runbook lists the required evidence commands;
- manual signed-response copy remains available as fallback;
- managed relay remains explicit opt-in;
- endpoint auto-start is disabled;
- public bind is off;
- route envelopes, payload keys, ciphertext, raw tokens, approval response
  payload material, and private key material stay out of closeout evidence.

## Next Slice

Release follow-up external evidence closeout:

- run MSI packaging evidence on a Windows native Rust/MSVC/WiX host;
- register and verify Android release signing secret names;
- capture F-Droid build or buildserver evidence for app id/version/result and
  artifact markers;
- close release follow-up docs only when `npm run check:release-followup`
  reports no blocked items and `closeout.canCloseDocs=true`.

## Verification

```powershell
npm run test:pwa
npm run check:pwa-relay-managed-runtime-operator-setup-production-closeout
npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-daemon-bridge-evidence
npm run check:pwa-relay-next-mode-planning
npm run check:pwa-relay-deployment-runbook
npm run check:release-followup
git diff --check
```
