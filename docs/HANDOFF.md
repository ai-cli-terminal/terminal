# HANDOFF — ai-cli-terminal (2026-07-01)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`,
`docs/HISTORY.md`, `CHANGELOG.md`, `docs/INSTALL.md`, `docs/superpowers/` 아래
spec/plan 문서다. 이 파일은 재개 가이드와 다음 작업 우선순위만 압축한다.

## 1. 현재 상태 — v0.3.3 릴리스 완료

작업 repo는 `D:\workspace\terminal-project\terminal`. v0.3.3 릴리스 태그는
`main`의 `c3aa63a Release v0.3.3` 기준이며, 후속 개발은 `develop`
(`792e0ee feat(desktop): track pending apt launch selection (#59)` 기준)에서 이어지고 있다.

공개 릴리스: <https://github.com/ai-cli-terminal/terminal/releases/tag/v0.3.3>

v0.3.3은 Windows 사용자가 더블클릭해 여는 독립 GUI 터미널
`ai-terminal.exe`를 릴리스 자산으로 배포한다. `ai-windows-x86_64.exe`는
GUI가 아니라 CLI helper이며, 더블클릭 안내 문구는 GUI 자산
(`ai-terminal-windows-*.zip` 또는 `AI.Terminal_*_x64-setup.exe`)을 가리키도록
수정되어 있다.

릴리스 자산 핵심:

| 자산 | 역할 |
|---|---|
| `ai-terminal-windows-x86_64-pc-windows-msvc.zip` | Windows portable GUI package |
| `AI.Terminal_0.3.3_x64-setup.exe` | Windows NSIS installer |
| `ai-windows-x86_64.exe` | CLI helper `ai.exe` |
| `ash-windows-x86_64.exe` | CLI/runtime shell `ash.exe` |
| `ai-terminal-android-universal-unsigned.apk` | Android unsigned universal APK |
| Linux `ai`/`ash` binaries | Linux CLI/runtime assets |

## 2. v0.3.3 릴리스 산출물 검증 — 완료

2026-06-29에 GitHub Release에서 실제 공개 자산을 다시 내려받아 검증했다.
검증 디렉터리: `artifacts/release-v0.3.3-smoke` (작업 산출물, 커밋 대상 아님).

체크섬:

- `ai-terminal-windows-x86_64-pc-windows-msvc.zip`:
  `d46fdbf9a5e3557d40ea7d46184212b42513575e518cf886cbba720d93e70f24`
- `AI.Terminal_0.3.3_x64-setup.exe`:
  `89054f320280eb87336b769b4f621adc33ce798819f78ea2b1f92439b0b987cc`

Portable GUI smoke:

- 명령:
  `pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-gui.ps1 -PackageDir <extracted-msvc-package> -StartupTimeoutSeconds 30`
- 결과: `GUI_SMOKE_OK`
- Evidence:
  `artifacts/release-v0.3.3-smoke/zip-extracted/ai-terminal-windows-x86_64-pc-windows-msvc/gui-smoke-evidence.json`
- 확인 내용: package manifest checksum 4개 일치, visible `ai-terminal.exe` window,
  `ash.exe` child, 외부 terminal descendant 없음, transcript output, resize,
  Ctrl-C recovery, Ctrl-D exit, frontend selection/copy/paste/scrollback,
  GUI 내부 AI routing/safety gate/storage-audit 모두 통과.

NSIS installer smoke:

- 최초 실행은 `scripts/smoke-nsis.ps1`가 `WebView2Loader.dll`을 필수 파일로
  요구해 실패했다. MSVC/Tauri 릴리스 산출물은 별도 DLL 없이 동작하므로
  portable packaging과 동일하게 optional로 정정했다.
- 재실행 명령:
  `pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-nsis.ps1 -InstallerPath artifacts\release-v0.3.3-smoke\AI.Terminal_0.3.3_x64-setup.exe`
- 결과: `NSIS_SMOKE_OK`
- Evidence: `artifacts/nsis-install-smoke/nsis-smoke-evidence.json`
- 설치 파일: `ai-terminal.exe`, `ash.exe`, `ai.exe`, `uninstall.exe`
- Optional missing: `WebView2Loader.dll`
- install/uninstall exit code: `0`/`0`; installed GUI smoke도 통과.

