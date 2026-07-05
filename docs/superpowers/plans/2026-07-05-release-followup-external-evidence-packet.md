# 2026-07-05 Release Follow-up External Evidence Packet

## Purpose

Prepare a secret-free operator packet for the remaining `v0.3.3` external
release follow-up work. The packet lets a Windows MSI host, GitHub release
signing operator, or F-Droid buildserver operator see exactly what remains and
which evidence must be returned.

## Status

Completed as a local handoff slice. The actual release follow-up remains blocked
until MSI, Android signing secrets, and F-Droid build/buildserver evidence are
ready.

## Scope

- Add `scripts/export-release-followup-evidence-packet.ps1`.
- Add `npm run export:release-followup-evidence-packet`.
- Export a JSON and Markdown packet under
  `artifacts/release-followup-evidence-packet/`.
- Keep secret values out of packet output.
- Link the packet from the release follow-up runbook and troubleshooting docs.

## Non-Goals

- Do not register Android signing secrets from this repository automation.
- Do not create or commit keystores, passwords, APK signing material, or
  `artifacts/` output.
- Do not mark release follow-up docs closed while
  `closeout.canCloseDocs=false` or `closeout.blockedItems` is non-empty.
- Do not change the release tag or existing release assets.

## Work Added

- Added an export script that reads the combined release follow-up preflight
  evidence and status summary.
- The packet records ready items, blocked items, blockers, next actions,
  closeout state, required external commands, docs references, and safety rules.
- The packet records GitHub Android signing secret names only, never secret
  values.

## Verification

```powershell
npm run export:release-followup-evidence-packet
npm run check:release-followup
git diff --check
```

Expected current host state:

- Packet export succeeds.
- `npm run check:release-followup` reports blocked items:
  `msi`, `androidSigningSecrets`, `fdroidBuild`.

## Next Slice

External release follow-up evidence closeout:

- run MSI evidence on a Windows-native Rust/MSVC/WiX host;
- register and verify Android release signing secret names;
- capture F-Droid build/buildserver evidence;
- rerun `npm run check:release-followup`;
- close docs only when `closeout.canCloseDocs=true`.
