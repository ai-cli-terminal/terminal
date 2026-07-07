# 2026-07-07 P3 Signed policy.d Runtime Wiring

## Scope

Wire the signed `policy.d` substrate into runtime profile resolution while keeping
the feature behind `trust`. Normal builds without `trust` keep the existing
active-profile behavior.

## Completed

- Default signed organization policy file set:
  - `config_dir()/policy.d/org.toml`
  - `config_dir()/policy.d/org.toml.manifest.json`
  - `config_dir()/policy.d/org-root.json`
- If none of those files exist, runtime policy resolution uses the user active
  profile exactly as before.
- If any file in the set exists, all three must exist and verify successfully.
  Incomplete, unsigned, modified, expired, rollback, subject-mismatched, or
  non-readonly policy files fail closed.
- `ai policy show` discloses effective policy source, organization subject,
  version, manifest id, and policy path when org policy is active.
- `ai policy set` validates the organization policy before writing the user
  active profile and warns when org policy still overrides the effective profile.
- `ai risk`, `ai verify`, `ai route`, `ai exec`, `ai dispatch`, and `ai tui`
  resolve the effective profile through signed org policy when built with
  `trust`.
- `ash` external command execution preserves policy resolution errors and refuses
  to execute commands when organization policy is unavailable or invalid.
- `ash` AI routing also uses the same effective profile; invalid organization
  policy disables the AI router rather than bypassing policy.

## Boundary

- The organization policy payload is still intentionally minimal:

  ```toml
  profile = "paranoid"
  ```

- Anchor loading is file-based JSON for now, not OS trust store or MDM.
- Runtime wiring does not yet add a dedicated `ai policy org` management command.
- Audit records that call `config::get_active_profile()` directly may still show
  the user active profile rather than the effective org override; enforcement
  paths use the effective profile.

## Verification

```powershell
cargo test --features trust policy_d::
cargo test --features trust
```

Local host note: this Codex PowerShell environment currently has no `cargo`, and
WSL has no usable distro, so Rust verification must run in CI or a Rust-enabled
host.

## Next

- Add OS trust store/MDM anchor loading or a documented managed anchor install
  path.
- Add explicit `ai policy org status` diagnostics.
- Extend audit/profile reporting to store effective policy source.
