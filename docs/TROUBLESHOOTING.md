# TROUBLESHOOTING — ai-cli-terminal

이 문서는 구현 시작부터 현재 RA/PWA live companion 작업까지 반복해서 나온
문제, 블로커, 우회 방법을 한곳에 모은다. 최신 진행 상태와 우선순위는
`docs/HANDOFF.md`, `docs/TASK.md`, `docs/HISTORY.md`가 정본이고, 이 파일은
실패 원인과 재현/복구 절차를 빠르게 찾기 위한 운영 문서다.

## 빠른 상태 확인

```powershell
git status --short --branch
git log --oneline -5
git diff --check
node pwa/app.test.mjs
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-approval.ps1
```

Rust 검증은 Windows host가 아니라 WSL 기준으로 실행한다.

```powershell
$env:MSYS_NO_PATHCONV = '1'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo test --features remote'
```

## PowerShell / Repo 작업 환경

| 증상 | 원인 | 조치 |
|---|---|---|
| 명령 출력 끝에 `Set-PSReadLineOption` 경고가 섞임 | 사용자 PowerShell profile이 redirect된 콘솔에서 prediction/list view를 켜려고 함 | 자동화 명령은 `pwsh -NoProfile ...`로 실행한다. Codex shell tool을 쓸 때는 `login:false`를 선호한다. |
| `git status`가 `could not open directory ' /'` 경고를 냄 | repo 루트 아래 trailing-space 디렉터리(`terminal\ `)가 생겼던 상태 | 해당 디렉터리는 제거 완료. 재발하면 `Get-ChildItem -Force`로 실제 경로를 확인한 뒤 작업 디렉터리 안인지 먼저 검증하고 제거한다. |
| PowerShell에서 `rg docs/*.md`류 glob이 기대와 다르게 동작 | PowerShell glob 확장과 ripgrep glob 의미가 섞임 | `rg --glob '*.md' PATTERN docs` 또는 `rg --files docs | rg PATTERN` 형식을 쓴다. |
| `artifacts/`에 많은 evidence가 남음 | smoke 결과 작업 디렉터리 | 커밋 대상 아님. 필요한 evidence path만 문서에 기록한다. `git add -A`는 피하고 파일을 명시 stage한다. |

## Rust / WSL

| 증상 | 원인 | 조치 |
|---|---|---|
| Windows host에서 `cargo`/`rustc`가 없음 | 이 repo의 일반 Rust 검증은 WSL 중심으로 운영 중 | `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; cargo ...'`로 실행한다. |
| WSL 경로가 이상하게 변환됨 | MSYS path conversion이 Windows 경로를 변환 | PowerShell에서 `$env:MSYS_NO_PATHCONV='1'` 설정 후 `wsl.exe`를 호출한다. |
| `cargo fmt`가 `imports_granularity`/`group_imports` 경고를 냄 | stable rustfmt에서 unstable config key 경고 | 현재는 알려진 경고이며 실패가 아니다. exit code와 실제 diff 여부를 본다. |
| remote feature 테스트가 빠짐 | 기본 `cargo test`만 실행 | RA/PWA 관련 검증은 `cargo test --features remote` 또는 targeted `cargo test --features remote companion_live`를 사용한다. |

## Windows GUI / Release Packaging

