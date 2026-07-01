# 2026-07-01 RA/PWA Monitoring View

## 목적

P4b browser/operator evidence가 끝나면서 PWA companion은 실제 daemon live endpoint에
연결해 High 명령을 approve/reject할 수 있음이 확인됐다. 다음 사용성 gap은 연결 상태를
운영자가 한 화면에서 볼 수 없다는 점이다.

이 slice는 disabled `Monitor` tab을 실제 monitoring view로 전환한다.

## 현재 상태

- PWA는 live endpoint URL을 입력하고 `hello`를 보낸 뒤 `/events`를 `EventSource`로 읽는다.
- incoming `approval_request`는 approval panel과 pending queue에 렌더링된다.
- approve/reject는 signed `approval_response`를 `/message`로 POST한다.
- 화면의 live 상태 표시는 `State`와 `Last event`에 그친다.

## 구현 범위

- `Monitor` tab을 enabled 상태로 만든다.
- Monitor section을 추가해 다음 정보를 표시한다.
  - live connection state
  - endpoint URL
  - active device id
  - pending approval count
  - received approval count
  - sent response count
  - approve/reject count
  - last heartbeat time
  - last response time
  - recent live event history
- PWA helper에 deterministic monitor-state reducer를 추가해 Node test로 고정한다.
- 기존 manual approval/pairing flow는 그대로 유지한다.

## 비목표

- daemon server-side metrics endpoint를 새로 만들지 않는다.
- relay/M2 transport를 붙이지 않는다.
- PWA private key export나 registry 우회 자동화를 추가하지 않는다.

## 수용 기준

- `node pwa/app.test.mjs`가 monitor reducer를 검증한다.
- `scripts/smoke-pwa-live-approval.ps1` selector surface가 Monitor UI를 확인한다.
- `npm run smoke:pwa-live-browser-preflight`가 통과한다.
- `npm run smoke:pwa-live-browser-evidence`가 기존 approve/reject evidence 흐름을 유지한다.

## 실행 결과

- `node pwa/app.test.mjs`: `PWA_COMPANION_TEST_OK`
- `pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-pwa-live-approval.ps1`:
  `RA_PWA_LIVE_EVIDENCE_OK`
- `npm run smoke:pwa-live-browser-preflight`:
  `RA_PWA_LIVE_BROWSER_PREFLIGHT_READY`
- `npm run smoke:pwa-live-browser-evidence`:
  `RA_PWA_LIVE_BROWSER_EVIDENCE_OK`

Browser evidence JSON now includes:

```json
{
  "monitor": {
    "state": "Connected",
    "pending": "0",
    "received": "2",
    "sent": "2",
    "approved": "1",
    "rejected": "1"
  }
}
```

## 다음 후속

Transport mode decision까지 완료되면 RA/PWA local live path의 남은 큰 gap은 relay/M2다.
release follow-up과 Android/Windows packaging evidence는
`docs/superpowers/plans/2026-07-01-remaining-work-priority.md`를 기준으로 진행한다.