## 3. 이번 후속 변경

- `scripts/smoke-nsis.ps1`: `WebView2Loader.dll`을 optional installed file로
  처리하고, 실제 설치된 optional 파일/누락 optional 파일을 evidence JSON에 기록한다.
- `docs/HANDOFF.md`: v0.3.0 중심의 stale 인계를 v0.3.3 릴리스/실 자산 smoke 기준으로 갱신.
- `docs/HISTORY.md`: 2026-06-29 v0.3.3 릴리스 자산 smoke 결과 추가.
- 2026-06-30 후속: `scripts/smoke-msi-preflight.ps1` 추가(MSI toolchain blocked evidence),
  `scripts/smoke-gui.ps1 -LaunchMode ShellOpen` 추가 및 Shell open-verb GUI evidence 확보,
  `cmdparse` 리다이렉트 파싱 공용화, `command_executed` audit payload/source 통일,
  RA-1 실제 UnixListener substrate helper/test 및 daemon-owned `device.sock` one-shot/repeated/queue-backed listener와
  `ai remote daemon` 시작 결선 착수, RA-2 `remote-devices.json` registry/검증 helper와
  `remote-daemon-key.json` key persistence, `ai remote pair` start/complete CLI 및 PWA pair payload/url 출력 추가,
  RA-3 High opt-in approval plan/response folding helper, queue-backed listener roundtrip test,
  `serve_with_remote`/`DaemonRuntime` 기반 실제 daemon gate path 결선, RA-4 context hash/recompute와
  `ai __gate` shell-origin context IPC 전달 추가.

## 4. 빌드·검증 환경 메모

- Rust 툴체인은 WSL(Ubuntu) 중심이다. Windows host에서는 PowerShell smoke,
  release asset download, NSIS install smoke를 실행했다.
- 일반 Rust 검증은 WSL에서:
  `MSYS_NO_PATHCONV=1 wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; <cmd>'`
- Android 실제 프로젝트는 `terminal/android`다. repo 루트 밖
  `terminal-project/android` 스텁과 혼동 금지.
- `artifacts/`는 smoke evidence 작업 디렉터리이며 커밋 대상이 아니다.
- `git add -A` 금지. 필요한 파일만 명시 stage한다.

## 5. 다음 작업 후보

1. **Release notes 후속**: v0.3.2 GitHub Release note에는 이미 v0.3.3으로
   대체됐다는 superseded 안내가 들어가 있다. 2026-07-01에 v0.3.3 public
   release body도 보강했다. 태그/자산은 수정하지 않았고, body는 Windows GUI
   asset, CLI/runtime asset, unsigned Android APK, checksum 검증, 남은 MSI/Android
   signing 후속을 설명한다. 원문은 `docs/releases/v0.3.3-release-body.md`.
2. **Explorer double-click evidence**: Windows Shell open-verb evidence는
   `artifacts/explorer-shell-open-smoke/gui-shell-open-evidence.json`에 확보했다.
   엄밀한 "사람이 Explorer에서 더블클릭" 영상/캡처가 필요하면 별도 수동 operator 단계로 캡처한다.
3. **Android/F-Droid 후속**: 2026-07-01 재검증 기준
   `:app:verifyFdroidReleaseInputs`, fdroid metadata lint/rewritemeta,
   GitHub signing preflight(throwaway keystore), fdroid activation dry-run은 모두 통과했다.
   `gh secret list`는 빈 목록이므로 실제 GitHub Android signing secrets 등록/검증과
   실제 `fdroid build`/buildserver 검증이 남아 있다.
4. **Windows MSI 후속**: MSI는 여전히 Windows-native Rust+MSVC+WiX packaging host에서
   재검토해야 한다. 현재 host preflight는 `MSI_PREFLIGHT_BLOCKED`이며 release gate는 portable zip + NSIS installer다.
