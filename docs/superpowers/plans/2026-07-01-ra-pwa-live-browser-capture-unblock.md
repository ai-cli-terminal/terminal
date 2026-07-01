# 2026-07-01 RA/PWA Browser Capture Unblock

## 목적

P4b browser/operator evidence는 실제 PWA 화면과 terminal transcript를 함께 남겨야 한다.
이 slice는 P4b 본 실행 전에 browser capture 환경을 `blocked`에서 `ready`에 가깝게
만드는 작업이다.

## 현재 preflight 상태

첫 실행 결과:

- evidence: `artifacts/ra-pwa-live-browser-preflight/ra-pwa-live-browser-preflight.json`
- passed: required files, Node, PWA helper tests, PWA browser surface, WSL Rust, P4a live harness
- blocked: `playwright` package missing, browser command missing on `PATH`

추가 확인 결과 이 host에는 browser binary가 설치되어 있다.

- `C:\Program Files\Google\Chrome\Application\chrome.exe`
- `C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe`

따라서 `browser-binary-path` blocker는 제품/환경 결함이 아니라 preflight 감지 범위가
좁았던 문제다.

## 이번 작업

- `scripts/smoke-pwa-live-browser-preflight.ps1`가 PATH뿐 아니라 일반 Chrome/Edge 설치
  경로도 검사하도록 수정한다.
- preflight를 다시 실행해 browser binary blocker가 해소됐는지 evidence에 기록한다.
- 남는 blocker가 `playwright`뿐이면, 다음 선택지를 명확히 한다.

## 다음 선택지

1. Repo에 Playwright dev dependency를 추가하고 system Chrome/Edge executable path를 사용한다.
2. Browser automation이 이미 준비된 host에서 preflight와 P4b evidence를 실행한다.
3. 제품 보안 경계를 유지하는 test-only browser harness를 별도 설계한다.

선호 순서는 1번이다. PWA private key export를 허용하지 않고도 disposable browser profile에서
identity 생성, pairing, live connect, approve/reject를 자동화할 수 있기 때문이다.

## 완료 기준

- `browser-binary-path` step이 installed Chrome 또는 Edge 경로로 `passed`가 된다.
- `playwright-automation` blocker가 남아 있으면 그 상태를 문서와 handoff에 정확히 남긴다.
- P4b 본 evidence는 아직 완료로 표시하지 않는다.

## 실행 결과

조치:

- root `package.json`/`package-lock.json`에 Playwright dev dependency 추가
- `.gitignore`에 `/node_modules/` 추가
- preflight browser binary detection을 PATH + common install paths로 확장

재실행:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-browser-preflight.ps1
```

결과:

- status: `ready`
- marker: `RA_PWA_LIVE_BROWSER_PREFLIGHT_READY`
- evidence: `artifacts/ra-pwa-live-browser-preflight/ra-pwa-live-browser-preflight.json`
- browser: `C:\Program Files\Google\Chrome\Application\chrome.exe`

남은 작업은 P4b 본 evidence capture다.
