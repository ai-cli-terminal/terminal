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
- Added `ai skill registry update --registry <file> --manifest <file>` to
  verify signed registry snapshots before installing them into the active
  `config_dir()/skills` registry location.
- Added `ai skill registry revoke --registry <file> --manifest <file>` for
  signed snapshots that contain revoked entries; the command refuses non-revoke
  snapshots so revocation remains signed registry data rather than unsigned
  local mutation.
- `storage` builds record `skill_registry_updated`,
  `skill_registry_revoked`, and `skill_registry_enforced` audit events with
  manifest/key/count metadata only.
- User-config external skills under `config_dir()/skills` are hidden by default
  and require explicit `ai skill enable <name>` before appearing in `ai skill`
  discovery. Workspace-local `.ai-terminal/skills` remains discoverable.
- Added `ai skill disable <name>` and `ai skill enabled`; storage builds record
  path-free `skill_enabled`/`skill_disabled` audit metadata.
- Signed `policy.d` may set `[skills].external_sources` to `user-enabled`,
  `registry-only`, or `disabled`; `ai skill enable` applies that policy
  fail-closed before writing `enabled.json`.
- `ai skill enable <name>` prompts for the exact skill name before enabling an
  external skill; `--yes` is the explicit automation path.

## Boundary

- This does not execute skills. Skill content remains zero-trust data.
- This does not implement registry download or signing-key management commands.
- Enable/disable remains CLI-only; there is no graphical skill management
  surface.
- This is a stacked follow-up on the P3 trust channel PR.

## Verification

```powershell
cargo test --features trust skill_registry::
cargo test --features trust skill::
cargo test --features trust cli_parses_skill_command
cargo test --features trust cli_parses_skill_registry_status
cargo test --features trust cli_parses_skill_registry_update_and_revoke
cargo test --features trust cli_parses_skill_enable_disable_and_enabled
cargo test --features trust skill_enable_confirmation_requires_exact_skill_name
cargo test --features trust skill::discovers_with_source_and_filters_external_by_enabled_name
cargo test --features trust policy_d::verified_policy_carries_external_skill_source_policy
cargo test --features "storage trust" skill_registry::
```

Local host note: this Codex PowerShell environment currently has no `cargo` or
`rustfmt`, and WSL has no usable distro, so Rust verification must run in CI or
a Rust-enabled host.

## Next

- Move to P3-1-4 binary signing.
