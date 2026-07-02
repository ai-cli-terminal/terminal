# 2026-07-02 RA/PWA Relay Daemon Ticket Issuer Policy

## Purpose

The relay track now has signed tickets plus reconnect/rotation evidence. This
slice defines the daemon-side issuer boundary before any hosted relay decision:
the daemon owns ticket signing, bridges verify against a bounded keyring, and
old keys are retained only long enough for already-issued tickets to expire.

## Scope

- Add a Rust `CompanionRelayTicketIssuer` helper.
- Issue WebSocket relay tickets with the active HMAC-SHA256 key.
- Verify tickets against the active key plus retained previous keys.
- Limit the verifier keyring to `MAX_COMPANION_RELAY_TICKET_HMAC_VERIFY_KEYS`
  keys.
- Reject short HMAC keys, duplicate keys, and over-retained keyrings.
- Keep the signed ticket wire shape unchanged: no key id is added in this
  local prototype slice.

## Policy

- The daemon is the only component that may issue relay tickets.
- The PWA never receives the relay ticket HMAC secret.
- The relay bridge verifies signed tickets but does not mint or mutate them.
- A rotation creates a new active HMAC key and immediately signs new tickets
  with that key.
- Previously active keys may remain in the verifier keyring only until the last
  ticket signed by that key has expired, plus deployment-specific clock skew.
- Retired keys must be removed from the verifier keyring before accepting
  product traffic.
- Ticket TTL remains bounded by `DEFAULT_COMPANION_RELAY_SESSION_TTL_MS`.
- Because the current signed wrapper has no key id, verification tries the
  bounded keyring. A hosted deployment may add a non-secret key id later if
  route efficiency or observability requires it.

## Non-Goals

- No hosted relay deployment.
- No persistent key store implementation.
- No secret generation command or operator secret UX yet.
- No signed ticket wire-shape migration.
- No product default change away from `live-loopback`.

## Evidence Shape

Rust tests now prove:

- Active-key issuance signs tickets that validate with the same key.
- The issuer validates daemon/companion connect messages through the signed
  ticket boundary.
- A rotated issuer accepts a previous-key ticket while that previous key is
  retained.
- New tickets are signed by the new active key, not the previous key.
- A retired key is rejected once it is absent from the verifier keyring.
- Short, duplicate, and over-retained key state fails closed.

## Follow-Up

1. Decide deployment shape: self-hosted relay, private-network/Tailscale direct
   mode, or managed relay.
2. Add persistent secret storage and key id migration only after deployment
   shape is chosen.
3. Add visible relay setup UI only after deployment mode and operator copy are
   ready.

## Progress

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-ux-preflight.md` added the PWA
visibility preflight and evidence command. Relay remains hidden until all
operator-facing readiness inputs are present.

## Verification

```powershell
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --features remote remote_transport'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets --features "storage tls remote" -- -D warnings'
```
