# 2026-07-01 Remaining Work Priority

## 목적

현재 repo는 v0.3.3 Windows GUI release smoke, Android/F-Droid local preflight,
RA/PWA live transport/backend/PWA UX/P4a evidence까지 완료했다. 이 문서는 남은
작업을 우선순위로 고정해 다음 세션이 바로 이어갈 수 있게 한다.

## 2026-08-15 현재 우선순위 갱신 — 외부 릴리스 blocker 0

repo admin이 Android signing secret 4개를 등록해 마지막 외부 게이트가 풀렸다.
`scripts/check-release-followup.ps1 -RunMsiBuild -FdroidBuildEvidencePath ...` 결과가
`closeout.canCloseDocs=true`, `blockedItems=[]`이고 ready 항목은
`msi`·`androidSigningSecrets`·`fdroidBuild` 셋 전부다. MSI는 2026-08-15 재빌드분
(SHA256 `0e6f224be4de44ead239fc592abc61b4c759067cdf345e9c0e427cb303458366`).
`releaseTagAction`/`assetAction`은 `unchanged`라 기존 태그·자산은 그대로 둔다.

**따라서 P1 external은 종료됐다.** 남은 즉시 작업은 실기기 `SM-F956N` openai 실 응답
검증(device-only)이며, 그다음이 macOS/Xcode가 필요한 iOS scaffold다.

| 우선순위 | 작업 | 완료 조건 | 블로커/주의 |
|---|---|---|---|
| P1 device | Android 실기기 openai 실 응답 검증 | DEBUG APK + 실 config로 자연어 입력 → openai 실 응답, 제안만(§3-11), fail-soft(§3-3), logcat api_key 미노출, 핸들 재사용 | 실기기 + 실 API 키 필요. 키는 문서·로그·스크린샷에 남기지 않고 검증 후 기기에서 삭제 |
| P2 local | iOS TestFlight SwiftUI REPL scaffold | 아래 표와 동일 | macOS/Xcode 필요 |
| P3 | Enterprise/security hardening | 아래 표와 동일 | 외부 blocker가 해소됐으므로 이제 재평가 가능 |

## (이력) 2026-08-01 우선순위 갱신

Android real-device smoke capture와 post-Android release follow-up recheck까지
완료했다. 2026-07-24 Windows-native Rust/MSVC + Tauri-managed WiX 3.14로
MSI 실제 빌드·경로·SHA256 evidence를 확보해 `msi`는 ready가 됐다.
2026-08-01 같은 host에서 `-RunMsiBuild`와
`artifacts/fdroid-container-build/fdroid-build-evidence.json`을 함께 넘긴
`scripts/check-release-followup.ps1`을 재실행해 `msi`와 `fdroidBuild`가 ready임을
재확인했다. release follow-up은 여전히 blocked이며 남은 blocked item은
`androidSigningSecrets` 하나다. Managed Relay/M2 local path는
operator setup production closeout까지 완료됐고, `live-loopback`은 계속 product
default다. Android/mobile local track의 imported reader metadata, UTF-8 preview
boundary polish, SAF export, selected-file helpers, Termux shared staging
diagnostics, real-device smoke capture는 모두 완료됐으므로, 외부 blocker 해소 전
새 로컬 작업은 별도 계획으로 먼저 범위를 정한다. PM-4 iOS/iPadOS research는
2026-07-06에 policy/product boundary, Rust-side common mobile JSON bridge,
Swift/Objective-C용 C ABI surface까지 닫았고, 다음 local implementation candidate는
macOS/Xcode-hosted TestFlight SwiftUI `shellcore` REPL scaffold다.

현 시점의 남은 작업은 다음 순서로 본다.

| 우선순위 | 작업 | 완료 조건 | 블로커/주의 |
|---|---|---|---|
| ~~P1 external~~ **완료(2026-08-15)** | Android signing secrets | GitHub repository secret names와 `.github/workflows/release.yml` references가 실제 `AI_TERMINAL_ANDROID_*` signing secret set과 일치하고 release follow-up evidence에서 `androidSigningSecrets` blocker가 사라짐 → **충족** | secret 값은 문서/로그에 기록하지 않는다(등록 시에도 읽지 않았다) |
| P2 local | iOS TestFlight SwiftUI REPL scaffold | PM-4 boundary, common JSON bridge, C ABI surface에 맞춰 SwiftUI REPL, app-private/document-picker workspace, pure/builtin command subset, unknown/external command fail-closed evidence 확보 | 실제 구현/빌드는 macOS/Xcode/iOS project 환경 필요. iOS 기본 약속은 Linux terminal이 아니라 constrained local structured terminal |
| P3 | Enterprise/security hardening | fleet/enterprise policy와 broader security hardening 계획 재정렬 | release follow-up 외부 blocker 해소 뒤 재평가 |