| 증상 | 원인 | 조치 |
|---|---|---|
| 사용자가 `ai-windows-x86_64.exe`를 더블클릭하려 함 | 이 파일은 GUI가 아니라 CLI helper | 더블클릭 안내는 `ai-terminal-windows-*.zip`의 `ai-terminal.exe` 또는 `AI.Terminal_*_x64-setup.exe`를 가리킨다. |
| NSIS smoke가 `WebView2Loader.dll` 누락으로 실패 | MSVC/Tauri 릴리스 산출물은 별도 loader DLL 없이 동작 가능 | `scripts/smoke-nsis.ps1`는 해당 DLL을 optional로 처리하도록 수정 완료. 최신 스크립트로 재실행한다. |
| MSI preflight가 blocked | 현재 host에 Windows-native Rust/Cargo, MSVC `cl`/`link`/`rc`, WiX가 없음 | `scripts/smoke-msi-preflight.ps1` 결과 `MSI_PREFLIGHT_BLOCKED`가 정상 상태다. MSI는 native MSVC+WiX host에서만 재검토한다. |
| ShellOpen smoke에서 AI/storage 세부 검증이 빠짐 | Windows Shell open verb는 per-process env 주입이 제한됨 | ShellOpen evidence는 launch/window/child/resize/cleanup 범위로 해석한다. 완전 기능 smoke는 portable/installed GUI smoke를 사용한다. |
| literal Explorer double-click 영상이 없음 | 자동 smoke는 Shell open-verb evidence까지만 확보 | 영상/캡처가 필요하면 수동 operator 단계로 별도 기록한다. 릴리스 gate는 portable zip + NSIS smoke evidence가 기준이다. |
| SQLite `ai-terminal.db-shm` 파일 때문에 파일 열거 경고/skip | GUI smoke 중 WAL shared-memory 파일이 열려 있음 | smoke evidence에서 locked range skip은 허용한다. DB 무결성은 별도 storage/audit 검증을 본다. |

## Android / F-Droid / Termux

| 증상 | 원인 | 조치 |
|---|---|---|
| `gh secret list`가 비어 있음 | 실제 Android release signing secrets가 등록되지 않음 | throwaway keystore preflight는 통과했지만, 실제 secrets 등록/검증은 남은 릴리스 운영 작업이다. |
| `fdroid build`/buildserver evidence가 없음 | local metadata/input 검증까지만 완료 | `fdroid build` 또는 buildserver 검증을 별도 환경에서 실행하고 evidence path를 문서화한다. |
| repo 루트 밖 `terminal-project/android`와 혼동 | 실제 Android 프로젝트는 `terminal/android` | Gradle 명령은 `gradle -p android ...`로 repo 안 프로젝트를 지정한다. |
| Termux helper가 `/sdcard`에 쓰지 못함 | Termux storage permission 미부여 | 사용자가 Termux storage permission을 부여한 뒤 shared staging path를 다시 검증한다. |
| 앱이 `events.ndjson`를 읽지 못함 | helper가 쓰기 전에 앱이 파일을 만들지 않아 EACCES 발생 | 앱이 event file을 먼저 생성하는 경계가 필요하다. 현재 helper protocol은 이 실패를 기록했다. |
| shared storage FIFO가 동작하지 않음 | Android shared storage는 FIFO를 지원하지 않음 | FIFO 대신 regular stdout/stderr log polling fallback을 사용한다. |
| Android native `.so` 로드 실패 | dev 환경에서 JNI 산출물이 아직 packaging되지 않음 | `android/build-rust-jni.ps1` 또는 Android JNI packaging CI 경로를 사용한다. |

## Shell / Gate / Remote Approval

| 증상 | 원인 | 조치 |
|---|---|---|
| 초기 WSL e2e가 hang | bash readline이 probe marker `\x1f`를 undo로 삼킴 | bash spawn은 `--noediting` 경로를 사용한다. |
| zsh에서 preexec만으로 명령 차단이 안 됨 | zsh preexec는 실행 직전 알림이고 차단점이 아님 | zsh는 ZLE 기반 차단, bash는 extdebug 기반 차단을 사용한다. |
| High 명령이 원격으로 안 감 | `ai remote arm --allow-high`가 켜지지 않았거나 디바이스 선택이 모호 | `ai remote devices`로 등록 상태를 확인하고, 복수 디바이스면 `ai remote daemon --device-id <id>`로 명시한다. |
| zero/multiple registered devices에서 daemon이 fail closed | 승인 대상이 모호한 상태 | 한 대만 등록하거나 `--device-id`를 지정한다. 이 동작은 의도된 안전 경계다. |
| timed-out approval 뒤 다음 요청이 이상하게 실패 | 과거 queue-backed listener가 stale accept/read path를 남길 수 있었음 | per-request response channel + accept timeout으로 수정 완료. 회귀 테스트는 `device_listener_timeout_does_not_poison_next_request`. |
| 승인 응답이 `ContextDrift`/invalid로 막힘 | cwd/env/target path context hash가 승인 요청 이후 변함 | 승인 요청 직후 같은 작업 context에서 응답한다. daemon은 응답 검증 직전에 context hash를 재계산한다. |
| expired approval이 통과하지 않음 | TTL 초과 | 새 High 요청을 다시 발생시킨다. 만료/nonce replay는 fail-closed가 정상이다. |

