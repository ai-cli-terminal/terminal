# RA/PWA Self-Hosted Relay Deployment Runbook

This runbook covers the first deployable Relay/M2 shape: a self-hosted
WebSocket relay used by an explicit `ai remote daemon --transport relay`
session and a PWA companion that loads daemon-issued runtime setup JSON.

## Current Readiness

Self-hosted relay is ready for explicit setup/staging use, not product default
use.

- Product default remains `live-loopback`.
- Local relay evidence uses `ws://127.0.0.1:<port>/relay`.
- The PWA Relay tab can connect, receive High approval requests, and send
  signed approve/reject responses.
- Daemon runtime WSS client support is available in `remote,tls` builds.
  Builds without the `tls` feature fail closed for `wss://` endpoints.
- The repository includes a production-oriented relay service artifact and
  deploy recipe: `scripts/relay-self-hosted-service.mjs` and
  `docs/relay-self-hosted-deploy.md`.
- Relay frame JSON currently contains `payload_json`; this runbook records an
  explicit self-hosted relay-operator trust decision. Failure-mode evidence is
  now covered by service and local bridge smokes.

## Architecture

The relay is a transport component, not an approval authority.

| Component | Responsibility |
|---|---|
| Daemon | Owns registered device state, approval validation, context drift checks, relay ticket issuance, and gate allow/block decisions. |
| Relay service | Registers signed session tickets, authenticates daemon/companion connect JSON, enforces session/role/sequence/expiry routing, and forwards frames. |
| PWA companion | Loads runtime setup JSON, validates companion identity, connects as companion, signs approve/reject responses, and sends response frames. |

The relay must not decide whether a command is safe, whether a device is
registered, or whether an approval signature is valid. Those checks remain
daemon-side.

## Endpoint Policy

- Local evidence may use `ws://127.0.0.1:<port>/relay` or
  `ws://localhost:<port>/relay`.
- Hosted/staging endpoints must be planned as `wss://.../relay`.
- Do not expose public `ws://` relay endpoints.
- Do not switch the product default away from `live-loopback`.
- Do not make relay broadly user-selectable by default; `live-loopback` remains
  the product default even when explicit self-hosted relay readiness is green.

## Relay Service Contract

A compatible self-hosted relay must expose:

| Route | Requirement |
|---|---|
| `GET /health` | Returns JSON status, active session count, registered ticket count, queued frame count, and routing counters. |
| `POST /sessions` | Accepts a signed session ticket JSON before any peer can connect. Rejects unsigned, bad-MAC, expired, malformed, or unknown-key tickets fail-closed. |
| `GET /relay?session_id=<id>&role=<daemon|companion>` | Upgrades to WebSocket. The first text message must be daemon or companion connect JSON matching the registered ticket. |

Frame routing requirements:

- Accept only JSON frames with relay protocol version `1`.
- Route by `session_id` and `sender`.
- Reject frames whose sender does not match the authenticated socket role.
- Reject non-increasing per-sender sequence numbers.
- Drop expired frames.
- Keep queued frames isolated by session.
- Do not log `payload_json`, session tokens, HMAC secrets, approval signatures,
  or full setup JSON.

Repository artifact:

```powershell
npm run relay:self-hosted
npm run smoke:pwa-relay-service-artifact
```

Deployment recipe: `docs/relay-self-hosted-deploy.md`.

## Secret And Key Handling

- The PWA never receives relay HMAC key material.
- Daemon setup JSON may include signed tickets, connect JSON, companion
  identity, endpoint URL, expiry, and non-secret key id.
- `remote-relay-ticket-keys.json` is daemon-owned local state and must not be
  committed, copied into PWA setup, or written to evidence artifacts.
- The relay service supports Ed25519 public-key ticket verification so hosted
  relay operators can configure public verifier keys instead of daemon-owned
  HMAC secrets.
- HMAC verifier secrets remain a legacy/local compatibility path. Hosted
  production should prefer Ed25519 verifier keys.
- Previous verifier keys may be retained only until tickets signed by those
  keys have expired plus bounded clock skew.

## Relay Operator Trust Decision

The current self-hosted relay shape does not provide end-to-end payload
confidentiality from the relay operator. Relay frames contain `payload_json`, so
the relay process can observe approval transport payloads while forwarding them.