바로 실행할 검증:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\check-release-followup.ps1 -RunMsiBuild -FdroidBuildEvidencePath .\artifacts\fdroid-container-build\fdroid-build-evidence.json
npm run export:release-followup-evidence-packet
npm run check:pwa-relay-managed-runtime-operator-setup-production-closeout
npm run check:pwa-relay-next-mode-planning
```

## 현재 완료 기준

- Windows GUI: portable zip + NSIS installer smoke green. 2026-07-24 MSI 실제 build/path/SHA256 evidence도 ready.
- Android/F-Droid: local input/metadata/signing throwaway/activation dry-run green. Docker-local `fdroid build` evidence ready. 2026-08-15 실제 GitHub signing secrets 4개 등록 완료 → release follow-up closeout `canCloseDocs=true`.
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
- Release follow-up evidence packet: `npm run export:release-followup-evidence-packet` exports a secret-free JSON/Markdown handoff packet with current ready MSI/F-Droid evidence and the remaining Android signing operator action.
- Session closeout handoff: `docs/superpowers/plans/2026-07-01-session-closeout-handoff.md` records the final PR/merge handoff, validation commands, known external blockers, and next-session start procedure.
- Android imported document reader metadata: `docs/superpowers/plans/2026-07-05-android-imported-document-reader-metadata.md` extends imported/opened document results with content kind, byte count, preview bytes read, and preview line count. Binary or non-UTF-8 imported files reopen as metadata summaries instead of rendering raw bytes, while outside-workspace reopen remains rejected.
- Android UTF-8 preview boundary polish: `docs/superpowers/plans/2026-07-06-android-utf8-preview-boundary-polish.md` keeps valid UTF-8 text previews alive when the byte limit cuts a trailing multi-byte character, while preserving binary/non-UTF-8 fallback for invalid bytes before the boundary.
- Android workspace document export affordance: `Export Last` copies the most recent imported app-private workspace document to a user-selected SAF destination while reusing canonical workspace checks and keeping app-private paths out of shared storage by default.
- Android selected-file shellcore helpers: `List Files` and `Find Last` prepare shellcore-safe commands without auto-running them or exposing app-private absolute paths.
- Android Termux shared staging diagnostics: `Verify` records app-write and helper-marker diagnostics separately, requires the `ASH_SHARED_STAGING_OK` helper marker, and keeps external commands disabled on incomplete staging evidence.
- Android real-device smoke capture: `SM-F956N` manual UI/instrumentation evidence confirmed DocumentsUI import/open/export, selected-file helpers, Termux staging app-write/helper-marker, and `external / staging` state.
- Release follow-up evidence recheck: 2026-08-01 direct `scripts/check-release-followup.ps1 -RunMsiBuild -FdroidBuildEvidencePath .\artifacts\fdroid-container-build\fdroid-build-evidence.json` run reports `msi` and `fdroidBuild` ready, `androidSigningSecrets` blocked, and `closeout.canCloseDocs=false`; external operator packet was regenerated.
- iOS/iPadOS research boundary: `docs/superpowers/plans/2026-07-06-ios-ipados-local-terminal-research-boundary.md` fixes the App Review/TestFlight boundary, self-contained `shellcore`, app container/document picker workspace, policy-safe command subset, and excluded Linux/userland/downloaded-code/process promises.
- iOS mobile common JSON bridge: `docs/superpowers/plans/2026-07-06-ios-mobile-common-json-bridge.md` moves state/eval JSON handling into common Rust `mobile` helpers, keeps Android JNI as a thin wrapper, and gates `mobile_jni` to Android targets.
- iOS mobile C ABI bridge: `docs/superpowers/plans/2026-07-06-ios-mobile-c-abi-bridge.md` exposes initial state, eval, and free functions for future Swift/Objective-C wrappers while preserving JSON-in/JSON-out and structured error results.
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
- Private-network relay runtime guardrails: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-private-network-runtime-guardrails.md`, daemon `--relay-deployment-mode private-network`, `privateNetworkName` setup JSON emission, `relayPrivateNetworkRuntimeSetupPreflight()`, and `npm run check:pwa-relay-private-network-runtime-guardrails` add runtime setup boundaries while keeping self-hosted/default paths unchanged.
- Private-network relay operator evidence: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-private-network-operator-evidence.md` and `npm run smoke:pwa-relay-private-network-operator-evidence` capture CLI-emitted private-network setup JSON, PWA private runtime preflight evidence, setup-derived frame roundtrip evidence, and public `ws://` blocking.
- Private-network relay visible import path: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-private-network-visible-import-path.md`, `parseRelayPrivateNetworkRuntimeSetupInput()`, the PWA Relay tab private-network status block, and `npm run smoke:pwa-relay-private-network-visible-import` add a visible import/status path while keeping self-hosted connect behavior unchanged.
- Private-network relay connection controls: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-private-network-connection-controls.md`, `relayPrivateNetworkCompanionEndpointLoopFromSetup()`, private-network PWA connect/disconnect controls, and `npm run smoke:pwa-relay-private-network-connection-controls` add browser connect evidence while keeping self-hosted controls unchanged.
- Private-network relay approval flow evidence: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-private-network-approval-flow-evidence.md` and `npm run smoke:pwa-relay-private-network-approval-flow-evidence` capture private-network approve/reject browser evidence and daemon-side response delivery.
- Private-network relay runbook closeout: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-private-network-runbook-closeout.md`, `docs/relay-self-hosted-runbook.md`, and `npm run check:pwa-relay-deployment-runbook` close the private-network evidence map while keeping self-hosted readiness explicit.
- Managed relay operations planning: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-operations-planning.md`, `relayManagedOperationsPlan()`, and `npm run check:pwa-relay-managed-operations-planning` define the required operations contracts before managed relay implementation can start.
- Managed relay control-plane contract: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-control-plane-contract.md`, `relayManagedControlPlaneContract()`, and `npm run check:pwa-relay-managed-control-plane-contract` define managed relay roles, tenant/session boundaries, operator-visible state, audit constraints, and prohibited control-plane data.
- Managed relay abuse retention policy: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-abuse-retention-policy.md`, `relayManagedAbuseRetentionPolicy()`, and `npm run check:pwa-relay-managed-abuse-retention-policy` define tenant-scoped rate limits, abuse signals, retention windows, deletion requirements, and support workflow constraints.
- Managed relay payload confidentiality plan: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-payload-confidentiality-plan.md`, `relayManagedPayloadConfidentialityPlan()`, and `npm run check:pwa-relay-managed-payload-confidentiality-plan` define managed relay as payload-blind, prohibit payload/command/context visibility, and limit relay visibility to routing metadata plus aggregate health.
- Managed relay verifier-key operations policy: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-verifier-key-operations-policy.md`, `relayManagedVerifierKeyOperationsPolicy()`, and `npm run check:pwa-relay-managed-verifier-key-operations-policy` define public verifier-key-only distribution, private signing-key exclusion, rotation overlap, revocation fail-closed behavior, and key id/version audit boundaries.
- Managed relay billing/quota policy: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-billing-quota-policy.md`, `relayManagedBillingQuotaPolicy()`, and `npm run check:pwa-relay-managed-billing-quota-policy` define tenant-scoped quota enforcement, metered usage dimensions, aggregate-only tenant usage evidence, and billing records without payloads or secrets.
- Managed relay runtime readiness gate: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-runtime-readiness-gate.md`, `relayManagedRuntimeReadinessGate()`, and `npm run check:pwa-relay-managed-runtime-readiness-gate` aggregate runtime blockers, define minimum green evidence, and now report a green gate after billing/abuse boundary review completion.
- Managed relay payload-blind frame encryption spike: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-payload-blind-frame-encryption-spike.md`, `relayManagedPayloadBlindFrameEncryptionSpike()`, managed AES-GCM encrypted frame helpers, and `npm run check:pwa-relay-managed-payload-blind-frame-encryption-spike` prove route-visible state stays payload-blind while request/response frames decrypt only at the client endpoint boundary.
- Managed relay client key agreement runtime smoke: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-client-key-agreement-runtime-smoke.md`, `managedRelayDeriveSessionPayloadKeyHex()`, `relayManagedClientKeyAgreementRuntimeSmoke()`, and `npm run check:pwa-relay-managed-client-key-agreement-runtime-smoke` prove daemon/companion endpoints derive the same session-bound payload key while route-visible public metadata cannot derive it.
- Managed relay metadata minimization review: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-metadata-minimization-review.md`, `relayManagedMetadataMinimizationReview()`, and `npm run check:pwa-relay-managed-metadata-minimization-review` prove route/control/billing/support/audit metadata allowlists exclude raw ciphertext, payload keys, shared secrets, private keys, command text, and context data.
- Managed relay public verifier-key registry runtime smoke: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-public-verifier-key-registry-runtime-smoke.md`, `relayManagedPublicVerifierKeyRegistryRuntimeSmoke()`, Ed25519 public verifier registry helpers, and `npm run check:pwa-relay-managed-public-verifier-key-registry-runtime-smoke` prove tenant/key-id/key-version public verifier lookup and ticket verification without private signing keys or HMAC secrets in the managed relay boundary.
- Managed relay revocation and rotation propagation smoke: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-revocation-and-rotation-propagation-smoke.md`, `relayManagedRevocationAndRotationPropagationSmoke()`, verifier registry snapshot helpers, and `npm run check:pwa-relay-managed-revocation-and-rotation-propagation-smoke` prove active/rotating overlap, retiring/revoked fail-closed behavior, registry snapshot propagation, and tenant/key-id/key-version/snapshot audit metadata.
- Managed relay tenant session registration quota smoke: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-tenant-session-registration-quota-smoke.md`, `createManagedRelayTenantSessionRegistrationQuotaState()`, `evaluateManagedRelayTenantSessionRegistrationQuota()`, `relayManagedTenantSessionRegistrationQuotaSmoke()`, and `npm run check:pwa-relay-managed-tenant-session-registration-quota-smoke` prove tenant registration quota preflight accepts within-limit registrations, rejects exhausted or ineffective windows before session creation, preserves payload/secret-free quota audit metadata, and separates billing deltas from abuse/rate-limit signals.
- Managed relay active session and byte quota smoke: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-active-session-and-byte-quota-smoke.md`, `createManagedRelayActiveSessionAndByteQuotaState()`, `evaluateManagedRelayActiveSessionAndByteQuota()`, `relayManagedActiveSessionAndByteQuotaSmoke()`, and `npm run check:pwa-relay-managed-active-session-and-byte-quota-smoke` prove tenant/daemon active-session ceilings plus relay frame/byte quota preflight before session activation or routing, while preserving payload/secret-free usage audit metadata and separating billing deltas from abuse/rate-limit signals.
- Managed relay tenant aggregate usage export smoke: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-tenant-aggregate-usage-export-smoke.md`, `createManagedRelayTenantAggregateUsageExport()`, `relayManagedTenantAggregateUsageExportSmoke()`, and `npm run check:pwa-relay-managed-tenant-aggregate-usage-export-smoke` prove aggregate tenant usage exports include session registration, active session, relay frame, relay byte, invalid ticket, and quota denial counters while excluding payloads/secrets and separating billing usage from abuse signals.
- Managed relay support redaction/access review evidence: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-support-redaction-access-review-evidence.md`, `createManagedRelaySupportRedactionAccessReview()`, `relayManagedSupportRedactionAndAccessReviewEvidence()`, and `npm run check:pwa-relay-managed-support-redaction-access-review-evidence` prove redacted aggregate-only support views require hashed identifiers, tenant-admin approval, and time-bounded audited access while rejecting raw identifiers and payload/secret data.
- Managed relay billing/abuse boundary review: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-billing-abuse-boundary-review.md`, `createManagedRelayBillingAbuseBoundaryReview()`, `relayManagedBillingAbuseBoundaryReview()`, and `npm run check:pwa-relay-managed-billing-abuse-boundary-review` prove billing usage and abuse signals remain separately reviewed, support evidence is not a billing source, tenant aggregate usage exports remain payload-free, and the managed runtime readiness gate is green while `selectedRuntime` remains deferred.
- Managed relay runtime implementation plan: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-runtime-implementation-plan.md`, `relayManagedRuntimeImplementationPlan()`, and `npm run check:pwa-relay-managed-runtime-implementation-plan` define the managed service boundary, implementation phases, exposure gates, and regression checks while keeping `selectedRuntime=deferred`, `runtimeDefault=not-selected`, and product default `live-loopback`.
- Managed relay runtime service scaffold: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-runtime-service-scaffold.md`, `createManagedRelayRuntimeServiceScaffold()`, `relayManagedRuntimeServiceScaffold()`, and `npm run check:pwa-relay-managed-runtime-service-scaffold` define a scaffold startup contract with endpoint mode disabled, public bind off, PWA exposure disabled, aggregate-only health, and live-loopback rollback.
- Managed relay runtime control-plane contract wiring: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-runtime-control-plane-contract-wiring.md`, `createManagedRelayRuntimeControlPlaneContractWiring()`, `relayManagedRuntimeControlPlaneContractWiring()`, and `npm run check:pwa-relay-managed-runtime-control-plane-contract-wiring` wire tenant identity, session registration, public verifier-key lookup, quota preflight, and payload-free audit contracts while keeping route runtime not wired, PWA exposure disabled, and live-loopback rollback.
- Managed relay runtime encrypted frame routing: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-runtime-encrypted-frame-routing.md`, `createManagedRelayRuntimeEncryptedFrameRouting()`, `routeManagedRelayRuntimeEncryptedFrame()`, `relayManagedRuntimeEncryptedFrameRouting()`, and `npm run check:pwa-relay-managed-runtime-encrypted-frame-routing` route only validated encrypted frames while keeping ciphertext hex, nonce hex, payload keys, plaintext payloads, command/context data, and approval payloads out of route-visible surfaces.
- Managed relay runtime quota and metering integration: `docs/superpowers/plans/2026-07-04-ra-pwa-relay-managed-runtime-quota-and-metering-integration.md`, `createManagedRelayRuntimeQuotaAndMeteringIntegration()`, `routeManagedRelayRuntimeQuotaMeteredFrame()`, `relayManagedRuntimeQuotaAndMeteringIntegration()`, and `npm run check:pwa-relay-managed-runtime-quota-and-metering-integration` enforce active-session, frame, and byte quota before encrypted frame delivery while recording aggregate billing meter deltas separately from abuse signal deltas.
- Git 상태 기준(2026-07-06 재확인): `main...origin/main` 기준에서 작업을 이어간다. 다음 작업 전
  `git status --short --branch`와 `git log --oneline -5`를 다시 확인한다.

## 우선순위

| 우선순위 | 작업 | 완료 조건 | 블로커/주의 |
|---|---|---|---|
| P1 external | Android signing secrets | GitHub repository secret names + workflow references가 실제 release signing secret names와 일치 | secret 값은 읽거나 문서화하지 않는다 |
| P2 local | iOS TestFlight SwiftUI REPL scaffold | self-contained iOS REPL bound to common mobile JSON bridge/C ABI, container/document picker workspace, policy-safe command subset, external command fail-closed evidence | macOS/Xcode/iOS project 환경 필요. Linux terminal/userland promise 금지 |
| P3 | Enterprise/security hardening | fleet/enterprise policy, broader security hardening | release follow-up 외부 blocker 해소 뒤 재평가 |

## 바로 하지 않을 것

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸지 않는다. GUI 자산은 `ai-terminal.exe`다.
- PWA private key를 export 가능하게 바꾸지 않는다. 자동화를 위해 제품 보안 경계를 낮추지 않는다.
- Android 기본 실행 경계를 Termux/userland 직접 실행으로 바꾸지 않는다. Termux는 explicit opt-in bridge다.
- iOS/iPadOS를 Linux terminal, package manager, Termux-equivalent userland로 설명하지 않는다.
- P4a smoke를 P4b 완료로 간주하지 않는다. P4b는 실제 browser/operator 왕복 evidence가 필요하다.

## 다음 작업 선택

P4b browser/operator evidence, PWA monitoring view, RA transport mode decision,
v0.3.3 release body 보강, release follow-up preflight/runbook/status/check
commands, Relay/M2 self-hosted readiness, private-network evidence map, managed
runtime readiness, managed runtime service/control/route/quota/support evidence,
managed PWA exposure, managed operator setup contract/import/browser/connection
controls/session handshake/approval flow/runbook closeout, approval response
delivery boundary, endpoint delivery, endpoint browser evidence, daemon bridge
evidence, and managed operator setup production closeout are complete.

가장 높은 가치의 다음 release 작업은 외부 환경에서 runbook을 실행하는
**Android signing secrets 검증**이다. Windows MSI는 2026-07-24 실제 build/path/SHA256
evidence로 ready가 됐고, F-Droid Docker-local build evidence도 2026-08-01 combined
check에서 ready로 재확인됐다. 현재 개발 host에서 바로 확인 가능한 gate는
`scripts/check-release-followup.ps1 -RunMsiBuild -FdroidBuildEvidencePath .\artifacts\fdroid-container-build\fdroid-build-evidence.json`이며,
blocked items는 `androidSigningSecrets`뿐이다. 외부 blocker 해소 전
로컬에서 더 진행할 경우 다음 후보는 PM-4의 TestFlight SwiftUI `shellcore` REPL
scaffold다. 이번 세션에서는 Android imported document reader metadata, UTF-8
preview boundary polish, SAF import/export affordance, selected-file command helper,
Termux shared staging diagnostics, real-device smoke capture, post-Android release
follow-up recheck, PR #64 merge, iOS/iPadOS research boundary, common mobile JSON
bridge, C ABI bridge 문서화를 완료했다.