## RA/PWA Live Companion

| 증상 | 원인 | 조치 |
|---|---|---|
| `/events`가 409 `companion hello required`를 반환 | browser companion이 `hello`를 보내기 전에 SSE를 열었음 | PWA에서 daemon이 출력한 base URL을 넣고 `Connect`를 먼저 누른다. |
| `hello`가 거부됨 | PWA identity의 device id/public keys가 registry 기록과 다름 | 같은 PWA identity로 `ai remote pair` complete를 마쳤는지 확인한다. 필요하면 PWA identity를 재생성하고 다시 pair한다. |
| PWA가 private key를 export할 수 없음 | private CryptoKey는 IndexedDB에 non-extractable로 저장 | 의도된 보안 경계다. 자동화가 필요하면 제품 보안을 약화하지 말고 별도 test-only pairing/evidence harness를 설계한다. |
| live endpoint URL 연결 실패 | daemon이 출력한 base URL이 아니거나 daemon 종료 | `ai remote daemon`이 출력한 `http://127.0.0.1:<port>` base URL을 그대로 사용한다. |
| `approval_response` POST가 409 mismatch | 응답의 approval id/nonce가 현재 pending request와 다름 | PWA pending queue의 최신 요청에서 approve/reject한다. mismatch는 pending request를 유지하는 fail-closed 동작이다. |
| manual approval만 가능하고 live가 안 됨 | live 연결 전이거나 browser endpoint가 없는 빌드 | 기존 manual flow로 signed response를 복사하고 `ai remote approval-verify --device-id ...`로 확인한다. |
| P4a evidence는 있는데 실제 browser/operator evidence가 없음 | 현재 `scripts/smoke-pwa-live-approval.ps1`는 Node/PWA selector/Rust endpoint tests만 검증 | P4b에서 daemon + browser/PWA + High command approve/reject transcript/screenshot evidence를 추가해야 한다. |

## Evidence Harness

| Harness | 목적 | 상태 |
|---|---|---|
| `scripts/smoke-pwa-live-approval.ps1` | PWA helper tests, live selector surface, Rust `companion_live` endpoint/bridge tests | P4a green, evidence: `artifacts/ra-pwa-live-evidence/ra-pwa-live-evidence.json` |
| `scripts/smoke-pwa-live-browser-preflight.ps1` | P4b browser/operator evidence 실행 전 환경 readiness/blocker 기록 | 2026-07-01 실행 결과 blocked. P4a harness까지는 passed, `playwright` package와 PATH상의 Edge/Chrome/Chromium이 blocked. |
| `scripts/smoke-gui.ps1` | portable/installed Windows GUI launch, PTY, Ctrl-C/Ctrl-D, frontend, AI/safety/storage | v0.3.3 GUI evidence green |
| `scripts/smoke-nsis.ps1` | NSIS install/run/uninstall smoke | v0.3.3 NSIS evidence green |
| `scripts/smoke-msi-preflight.ps1` | MSI packaging prerequisites 확인 | 현재 host는 blocked |

## Release Follow-up

| 항목 | 현재 상태 | 다음 조치 |
|---|---|---|
| v0.3.2 release note | v0.3.3으로 superseded 안내 있음 | 추가 조치 없음 |
| v0.3.3 release body | public release body가 비어 있음 | 태그/자산을 바꾸지 말고 release body만 보강 |
| Windows MSI | native MSVC+WiX host 부재로 blocked | 별도 Windows packaging host에서 preflight 재실행 |
| Android signing | local throwaway preflight green, 실제 GitHub secrets 없음 | 실제 signing secrets 등록 후 CI/activation 검증 |
| F-Droid buildserver | local metadata/input 검증 green | 실제 `fdroid build`/buildserver evidence 확보 |
