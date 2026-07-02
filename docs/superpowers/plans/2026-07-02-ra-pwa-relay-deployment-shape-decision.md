# 2026-07-02 RA/PWA Relay Deployment Shape Decision

## Purpose

Relay now has WebSocket substrate evidence, signed session tickets, reconnect
coverage, daemon ticket issuer policy, and PWA UX preflight. This slice chooses
the first deployment shape so later storage and setup UI work has one concrete
target.

## Decision

Use **self-hosted WebSocket relay** as the first deployable relay shape.

Keep **managed relay** and **private-network/Tailscale** as deferred candidates.
They are still known deployment modes, but the PWA relay UX must not become
ready for them until separate operations, network, and support evidence exists.

Keep the product default as **`live-loopback`**. This decision does not make
relay product-visible.

## Why Self-Hosted First

- It matches the current local WebSocket bridge, auth, signed ticket, rotation,
  and browser parity evidence.
- It avoids depending on a managed relay service before the trust, billing,
  abuse, retention, and availability model exists.
- It keeps relay ticket HMAC secrets daemon-owned; relay infrastructure only
  receives signed tickets and validates MACs.
- It gives operators one concrete endpoint model to configure before visible
  setup UI is added.
- Tailscale/private-network remains useful, but it is a separate direct-network
  path rather than the first browser relay deployment.

## Guardrails

- Production relay endpoint URLs must use `wss://`.
- Local `ws://` remains allowed only for localhost smoke/development.
- PWA relay preflight is ready only for the selected `self-hosted` deployment
  mode.
- `managed` and `private-network` remain hidden with
  `relay_deployment_mode_not_selected`.
- Do not switch the product default away from `live-loopback`.
- Do not add operator-visible relay UI until persistent secret storage/key id
  migration and operator copy are complete.

## Work Added

- Add `relayDeploymentShapeDecision()` and stable deployment decision constants
  to the PWA helper module.
- Tighten `relayTransportUxPreflight()` so valid-but-deferred deployment modes
  stay hidden.
- Add `npm run check:pwa-relay-deployment-decision`.
- Update UX preflight evidence to prove only `self-hosted` can become ready.

Evidence path:

```text
artifacts/ra-pwa-relay-deployment-decision/ra-pwa-relay-deployment-decision.json
```

## Follow-Up

1. Add visible self-hosted relay setup UI after endpoint, ticket, identity, and
   operator copy are ready.
2. Add browser evidence that the runtime setup bundle drives relay UX preflight
   to ready without changing the product default.
3. Revisit managed relay and private-network/Tailscale only after separate
   deployment and support evidence exists.

## Progress

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-secret-keyring-migration.md`
added persistent daemon-owned HMAC keyring records, optional signed ticket key
ids, and legacy no-key-id ticket validation fallback.

2026-07-02 follow-up:
`docs/superpowers/plans/2026-07-02-ra-pwa-relay-daemon-runtime-issuer.md`
added keyring-backed self-hosted runtime setup issuance for this selected
deployment shape.

## Verification

```powershell
npm run test:pwa
npm run check:pwa-relay-deployment-decision
npm run check:pwa-relay-ux-preflight
npm run check:pwa-relay-transport-decision
```
