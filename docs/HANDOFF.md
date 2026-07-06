# HANDOFF — ai-cli-terminal (2026-07-02)

다음 세션 이관 문서. 권위 기록은 `docs/TASK.md`, `docs/WORKFLOW.md`,
`docs/HISTORY.md`, `CHANGELOG.md`, `docs/INSTALL.md`, `docs/superpowers/` 아래
spec/plan 문서다. 이 파일은 재개 가이드와 다음 작업 우선순위만 압축한다.

## 0. 2026-07-06 현재 재개점

- PR #64 (`v0.3.4 Android reader follow-up hardening`)는 `main`에 squash
  merge됐고 merge commit은 `60ac71c`다. 현재 로컬 `main`은 `origin/main`과
  동기화됐다.
- 외부 release follow-up blocker는 그대로다: Windows MSI evidence,
  Android signing secrets, F-Droid build/buildserver evidence.
- 외부 blocker를 이 host에서 닫을 수 없어 PM-4 iOS/iPadOS research를 진행했고,
  `docs/superpowers/plans/2026-07-06-ios-ipados-local-terminal-research-boundary.md`
  에서 정책/제품 경계를 고정했다.
- iOS/iPadOS 약속은 self-contained `shellcore`, app-private workspace,
  explicit document import/export, pure/builtin command subset이다. Linux terminal,
  package manager, Termux-equivalent userland, downloaded functionality-changing
  code, arbitrary subprocess/PTY/background daemon은 약속하지 않는다.
- 다음 구현 후보는 TestFlight self-contained `shellcore` REPL spike다. 이 repo에는
  아직 iOS/Xcode project가 없고 현재 host는 Windows이므로 실제 TestFlight build
  evidence는 macOS/Xcode 환경에서 시작해야 한다.

## 1. 2026-07-02 세션 closeout

