# Signed Binary Manifest Bootstrap Runbook

This runbook closes the strict fresh-install bootstrap path for organization
deployments that require signed binary release manifests.

## Scope

Use this runbook when an organization sets `AI_REQUIRE_SIGNED_MANIFEST=1` for
`scripts/install.sh` or `scripts/install.ps1`.

The default public install path remains checksum-compatible. Strict mode is an
organization deployment mode and requires an already trusted verifier plus an
organization trust anchor before the install script downloads release binaries.

## Trust Model

- Release assets may include `binary-manifest.json` and
  `binary-manifest.manifest.json`.
- The manifest payload lists each release artifact name, digest, size, and
  manifest version.
- The manifest signature is verified with the organization trust anchor selected
  by `AI_TERMINAL_ORG_TRUST_ANCHOR` or the managed OS path.
- The private release signing key never ships to clients.
- A fresh machine must never use the just-downloaded `ai` binary to verify that
  same `ai` binary.

## Selected Bootstrap Path

The selected organization path is a managed verifier bundle distributed before
the terminal install:

1. Provision the organization trust anchor through MDM, a golden image, or an
   internal package manager.
2. Provision a trust-enabled `ai` verifier through the same trusted channel.
3. Invoke the install script with `AI_REQUIRE_SIGNED_MANIFEST=1`,
   `AI_TERMINAL_ORG_TRUST_ANCHOR`, and `AI_MANIFEST_VERIFIER`.
4. Let the install script download release assets, verify them with the external
   verifier, and then install `ai` and `ash`.

The verifier bundle is not fetched from the public release being installed. It
must come from a channel the organization already trusts.

## Supported Paths

| Path | Verifier source | Strict mode |
|---|---|---|
| Managed fresh install | MDM, golden image, or internal package manager verifier bundle | Yes |
| Managed update | Existing installed trust-enabled `ai` or explicit verifier | Yes |
| Public checksum install | `.sha256` release checksums | No |
| Emergency pinned install | Explicit verifier plus `AI_MIN_MANIFEST_VERSION` floor | Yes |

## Linux Or WSL Fresh Install

```bash
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | \
  AI_TERMINAL_ORG_TRUST_ANCHOR=/etc/ai-terminal/policy.d/org-root.json \
  AI_REQUIRE_SIGNED_MANIFEST=1 \
  AI_MANIFEST_VERIFIER=/opt/ai-terminal-verifier/bin/ai \
  AI_MIN_MANIFEST_VERSION=12 \
  bash
```

The verifier path must point to the pre-provisioned trust-enabled `ai`. The
install script may use an existing installed `ai` during updates, but fresh
strict installs should always pass `AI_MANIFEST_VERIFIER`.

## Windows Fresh Install

```powershell
$env:AI_TERMINAL_ORG_TRUST_ANCHOR = 'C:\ProgramData\ai-terminal\policy.d\org-root.json'
$env:AI_REQUIRE_SIGNED_MANIFEST = '1'
$env:AI_MANIFEST_VERIFIER = 'C:\Program Files\ai-terminal-verifier\ai.exe'
$env:AI_MIN_MANIFEST_VERSION = '12'
irm https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.ps1 | iex
```

The verifier path must point to the pre-provisioned trust-enabled `ai.exe`.
Do not point `AI_MANIFEST_VERIFIER` at a file downloaded by the same install
run.

## Managed Update

After a successful signed install, the installed `ai` can be used as the
verifier for later updates. Operators may still pass `AI_MANIFEST_VERIFIER` to
pin verification to a separately managed verifier bundle.

```bash
curl -fsSL https://raw.githubusercontent.com/ai-cli-terminal/terminal/main/scripts/install.sh | \
  AI_TERMINAL_ORG_TRUST_ANCHOR=/etc/ai-terminal/policy.d/org-root.json \
  AI_REQUIRE_SIGNED_MANIFEST=1 \
  AI_MIN_MANIFEST_VERSION=12 \
  bash
```

