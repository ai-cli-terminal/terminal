# 2026-07-01 RA/PWA Live P4b Browser Evidence

## 목표

P4a는 Node/PWA helper, selector surface, Rust live endpoint/bridge tests를 검증했다.
P4b는 실제 operator 흐름을 증명한다.

완료 문장:

> 등록된 PWA companion이 `ai remote daemon`의 live loopback endpoint에 연결하고,
> High-risk 명령을 approve와 reject 양쪽으로 처리한 transcript/screenshot/evidence를 남겼다.

## 사전 조건

- WSL Rust toolchain이 동작한다.
- `node pwa/app.test.mjs`가 통과한다.
- `scripts/smoke-pwa-live-approval.ps1`가 통과한다.
- browser automation 또는 수동 operator evidence 경로가 준비되어 있다.
- 테스트용 config/data directory를 격리한다. 기존 사용자 registry를 오염시키지 않는다.

## 핵심 난점

PWA identity private keys는 IndexedDB의 non-extractable CryptoKey로 저장된다. 이것은
제품 보안 경계다. P4b 자동화를 위해 private key export를 허용하거나 registry에 맞지
않는 키를 주입하면 안 된다.

자동화 후보는 두 가지다.

1. Browser UI가 identity를 생성하게 한 뒤, 표시된 complete command로 실제
   `ai remote pair`를 완료한다.
2. 별도 test-only harness를 만들어 제품 PWA 보안 경계를 건드리지 않고 disposable
   registry와 disposable browser profile만 사용한다.

## 실행 흐름

1. 격리 환경 생성
   - `XDG_CONFIG_HOME=<temp>\config`
   - `XDG_DATA_HOME=<temp>\data`
   - evidence root: `artifacts/ra-pwa-live-browser-evidence`
2. remote 빌드 확인
   - WSL에서 `cargo test --features remote companion_live`
3. PWA helper 확인
   - `node pwa/app.test.mjs`
4. PWA companion 열기
   - static file 또는 local HTTP server로 `pwa/index.html`을 연다.
5. PWA identity 생성
   - device id를 `phone-1` 같은 deterministic id로 둔다.
   - PWA가 생성한 Noise/approval pubkey를 complete command에서 읽는다.
6. pairing 완료
   - `ai remote pair` start payload를 만들고 PWA가 parse한다.
   - PWA complete command를 격리 config에서 실행한다.
   - `ai remote devices`로 등록 device를 확인한다.
7. daemon 시작
   - `ai remote daemon --device-id phone-1`
   - 출력된 live base URL, `/message`, `/events` endpoint를 transcript에 저장한다.
8. PWA live 연결
   - live base URL 입력
   - `Connect`
   - hello 성공과 connected state를 screenshot으로 남긴다.
9. approve case
   - `ai remote arm --allow-high`
   - High-risk 명령을 실행해 approval request를 발생시킨다.
   - PWA pending item을 approve한다.
   - terminal 결과가 allow/approved path로 끝나는지 transcript에 저장한다.
10. reject case
    - 새 High-risk 명령을 실행한다.
    - PWA pending item을 reject한다.
    - terminal 결과가 blocked/rejected path로 끝나는지 transcript에 저장한다.
11. evidence 작성
    - JSON evidence에 command transcript, daemon endpoint, screenshot paths,
      approve/reject 결과, git commit, tool versions를 기록한다.

## 수용 기준

- evidence JSON이 `status: passed`를 기록한다.
- approve와 reject가 모두 같은 daemon/PWA live 연결에서 수행된다.
- screenshot에는 connected PWA state와 pending/handled approval UI가 보인다.
- transcript에는 `ai remote daemon --device-id <id>`, live endpoint URL,
  `ai remote arm --allow-high`, approve/reject 대상 High 명령과 결과가 포함된다.
- mismatch/expired/replay를 통과시키는 우회가 없다.
- 사용자 config/data registry를 오염시키지 않는다.

## 이번 slice

이번 진행은 full P4b evidence 전에 `scripts/smoke-pwa-live-browser-preflight.ps1`를
추가해 다음을 먼저 고정한다.

- P4a live harness가 여전히 green인지 확인한다.
- PWA live selector/browser surface가 남아 있는지 확인한다.
- WSL Rust, Node, browser automation availability를 evidence JSON으로 기록한다.
- blocked이면 무엇이 blocked인지 명확히 남긴다.

이 preflight가 `ready`가 되면 다음 slice에서 실제 browser/operator 왕복 evidence를 구현한다.

## 2026-07-01 실행 결과

`pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-browser-preflight.ps1`
실행 결과:

- status: `blocked`
- evidence: `artifacts/ra-pwa-live-browser-preflight/ra-pwa-live-browser-preflight.json`
- passed: required files, Node `v24.12.0`, `node pwa/app.test.mjs`, PWA browser surface,
  WSL Rust toolchain, P4a live harness
- blocked: `playwright` package missing, no Edge/Chrome/Chromium command on `PATH`

다음 slice는 browser capture 환경을 준비한 뒤 preflight를 `ready`로 만들고, 그 다음 실제
approve/reject 왕복 evidence를 캡처한다.