5. **RA/PWA companion**: RA-1 substrate + daemon-owned queue-backed listener는 시작됐고
   native `device.sock` substrate/helper는 코드와 테스트에 남아 있다. 2026-07-01 P2c 이후
   `ai remote daemon`의 기본 승인 대기 경로는 live loopback endpoint가 소유한
   `DeviceListenerHandle`이다. RA-2 저장/CLI substrate로
   `remote-devices.json` registry, 등록 디바이스 기반 승인 응답 검증 helper,
   `remote-daemon-key.json` daemon key persistence, `ai remote pair` start/complete CLI, versioned
   `pair_payload_json`/`aiterminal://pair?...` 출력도 추가됐다.
  RA-3 orchestration helper는 High opt-in 명령을 registered-device approval plan으로 만들고
  queue-backed listener 응답을 nonce consume/validate 후 GateReply로 접으며, `serve_with_remote`를 통해
  실제 `handle_conn` daemon path에도 연결됐다. RA-4 context hash는 canonical cwd, allowlisted env,
  realpath target을 포함하고, `ai __gate`가 넘긴 shell-origin cwd/env를 기준으로 응답 검증 직전
  재계산된다. RA-5 첫 조각으로 static `pwa/` companion shell도 추가되어 `pair_payload_json` 또는
  `aiterminal://pair?payload=...`를 파싱하고, WebCrypto X25519/Ed25519 identity를 생성/복원하며,
  complete command를 생성한다. private CryptoKey는 IndexedDB에 non-extractable로 저장하고, matching key
  material이 있을 때만 identity를 복원한다. PWA는 `ApprovalRequestMsg` JSON 또는 `?approval=...` URL payload를 받으면
  masked command/context를 표시하고, 저장된 approval key로 approve/reject `ApprovalResponseMsg` JSON을 서명해 생성/복사한다.
  `ai remote pair`는 terminal QR을 출력하고, `--pwa-url <url>`이 있으면 해당 PWA URL에 payload를 붙인
  `pwa_pair_url`/`pwa_pair_qr`도 출력한다. `ai remote approval-url --request-json ...`도 추가되어
  `aiterminal://approve?...` 승인 URL/QR과 `--pwa-url <url>` 기반 `pwa_approval_url`/`pwa_approval_qr`를 생성한다.
  `ai remote approval-verify --request-json ... --response-json ... --device-id ...`는 PWA가 만든
  승인 응답 JSON을 `remote-devices.json`의 등록 디바이스 기준으로 Rust `approval::validate` 경계에서 재검증한다
  (테스트용 직접 pubkey 모드는 `--approval-pubkey-hex ...`).
  PWA approval 화면은 signed response와 함께 registry 기반 `ai remote approval-verify --device-id ...` 명령도 생성/복사한다.
  다음 작업 문서는 `docs/superpowers/plans/2026-07-01-ra-pwa-live-companion-next.md`다.
  2026-07-01에 multi-device selection floor를 먼저 닫아 `ai remote daemon --device-id <id>`와
  `ai remote devices`가 추가됐고, 이어서 live loopback endpoint/backend approval bridge/PWA live UX도 연결됐다.
  이후 실 companion 왕복 evidence, monitoring view, transport mode decision까지 완료됐다.
- 2026-07-01 후속: repo 루트 아래 실수로 생성된 trailing-space 디렉터리(`terminal\ `)를 제거해
  `git status`의 `could not open directory ' /'` 경고를 없앴고, `README.md`/`docs/INSTALL.md`를
  v0.3.3 Windows GUI 릴리스 자산 기준으로 갱신했다.
- 2026-07-01 검증: `scripts/smoke-msi-preflight.ps1`는 이 host에서 여전히
  `MSI_PREFLIGHT_BLOCKED`(Rust/Cargo, MSVC, WiX 부재)이고, Android/F-Droid local preflight들은
  green이다. RA queue-backed device listener는 per-request response channel + accept timeout으로
  보강해 timed-out request가 다음 승인 요청을 오염시키지 않게 했다.
- 2026-07-01 RA next-work 진행: `docs/superpowers/plans/2026-07-01-ra-pwa-live-companion-next.md`
  추가. P0/P1로 `DeviceRegistry::select_device`, `ai remote daemon --device-id <id>`,
  `ai remote devices`를 구현해 복수 등록 디바이스 상태에서도 명시 선택으로 승인 대상을 고정할 수 있다.
