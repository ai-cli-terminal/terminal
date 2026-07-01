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
- F-Droid build evidence gate: supplied F-Droid build/buildserver evidence must now include expected app id, versionName, versionCode, successful result, and APK/buildserver artifact markers before `fdroidBuild.status` becomes `ready`.
- Git 상태 기준: `develop`은 v0.3.3 후속 RA/PWA 작업 커밋을 포함해 `origin/develop`보다 앞서 있다.

## 우선순위

| 우선순위 | 작업 | 완료 조건 | 블로커/주의 |
|---|---|---|---|
| P1 | Windows MSI 재검토 | Runbook 절차대로 `smoke-release-followup-preflight.ps1 -RunMsiBuild`가 native Rust/MSVC/WiX host에서 MSI evidence 기록 | 현재 host는 MSI toolchain 부재로 blocked |
| P1 | Android signing/buildserver | Runbook 절차대로 GitHub signing secret names present + 실제 `fdroid build`/buildserver evidence가 expected app/version/result/artifact marker를 포함 | throwaway keystore/local metadata green은 실제 릴리스 완료가 아님 |
| P3 | Android/mobile local terminal 후속 | SAF-backed staging UX, richer imported file readers, Termux bridge hardening | Android 기본 약속은 계속 shellcore-only |
| P4 | Relay/M2 and enterprise/security | relay transport, fleet/enterprise policy, broader security hardening | RA/PWA local live loopback evidence 후 재개 |

## 바로 하지 않을 것

- `ai-windows-x86_64.exe`를 GUI 앱으로 바꾸지 않는다. GUI 자산은 `ai-terminal.exe`다.
- PWA private key를 export 가능하게 바꾸지 않는다. 자동화를 위해 제품 보안 경계를 낮추지 않는다.
- Android 기본 실행 경계를 Termux/userland 직접 실행으로 바꾸지 않는다. Termux는 explicit opt-in bridge다.
- P4a smoke를 P4b 완료로 간주하지 않는다. P4b는 실제 browser/operator 왕복 evidence가 필요하다.

## 다음 작업 선택

P4b browser/operator evidence, PWA monitoring view, RA transport mode decision,
v0.3.3 release body 보강, release follow-up preflight/runbook, F-Droid build
evidence gate는 완료됐다. 가장 높은 가치의
다음 작업은 외부 환경에서 runbook을 실행하는 **Windows MSI 재검토**와
**Android signing/buildserver evidence**다.