최신 Relay production-readiness 작업 문서는
`docs/superpowers/plans/2026-07-05-ra-pwa-relay-managed-runtime-operator-setup-production-closeout.md`다. Relay/M2는
self-hosted WebSocket prototype 기반, PWA Relay setup UI, setup-derived endpoint loop,
daemon `--transport relay` setup issuance, relay gate bridge helper, 실제 daemon relay
runtime loop 결선, PWA Relay tab 기반 approve/reject browser/operator evidence까지
닫았고, `docs/relay-self-hosted-runbook.md`에 self-hosted relay deployment runbook도
추가했다. `npm run check:pwa-relay-hosted-readiness`는 PWA `wss://` setup readiness와
daemon `remote,tls` WSS runtime readiness, production relay service artifact, Ed25519
public verifier key readiness, explicit relay-operator trust decision을 분리해
기록하고, aggregate-only observability/retention 및 failure-mode evidence도
닫았다. Managed relay runtime readiness gate는 blocker audit로 닫혔고 payload-blind
encrypted frame envelope smoke, session-bound client-held key agreement smoke,
metadata minimization review, public verifier-key registry runtime smoke도
완료됐고 revocation/rotation propagation smoke와 tenant session registration quota smoke도 닫혔지만
active session and byte quota smoke, tenant aggregate usage export smoke,
support redaction/access review evidence, billing/abuse boundary review까지 닫혔다.
Managed runtime readiness gate는 `runtime-evidence-green`이고
`implementationCanStart=true`다. Managed relay runtime implementation plan과 runtime
service scaffold도 닫혀 service boundary, implementation phases, startup contract,
aggregate-only health surface, exposure gates, regression checks가 고정됐지만,
control-plane contract wiring까지 닫혀 tenant identity, session registration, public
verifier-key lookup, quota preflight, payload-free audit contract가 scaffold 위에
배선됐고, encrypted frame routing도 닫혀 route-visible allowlist와 expired/plaintext
fail-closed 경계가 고정됐다. Managed runtime quota/metering integration도 닫혀
active-session, frame, byte quota를 encrypted frame delivery 전에 평가하고,
aggregate billing meter delta와 abuse signal delta를 분리해 기록한다.
Managed runtime support/abuse operations integration도 닫혀 runtime-visible
support view는 aggregate-only/redacted/hash-only/audited access로 제한되고,
abuse operation counters는 billing source data로 재분류되지 않는다.
Managed runtime PWA exposure gate도 닫혀 managed relay는 PWA-visible
`explicit-opt-in` setup/copy surface로만 노출된다. Product default는 계속
`live-loopback`, `runtimeDefault`는 `not-selected`, endpoint auto-start는 disabled,
public bind는 off이며 실제 managed endpoint 시작은 operator-issued setup 이후
별도 경계로 남는다. Managed runtime browser/operator evidence도 닫혀 실제
Chromium desktop/mobile 캡처에서 Managed Relay 패널 visible, no mobile horizontal
overflow, explicit opt-in only copy, public bind off, endpoint auto-start disabled,
prohibited token/secret/raw identifier 미노출을 확인했다. Managed runtime operator
setup contract도 닫혀 operator-issued setup payload는 `wss://` endpoint,
metadata-only fields, hashed identifiers, manual connect, disabled endpoint
auto-start, public bind off, `live-loopback` rollback만 허용한다. Managed runtime
operator setup import preflight도 닫혀 PWA Relay tab은 operator setup payload를
파싱/검증한 뒤 원본 JSON을 숨기고 sanitized metadata summary만 표시한다.
Import는 endpoint를 시작하지 않고 connect도 열지 않는다. Managed runtime
operator setup browser evidence도 닫혀 Chromium desktop/mobile 캡처에서 import
state ready, 원본 JSON hidden, sanitized metadata-only summary, prohibited
setup field names/tickets/tokens/payloads/key material/raw identifiers/operator
setup text/support contact metadata 미노출, no mobile horizontal overflow,
endpoint auto-start disabled, public bind off, connect controls out of scope를
확인했다. Managed runtime operator setup connection controls도 닫혀 PWA Relay
tab은 ready import 이후에만 request/cancel managed connect controls를 열고,
manual request는 connection state를 갱신하지만 WebSocket을 만들지 않고 endpoint를
시작하지 않으며 public bind도 켜지 않는다. Managed runtime operator setup
session handshake도 닫혀 ready import와 manual request 이후에만 metadata-only
capability envelope을 만들고, PWA에는 `managed-cap:*` handle과 `sha256:*`
transcript hash만 표시한다. Capability envelope JSON, signed tickets, raw
tokens, payloads, private key material은 렌더링되지 않고 WebSocket/endpoint
시작도 없다. Managed runtime operator setup approval-flow evidence도 닫혀
ready session handshake 이후에만 session-capability-derived approval request를
기존 Approve panel에 로드하고, managed setup surface에는 source/context hash만
표시한다. Approve/Reject response는 manual signed-response copy 경계에 머물며
WebSocket/endpoint 시작도 없다. Managed runtime operator setup runbook closeout도
닫혀 `docs/relay-self-hosted-runbook.md`의 Managed Relay Operator Setup Evidence
Map이 setup contract, import preflight, browser evidence, connection controls,
session handshake, approval-flow evidence 명령을 연결한다. explicit self-hosted
relay readiness는 green이다. Managed runtime operator
setup approval response delivery boundary도 닫혀 signed approval response는
ready approval flow와 request-matching response 검증 뒤에만 delivery-ready가 되고,
delivery mode는 `manual-signed-response-copy-only`로 고정된다. Copy/verify
controls는 기존 Approve panel에 남고 managed setup surface는 response payload를
보여주지 않는다. WebSocket 생성, endpoint 시작, public bind 활성화는 여전히
없다. Managed runtime operator setup approval response endpoint delivery
evidence도 닫혀 operator-started endpoint, manual connect, valid session id,
client-held payload key가 있을 때만 approval response가 encrypted managed frame
route envelope로 daemon boundary까지 전달된다. Manual signed-response copy는
fallback으로 유지되고 payload key/ciphertext/approval response payload/private key
material은 evidence에 기록하지 않는다. Managed runtime operator setup approval
response endpoint browser evidence도 닫혀 PWA는 signed response 이후에만 endpoint
delivery control을 열고, browser/operator evidence는 endpoint delivery ready
state, encrypted route status, daemon receipt status, manual copy fallback을
보여주면서 route envelope/payload key/ciphertext/private key material은 렌더링하지
않는다. Managed runtime operator setup approval response daemon bridge evidence도
닫혀 endpoint-delivered approval response를 기존 daemon approval verification
boundary(`approval::validate`/`ai remote approval-verify`)로 검증하고 signature,
context hash, request match를 확인한다. Daemon bridge evidence는 route envelope,
payload key, ciphertext, raw token, private key material을 evidence/log surface에
재노출하지 않는다. Managed runtime operator setup production closeout도 닫혀
contract/import/browser/connection/session/approval/runbook/delivery/endpoint/
browser/daemon-bridge evidence chain을 하나의 local 완료 게이트로 묶었다.
다음 작업은 release follow-up external evidence closeout이다. 현재 host 기준
`npm run check:release-followup`는 `msi`, `androidSigningSecrets`, `fdroidBuild`
blocked items를 보고한다. 외부 작업자에게 넘길 handoff packet은
`npm run export:release-followup-evidence-packet`로 생성한다.

