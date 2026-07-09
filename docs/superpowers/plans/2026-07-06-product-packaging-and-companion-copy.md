# 2026-07-06 Product Packaging And Companion Copy

## Purpose

After the non-iPhone priority reset, the next local work is documentation-only
PM-5 plus PM-6 cleanup that can be completed on this host:

- Clarify the `ai`, `ash`, and `ai-terminal.exe` product roles.
- Split README platform support into current distribution state and target
  matrix.
- Add a migration note for the older `document/` v3.3 Linux-terminal framing
  versus the current `terminal/` independent `ash` pivot.
- Lock user-facing Mobile/PWA copy and the RA device identity boundary.

## Scope

In scope:

- README product-role and platform-support edits.
- A product packaging/migration reference doc.
- `docs/TASK.md`, `docs/HISTORY.md`, `docs/HANDOFF.md`, and remaining-work
  priority updates that mark PM-5/PM-6 copy cleanup complete.

Out of scope:

- iPhone/iOS implementation or TestFlight evidence.
- Release follow-up external evidence for MSI, Android signing secrets, or
  F-Droid build/buildserver.
- Binary/package changes.

## Decisions

| Topic | Decision |
|---|---|
| `ai-terminal.exe` | Windows GUI product surface. Double-clickable app that hosts bundled `ash.exe` through PTY/ConPTY. |
| `ash` | Independent structured shell runtime and long-term local-terminal core across desktop and mobile tracks. |
| `ai` | Compatibility CLI/helper for diagnostics, policy, install, and existing wrapper workflows. It remains shipped with `ash`. |
| Versioning | Public artifacts share the repo release version from `VERSION` / `Cargo.toml`; per-platform readiness is described in release notes/docs instead of using separate product versions. |
| Mobile ash app | Local terminal product body. Android is the active mobile track; iOS stays TODO/deferred until a macOS/Xcode host exists. |
| PWA companion | Approve/pair/monitor/demo surface only. It is not a mobile terminal substitute. |
| RA identity | RA device identity stays separate from mobile terminal body identity until mobile terminal runtimes have stable contracts. |

## Completion Criteria

- README has separate "current distribution" and "target matrix" platform
  tables.
- README links to the migration/reference doc.
- The migration/reference doc states the role/version policy and Mobile/PWA
  identity split.
- TASK PM-5 and the two PM-6 copy/identity checkboxes are marked complete.
- HISTORY/HANDOFF/remaining-work priority no longer present PM-5/PM-6 copy
  cleanup as pending local work.

## Verification

```powershell
rg -n "현재 배포|목표 매트릭스|Mobile ash app|PWA companion|RA device identity|v3.3" README.md docs
git diff --check
```
