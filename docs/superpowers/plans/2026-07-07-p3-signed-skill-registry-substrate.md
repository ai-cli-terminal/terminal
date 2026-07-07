# 2026-07-07 P3 Signed Skill Registry Substrate

## Scope

Start P3-1-3 by adding a signed organization skill registry substrate and
runtime discovery enforcement. Registry update commands and audit events remain
future slices.

## Completed

- Added `src/skill_registry.rs` behind the `trust` feature.
- Registry payload path:
  - `config_dir()/skills/org-registry.json`
  - `config_dir()/skills/org-registry.manifest.json`
- Registry payloads are verified with the shared trust-channel signed manifest
  and selected organization anchor.
- Registry manifest subject is fixed to `skills/org-registry.json`.
- Registry entries require:
  - `name`
  - `skill_sha256` as a 64-hex SHA-256 digest of raw `SKILL.md`
  - `status` as `active` or `revoked`
- `ai skill` behavior:
  - no registry or manifest: existing local discovery behavior is unchanged
  - registry or manifest present: incomplete/invalid set fails closed
  - valid registry: only active name+hash matches are shown
  - revoked, unsigned, unknown, or modified skills are hidden
- Added `ai skill registry status` diagnostics for registry/manifest/anchor
  paths, selected anchor source, expected subject, active/absent/invalid state,
  manifest metadata, and active/revoked entry counts.

## Boundary

- This does not execute skills. Skill content remains zero-trust data.
- This does not implement registry update/download commands.
- This does not yet record skill registry enforcement or update events to
  storage audit tables.
- This is a stacked follow-up on the P3 trust channel PR.

## Verification

```powershell
cargo test --features trust skill_registry::
cargo test --features trust skill::
cargo test --features trust cli_parses_skill_command
cargo test --features trust cli_parses_skill_registry_status
```

Local host note: this Codex PowerShell environment currently has no `cargo` or
`rustfmt`, and WSL has no usable distro, so Rust verification must run in CI or
a Rust-enabled host.

## Next

- Add signed registry update/revoke command flow with storage audit events.