2026-07-05에 `develop`을 push하고 PR #62
(`Release follow-up and managed relay evidence closeout`)를 `main`에 squash
merge했다. merge commit은 `a66a9e9`이며 CI 4개(`fmt · clippy · test`,
`cargo audit`, `android JNI packaging`, `windows build + self-contained check`)가
통과했다. 이후 release follow-up gate를 재실행했지만 외부 blocker는 동일하게
남아 있다. 외부 blocker를 이 host에서 닫을 수 없어 로컬 P2 Android/mobile
track을 재개했고, `docs/superpowers/plans/2026-07-05-android-imported-document-reader-metadata.md`
slice에서 imported workspace document reader metadata를 진행했다. 이 slice는
import/open 결과에 content kind, byte count, preview bytes/lines metadata를
추가하고 binary/non-UTF-8 reopen을 raw byte rendering 대신 safe metadata summary로
처리한다.

2026-07-06에는 Android workspace document export affordance를 진행했다. `Export Last`는
마지막 imported app-private workspace 파일을 사용자가 고른 SAF document destination으로
복사한다. Transcript export는 `Export Log`로 분리했고, imported file export는
`Open Last` 옆의 별도 action으로 둔다. Export는 `Open Last`와 같은 canonical
workspace boundary를 재사용해 workspace 밖 path와 directory를 거부하며,
app-private path를 Termux나 shared storage에 자동 노출하지 않는다. Targeted 검증은
`gradle -p android :app:testDebugUnitTest --tests dev.aiterminal.android.WorkspaceDocumentsTest`
green이다.

같은 날 이어서 Android selected-file shellcore helper도 진행했다. `List Files`는
input에 `ls`를 준비하고, `Find Last`는 마지막 imported file을
workspace-relative `ls <dir> | where name == <file> | first 1` 명령으로 준비한다.
두 helper는 자동 실행하지 않고, app-private absolute path를 transcript/input에
노출하지 않으며, raw file read는 계속 bounded `Open Last` preview 경계에 둔다.
Targeted 검증은
`gradle -p android :app:testDebugUnitTest --tests dev.aiterminal.android.WorkspaceDocumentsTest --tests dev.aiterminal.android.TerminalViewModelTermuxTest`
green이다. 외부 release blocker가 계속 unavailable이면 다음 로컬 slice는
Termux shared staging diagnostics다.

Termux shared staging diagnostics도 이어서 닫았다. `Verify`는 이제
`termux staging app-write`와 `termux staging helper-marker`를 transcript에
분리해 기록한다. App validation은 shared staging root에 probe file을 쓰고
다시 읽은 뒤 삭제하며, helper smoke가 성공하더라도 `ASH_SHARED_STAGING_OK`
marker가 없으면 `Termux shared staging marker missing`으로 fail-closed하고
external commands를 켜지 않는다. Targeted 검증은
`gradle -p android :app:testDebugUnitTest --tests dev.aiterminal.android.TerminalViewModelTermuxTest`
green이다. 외부 release blocker가 계속 unavailable이면 다음 로컬 follow-up은
Android real-device smoke capture로 import/export, selected-file helpers,
Termux staging diagnostics를 같이 확인하는 것이다.

