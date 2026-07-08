# 2026-07-06 Android Termux Shared Staging Diagnostics

## Purpose

Continue local Android/mobile terminal hardening while release follow-up remains
blocked on external MSI, Android signing secrets, and F-Droid build evidence.
This slice makes the Termux shared staging gate easier to diagnose without
loosening the product boundary: external commands remain disabled until both
the app-side staging probe and helper marker smoke pass.

## Status

Completed as a local Android slice. External release follow-up blockers remain
unchanged.

## Scope

- Split `Verify` transcript diagnostics into `app-write` and `helper-marker`.
- Prove app-side shared staging read/write by writing, reading, and deleting a
  probe file before helper smoke starts.
- Keep helper smoke fail-closed unless `ASH_SHARED_STAGING_OK` is observed.
- Report a specific marker-missing error when helper execution finishes without
  the expected marker.
- Keep dynamic external adapter attach gated behind both diagnostics.

## Non-Goals

- Do not enable external commands from T0 probe or helper self-test alone.
- Do not make Termux/shared staging the Android default.
- Do not add a new execution backend.
- Do not close external release follow-up blockers from this host.

## Diagnostics Contract

`Verify` now emits:

```text
termux staging app-write: ok <staging-name>
> termux shared staging smoke
termux staging helper-marker: ok
termux staging: ok
```

Failure modes stay fail-closed:

| Failure | User-visible result |
|---|---|
| App cannot write/read staging root | `termux staging app-write: fail ...` |
| Helper exits without marker | `Termux shared staging marker missing` |
| Helper stderr mentions permission denial | `Termux storage permission required for shared staging` |
| Smoke cancelled | `termux staging helper-marker: cancelled` |

`ShellWorker.externalCommandsEnabled` is set to `true` only after app-write is
ok, helper marker is observed, and the helper result is ok.

## Verification

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
gradle -p android :app:testDebugUnitTest --tests dev.aiterminal.android.TerminalViewModelTermuxTest
```

Result: green.

## Next Slice

The highest-priority project work is still external release follow-up closeout.
If those blockers are unavailable, run Android real-device smoke capture for
the local mobile track:

- import/open/export imported workspace document;
- selected-file `List Files` / `Find Last` helpers;
- Termux shared staging diagnostics success and common failure copy.