Decision: this is acceptable only for the explicit self-hosted setup/debug path
where the operator controls and trusts the relay service. It is not a blanket
decision for managed relay, untrusted relay infrastructure, or changing the
product default.

Bounds:

- `live-loopback` remains the product default.
- Relay remains explicit setup/debug path even after hosted failure-mode
  evidence is green.
- Hosted/staging endpoints must use `wss://`.
- Relay logs, health, metrics, and evidence must not include `payload_json`,
  session tokens, setup JSON, HMAC secrets, approval signatures, command text
  beyond already masked fields, or private key material.
- The relay remains transport-only and must not validate approvals or decide
  command safety.
- Payload encryption remains the required path for future untrusted or managed
  relay infrastructure.

## Local Staging Procedure

Use the automated smoke first. It starts an isolated WSL daemon, a local
self-hosted relay harness, and a Playwright/Chrome PWA operator session.

```powershell
npm run smoke:pwa-relay-approve-reject-evidence
```

Expected marker:

```text
RA_PWA_RELAY_APPROVE_REJECT_EVIDENCE_OK artifacts\ra-pwa-relay-approve-reject-evidence\ra-pwa-relay-approve-reject-evidence.json
```

Completion evidence:

- Relay connection is `Connected`.
- Relay counters show `received=2`, `sent=2`, `approved=1`, `rejected=1`,
  `pending=0`.
- Approve command exits `0`.
- Reject command exits non-zero.
- Bridge health shows no queued frames.
- Screenshots exist for connected, approve pending, reject pending, and final
  states.

## Private-Network Evidence Map

Private-network relay is an explicit advanced setup path layered on the same
relay service contract. It is not the product default, and it does not make
managed relay ready. `live-loopback` remains the default transport.

Use this evidence chain when validating the private-network path:

| Evidence | Command | Confirms |
|---|---|---|
| Setup contract | `npm run check:pwa-relay-private-network-contract` | Private-network setup requires relay transport, private-network deployment mode, `privateNetworkName`, signed ticket, companion identity, operator setup text, and no public `ws://` endpoint. |
| Runtime guardrails | `npm run check:pwa-relay-private-network-runtime-guardrails` | Daemon setup emits `privateNetworkName` only for private-network mode and keeps public `ws://` blocked. |
| Operator setup evidence | `npm run smoke:pwa-relay-private-network-operator-evidence` | CLI-emitted setup JSON imports through PWA helpers and can roundtrip setup-derived daemon/companion relay frames. |
| Visible import path | `npm run smoke:pwa-relay-private-network-visible-import` | PWA Relay tab exposes a separate private-network import/status path without changing self-hosted setup/connect behavior. |
| Connection controls | `npm run smoke:pwa-relay-private-network-connection-controls` | PWA private-network connect/disconnect controls build a setup-derived companion WebSocket loop and connect to the relay service artifact. |
| Approval flow | `npm run smoke:pwa-relay-private-network-approval-flow-evidence` | Private-network approval requests reach the PWA as `Private Relay`, and approve/reject responses are delivered back to the daemon-side relay endpoint. |

Completion evidence for the private-network path:

- Private-network PWA status is `Ready`.
- Private-network connection state is `Connected`.
- Private-network approval counters show `received=2`, `sent=2`,
  `approved=1`, `rejected=1`, `pending=0`.
- The self-hosted relay controls and evidence remain separate.
- No relay HMAC secret, setup secret field, private key material, or full
  payload retention is introduced in evidence.

## Managed Relay Operator Setup Evidence Map

Managed relay remains an explicit opt-in setup path. It is not the product
default, and endpoint delivery is allowed only through explicit operator-started
evidence with manual signed-response copy as fallback. `live-loopback` remains
the default transport.

Use this evidence chain when validating the managed operator setup path:

| Evidence | Command | Confirms |
|---|---|---|
| Operator setup contract | `npm run check:pwa-relay-managed-runtime-operator-setup-contract` | Managed setup requires a `wss://` endpoint, hashed identifiers, metadata-only fields, manual connect, endpoint auto-start disabled, public bind off, and `live-loopback` rollback. |
| Import preflight | `npm run check:pwa-relay-managed-runtime-operator-setup-import-preflight` | PWA parsing rejects unknown/prohibited setup fields and renders only sanitized metadata after import. |
| Browser evidence | `npm run smoke:pwa-relay-managed-runtime-operator-setup-browser-evidence` | Chromium desktop/mobile evidence confirms original setup JSON, signed tickets, raw tokens, payloads, key material, raw identifiers, operator setup text, and support contact metadata are hidden. |
| Connection controls | `npm run smoke:pwa-relay-managed-runtime-operator-setup-connection-controls` | Managed request/cancel controls are enabled only after ready import and remain manual/status-only without creating a WebSocket or starting an endpoint. |
| Session handshake | `npm run smoke:pwa-relay-managed-runtime-operator-setup-session-handshake` | Session handshake requires ready import plus manual request, displays only a `managed-cap:*` handle and `sha256:*` transcript hash, and keeps the capability envelope hidden. |
| Approval flow | `npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-flow-evidence` | Managed approval flow loads a session-capability-derived request into the existing Approve panel as `Managed Relay`, signs approve/reject responses manually, and does not create a WebSocket or start an endpoint. |
| Approval response delivery boundary | `npm run check:pwa-relay-next-mode-planning` | Managed approval response delivery remains manual signed-response copy only, copy/verify controls stay in the existing approval panel, the managed setup surface hides responses, and endpoint delivery remains future evidence. |
| Approval response endpoint delivery evidence | `npm run check:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-delivery-evidence` | Managed approval response endpoint delivery requires an operator-started endpoint, manual connect, and a client-held payload key; it routes an encrypted frame envelope only and keeps manual signed-response copy as fallback. |
| Approval response endpoint browser evidence | `npm run smoke:pwa-relay-managed-runtime-operator-setup-approval-response-endpoint-browser-evidence` | Managed approval response endpoint browser evidence shows the endpoint delivery control, ready state, encrypted route status, daemon receipt status, and manual copy fallback without rendering route envelopes, payload keys, or ciphertext. |

Completion evidence for the managed operator setup path:

- Managed Relay setup import state is `Ready`.
- Managed connection controls remain manual/status-only.
- Managed session handshake shows only `managed-cap:*` and `sha256:*` values.
- Managed approval flow uses the existing Approve panel with source
  `Managed Relay`.
- Managed approval response delivery remains `manual-signed-response-copy-only`.
- Managed approval response copy/verify controls remain in the existing
  approval panel, and the managed setup surface does not show response payloads.
- Managed approval response endpoint delivery uses an explicit operator-started
  encrypted frame path and keeps manual signed-response copy as fallback.
- Managed approval response endpoint browser evidence shows the endpoint delivery
  control, ready state, encrypted route status, daemon receipt status, and manual
  copy fallback in the browser without rendering route envelopes, payload keys,
  or ciphertext.
- Endpoint auto-start remains disabled.
- Public bind remains off.
- No managed WebSocket is created during setup, handshake, approval-flow, or
  delivery-boundary evidence. Endpoint delivery evidence only creates the
  explicit operator-started delivery path.
- Daemon bridge proof of endpoint delivery remains out of scope
  until
  `managed-relay-runtime-operator-setup-approval-response-daemon-bridge-evidence`.
- No capability envelope JSON, signed ticket, raw token, payload material,
  payload key, ciphertext, private key material, operator setup text, support
  contact metadata, or raw identifier is introduced in visible managed setup
  evidence.

## Manual Staging Procedure

Use this only with a compatible local relay service that implements the contract
above.

1. Start the relay service on localhost.

   ```powershell
   # Example endpoint shape only; use the actual relay process command.
   $relay = "ws://127.0.0.1:<port>/relay"
   ```

2. Pair or select a registered companion device.

   ```powershell
   ai remote devices
   ```

3. Start the daemon with explicit relay transport.

   ```powershell
   ai remote daemon --device-id <device-id> --transport relay --relay-endpoint-url $relay --relay-ttl-seconds 300
   ```

4. Copy the printed `PWA relay setup json` into the PWA Relay tab and click
   `Load setup`.

5. Click `Connect relay` and confirm Relay connection state is `Connected`.