Android real-device smoke capture도 이어서 닫았다. `SM-F956N` /
`R3CX60P3R5K`가 authorized 된 뒤 debug APK install, Termux permission grant,
`connectedDebugAndroidTest` helper smoke, manual DocumentsUI import/open/export,
`List Files`/`Find Last` command preparation, and `Verify` staging diagnostics가
green이다. Manual UI transcript는 `ai-terminal-reader-smoke.txt` import preview,
`Open Last` safe reopen, SAF `Export Last`, `prepared command: ls`,
workspace-relative `ls "." | where name == "ai-terminal-reader-smoke.txt" | first 1`,
`termux staging app-write: ok`, `ASH_SHARED_STAGING_OK`,
`termux staging helper-marker: ok`, `external / staging`을 확인했다. 로컬
evidence는 ignored `artifacts/android-real-device-smoke/` 아래에 있다. 다음
우선순위는 다시 release follow-up external evidence closeout이다.

Release follow-up도 Android smoke 직후 재확인했다. 이 Codex PowerShell PATH에는
`npm`이 없어 `pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\check-release-followup.ps1`
를 직접 실행했고, 첫 sandbox run은 GitHub CLI config 접근이 막혀 escalated run으로
다시 확인했다. 결과는 여전히 `blocked`이며 `closeout.canCloseDocs=false`,
blocked items는 `msi`, `androidSigningSecrets`, `fdroidBuild`다. Escalated run
기준 `.github/workflows/release.yml`은 네 signing secret reference를 모두 갖고
있지만 repo secret names는 아직 비어 있다. 외부 operator packet은
`artifacts/release-followup-evidence-packet/` 아래에 최신 상태로 재생성했다.
`v0.3.4` 태그는 현재 `release/v0.3.4-android-reader` 브랜치의 조상이 아니므로,
태그/asset/release body는 별도 release decision 없이 바꾸지 않는다.

그 뒤 새 로컬 Android polish slice로 UTF-8 preview boundary를 닫았다. Import/open
preview byte limit이 한글 같은 multi-byte 문자 중간에서 끊겨도 유효한 UTF-8
prefix를 truncated text로 보여주며, boundary 이전 invalid UTF-8과 orphan
continuation byte는 계속 binary/non-UTF-8 metadata summary로 처리한다. 작업 문서는
`docs/superpowers/plans/2026-07-06-android-utf8-preview-boundary-polish.md`이고,
targeted 검증은
`gradle -p android :app:testDebugUnitTest --tests dev.aiterminal.android.WorkspaceDocumentsTest`
green이다. Sandbox run은 Android Gradle Plugin resolve 실패로 막혀 external
Gradle cache/network access로 재실행했다.

## 1. 현재 상태 — v0.3.3 릴리스 완료

작업 repo는 `D:\workspace\terminal-project\terminal`. v0.3.3 릴리스 태그는
`main`의 `c3aa63a Release v0.3.3` 기준이며, 후속 개발은 `develop`에서
이어지고 있다. 재개 시점의 정확한 기준은 `git status --short --branch`와
`git log --oneline -5`로 확인한다.

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
  한 번에 검증한다. 당시 남아 있던 실제 browser/operator evidence는 아래 P4b smoke로 완료됐다.
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
  이후 실제 daemon + browser/PWA approve/reject evidence 캡처까지 완료됐다.
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
  evidence 파일의 존재뿐 아니라 현재 release target인 `dev.aiterminal.android`,
  `0.3.4`, `304`,
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
- 2026-07-01 release follow-up status smoke 추가:
  `scripts/smoke-release-followup-status.ps1`와 npm script
  `smoke:release-followup-status`를 추가했다. synthetic blocked/ready evidence로
  status command의 text, `-Json`, `-FailOnBlocked` 계약을 검증한다. 작업 문서는
  `docs/superpowers/plans/2026-07-01-release-followup-status-smoke.md`다.
- 2026-07-01 release follow-up check command 추가:
  `scripts/check-release-followup.ps1`와 npm script `check:release-followup`을
  추가했다. status smoke, combined preflight, status summary를 한 번에 실행하고
  aggregate evidence를 `artifacts/release-followup-check/`에 기록한다. 작업 문서는
  `docs/superpowers/plans/2026-07-01-release-followup-check-command.md`다.
- 2026-07-01 session closeout handoff 추가:
  `docs/superpowers/plans/2026-07-01-session-closeout-handoff.md`에 PR/머지 후 다음
  세션이 바로 이어갈 수 있는 검증 명령, 남은 외부 환경 blocker, release follow-up
  closeout 순서를 모았다.

