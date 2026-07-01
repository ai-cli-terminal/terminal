# RA/PWA Live E2E Evidence — 2026-07-01

> P4 evidence plan for proving the live companion path beyond unit and bridge
> tests. This document separates repeatable automated evidence from the remaining
> operator/browser evidence.

## Problem

P0 through P3 prove the pieces:

- the daemon has a live loopback endpoint;
- the endpoint bridges pending gate approvals into `/events` and `/message`;
- the PWA can connect, render pending approvals, sign decisions, and POST them
  back.

The remaining product question is whether a full local operator workflow has
repeatable evidence:

```text
ai remote daemon
  -> PWA connects to printed live endpoint
  -> ai remote arm --allow-high
  -> High-risk command creates a pending approval
  -> PWA approve/reject resumes the gate path
  -> allow/block result is recorded
```

## Evidence Layers

### P4a — Repeatable harness evidence

This layer is scriptable on the current repo without opening a browser. It should
write JSON evidence under `artifacts/ra-pwa-live-evidence/`.

Acceptance:

- [x] Run `node pwa/app.test.mjs` and record output.
- [x] Run targeted Rust remote live tests for endpoint + approval bridge and
      record output.
- [x] Verify the static PWA exposes the live endpoint controls used by the
      operator flow.
- [x] Write a JSON evidence file with command status, timestamps, and paths.

Evidence:

- Script: `scripts/smoke-pwa-live-approval.ps1`
- Result: `RA_PWA_LIVE_EVIDENCE_OK`
- File: `artifacts/ra-pwa-live-evidence/ra-pwa-live-evidence.json`

### P4b — Browser/operator evidence

This layer proves the actual first-class workflow and can be manual or automated
with a browser controller.

Acceptance:

- [ ] Start `ai remote daemon --device-id <id>` with isolated
      `XDG_CONFIG_HOME`.
- [ ] Open the PWA, generate or restore the matching companion identity, and
      connect to the printed endpoint.
- [ ] Run `ai remote arm --allow-high`.
- [ ] Trigger a High-risk command and approve it from the PWA.
- [ ] Trigger a High-risk command and reject it from the PWA.
- [ ] Record transcript, browser screenshot, and resulting allow/block evidence.

## Implementation Choice

Start with P4a. It is not a replacement for browser/operator evidence, but it
prevents regressions in the live transport and gives the next session one command
to run before attempting browser automation.

## Validation

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-approval.ps1
git diff --check
```

Expected result:

```text
RA_PWA_LIVE_EVIDENCE_OK artifacts\ra-pwa-live-evidence\ra-pwa-live-evidence.json
```
