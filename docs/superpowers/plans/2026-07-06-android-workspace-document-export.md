# 2026-07-06 Android Workspace Document Export

## Purpose

Continue the local Android/mobile terminal track while release follow-up remains
blocked on external MSI, Android signing secrets, and F-Droid build evidence.
This slice closes the SAF affordance gap around imported app-private workspace
files without changing the Android MVP boundary: `shellcore-only` remains the
default and Termux external commands remain explicit opt-in through shared
staging.

## Status

Completed as a local Android slice. External release follow-up blockers remain
unchanged.

## Scope

- Add `Export Last` for the most recent imported workspace document.
- Copy the app-private workspace file to a user-selected SAF document
  destination.
- Keep transcript export separate as `Export Log`.
- Reuse canonical workspace checks before export.
- Reject outside-workspace paths and directories before opening the SAF output.

## Non-Goals

- Do not expose app-private workspace paths through Termux automatically.
- Do not make shared staging the Android default.
- Do not add arbitrary file editing.
- Do not close external release follow-up blockers from this host.

## Export Contract

`Export Last` only exports the last document imported into the app-private
workspace during the current view-model session. Before export, the source path
is canonicalized and must remain below the workspace root. The destination is a
SAF document URI selected by the user, and the app copies bytes without rendering
binary payloads into the transcript.

| Case | Behavior |
|---|---|
| Last imported file inside workspace | Copy bytes to the selected SAF destination |
| No imported file | Transcript error: no imported document |
| Source outside workspace | Fail before destination write |
| Source is a directory | Fail before destination write |

## Verification

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
gradle -p android :app:testDebugUnitTest --tests dev.aiterminal.android.WorkspaceDocumentsTest
```

Result: green.

## Next Slice

The highest-priority project work is still external release follow-up closeout.
If those blockers are unavailable, continue Android/mobile local terminal
hardening with one of:

- selected-file command helpers for shellcore-safe read/list workflows;
- Termux shared staging diagnostics that keep external commands disabled until
  the smoke marker proves both app and helper can read/write the same directory.
