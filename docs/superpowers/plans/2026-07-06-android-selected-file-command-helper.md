# 2026-07-06 Android Selected-File Shellcore Helper

## Purpose

Continue local Android/mobile terminal hardening while release follow-up remains
blocked on external MSI, Android signing secrets, and F-Droid build evidence.
This slice gives users one-tap helpers for safe shellcore file listing around
imported workspace files without adding raw file reads or enabling Termux.

## Status

Completed as a local Android slice. External release follow-up blockers remain
unchanged.

## Scope

- Add `List Files` to prepare `ls` in the Android command input.
- Add `Find Last` to prepare a shellcore-safe command for the most recent
  imported file.
- Build the selected-file command from workspace-relative paths.
- Reject outside-workspace current directories and outside-workspace selected
  files.
- Keep helper actions as input preparation only. The user still taps `Run`.

## Non-Goals

- Do not auto-run selected-file helper commands.
- Do not expose app-private absolute paths in input or transcript.
- Do not add raw file read shellcore builtins.
- Do not make Termux/shared staging the Android default.
- Do not close external release follow-up blockers from this host.

## Helper Contract

`Find Last` prepares:

```text
ls <relative-dir> | where name == <file> | first 1
```

The relative directory is computed from the current shell cwd to the imported
file's parent directory. If the current cwd is the workspace root, this becomes
`ls "." | where name == "file" | first 1`. If the user is inside a workspace
subdirectory, this may become `ls ".." | where name == "file" | first 1`. The
command never uses the app-private absolute path.

Raw file reading remains on the existing `Open Last` path, which uses bounded
UTF-8 preview and safe metadata summaries for binary or non-UTF-8 content.

## Verification

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
gradle -p android :app:testDebugUnitTest --tests dev.aiterminal.android.WorkspaceDocumentsTest --tests dev.aiterminal.android.TerminalViewModelTermuxTest
```

Result: green.

## Next Slice

The highest-priority project work is still external release follow-up closeout.
If those blockers are unavailable, continue Android/mobile local terminal
hardening with Termux shared staging diagnostics that keep external commands
disabled until the smoke marker proves both app and helper can read/write the
same directory.