For update fleets, keep the verifier package and the terminal package on
separate rollout tracks. This preserves the ability to recover from a broken
terminal release without trusting the release under verification.

## Downgrade Prevention

Signed verification records the verified manifest version in the install
directory as `.ai-terminal-release-manifest-version`.

Strict verified installs fail if:

- the candidate manifest version is lower than the recorded version;
- `AI_MIN_MANIFEST_VERSION` is set and the candidate version is lower;
- the organization trust anchor has a higher `min_version`;
- manifest assets are missing while `AI_REQUIRE_SIGNED_MANIFEST=1`;
- the verifier is missing, not executable, or lacks `ai release manifest verify`;
- the artifact digest does not match the signed manifest.

## Operator Checklist

- Confirm the release uploaded `binary-manifest.json` and
  `binary-manifest.manifest.json`.
- Confirm the release signing key id matches the organization trust anchor.
- Confirm the trust anchor is present at the managed path or selected with
  `AI_TERMINAL_ORG_TRUST_ANCHOR`.
- Confirm the verifier bundle was installed from MDM, a golden image, or an
  internal package manager before running the terminal installer.
- Confirm fresh strict installs pass `AI_MANIFEST_VERIFIER`.
- Confirm update installs either pass `AI_MANIFEST_VERIFIER` or have an existing
  trust-enabled `ai`.
- Confirm the release manifest version is at or above
  `.ai-terminal-release-manifest-version`, `AI_MIN_MANIFEST_VERSION`, and the
  anchor `min_version`.
- Run `npm run check:release-manifest-bootstrap` before marking the bootstrap
  procedure ready.

## Evidence Closeout

External organization deployment evidence is required before marking the strict
fresh-install path complete. The evidence file should follow
`docs/releases/signed-binary-manifest-bootstrap-evidence.sample.json` and should
not include private release signing keys, tokens, passwords, or secret values.

By default the check reads:

```text
artifacts/release-manifest-bootstrap-external/evidence.json
```

Run:

```powershell
npm run check:release-manifest-bootstrap-evidence
```

The gate remains `blocked` until evidence proves:

- manifest release assets were present;
- the verifier bundle was preinstalled by MDM, a golden image, or an internal
  package manager;
- `AI_MANIFEST_VERIFIER` pointed at that preinstalled verifier;
- the organization trust anchor was preinstalled;
- `AI_REQUIRE_SIGNED_MANIFEST=1` was set;
- the strict install exited successfully;
- the just-downloaded `ai` was not used as verifier;
- `.ai-terminal-release-manifest-version` was written;
- a lower manifest version was rejected.

## Failure Handling

| Failure | Expected action |
|---|---|
| Missing manifest assets in strict mode | Stop the rollout and publish signed manifest assets |
| Missing verifier on fresh install | Provision the managed verifier bundle first |
| Missing anchor | Provision the organization anchor first |
| Signature/key mismatch | Stop rollout and investigate signing key or anchor drift |
| Artifact digest mismatch | Treat release asset as untrusted and stop rollout |
| Manifest downgrade | Publish a higher manifest version or raise the fleet floor intentionally |
| Verifier package broken | Roll back or replace the verifier bundle through the trusted channel |

## Completion Criteria

The bootstrap path is ready when the runbook, install docs, and release docs all
state the same strict-mode contract:

- public installs keep checksum verification;
- strict mode requires `AI_REQUIRE_SIGNED_MANIFEST=1`;
- fresh strict installs require an external `AI_MANIFEST_VERIFIER`;
- the organization anchor is selected by `AI_TERMINAL_ORG_TRUST_ANCHOR` or a
  managed OS path;
- the just-downloaded `ai` is never used to verify itself;
- verified manifest versions are monotonic through
  `.ai-terminal-release-manifest-version`, `AI_MIN_MANIFEST_VERSION`, and anchor
  `min_version`.
