# 2026-07-07 P3 Policy Org Status

## Scope

Add an operator-facing diagnostic command for the signed organization policy
runtime wiring.

## Completed

- Added `ai policy org status`.
- In `trust` builds, the command prints:
  - default policy, manifest, and anchor paths
  - selected anchor source (`environment`, `managed_path`, or `user_config`)
  - existence and readonly state for the policy payload
  - expected manifest subject
  - active / absent / invalid status
  - active manifest id, version, key id, issued/expires timestamps, and effective
    organization profile
  - fail-closed runtime state and error message for invalid policy sets
- In non-`trust` builds, the command parses and reports that organization policy
  diagnostics are unavailable because the binary lacks the `trust` feature.

## Boundary

This is read-only diagnostics. It does not install anchors, generate manifests,
change active profiles, or weaken the fail-closed runtime behavior.

## Verification

```powershell
cargo test --features trust cli_parses_policy_org_status
cargo test --features trust policy_d::
```

Local host note: this Codex PowerShell environment currently has no `cargo`, and
WSL has no usable distro, so Rust verification must run in CI or a Rust-enabled
host.

## Next

- Carry effective policy source into future central audit export records.
- Add native OS trust store/MDM profile integration if managed file paths are
  not sufficient for deployment.
