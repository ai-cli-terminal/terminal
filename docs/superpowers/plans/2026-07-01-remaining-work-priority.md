# 2026-07-01 Remaining Work Priority

## 목적

현재 repo는 v0.3.3 Windows GUI release smoke, Android/F-Droid local preflight,
RA/PWA live transport/backend/PWA UX/P4a evidence까지 완료했다. 이 문서는 남은
작업을 우선순위로 고정해 다음 세션이 바로 이어갈 수 있게 한다.

## 현재 완료 기준

- Windows GUI: portable zip + NSIS installer smoke green. MSI는 후속 검토.
- Android/F-Droid: local input/metadata/signing throwaway/activation dry-run green. 실제 signing secrets와 buildserver evidence는 후속.
- RA/PWA companion: multi-device selection floor, live transport envelope, loopback HTTP/SSE endpoint, backend approval bridge, PWA live UX, P4a smoke evidence green.
- RA/PWA P4b browser/operator evidence: actual daemon + Playwright/Chrome PWA approve/reject smoke green.
- RA/PWA monitoring view: PWA Monitor tab shows connection, endpoint, device, pending/request/response counts, approve/reject counts, heartbeat, response timestamp, and event history.
- RA/PWA transport mode decision: live loopback is the default product path; native `device.sock` remains an internal/test substrate and future fallback candidate.
- v0.3.3 release body: published GitHub Release body now explains Windows GUI assets, CLI/runtime assets, unsigned Android APK, checksums, and known follow-ups without changing tag/assets.
- Release follow-up preflight: `scripts/smoke-release-followup-preflight.ps1` records combined MSI/Android signing/F-Droid buildserver readiness and current blockers.
- Release follow-up runbook: `docs/releases/release-followup-runbook.md` documents the external MSI/signing/F-Droid closure steps.
- MSI build evidence gate: `-RunMsiBuild` must produce a successful build command, generated `.msi`, and SHA256 hash before MSI follow-up is ready.
- F-Droid build evidence gate: supplied F-Droid build/buildserver evidence must now include expected app id, versionName, versionCode, successful result, and APK/buildserver artifact markers before `fdroidBuild.status` becomes `ready`.
- Android signing workflow gate: Android signing readiness now checks both GitHub repository secret names and `.github/workflows/release.yml` references to the same four `AI_TERMINAL_ANDROID_*` names without reading secret values.
- Release follow-up closeout gate: combined evidence now records `closeout.canCloseDocs`, `closeout.readyItems`, `closeout.blockedItems`, and unchanged tag/asset actions; docs should only be marked closed when `closeout.canCloseDocs=true`.
- Release follow-up status command: `npm run status:release-followup` summarizes the combined evidence, supports `-Json` for automation, and supports `-FailOnBlocked` for gates.
- Release follow-up status smoke: `npm run smoke:release-followup-status` validates the status command against synthetic blocked/ready evidence without depending on host MSI/secrets/F-Droid state.
- Release follow-up check command: `npm run check:release-followup` runs status smoke, combined preflight, and status summary in one operator-facing check.
- Session closeout handoff: `docs/superpowers/plans/2026-07-01-session-closeout-handoff.md` records the final PR/merge handoff, validation commands, known external blockers, and next-session start procedure.
- Relay/M2 daemon runtime loop: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-daemon-runtime-loop.md`에 따라 explicit `--transport relay` daemon startup이 setup-derived relay runtime bridge를 사용한다. 기본 product transport는 계속 `live-loopback`이다.
- PWA Relay approve/reject browser/operator evidence: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-approve-reject-evidence.md`와 `npm run smoke:pwa-relay-approve-reject-evidence`가 visible Relay tab connect, High command approve/reject, `received=2`, `sent=2`, `approved=1`, `rejected=1`, `pending=0` evidence를 기록한다.
- Self-hosted relay deployment runbook: `docs/relay-self-hosted-runbook.md`와 `docs/superpowers/plans/2026-07-04-ra-pwa-relay-self-hosted-deployment-runbook.md`가 self-hosted relay service contract, local/manual staging, observability, failure-mode evidence, rollback, production blockers를 문서화한다.
- Relay hosted/WSS readiness gate: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-hosted-wss-readiness-gate.md`와 `npm run check:pwa-relay-hosted-readiness`가 PWA `wss://` setup readiness와 daemon `wss://` runtime blocker를 분리해 기록한다.
- Daemon WSS relay runtime support: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-daemon-wss-runtime-support.md`에 따라 `remote,tls` build에서 daemon relay ticket registration은 HTTPS/TLS를 사용하고 WebSocket upgrade도 `wss://` 위에서 수행한다. `remote` without `tls` build는 `wss://`에서 명확히 fail-closed된다.
- Production relay service artifact: `scripts/relay-self-hosted-service.mjs`, `docs/relay-self-hosted-deploy.md`, `docs/superpowers/plans/2026-07-04-ra-pwa-relay-production-service-artifact.md`, `npm run relay:self-hosted`, `npm run smoke:pwa-relay-service-artifact`가 self-hosted relay service entrypoint, deploy recipe, health/config surface, signed-ticket registration, daemon/companion routing, no payload/secret health evidence를 고정한다.
- Relay public-key ticket verification: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-public-key-ticket-signing.md`, Ed25519 public verifier key config, PWA signed-ticket metadata support, and `npm run smoke:pwa-relay-service-artifact` now prove hosted relay service verification without private signing key or HMAC secret material in the relay process.
- Relay payload trust decision: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-payload-trust-decision.md` records that the current self-hosted relay shape does not provide end-to-end payload confidentiality from the relay operator and is acceptable only for explicit self-hosted setup/debug use where the operator controls and trusts the relay service.
- Relay observability/retention evidence: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-observability-retention-evidence.md` and `npm run smoke:pwa-relay-service-artifact` now prove aggregate-only health observability and memory-only retention policy without payloads or secrets.
- Relay failure-mode evidence: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-failure-mode-evidence.md`, `npm run smoke:pwa-relay-service-artifact`, and `npm run smoke:pwa-relay-websocket-bridge` now cover bad tickets, bad connects, wrong sender frames, duplicate sequences, expired frame drops, and local bridge reconnect/session isolation evidence. Explicit self-hosted relay readiness is green while `live-loopback` remains the product default.
- Relay managed/private-network planning: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-private-network-planning.md` and `npm run check:pwa-relay-next-mode-planning` select private-network relay setup contract as the next local slice while managed relay stays deferred.
- Private-network relay setup contract: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-private-network-setup-contract.md`, `relayPrivateNetworkSetupContract()`, `relayPrivateNetworkSetupPreflight()`, and `npm run check:pwa-relay-private-network-contract` define the next private-network setup boundary while keeping `live-loopback` default and public `ws://` blocked.
- Git 상태 기준(2026-07-04 재확인): `develop...origin/develop` 기준에서 작업을 이어간다. 다음 작업 전
  `git status --short --branch`와 `git log --oneline -5`를 다시 확인한다.

