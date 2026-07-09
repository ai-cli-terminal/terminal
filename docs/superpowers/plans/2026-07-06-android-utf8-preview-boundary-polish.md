# Android UTF-8 Preview Boundary Polish

## Context

Android workspace import/open now shows bounded text previews and safe metadata
summaries for binary or non-UTF-8 files. The preview reader currently decodes a
fixed byte prefix with strict UTF-8. If the byte limit lands in the middle of a
multi-byte character, valid text can be misclassified as binary/non-UTF-8.

This is a local Android polish slice after real-device import/open/export/helper
smoke passed. It does not change release follow-up blockers: Windows MSI, real
Android signing secrets, and F-Droid build/buildserver evidence remain external.

## Goal

Keep transcript-safe previews while treating UTF-8 boundary cuts as truncation,
not as binary content.

## Scope

- Decode the largest valid UTF-8 prefix when the preview byte limit cuts a
  trailing multi-byte sequence.
- Continue rejecting NUL bytes and invalid UTF-8 that appears before the
  trailing boundary.
- Preserve existing binary/non-UTF-8 metadata summary behavior.
- Add focused JVM tests for boundary-split UTF-8 and invalid UTF-8 before the
  boundary.

## Verification

- `gradle -p android :app:testDebugUnitTest --tests dev.aiterminal.android.WorkspaceDocumentsTest`
- `git diff --check`
