# How to Finish the Release Follow-Ups

This runbook closes the remaining `v0.3.3` follow-up work: Windows MSI evidence,
real Android release signing, and F-Droid build/buildserver evidence.

## Prerequisites

- A clean checkout of `ai-cli-terminal/terminal`.
- GitHub CLI authenticated with repository admin access for secret registration.
- Windows MSI work: a Windows-native host with Rust MSVC, Visual Studio Build
  Tools, Windows SDK resource compiler, Node/npm, and WiX or WiX Toolset.
- Android signing work: the real release keystore and passwords, handled outside
  the repository.
- F-Droid work: fdroidserver available in a working fdroiddata checkout or
  buildserver environment.

Never commit keystores, password files, decoded secrets, APK signing material,
or `artifacts/` output.

## Step 1: Check Current Readiness

Run the combined preflight from the repository root.

```powershell
npm run smoke:release-followup-preflight
```

Expected current result on the usual development host:

```text
RELEASE_FOLLOWUP_PREFLIGHT_BLOCKED artifacts\release-followup-preflight\release-followup-preflight-evidence.json
```

Read the evidence file and confirm which blockers remain.

```powershell
$json = Get-Content artifacts\release-followup-preflight\release-followup-preflight-evidence.json -Raw | ConvertFrom-Json
$json.blockers
```

## Step 2: Produce Windows MSI Evidence

Move to a Windows-native packaging host. This is not the WSL/Linux cross-host
used for the existing NSIS evidence.

Install or verify:

- Rust with the `x86_64-pc-windows-msvc` host or target.
- Visual Studio Build Tools with MSVC, `cl`, `link`, and Windows SDK `rc`.
- Node.js and npm.
- WiX CLI or WiX Toolset commands.

Run:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1 -RunMsiBuild
```

Completion evidence:

- The command prints `RELEASE_FOLLOWUP_PREFLIGHT_READY` only if Android/F-Droid
  blockers are also satisfied.
- The nested MSI evidence is always written under
  `artifacts\release-followup-preflight\msi-preflight-evidence.json`.
- For MSI completion, that nested evidence must show `status: ready`, a generated
  `.msi` path, and a SHA256 hash.

If MSI remains blocked, use the nested `missing` list. The common blocker on the
current host is missing `cargo`, `rustc`, `cl`, `link`, `rc`, and
`wix-or-wix-toolset` in the Windows-native environment.

## Step 3: Register Android Signing Secrets

Register the four repository secrets expected by `.github/workflows/release.yml`.

```powershell
gh secret set AI_TERMINAL_ANDROID_KEYSTORE_BASE64
gh secret set AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD
gh secret set AI_TERMINAL_ANDROID_KEY_ALIAS
gh secret set AI_TERMINAL_ANDROID_KEY_PASSWORD
```

Use interactive input or stdin from a secure local secret source. Do not put
secret values in shell history, docs, screenshots, or evidence files.

Verify names only:

```powershell
gh secret list --json name,updatedAt
npm run smoke:release-followup-preflight
```

Completion evidence:

- `androidSigningSecrets.status` is `ready`.
- `androidSigningSecrets.present` contains all four secret names.
- The preflight evidence does not include secret values.

To verify the Gradle signing wiring without real secrets, run the existing
throwaway path:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\android\smoke-github-signing-secrets.ps1 -UseThrowawayKeystore
```

That throwaway smoke proves decode/signing mechanics only. It does not close the
real release signing follow-up.

## Step 4: Produce F-Droid Build Evidence

The fdroiddata draft currently targets:

```text
Application ID: dev.aiterminal.android
versionName: 0.3.3
versionCode: 303
```

First activate a metadata copy using the full 40-character release commit that
contains the Android release metadata.

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\android\smoke-fdroid-release-activation.ps1 -Commit <40-char-release-commit>
```

Then run the real `fdroid build` or buildserver flow in the fdroiddata
environment using the activated metadata. Capture a JSON or text evidence file
that records:

- fdroidserver version.
- metadata file used.
- app id `dev.aiterminal.android`.
- version code `303`.
- build command.
- result status.
- output APK path or buildserver artifact reference.
- relevant log paths.

Pass that evidence path into the combined preflight:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1 `
  -FdroidBuildEvidencePath <path-to-fdroid-build-evidence.json>
```

Completion evidence:

- `fdroidBuild.status` is `ready`.
- `fdroidBuild.evidencePath` points to an existing file.
- Local metadata smokes are still useful, but they do not replace real
  `fdroid build` or buildserver evidence.

## Step 5: Close the Follow-Up

After MSI, Android signing, and F-Droid build evidence are all ready, rerun:

```powershell
npm run smoke:release-followup-preflight
```

If all blockers are closed, update:

- `docs/superpowers/plans/2026-07-01-remaining-work-priority.md`
- `docs/TROUBLESHOOTING.md`
- `docs/HANDOFF.md`
- `docs/HISTORY.md`
- `docs/TASK.md`

Keep release tag and existing assets unchanged unless there is a separate release
decision to republish assets.

## Troubleshooting

`gh secret list` returns no Android signing names:
Register the four `AI_TERMINAL_ANDROID_*` secrets, then rerun the preflight.

MSI evidence remains blocked:
Run from a Windows-native MSVC host, not WSL. Check `msi.missing` in the nested
evidence file.

F-Droid status remains blocked:
Pass an existing build/buildserver evidence path with `-FdroidBuildEvidencePath`.
The local metadata and activation smokes are not enough.

The combined preflight prints `blocked` after one item is fixed:
That is expected until all three release follow-up gates are ready.
