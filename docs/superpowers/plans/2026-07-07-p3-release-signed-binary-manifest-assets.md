# P3 Release Signed Binary Manifest Assets

## Objective

Continue P3-1-4 by making tag releases capable of publishing signed binary
manifest assets once release signing secrets are configured.

## Completed

- Added `ai release manifest create`.
  - Inputs: repeated `--artifact <file>` plus optional `--release-version`.
  - Output: deterministic `binary-manifest.json` with artifact names, SHA-256
    hashes, and optional version metadata.
- Added `ai release manifest sign`.
  - Reads the Ed25519 private signing key from an environment variable, not a
    command-line value.
  - Writes `binary-manifest.manifest.json` with trust manifest subject
    `release/binary-manifest.json`, payload SHA-256, version, issued/expires,
    key id, and signature.
- Added a release workflow aggregate job.
  - CLI, Windows GUI, and Android jobs stage their release assets as Actions
    artifacts.
  - The aggregate job downloads all staged assets and creates
    `binary-manifest.json`.
  - If `AI_TERMINAL_RELEASE_SIGNING_KEY_HEX` and
    `AI_TERMINAL_RELEASE_KEY_ID` secrets exist, it signs and uploads
    `binary-manifest.json` and `binary-manifest.manifest.json` to the release.

## Boundaries

- Missing release signing secrets intentionally do not fail the existing release
  workflow. Checksum-only releases remain possible until enforcement is turned
  on deliberately.
- This slice does not teach install/update scripts to require the signed
  manifest.
- Downgrade prevention is not yet wired into installers; the signed manifest
  version uses `GITHUB_RUN_NUMBER` for monotonic release-workflow versions.

## Verification

Expected verification:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --features trust -- -D warnings
cargo test --features trust
git diff --check
```

This local Windows workspace lacks Rust/Cargo, so GitHub Actions remains the
source of truth for cargo verification.

## Next

- Add install/update script support for downloading and verifying
  `binary-manifest.json` plus `binary-manifest.manifest.json`.
- Decide the fresh-install trust bootstrap path, since a clean machine cannot
  use an already-installed `ai` binary to verify the first downloaded `ai`.
- Enforce downgrade prevention with the selected organization trust anchor
  `min_version`.
