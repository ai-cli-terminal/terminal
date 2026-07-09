# P3 Signed Binary Manifest Bootstrap Evidence Gate

## Objective

Prepare the closeout gate for strict fresh-install organization deployment
evidence, while keeping the actual evidence external to this host.

## Completed

- Added `scripts/check-release-manifest-bootstrap-evidence.mjs`.
  - Reads external evidence from
    `artifacts/release-manifest-bootstrap-external/evidence.json` by default.
  - Emits a blocked status when external evidence is absent.
  - Validates ready evidence for release manifest assets, preinstalled verifier
    bundle, preinstalled organization anchor, strict-mode environment, install
    success, self-verification prohibition, recorded manifest version, and
    lower-version rejection.
  - Rejects evidence fields that look like private keys, signing secrets,
    tokens, passwords, or other secret material.
- Added a sample evidence file at
  `docs/releases/signed-binary-manifest-bootstrap-evidence.sample.json`.
  - The sample is marked with `"sample": true`.
  - Production closeout rejects sample evidence unless the smoke command passes
    `--allow-sample`.
- Added npm scripts:
  - `check:release-manifest-bootstrap-evidence`
  - `smoke:release-manifest-bootstrap-evidence`
- Updated the bootstrap runbook and release docs index with the evidence
  closeout command.

## Boundary

This slice does not claim real organization deployment evidence. It creates the
gate that will remain blocked until an operator supplies external evidence from
a managed host.

## Verification

```bash
node scripts/check-release-manifest-bootstrap-evidence.mjs
node scripts/check-release-manifest-bootstrap-evidence.mjs --input docs/releases/signed-binary-manifest-bootstrap-evidence.sample.json --allow-sample --output artifacts/release-manifest-bootstrap-evidence/sample-evidence-check.json --fail-on-blocked
node -e "JSON.parse(require('fs').readFileSync('package.json','utf8')); console.log('PACKAGE_JSON_OK')"
git diff --check
```

Cargo verification remains GitHub Actions based on this Windows workspace
because no local Rust/Cargo toolchain is available.

## Next

- Collect real organization evidence from a managed verifier bundle rollout.
- Run the first actual signed release smoke once release signing secrets are
  configured.
