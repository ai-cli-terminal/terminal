# RA/PWA Self-Hosted Relay Deployment Runbook

This runbook covers the first deployable Relay/M2 shape: a self-hosted
WebSocket relay used by an explicit `ai remote daemon --transport relay`
session and a PWA companion that loads daemon-issued runtime setup JSON.

## Current Readiness

Self-hosted relay is ready for local/staging evidence, not product default use.

- Product default remains `live-loopback`.
- Local relay evidence uses `ws://127.0.0.1:<port>/relay`.
- The PWA Relay tab can connect, receive High approval requests, and send
  signed approve/reject responses.
- The daemon relay runtime currently rejects `wss://` endpoints with a clear
  runtime error. Hosted production relay is blocked until daemon-side WSS
  support exists.
- The repository does not include a production relay service artifact.
- Relay frame JSON currently contains `payload_json`. A self-hosted operator
  must treat the relay process as trusted transport infrastructure. Product
  visible hosted relay remains blocked until payload confidentiality is added
  or an explicit trust decision is recorded.

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
- Do not make relay broadly user-selectable until hosted deploy, WSS runtime,
  observability, failure-mode evidence, and payload confidentiality/trust
  evidence are all green.

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

## Secret And Key Handling

- The PWA never receives relay HMAC key material.
- Daemon setup JSON may include signed tickets, connect JSON, companion
  identity, endpoint URL, expiry, and non-secret key id.
- `remote-relay-ticket-keys.json` is daemon-owned local state and must not be
  committed, copied into PWA setup, or written to evidence artifacts.
- A production relay needs a documented verifier-key distribution model or a
  public-key ticket-signing replacement. The current local evidence harnesses
  are not a production secret distribution design.
- Previous verifier keys may be retained only until tickets signed by those
  keys have expired plus bounded clock skew.

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

Hosted production is not ready in this repo state. It remains blocked by:

- Daemon runtime WSS client support.
- A production relay service artifact and deployment recipe.
- Verifier-key distribution or public-key ticket signing.
- Payload confidentiality or an explicit relay-operator trust decision.
- Hosted observability and retention policy evidence.
- Hosted failure-mode evidence matching or exceeding the local bridge smoke.

Until those items are closed, relay remains an explicit setup/debug path and
`live-loopback` remains the product default.

Run the hosted-readiness gate before claiming hosted relay progress:

```powershell
npm run check:pwa-relay-hosted-readiness
```

Current expected marker is blocked, because the PWA setup path accepts hosted
`wss://` metadata but the daemon runtime still fail-closes `wss://` before
connecting.

## Observability

The relay service should expose or record:

- Registered tickets accepted/rejected.
- Authenticated daemon/companion connects accepted/rejected.
- Open and closed WebSocket connections.
- Accepted, delivered, rejected, and expired frames.
- Queue depth by aggregate count, not payload.
- Session count and ticket count.
- Error classes for bad token, missing ticket, expired ticket, duplicate
  sequence, wrong sender, and malformed frame.
- Latency for session registration, connect authentication, and frame delivery.

Observability must not include secrets, session tokens, full setup JSON,
`payload_json`, approval signatures, command text beyond already masked fields,
or private key material.

## Failure-Mode Evidence

Before relay can become user-selectable, produce evidence for:

- Unsigned ticket rejection.
- Bad-MAC ticket rejection.
- Missing ticket connect rejection.
- Bad session token rejection.
- Expired ticket connect rejection.
- Duplicate sequence rejection.
- Expired frame drop.
- Rotated session reconnect and stale-session isolation.
- Wrong sender/role rejection.
- Daemon relay outage fail-closed gate behavior.
- PWA setup identity mismatch blocked before connect.
- PWA disconnect/reconnect behavior.

Current local evidence sources:

```powershell
npm run smoke:pwa-relay-websocket-bridge
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
- The hosted-readiness gate records the remaining hosted production blockers
  and points to the next local implementation slice.
- HANDOFF and remaining-work priority point to the next local blocker after the
  runbook.

Relay itself is not production-ready until the hosted production gate above is
closed.
