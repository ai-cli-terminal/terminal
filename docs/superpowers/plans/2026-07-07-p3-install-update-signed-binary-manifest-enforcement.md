# P3 Install/Update Signed Binary Manifest Enforcement

## Objective

Continue P3-1-4 by teaching install/update scripts to consume signed binary
manifest release assets without weakening fresh-install trust bootstrap.

## Completed

- Updated `scripts/install.sh` and `scripts/install.ps1`.
  - Download `binary-manifest.json` and `binary-manifest.manifest.json` when
    the release provides them.
  - Keep existing `.sha256` verification for public checksum-only releases.
  - Verify downloaded `ai`/`ash` artifacts with `ai release manifest verify`
    when a trust-enabled verifier is available.
  - Use `AI_MANIFEST_VERIFIER` when supplied, otherwise prefer the existing
    installed `ai` and then `ai` on PATH.
  - Never use the just-downloaded `ai` binary to verify itself.
- Added strict organization mode.
  - `AI_REQUIRE_SIGNED_MANIFEST=1` makes missing manifest assets, missing
    verifier, missing/invalid anchor, failed manifest verification, and artifact
    mismatch fail closed.
  - `AI_TERMINAL_ORG_TRUST_ANCHOR` selects the organization trust anchor through
    the same runtime path as signed policy.d and skill registry.
- Added downgrade prevention for verified installs.
  - Successful signed verification stores the manifest version in the install
    directory as `.ai-terminal-release-manifest-version`.
  - Later verified installs fail if the candidate manifest version is lower.
  - `AI_MIN_MANIFEST_VERSION=<n>` provides an operator-supplied floor.
- Updated release/CI feature sets.
  - CLI release binaries include `trust` so future updates have a built-in
    manifest verifier.
  - CI release-feature checks now build Linux and Windows combinations with
    `trust`.

## Boundary

Fresh strict installs still require an external trusted verifier/bootstrap
anchor. A clean machine cannot safely execute a newly downloaded `ai` to prove
that same `ai` is trustworthy. Public installs remain checksum-compatible unless
an operator opts into `AI_REQUIRE_SIGNED_MANIFEST=1`.

## Verification

Local host verification:

```bash
"C:/Program Files/Git/usr/bin/bash.exe" -n scripts/install.sh
```

```powershell
$tokens = $null
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
  (Resolve-Path scripts/install.ps1),
  [ref]$tokens,
  [ref]$errors
) | Out-Null
if ($errors.Count) { throw $errors[0].Message }
```

Expected CI source of truth:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
cargo clippy --all-targets --features trust -- -D warnings
cargo test --features trust
```

This local Windows workspace does not have Cargo/Rust, so cargo verification
remains GitHub Actions based.

## Next

- Run the first actual signed release smoke once release signing secrets are
  configured.
- Close the fresh-install strict bootstrap runbook with the selected
  organization verifier distribution path.
