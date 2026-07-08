# 2026-07-06 iOS/iPadOS Local Terminal Research Boundary

## Context

PR #64 (`v0.3.4 Android reader follow-up hardening`) landed on `main` at
`60ac71c`. The remaining release closeout blockers are still external:
Windows MSI evidence, real Android signing secrets, and F-Droid
build/buildserver evidence. Because those cannot be closed on this Windows
host, the next local track is PM-4 iOS/iPadOS research.

This document narrows PM-4 before any iOS code spike. It is intentionally a
policy/product boundary, not a TestFlight implementation.

## Current Apple Policy Check

Official Apple sources checked on 2026-07-06:

- App Review Guidelines 2.2: beta apps belong in TestFlight and external
  TestFlight builds go through TestFlight App Review.
  https://developer.apple.com/app-store/review/guidelines/
- App Review Guidelines 2.5.2: iOS/iPadOS apps must be self-contained,
  operate inside designated containers, and must not download/install/execute
  code that introduces or changes app functionality.
  https://developer.apple.com/app-store/review/guidelines/#software-requirements
- App Review Guidelines 2.5.4: background services are limited to their stated
  platform purposes.
  https://developer.apple.com/app-store/review/guidelines/#software-requirements
- App Review Guidelines 2.5.15: apps that let users view/select files should
  include Files app and iCloud document locations.
  https://developer.apple.com/app-store/review/guidelines/#software-requirements
- TestFlight: external tester distribution requires beta app review
  information and the first external build must be approved by App Review.
  https://developer.apple.com/testflight/

## Decision

iOS/iPadOS remains a constrained local structured terminal research target.
The product promise is:

> AI Terminal on iOS/iPadOS is a local structured `ash` workspace for safe,
> self-contained shellcore evaluation and user-selected files.

The product promise is not:

> AI Terminal on iOS/iPadOS is a Linux terminal, a package manager, a remote
> code runner, or a Termux-equivalent userland.

## Allowed MVP Surface

- A self-contained Rust `shellcore` embedded in the app bundle.
- A SwiftUI terminal screen that evaluates pure/builtin `shellcore` commands.
- An app-private workspace root.
- Explicit import/export through iOS document picker / Files-compatible
  surfaces.
- Transcript-safe text previews with bounded byte/line limits.
- Metadata summaries for binary or unsupported files.
- User-visible copy that says "constrained local structured terminal" or
  equivalent plain language.

## Command Subset Boundary

Allowed in the first TestFlight spike:

- Literals, records, lists, tables, and pipelines.
- Pure transforms such as `where`, `first`, `length`, and structured field
  access.
- `print` and deterministic shellcore builtins that do not spawn a process.
- `pwd`, `cd`, and `ls` only through an iOS workspace adapter rooted in the
  app container or user-selected document location.
- Import/export actions implemented as app UI flows, not as external shell
  commands.

Excluded from the product promise:

- `sh`, `bash`, `zsh`, Python, Node.js, Git, curl, ssh, package managers, or
  any bundled/downloaded general userland unless a later policy review creates
  a narrower approved model.
- Downloaded executable code that introduces or changes app functionality.
- Arbitrary subprocess spawning, PTY emulation, background daemons, listeners,
  or long-running background command services.
- A claim that the iOS app provides a complete Linux distribution or complete
  desktop terminal.

## Workspace Model

The first iOS workspace model is:

1. App-private container workspace by default.
2. Explicit document import into the app workspace.
3. Explicit document export back to a user-selected destination.
4. Security-scoped bookmarks only if a later spike proves they are necessary
   and can be explained without implying broad filesystem access.

The app must not present app-private paths as if they were system paths, and it
must not imply access to arbitrary directories.

## RA/PWA Coupling

Remote approval device identity stays separate from the iOS terminal body until
Android and iOS local terminal tracks have their own stable runtime contracts.
PM-4 does not use PWA identity as the mobile terminal identity.

## TestFlight Spike Scope

The next code slice is a TestFlight-oriented self-contained REPL spike:

- Create or select an iOS project scaffold on a macOS/Xcode host.
- Add one SwiftUI terminal screen with input, transcript, cwd/workspace status,
  and import/export affordances.
- Bind to Rust `shellcore` when the iOS FFI path is selected; a fake local
  shellcore adapter is acceptable only for the first UI skeleton commit.
- Prove that unknown/external commands fail closed before any host process or
  downloaded code path exists.
- Add unit or UI evidence for the command subset, workspace containment, import
  preview, and export flow.
- Prepare TestFlight beta review notes that name the constrained feature set.

This Windows host does not contain an iOS/Xcode project, so it can close the
policy/research boundary but cannot produce TestFlight build evidence.

## Completion Criteria For PM-4 Research Boundary

- `docs/TASK.md` marks the policy boundary items complete while leaving the
  self-contained TestFlight REPL spike open.
- The remaining-work priority doc selects the TestFlight REPL spike as the next
  local implementation candidate when external release blockers remain
  unavailable.
- Handoff/history/troubleshooting docs state the non-Linux, self-contained,
  container/document-picker boundary.
- `git diff --check` passes.
