# Self-Hosted Relay Service Deploy Recipe

This recipe runs the repository relay service artifact for a self-hosted Relay/M2
deployment. It is a transport service only. The daemon remains the approval
authority.

## Artifact

```powershell
npm run relay:self-hosted
```

Entrypoint:

```text
scripts/relay-self-hosted-service.mjs
```

The service exposes:

| Route | Purpose |
|---|---|
| `GET /health` | Status, counts, queue depth, limits, and counters without payloads or secrets. |
| `POST /sessions` | Register a signed relay session ticket before peers connect. |
| `GET /relay?session_id=<id>&role=<daemon|companion>` | WebSocket upgrade. First text frame must be matching connect JSON. |

## Configuration

Required verifier key configuration:

```powershell
$env:AI_TERMINAL_RELAY_HMAC_SECRET = "<operator-managed secret>"
```

Optional key id:

```powershell
$env:AI_TERMINAL_RELAY_HMAC_KEY_ID = "relay-active-1"
```

Multiple keys for bounded rotation:

```powershell
$env:AI_TERMINAL_RELAY_HMAC_KEYS_JSON = '[{"key_id":"relay-active-1","secret":"..."},{"key_id":"relay-prev-1","secret":"..."}]'
```

Listener configuration:

```powershell
$env:AI_TERMINAL_RELAY_HOST = "127.0.0.1"
$env:AI_TERMINAL_RELAY_PORT = "8080"
```

Do not expose this plain HTTP listener directly on the public internet.

## TLS/WSS Shape

Run the service behind a TLS reverse proxy that terminates HTTPS/WSS and forwards
to the local HTTP/WS listener without rewriting paths:

```text
https://relay.example.test/health   -> http://127.0.0.1:8080/health
https://relay.example.test/sessions -> http://127.0.0.1:8080/sessions
wss://relay.example.test/relay      -> ws://127.0.0.1:8080/relay
```

The daemon should be built with `remote,tls` and started with the public
`wss://.../relay` endpoint.

## Smoke

```powershell
npm run smoke:pwa-relay-service-artifact
```

Expected marker:

```text
RA_PWA_RELAY_SERVICE_ARTIFACT_OK artifacts\ra-pwa-relay-service-artifact\ra-pwa-relay-service-artifact.json
```

The smoke proves:

- unsigned tickets are rejected;
- signed tickets with a configured key id are accepted;
- daemon and companion sockets authenticate against the registered ticket;
- daemon-to-companion and companion-to-daemon frames route successfully;
- ack and health evidence do not include `payload_json`, test payload values, or
  verifier secrets.

## Remaining Production Blockers

- Verifier-key distribution or public-key ticket signing.
- Payload confidentiality or explicit relay-operator trust decision.
- Hosted observability and retention policy evidence.
- Hosted failure-mode evidence matching or exceeding local bridge smoke.

Relay remains explicit setup/debug path until those blockers are closed.
