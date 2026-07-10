# 2026-07-07 P3 Managed Policy Anchor Path

## Scope

Add a first managed-install path for signed organization policy trust anchors
without implementing native OS certificate store or MDM profile APIs yet.

## Completed

- Added `AI_TERMINAL_ORG_TRUST_ANCHOR` as an explicit absolute-path override for
  the organization policy trust anchor.
- Added managed anchor probes:
  - Unix: `/etc/ai-terminal/policy.d/org-root.json`
  - Windows: `%ProgramData%\ai-terminal\policy.d\org-root.json`
- Kept `config_dir()/policy.d/org-root.json` as the user-config fallback.
- `ai policy org status` now reports the selected anchor source.
- Anchor-only installs are inert: a managed root can be preinstalled without
  forcing org policy until `org.toml` or `org.toml.manifest.json` exists.
- Once policy or manifest exists, policy, manifest, and selected anchor must all
  exist and verify or runtime policy resolution fails closed.

## Boundary

- This is a managed file path, not native OS trust store or MDM profile parsing.
- The organization policy payload and manifest still live under
  `config_dir()/policy.d/`.
- The selected anchor path is diagnostic metadata, not audit payload content.

## Verification

```powershell
cargo test --features trust policy_d::
cargo test --features "storage trust" shell_audit::
```

Local host note: this Codex PowerShell environment currently has no `cargo` or
`rustfmt`, and WSL has no usable distro, so Rust verification must run in CI or
a Rust-enabled host.

## Next

- Add native OS trust store/MDM profile integration if the deployment target
  requires it.
- Carry effective policy source into future central audit export records.
