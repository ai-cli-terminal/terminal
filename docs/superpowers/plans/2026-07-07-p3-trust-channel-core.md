# 2026-07-07 P3 Trust Channel Core

## Scope

External release follow-up remains blocked on Windows MSI evidence, Android
signing secrets, and F-Droid build/buildserver evidence. Following the handoff
priority, this slice starts P3 enterprise/security hardening with a small,
reusable trust channel verification substrate.

## Completed

- Added a `trust` feature that enables pure Rust Ed25519 + SHA-256 verification.
- Added `src/trust.rs` with deterministic signed manifest verification:
  - key id must match the selected trust anchor
  - Ed25519 signature binds the typed manifest
  - manifest `issued_at_unix` / `expires_at_unix` window is enforced
  - anchor `min_version` rejects rollback/downgrade manifests
  - optional payload bytes must match manifest `payload_sha256`
- Added unit coverage for valid manifests, forged manifests, expired manifests,
  rollback below anchor floor, and payload digest mismatch.

## Boundary

This is not yet OS trust store, MDM, signed `policy.d`, skill registry, or binary
update verification. Higher layers still need to load readonly anchors and apply
this verifier before accepting organization policy, skills, or release manifests.

## Verification

```powershell
cargo test --features trust trust::
```

## Next

Wire this substrate into signed `policy.d`: readonly organization policy should
require a verified manifest, reject unsigned or rollback policy, and override user
policy only after successful trust-channel validation.
