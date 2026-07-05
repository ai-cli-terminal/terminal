# 2026-07-05 Android Imported Document Reader Metadata

## Purpose

Resume the local Android/mobile terminal track after the Relay/M2 local closeout
and release follow-up external handoff. This slice improves the imported
workspace document reader without changing the Android MVP promise:
`shellcore-only` remains the default, and Termux external commands remain
explicit opt-in through shared staging.

## Status

Completed as a local Android slice. Release follow-up remains blocked on
external MSI, Android signing secrets, and F-Droid build/buildserver evidence.

## Scope

- Extend imported/opened document results with content kind, byte count, preview
  bytes read, and preview line count.
- Keep previews bounded and UTF-8 only.
- Reopen binary or non-UTF-8 imported files as safe metadata summaries instead
  of rendering raw bytes in transcript.
- Keep workspace canonicalization and outside-workspace rejection intact.
- Update Android docs, troubleshooting, history, handoff, and remaining-work
  priority.

## Non-Goals

- Do not add arbitrary file editing.
- Do not render binary content in transcript.
- Do not expose app-private workspace paths through Termux automatically.
- Do not make Termux/shared staging the Android default.
- Do not close release follow-up external blockers from this development host.

## Reader Contract

The Android document reader now treats an imported workspace document as one of
two transcript-safe kinds.

| Kind | Transcript behavior | Boundary |
|---|---|---|
| `Text` | Show bounded UTF-8 preview plus bytes/lines read metadata | Preview is capped by byte and line limits |
| `BinaryOrUnsupported` | Show file name, byte count, and preview-unavailable summary | Raw bytes are not rendered |

`Open Last` still canonicalizes the file path under the app-private workspace.
If a path escapes the workspace root, it remains a hard failure.

## Verification

```powershell
gradle -p android :app:testDebugUnitTest --tests dev.aiterminal.android.WorkspaceDocumentsTest
git diff --check
```

Expected release follow-up status on this host remains:

```powershell
npm run check:release-followup
```

`msi`, `androidSigningSecrets`, and `fdroidBuild` stay blocked until external
operators return evidence.

## Next Slice

The highest-priority project work is still external release follow-up closeout.
If those external blockers are unavailable, the next local Android slice should
continue mobile terminal UX hardening:

- SAF import/export affordances around app-private workspace files;
- selected-file command helpers for shellcore-safe read/list workflows;
- Termux shared staging diagnostics that keep external commands disabled until
  the smoke marker proves both app and helper can read/write the same directory.
