# 2026-07-07 P3 Signed policy.d Substrate

## Scope

Continue P3 enterprise/security hardening after the trust-channel manifest core.
This slice proves the signed `policy.d` boundary without yet changing normal
runtime policy resolution.

## Completed

- Added `src/policy_d.rs` behind the `trust` feature.
- Parses minimal organization policy payloads:

  ```toml
  profile = "paranoid"
  ```

- Verifies policy payload bytes with the trust-channel signed manifest:
  - manifest signature and key id are checked by `trust::verify_signed_manifest`
  - manifest subject must match the expected `policy.d/...` subject
  - manifest version/issued/expires/rollback checks come from the trust anchor
  - payload bytes must match the manifest SHA-256
  - unknown profile names are rejected
- Adds a readonly file loader that refuses non-readonly policy payload files.
- Adds `effective_profile()` so verified organization policy overrides the user
  active profile only after validation.

## Boundary

This does not yet alter `ai policy show`, `ai exec`, `ash`, or `ai_router`.
Runtime wiring should be a later slice so fail-closed behavior and diagnostics can
be reviewed as a single user-facing policy change.

## Verification

```powershell
cargo test --features trust policy_d::
```

Local host note: this Codex PowerShell environment currently has no `cargo`, and
WSL has no usable distro, so Rust verification must run in CI or a Rust-enabled
host.

## Next

Wire signed organization policy into CLI/runtime profile resolution:

- default path: `config_dir()/policy.d/org.toml` plus a signed manifest
- fail-closed for malformed signed org policy when the organization anchor is
  configured
- `ai policy show` should disclose organization override source/version
- `ai policy set` should not silently supersede verified organization policy
