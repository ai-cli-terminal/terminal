# 2026-07-01 Remaining Work Priority

## 목적

현재 repo는 v0.3.3 Windows GUI release smoke, Android/F-Droid local preflight,
RA/PWA live transport/backend/PWA UX/P4a evidence까지 완료했다. 이 문서는 남은
작업을 우선순위로 고정해 다음 세션이 바로 이어갈 수 있게 한다.

## 현재 완료 기준

- Windows GUI: portable zip + NSIS installer smoke green. MSI는 후속 검토.
- Android/F-Droid: local input/metadata/signing throwaway/activation dry-run green. 실제 signing secrets와 buildserver evidence는 후속.
- RA/PWA companion: multi-device selection floor, live transport envelope, loopback HTTP/SSE endpoint, backend approval bridge, PWA live UX, P4a smoke evidence green.
- RA/PWA P4b preflight: required files, Node, PWA helper tests, PWA browser surface, WSL Rust, P4a harness pass. Browser capture environment is blocked because `playwright` is not installed and no Edge/Chrome/Chromium command is on `PATH`.
- Git 상태 기준: `develop`은 v0.3.3 후속 RA/PWA 작업 커밋을 포함해 `origin/develop`보다 앞서 있다.

## 우선순위

| 우선순위 | 작업 | 완료 조건 | 블로커/주의 |
|---|---|---|---|
| P0 | P4b browser/operator evidence | 실제 `ai remote daemon` + PWA browser 연결 + `ai remote arm --allow-high` + High 명령 approve/reject transcript/screenshot/evidence | PWA IndexedDB private key는 non-extractable이므로 자동화가 pairing 보안을 우회하면 안 됨 |
| P1 | RA transport mode decision | native `device.sock` fallback/flag 유지 여부 또는 live loopback default 단순화 결정 문서화/코드 반영 | native substrate는 테스트와 fallback 후보로 남아 있음 |
| P1 | PWA monitoring view | disabled Monitor tab을 실제 heartbeat/status/pending history 화면으로 전환 | P4b evidence 이후가 적절 |
| P2 | v0.3.3 release body 보강 | GitHub release body에 사용자용 설치/자산 설명 추가 | 태그/자산 수정 금지 |
| P2 | Windows MSI 재검토 | Windows-native Rust/Cargo + MSVC + WiX host에서 preflight/build evidence | 현재 host는 `MSI_PREFLIGHT_BLOCKED`가 정상 |
| P2 | Android signing/buildserver | 실제 GitHub signing secrets 등록 및 `fdroid build`/buildserver evidence | throwaway keystore green은 실제 릴리스 서명 완료가 아님 |
| P3 | Android/mobile local terminal 후속 | SAF-backed staging UX, richer imported file readers, Termux bridge hardening | Android 기본 약속은 계속 shellcore-only |
| P4 | Relay/M2 and enterprise/security | relay transport, fleet/enterprise policy, broader security hardening | RA/PWA local live loopback evidence 후 재개 |

## 바로 하지 않을 것

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸지 않는다. GUI 자산은 `ai-terminal.exe`다.
- PWA private key를 export 가능하게 바꾸지 않는다. 자동화를 위해 제품 보안 경계를 낮추지 않는다.
- Android 기본 실행 경계를 Termux/userland 직접 실행으로 바꾸지 않는다. Termux는 explicit opt-in bridge다.
- P4a smoke를 P4b 완료로 간주하지 않는다. P4b는 실제 browser/operator 왕복 evidence가 필요하다.

## 다음 작업 선택

가장 높은 가치의 다음 작업은 P0인 **P4b browser/operator evidence**다. 현재 preflight는
browser capture 환경에서 blocked 상태이므로, 먼저 `playwright`와 browser binary가 있는
환경을 준비한 뒤 `scripts/smoke-pwa-live-browser-preflight.ps1`를 다시 실행한다. blocked
항목이 없어지면 daemon/PWA/browser 왕복 자동화 또는 수동 operator script로 evidence를 남긴다.