- 2026-07-01 RA/PWA live transport 계약 진행:
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-transport-contract.md` 추가.
  Rust `session::CompanionTransportMsg`와 PWA `live*Message`/`parseLiveTransportMessage`
  helper가 같은 JSON envelope(`hello`, `approval_request`, `approval_response`, `ping`, `pong`, `error`)와
  malformed message fail-closed 경계를 검증한다. browser endpoint/backend bridge/PWA live UX는 아래 후속으로 연결됐고,
  남은 gap은 end-to-end evidence다.
- 2026-07-01 RA/PWA live loopback endpoint 진행:
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-loopback-endpoint.md` 추가.
  `ai remote daemon`은 remote 빌드에서 dependency-free `127.0.0.1:<ephemeral>` HTTP/SSE endpoint를 열고,
  `PWA live endpoint`, `PWA message endpoint`, `PWA events endpoint`를 출력한다.
  `/health`는 endpoint metadata, `/events`는 typed SSE `ping`, `/message`는 shared
  `CompanionTransportMsg` POST를 처리한다. `hello`는 선택/등록 디바이스 id와 public key를 검증하고,
  malformed/unknown/incomplete request는 typed `error` envelope로 fail-closed 응답한다.
  PWA에는 `liveEndpointUrls`, `liveMessageRequest`, `postLiveTransportMessage` helper가 추가됐다.
- 2026-07-01 RA/PWA live approval bridge 진행:
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-approval-bridge.md` 추가.
  live endpoint가 기존 queue-backed `DeviceListenerHandle`을 소유하고, `serve_with_remote`는 그 listener로
  High opt-in gate request를 보낸다. valid `hello` 이후 `/events`는 pending `approval_request`를 SSE로 내보내고,
  `/message`의 matching `approval_response`는 원래 gate waiter를 깨운 뒤 기존 `finish_remote_gate_response`
  검증/nonce/replay 경계를 재사용한다. mismatched response는 409로 실패하고 pending request는 유지된다.
  native `device.sock` 경로는 substrate/test로 남아 있으므로 다음에는 fallback/flag 필요 여부를 결정하면 된다.
- 2026-07-01 RA/PWA PWA live UX 진행:
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-pwa-ux.md` 추가.
  static PWA는 출력된 live endpoint URL을 받아 `hello`를 보내고, `EventSource` `/events`에서
  `approval_request`를 받아 approval panel/queue에 렌더링한다. Approve/Reject는 기존 IndexedDB
  approval key로 서명한 뒤 live 연결이 있으면 `/message`로 `approval_response`를 POST하고,
  live 연결이 없으면 기존 manual signed-response/copy 흐름을 유지한다.
- 2026-07-01 RA/PWA P4a evidence harness 진행:
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-e2e-evidence.md`와
  `scripts/smoke-pwa-live-approval.ps1` 추가. 실행 결과
  `RA_PWA_LIVE_EVIDENCE_OK artifacts\ra-pwa-live-evidence\ra-pwa-live-evidence.json`.
  이 harness는 PWA live helper, live UI selector surface, Rust `companion_live` endpoint/bridge tests를
  한 번에 검증한다. 실제 브라우저에서 daemon에 연결해 High 명령을 approve/reject하는 evidence는 아직 남아 있다.
- 2026-07-01 RA/PWA P4b 문서화/사전점검 진행:
  `docs/TROUBLESHOOTING.md`,
  `docs/superpowers/plans/2026-07-01-remaining-work-priority.md`,
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-p4b-browser-evidence.md`,
  `scripts/smoke-pwa-live-browser-preflight.ps1` 추가. 실행 결과
  `RA_PWA_LIVE_BROWSER_PREFLIGHT_BLOCKED artifacts\ra-pwa-live-browser-preflight\ra-pwa-live-browser-preflight.json`.
  required files, Node, `node pwa/app.test.mjs`, PWA browser surface, WSL Rust toolchain,
  P4a harness는 모두 통과했다. 남은 blocker는 browser capture 환경이다:
  `playwright` package가 없고 `PATH`에서 Edge/Chrome/Chromium command를 찾지 못했다.