## 5.1. 바로 다음 RA/PWA 작업

1. **Release follow-up**: 먼저 `npm run check:release-followup`로 status smoke, combined preflight, status summary를 한 번에 확인하고, `npm run export:release-followup-evidence-packet`로 external operator packet을 생성한다. 이후 `docs/releases/release-followup-runbook.md`를 따라 외부 host에서 blocker를 닫고 Windows MSI native host 및 Android signing/buildserver evidence를 정리한다. MSI는 `-RunMsiBuild`와 generated MSI/hash evidence가 필요하고, Android signing은 workflow reference와 repository secret names가 모두 ready여야 하며, F-Droid evidence는 app id/version/result/artifact marker를 포함해야 `fdroidBuild.status=ready`가 된다. 후속 문서 완료 처리는 combined evidence의 `closeout.canCloseDocs=true`와 `closeout.blockedItems=[]`를 확인한 뒤 진행한다.
2. **Relay/M2**: `live-loopback`은 계속 product default다. Explicit self-hosted relay readiness는 `docs/superpowers/plans/2026-07-04-ra-pwa-relay-failure-mode-evidence.md`, `npm run smoke:pwa-relay-service-artifact`, `npm run smoke:pwa-relay-websocket-bridge`, `npm run check:pwa-relay-hosted-readiness` 기준으로 green이다. Managed relay planning/evidence slices through `docs/superpowers/plans/2026-07-05-ra-pwa-relay-managed-runtime-operator-setup-production-closeout.md` and `npm run check:pwa-relay-managed-runtime-operator-setup-production-closeout` are complete. The managed runtime readiness gate is green with `implementationCanStart=true`, managed relay is browser-verified as PWA-visible only for explicit opt-in setup/copy, the operator-issued setup payload contract is metadata-only with hashed identifiers, import preflight renders only sanitized metadata while keeping connect disabled, browser evidence confirms the original JSON/prohibited setup data is hidden with no mobile overflow, connection controls are manual request/cancel status controls that do not create WebSockets or start endpoints, session handshake shows only a capability handle plus transcript hash, approval-flow evidence loads a session-capability-derived request into the existing Approve panel without creating a WebSocket or starting an endpoint, the runbook links the managed operator setup evidence chain, approval response delivery boundary keeps copy/verify controls in the existing Approve panel only, endpoint delivery evidence proves an operator-started encrypted frame path with manual copy fallback, endpoint browser evidence shows the explicit delivery control/status without rendering route envelopes, payload keys, or ciphertext, daemon bridge evidence verifies the endpoint-delivered response through the existing approval validation boundary without logging route envelopes, payload keys, ciphertext, raw tokens, or private key material, and production closeout links the full local managed operator setup evidence chain. Product default remains `live-loopback`, public bind is off, endpoint auto-start is disabled, and `runtimeDefault` remains `not-selected`. Daemon gate bridge 문서는 `docs/superpowers/plans/2026-07-02-ra-pwa-relay-daemon-gate-bridge.md`이며, `decide_with_remote_relay_bridge`가 relay roundtrip으로 받은 approval response를 기존 nonce/signature/context 검증 경계에 접는다. HTTP polling은 fallback/diagnostics 후보로 유지한다. 다음 follow-up은 release follow-up external evidence closeout이다.

3. **다음 세션 시작점**: 최신 Relay production-readiness 문서는
   `docs/superpowers/plans/2026-07-05-ra-pwa-relay-managed-runtime-operator-setup-production-closeout.md`다. 첫 작업은
   release follow-up external evidence closeout을 진행하는 것이다. Android/mobile
   real-device smoke capture는 2026-07-06에 완료됐으므로, 외부 환경을 사용할 수
   없다면 `docs/superpowers/plans/2026-07-06-release-followup-post-android-smoke-recheck.md`
   상태를 기준으로 새 로컬 후속을 별도 계획 문서로 먼저 범위 지정한다.

## 6. 비목표

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸는 것은 비목표다. 이 파일은 CLI helper다.
- GUI 완료 기준을 Windows Terminal/PowerShell/Git Bash에서 `ash.exe` 수동 실행으로
  되돌리지 않는다. `ash.exe`는 GUI 내부 runtime 및 별도 CLI asset이다.