## 우선순위

| 우선순위 | 작업 | 완료 조건 | 블로커/주의 |
|---|---|---|---|
| P1 local | Private-network relay runtime guardrails | Add daemon-side mode parsing/startup guardrails, setup JSON emission boundary, and evidence for private-network `wss://` plus localhost development without changing `live-loopback` default | Private-network setup contract is defined; managed relay stays deferred |
| P1 external | Windows MSI 재검토 | Runbook 절차대로 `smoke-release-followup-preflight.ps1 -RunMsiBuild`가 native Rust/MSVC/WiX host에서 successful build + generated MSI + SHA256 evidence 기록 | 현재 host는 MSI toolchain 부재로 blocked |
| P1 external | Android signing/buildserver | Runbook 절차대로 workflow references + GitHub signing secret names ready, 실제 `fdroid build`/buildserver evidence가 expected app/version/result/artifact marker를 포함 | throwaway keystore/local metadata green은 실제 릴리스 완료가 아님 |
| P3 | Android/mobile local terminal 후속 | SAF-backed staging UX, richer imported file readers, Termux bridge hardening | Android 기본 약속은 계속 shellcore-only |
| P4 | Enterprise/security hardening | fleet/enterprise policy, broader security hardening | Relay/M2와 release follow-up 이후 재평가 |

## 바로 하지 않을 것

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸지 않는다. GUI 자산은 `ai-terminal.exe`다.
- PWA private key를 export 가능하게 바꾸지 않는다. 자동화를 위해 제품 보안 경계를 낮추지 않는다.
- Android 기본 실행 경계를 Termux/userland 직접 실행으로 바꾸지 않는다. Termux는 explicit opt-in bridge다.
- P4a smoke를 P4b 완료로 간주하지 않는다. P4b는 실제 browser/operator 왕복 evidence가 필요하다.

## 다음 작업 선택

P4b browser/operator evidence, PWA monitoring view, RA transport mode decision,
v0.3.3 release body 보강, release follow-up preflight/runbook, MSI build
evidence gate, F-Droid build evidence gate, Android signing workflow gate,
release follow-up closeout gate, release follow-up status command,
release follow-up status smoke, release follow-up check command, Relay/M2
transport kickoff, relay setup UI, setup-derived endpoint loop, daemon transport
selection, daemon gate bridge helper, relay daemon runtime loop, PWA Relay
approve/reject browser/operator evidence, self-hosted relay deployment runbook,
hosted/WSS readiness gate, daemon WSS relay runtime support, production relay
service artifact/deploy recipe, Ed25519 public-key relay ticket verification,
explicit relay-operator trust decision, hosted observability/retention evidence는
완료됐고, failure-mode evidence까지 닫혀 explicit self-hosted relay readiness는
green이다. Relay managed/private-network planning도 완료되어 다음 로컬 slice는
private-network relay setup contract였고, 해당 setup contract도 완료됐다.
가장 높은 가치의 다음 release 작업은 외부 환경에서 runbook을 실행하는
**Windows MSI 재검토**와 **Android signing/buildserver evidence**다. 현재 개발 host에서
바로 진행 가능한 다음 로컬 작업은 **Private-network relay runtime guardrails**다.