6. Arm High approvals and exercise approve/reject.

   ```powershell
   ai remote arm --allow-high
   ai __gate rm -rf build
   ai __gate rm -rf build
   ```

7. Capture the PWA final state, daemon output, relay `/health`, and the approve
   and reject exit codes.

## Hosted Production Gate

Explicit self-hosted relay readiness is green in this repo state. The gate is
ready for the self-hosted setup/debug path and remains separate from product
default selection.

Relay remains an explicit setup/debug path and `live-loopback` remains the
product default.

Run the hosted-readiness gate before claiming hosted relay progress:

```powershell
npm run check:pwa-relay-hosted-readiness
```

Current expected marker is ready for explicit self-hosted relay:

```text
RA_PWA_RELAY_HOSTED_READINESS_READY artifacts\ra-pwa-relay-hosted-readiness\ra-pwa-relay-hosted-readiness.json
```

## Observability

The relay service now exposes aggregate-only observability and retention policy
metadata through `GET /health`. `npm run smoke:pwa-relay-service-artifact`
asserts this contract.

The relay service exposes:

- Registered tickets accepted/rejected.
- Authenticated daemon/companion connects accepted/rejected.
- Open and closed WebSocket connections.
- Accepted, delivered, rejected, and expired frames.
- Queue depth by aggregate count, not payload.
- Session count and ticket count.
- Error classes for bad token, missing ticket, expired ticket, duplicate
  sequence, wrong sender, and malformed frame.

Observability must not include secrets, session tokens, full setup JSON,
`payload_json`, approval signatures, command text beyond already masked fields,
or private key material.

Retention policy:

- Persistent storage: none in the service artifact.
- Event logs: none in the service artifact.
- Sessions and tickets: memory only until expiry or service restart.
- Queued frames: memory only until delivery, expiry, or service restart.
- Payload JSON, session tokens, setup JSON, approval signatures, and private key
  material: not retained in health, logs, metrics, or evidence.

## Failure-Mode Evidence

Failure-mode evidence ready. The service artifact smoke and local WebSocket
bridge smoke cover:

- Unsigned ticket rejection.
- Bad-MAC ticket rejection.
- Missing ticket connect rejection.
- Bad session token rejection.
- Expired ticket connect rejection.
- Duplicate sequence rejection.
- Expired frame drop.
- Rotated session reconnect and stale-session isolation.
- Wrong sender/role rejection.
- PWA setup identity mismatch blocked before connect.
- PWA disconnect/reconnect behavior through existing local bridge/session
  evidence.

Current local evidence sources:

```powershell
npm run smoke:pwa-relay-websocket-bridge
npm run smoke:pwa-relay-service-artifact
npm run smoke:pwa-relay-approve-reject-evidence
npm run check:pwa-relay-transport-decision
npm run check:pwa-relay-deployment-decision
npm run check:pwa-relay-hosted-readiness
```

## Rollback

If relay staging fails:

1. Stop the relay daemon and relay service.
2. Restart `ai remote daemon` without `--transport relay`.
3. Keep or restore PWA Live workflow through `live-loopback`.
4. Remove or rotate relay ticket verifier keys used in the failed staging
   attempt.
5. Preserve failure evidence under `artifacts/` without committing secrets.

## Completion Criteria

This runbook slice is complete when:

- The runbook check passes.
- Existing relay deployment decision, WebSocket bridge, setup UI, and
  approve/reject evidence smokes still pass.
- The service artifact smoke passes and records no payload or secret leakage in
  health evidence.
- The hosted-readiness gate records the explicit self-hosted readiness green
  state.
- The private-network evidence map lists setup, runtime, operator, import,
  connection, and approval-flow evidence while keeping `live-loopback` as the
  product default.
- The managed relay operator setup evidence map lists setup contract, import
  preflight, browser evidence, connection controls, session handshake,
  approval-flow evidence, delivery boundary, endpoint delivery evidence, and
  endpoint browser evidence while keeping daemon bridge proof of endpoint delivery as the next local
  slice.
- HANDOFF and remaining-work priority point to the next local blocker after the
  runbook.

Relay remains explicit setup/debug path even when the hosted production gate
above is green; `live-loopback` remains the product default.
