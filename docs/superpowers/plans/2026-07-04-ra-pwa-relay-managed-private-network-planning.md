# 2026-07-04 RA/PWA Relay Managed Private-Network Planning

## Purpose

Choose the next Relay/M2 local slice after explicit self-hosted relay readiness
is green.

## Status

Completed in this planning slice. The follow-up setup contract, runtime
guardrails, operator evidence, visible import path, connection controls,
approval flow evidence, runbook closeout, and managed operations planning
slices are also complete; the next local implementation slice is managed relay
control-plane contract.

## Decision

Pick private-network relay planning before managed relay.

Self-hosted relay readiness is green for the explicit setup/debug path, but
managed relay would require new operational ownership: tenant isolation,
control-plane auth, abuse handling, support workflows, retention operations,
and public service monitoring. Private-network relay can reuse the current
operator-controlled trust boundary while defining a narrower endpoint discovery
and setup contract first.

## Scope

- Keep `live-loopback` as the product default.
- Keep self-hosted relay as the ready explicit setup/debug path.
- Select private-network relay as the next local mode-planning target.
- Keep managed relay deferred until separate operations and trust evidence are
  designed.
- Add a repeatable planning check that records the next local slice.

## Non-Goals

- Do not implement Tailscale/private-network transport in this slice.
- Do not make relay broadly user-selectable.
- Do not introduce a managed relay control plane.
- Do not change the self-hosted relay readiness gate.

## Follow-Up

Private-network relay setup contract, runtime guardrails, operator evidence,
visible import path, connection controls, and approval flow evidence are
complete:

- endpoint discovery contract;
- operator setup and authentication boundary;
- PWA setup preflight for private-network endpoints;
- daemon runtime guardrails that keep public `ws://` blocked.
- CLI-emitted setup JSON, PWA private runtime preflight, and frame roundtrip
  operator evidence.
- PWA visible private-network import/status path.
- PWA private-network connect/disconnect controls with setup-derived browser
  WebSocket connect evidence.
- Private-network approve/reject browser evidence with daemon-side response
  delivery.

The next slice is managed relay control-plane contract.

## Verification

```powershell
npm run check:pwa-relay-next-mode-planning
npm run check:pwa-relay-hosted-readiness
npm run test:pwa
git diff --check
```
