# 2026-07-01 Release Follow-Up Status Smoke

## 목적

`npm run status:release-followup`는 운영자가 release follow-up blocker를 빠르게
읽는 진입점이다. 하지만 실제 host 상태에만 기대면 status command 자체가 깨져도
외부 MSI/secret/F-Droid blocker와 구분하기 어렵다. 이번 작업은 synthetic evidence로
blocked와 ready 상태를 모두 검증하는 deterministic smoke를 추가한다.

## 조사

- `scripts/show-release-followup-status.ps1`는 combined evidence의 `closeout`
  객체를 읽는다.
- 현재 개발 host는 MSI toolchain, GitHub Android signing secret names,
  F-Droid build/buildserver evidence가 없어 blocked가 정상이다.
- status command의 핵심 계약은 host 상태가 아니라 다음 출력/exit code다.
  - 기본 출력은 `RELEASE_FOLLOWUP_STATUS <status>`를 포함한다.
  - `-Json`은 `status`, `canCloseDocs`, `readyItems`, `blockedItems`를 출력한다.
  - `-FailOnBlocked`는 blocked evidence에서 exit `2`, ready evidence에서 exit `0`이다.

## 문서화 계획

| 대상 | Tutorial | How-to | Reference | Explanation |
|---|---|---|---|---|
| Release follow-up status smoke | 없음 | runbook/troubleshooting에 검증 명령 추가 | npm script와 smoke evidence path | 이 plan 문서 |

## 구현 범위

- `scripts/smoke-release-followup-status.ps1`
  - synthetic blocked evidence를 생성한다.
  - synthetic ready evidence를 생성한다.
  - `show-release-followup-status.ps1`의 text, `-Json`, `-FailOnBlocked` 경로를
    검증한다.
  - smoke evidence를 `artifacts/release-followup-status-smoke/`에 기록한다.
- `package.json`
  - `npm run smoke:release-followup-status`를 추가한다.
- docs
  - runbook, troubleshooting, handoff, priority 문서에 smoke command를 연결한다.

## 검증

```powershell
npm run smoke:release-followup-status
```

기대 출력:

```text
RELEASE_FOLLOWUP_STATUS_SMOKE_OK artifacts\release-followup-status-smoke\release-followup-status-smoke-evidence.json
```

## 다음 작업

외부 release follow-up을 닫을 때는 status smoke로 도구 자체가 정상인지 먼저 확인한 뒤,
`npm run status:release-followup -- -Refresh -RunMsiBuild -FdroidBuildEvidencePath <path>`로
실제 evidence를 다시 요약한다.
