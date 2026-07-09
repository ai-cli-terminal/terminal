# P3 Signed Binary Manifest Bootstrap Runbook

## Objective

Close the P3-1-4 fresh-install strict bootstrap documentation gap without
changing the install script trust boundary.

## Completed

- Added `docs/releases/signed-binary-manifest-bootstrap-runbook.md`.
  - Selects the organization bootstrap path as a managed verifier bundle
    distributed by MDM, golden image, or internal package manager before the
    terminal installer runs.
  - Keeps public installs checksum-compatible.
  - Requires `AI_REQUIRE_SIGNED_MANIFEST=1`,
    `AI_TERMINAL_ORG_TRUST_ANCHOR`, and `AI_MANIFEST_VERIFIER` for fresh strict
    installs.
  - Explains update installs that can use an existing trust-enabled `ai`.
  - Documents downgrade prevention through
    `.ai-terminal-release-manifest-version`, `AI_MIN_MANIFEST_VERSION`, and
    anchor `min_version`.
  - States that the just-downloaded `ai` must never verify itself.
- Added `scripts/check-release-manifest-bootstrap-runbook.mjs` and
  `npm run check:release-manifest-bootstrap`.
  - Checks required runbook sections and strict-mode guardrail phrases.
  - Checks that release docs and install docs link the runbook.
  - Checks that install scripts still expose the strict-mode environment
    variables.
- Linked the runbook from `docs/releases/README.md` and `docs/INSTALL.md`.

## Boundary

This slice does not change release signing, manifest verification, or install
script behavior. It documents and guards the selected bootstrap path so strict
fresh installs do not accidentally trust the release artifact they are trying
to verify.

## Verification

```bash
npm run check:release-manifest-bootstrap
git diff --check
```

Cargo verification remains GitHub Actions based on this Windows workspace
because no local Rust/Cargo toolchain is available.

## Next

- Run the first actual signed release smoke once release signing secrets are
  configured.
- Capture external organization deployment evidence for the selected managed
  verifier bundle path.