- 2026-07-01 RA/PWA P4b browser capture unblock 진행:
  `docs/superpowers/plans/2026-07-01-ra-pwa-live-browser-capture-unblock.md` 추가,
  root `package.json`/`package-lock.json`에 Playwright dev dependency 추가,
  `.gitignore`에 `/node_modules/` 추가, preflight가 common Chrome/Edge install path도 찾도록 수정.
  재실행 결과
  `RA_PWA_LIVE_BROWSER_PREFLIGHT_READY artifacts\ra-pwa-live-browser-preflight\ra-pwa-live-browser-preflight.json`.
  이제 남은 P4b 작업은 실제 daemon + browser/PWA approve/reject evidence 캡처다.
- 2026-07-01 RA/PWA P4b browser/operator evidence 완료:
  `scripts/smoke-pwa-live-browser-evidence.mjs`와 npm script
  `smoke:pwa-live-browser-evidence` 추가. 실행 결과
  `RA_PWA_LIVE_BROWSER_EVIDENCE_OK artifacts\ra-pwa-live-browser-evidence\ra-pwa-live-browser-evidence.json`.
  이 smoke는 WSL remote `ai` build, local PWA static server, Playwright/Chrome browser,
  disposable PWA identity, isolated `ai remote pair`, `ai remote daemon --device-id <id>`,
  PWA live connect, `ai remote arm --allow-high`, High command approve/reject를 한 번에 검증한다.
  screenshots와 transcript는 `artifacts\ra-pwa-live-browser-evidence\` 아래에 있다.
- 2026-07-01 RA/PWA monitoring view 완료:
  `docs/superpowers/plans/2026-07-01-ra-pwa-monitoring-view.md` 추가.
  PWA `Monitor` tab을 enabled로 바꾸고 connection/endpoint/device/pending/request/response/
  approve/reject/heartbeat/history를 표시한다. `npm run smoke:pwa-live-browser-evidence`
  재실행 결과 `RA_PWA_LIVE_BROWSER_EVIDENCE_OK`; evidence JSON의 `monitor` snapshot은
  `received=2`, `sent=2`, `approved=1`, `rejected=1`을 기록한다.
- 2026-07-01 RA/PWA transport mode decision 완료:
  `docs/superpowers/plans/2026-07-01-ra-pwa-transport-mode-decision.md` 추가.
  기본 product transport는 live loopback으로 고정했다. `ai remote daemon`은
  `PWA transport mode : live-loopback`을 출력하고,
  `npm run smoke:pwa-live-browser-evidence`는 이 mode를 assert한 뒤 evidence JSON의
  `transportMode`에 기록한다. native `device.sock`은 user-facing flag가 아니라
  내부/test substrate와 future fallback candidate로 유지한다.
- 2026-07-01 v0.3.3 release body 보강 완료:
  `docs/releases/v0.3.3-release-body.md`와
  `docs/superpowers/plans/2026-07-01-v033-release-body.md` 추가. GitHub Release
  `v0.3.3` body가 비어 있음을 확인한 뒤 `gh release edit v0.3.3 --notes-file ...`로
  body만 갱신했다. 태그와 asset은 변경하지 않았다.
- 2026-07-01 release follow-up preflight 추가:
  `scripts/smoke-release-followup-preflight.ps1`와 npm script
  `smoke:release-followup-preflight` 추가. 기존 MSI preflight, GitHub Android
  signing secret name check, F-Droid build/buildserver evidence path check를
  하나의 JSON으로 묶는다. secret 값은 읽거나 저장하지 않는다. 현재 host 실행 결과는
  Windows MSI toolchain 부재, GitHub Android signing secrets 부재, F-Droid
  build/buildserver evidence 미제공으로 blocked가 정상이다.
- 2026-07-01 release follow-up runbook 추가:
  `docs/releases/README.md`, `docs/releases/release-followup-runbook.md`,
  `docs/superpowers/plans/2026-07-01-release-followup-runbook.md` 추가. README의
  문서 표에서 release docs index로 접근할 수 있다. runbook은 Windows MSI,
  GitHub Android signing secret names, F-Droid build/buildserver evidence를 닫는
  외부 환경 절차를 정리하고 secret 값 예시는 포함하지 않는다.
- 2026-07-01 F-Droid build evidence gate 보강:
  `scripts/smoke-release-followup-preflight.ps1`가 이제 supplied F-Droid
  evidence 파일의 존재뿐 아니라 `dev.aiterminal.android`, `0.3.3`, `303`,
  성공 status/result, APK/buildserver artifact marker를 확인한다. 작업 문서는
  `docs/superpowers/plans/2026-07-01-fdroid-build-evidence-gate.md`다.
- 2026-07-01 Android signing workflow gate 보강:
  같은 preflight가 이제 repository secret names와 `.github/workflows/release.yml`
  안의 네 `AI_TERMINAL_ANDROID_*` secret reference를 함께 확인한다. evidence에는
  secret 이름과 `updatedAt`만 남기며 값은 읽지 않는다. 작업 문서는
  `docs/superpowers/plans/2026-07-01-android-signing-workflow-gate.md`다.
- 2026-07-01 MSI build evidence gate 보강:
  `scripts/smoke-msi-preflight.ps1 -RunBuild`는 이제 build command 성공,
  generated `.msi`, SHA256 hash가 모두 있어야 `ready`다. combined preflight도
  `-RunMsiBuild` 없이 MSI follow-up을 complete로 보지 않는다. 작업 문서는
  `docs/superpowers/plans/2026-07-01-msi-build-evidence-gate.md`다.
- 2026-07-01 release follow-up closeout gate 보강:
  `scripts/smoke-release-followup-preflight.ps1` evidence에 `closeout` 객체를
  추가했다. `closeout.requiredEvidence`는 `msi`, `androidSigningSecrets`,
  `fdroidBuild`를 고정하고, `closeout.canCloseDocs=true`와
  `closeout.blockedItems=[]`가 같이 기록될 때만 후속 문서를 완료 상태로 닫는다.
  `releaseTagAction`/`assetAction`은 별도 release decision 없이는 `unchanged`다.
  작업 문서는 `docs/superpowers/plans/2026-07-01-release-followup-closeout-gate.md`다.
- 2026-07-01 release follow-up status command 추가:
  `scripts/show-release-followup-status.ps1`와 npm script
  `status:release-followup`을 추가했다. 기존 evidence를 사람이 읽는 status로 요약하고,
  `-Refresh`, `-Json`, `-FailOnBlocked`, `-RunMsiBuild`,
  `-FdroidBuildEvidencePath`를 지원한다. 작업 문서는
  `docs/superpowers/plans/2026-07-01-release-followup-status-command.md`다.

## 5.1. 바로 다음 RA/PWA 작업

1. **Release follow-up**: 먼저 `npm run status:release-followup`로 현재 blocker를 확인한 뒤, `docs/releases/release-followup-runbook.md`를 따라 외부 host에서 `npm run smoke:release-followup-preflight` blocker를 닫고 Windows MSI native host 및 Android signing/buildserver evidence를 정리한다. MSI는 `-RunMsiBuild`와 generated MSI/hash evidence가 필요하고, Android signing은 workflow reference와 repository secret names가 모두 ready여야 하며, F-Droid evidence는 app id/version/result/artifact marker를 포함해야 `fdroidBuild.status=ready`가 된다. 후속 문서 완료 처리는 combined evidence의 `closeout.canCloseDocs=true`와 `closeout.blockedItems=[]`를 확인한 뒤 진행한다.
2. **Relay/M2**: local live loopback default를 유지한 상태에서 relay/Tailscale/WebSocket transport를 별도 설계로 착수한다.

## 6. 비목표

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸는 것은 비목표다. 이 파일은 CLI helper다.
- GUI 완료 기준을 Windows Terminal/PowerShell/Git Bash에서 `ash.exe` 수동 실행으로
  되돌리지 않는다. `ash.exe`는 GUI 내부 runtime 및 별도 CLI asset이다.
