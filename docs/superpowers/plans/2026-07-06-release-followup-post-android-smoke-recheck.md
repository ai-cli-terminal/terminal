# 2026-07-06 Release Follow-Up Post-Android Smoke Recheck

## Purpose

Recheck release follow-up readiness after the Android real-device smoke capture
closed. The goal is to separate newly completed local Android evidence from the
remaining external release blockers.

## Status

Completed. The release follow-up remains blocked on external evidence only:
Windows MSI build evidence, GitHub Android signing secret names, and F-Droid
build/buildserver evidence.

## Commands Run

The usual `npm run check:release-followup` entrypoint was not available from
this Codex PowerShell PATH because `npm` was not found. The equivalent
PowerShell script was run directly.

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\check-release-followup.ps1
```

The first sandboxed run could not read the GitHub CLI config, so the check was
rerun with filesystem access outside the workspace. The escalated run confirmed
the real secret-name state without reading or recording secret values.

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\export-release-followup-evidence-packet.ps1
```

## Results

- `RELEASE_FOLLOWUP_STATUS_SMOKE_OK`
- `RELEASE_FOLLOWUP_PREFLIGHT_BLOCKED`
- `Release follow-up check: blocked`
- `Can close docs: False`
- Blocked items:
  - `msi`
  - `androidSigningSecrets`
  - `fdroidBuild`
- Refreshed ignored evidence:
  - `artifacts/release-followup-check/release-followup-check-evidence.json`
  - `artifacts/release-followup-preflight/release-followup-preflight-evidence.json`
  - `artifacts/release-followup-evidence-packet/release-followup-evidence-packet.json`
  - `artifacts/release-followup-evidence-packet/release-followup-evidence-packet.md`

## Blocker Details

- MSI: this host is missing Windows-native release packaging requirements:
  `cargo`, `rustc`, `cl`, `link`, `rc`, `wix-or-wix-toolset`, plus explicit
  `RunMsiBuild` evidence.
- Android signing: `.github/workflows/release.yml` references all four required
  secret names, but the repository currently does not expose these names through
  `gh secret list`:
  - `AI_TERMINAL_ANDROID_KEYSTORE_BASE64`
  - `AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD`
  - `AI_TERMINAL_ANDROID_KEY_ALIAS`
  - `AI_TERMINAL_ANDROID_KEY_PASSWORD`
- F-Droid: no build/buildserver evidence path was supplied for
  `dev.aiterminal.android` `0.3.4` / `304`.

## Decisions

- Do not close release follow-up docs while `closeout.blockedItems` is non-empty.
- Do not commit ignored `artifacts/` evidence.
- Do not change release tag or assets without a separate release decision.
- Do not update the existing `v0.3.4` release body from this recheck alone. The
  local `v0.3.4` tag is not an ancestor of the current
  `release/v0.3.4-android-reader` branch, so post-tag Android hardening remains
  tracked as unreleased branch work until the release decision is clarified.

## Next External Actions

1. Run MSI follow-up with `-RunMsiBuild` on a Windows-native Rust/MSVC/WiX host.
2. Register the four `AI_TERMINAL_ANDROID_*` GitHub repository signing secrets.
3. Capture real F-Droid build/buildserver evidence for
   `dev.aiterminal.android` `0.3.4` / `304` and pass its path to the combined
   preflight.
