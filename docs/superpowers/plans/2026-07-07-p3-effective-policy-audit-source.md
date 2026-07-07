# 2026-07-07 P3 Effective Policy Source in Audit Records

## Scope

Record the effective policy source in shell execution audit records without
changing the SQLite schema.

## Completed

- `src/shell_audit.rs` now resolves the effective policy profile before writing
  `audit_events` when `storage` is enabled.
- `audit_events.policy_profile` records the effective profile used for the
  command audit path.
- Audit payload JSON now includes `policy_source` for:
  - `command_executed`
  - `command_blocked`
  - `command_declined`
  - `command_backup_refused`
- The `policy_source` object distinguishes:
  - `user_active_profile`
  - `organization_policy`
  - `policy_resolution_error`
- Organization policy audit metadata includes profile, subject, manifest
  version, and manifest id.

## Boundary

- No database migration is required.
- Local policy file paths are intentionally omitted from audit payloads.
- This covers shell execution audit helpers. Other storage surfaces that still
  record profile snapshots can be revisited with the central audit export work.

## Verification

```powershell
cargo test --features "storage trust" shell_audit::
```

Local host note: this Codex PowerShell environment currently has no `cargo`, and
WSL has no usable distro, so Rust verification must run in CI or a Rust-enabled
host.

## Next

- Add OS trust store/MDM anchor loading or a documented managed anchor install
  path.
- Carry effective policy source into future central audit export records.
