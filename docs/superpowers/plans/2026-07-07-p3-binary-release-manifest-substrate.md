# P3 Binary Release Manifest Substrate

## Objective

Start P3-1-4 by adding the reusable verification substrate for signed binary
release manifests without yet changing installer behavior.

## Completed

- Added `src/binary_manifest.rs` behind the `trust` feature.
- Reused the shared trust-channel verifier:
  - Ed25519 signed manifest
  - payload SHA-256 binding
  - issued/expires validity
  - anchor `min_version` rollback guard
  - expected subject `release/binary-manifest.json`
- Added a release manifest payload schema with artifact `name`, `sha256`, and
  optional `version`, `platform`, and `kind`.
- Added `ai release manifest status` for default managed/user config paths.
- Added `ai release manifest verify --payload <file> --manifest <file>` with
  optional `--name <asset> --artifact <file>` hash matching.
- Added CI trust-feature checks so P3 trust code is compiled on pull requests.

## Boundaries

- This slice does not modify `scripts/install.sh`, `scripts/install.ps1`, or the
  release workflow to emit signed manifests.
- This slice does not yet persist the active release manifest as an update
  policy source.
- Downgrade prevention remains anchored by the existing trust manifest
  `min_version` field but still needs install/update integration.

## Verification

Expected local/CI verification:

```bash
cargo clippy --all-targets --features trust -- -D warnings
cargo test --features trust binary_manifest::
cargo test --features trust cli_parses_release_manifest_status_and_verify
git diff --check
```

This workspace currently lacks a local Rust toolchain, so GitHub Actions is the
source of truth for cargo verification.

## Next

- Emit `binary-manifest.json` and `binary-manifest.manifest.json` from the
  release workflow.
- Teach install/update scripts to require a matching signed manifest before
  installing release artifacts when trust enforcement is configured.
- Record downgrade-prevention behavior in the installer/update runbook.
