# 2026-07-01 Release Follow-Up Check Command

## 목적

Release follow-up 확인은 지금 세 명령으로 나뉘어 있다.

```powershell
npm run smoke:release-followup-status
npm run smoke:release-followup-preflight
npm run status:release-followup
```

이 순서는 정확하지만 운영자가 한 단계를 건너뛰기 쉽다. 이번 작업은 status smoke,
combined preflight, status summary를 한 번에 실행하는 operator-facing check command를
추가해 “도구 자체가 정상인지”와 “현재 blocker가 무엇인지”를 한 evidence로 묶는다.

## 조사

- `scripts/smoke-release-followup-status.ps1`는 synthetic blocked/ready evidence로
  status command 계약을 검증한다.
- `scripts/smoke-release-followup-preflight.ps1`는 실제 combined release follow-up
  evidence를 갱신한다.
- `scripts/show-release-followup-status.ps1`는 combined evidence를 사람이 읽는
  summary 또는 JSON으로 바꾼다.
- 현재 개발 host에서 expected status는 blocked다. blocked는 실패가 아니고,
  `-FailOnBlocked`를 명시했을 때만 exit `2`여야 한다.

## 문서화 계획

| 대상 | Tutorial | How-to | Reference | Explanation |
|---|---|---|---|---|
| Release follow-up check command | 없음 | runbook 빠른 확인 명령 | npm/script options | 이 plan 문서 |

## 구현 범위

- `scripts/check-release-followup.ps1`
  - status smoke를 실행한다.
  - combined preflight를 갱신한다.
  - status summary JSON을 생성한다.
  - aggregate evidence를 `artifacts/release-followup-check/`에 기록한다.
  - `-Json`은 aggregate evidence를 출력한다.
  - `-FailOnBlocked`는 ready가 아니면 exit `2`를 반환한다.
  - `-RunMsiBuild`, `-RunAndroidLocalSmokes`, `-FdroidBuildEvidencePath`를 preflight로
    전달한다.
- `package.json`
  - `npm run check:release-followup`을 추가한다.
- docs
  - release docs/runbook/troubleshooting/handoff/priority 문서에서 기본 확인 경로를
    check command로 노출한다.

## 검증

현재 host에서는 blocked가 정상이다.

```powershell
npm run check:release-followup
npm run check:release-followup -- -Json
```

`-FailOnBlocked`는 현재 host에서 exit `2`가 정상이다.

```powershell
npm run check:release-followup -- -FailOnBlocked
```

## 다음 작업

외부 host에서 release follow-up을 닫을 때는 다음처럼 실제 evidence path를 전달한다.

```powershell
npm run check:release-followup -- -RunMsiBuild -FdroidBuildEvidencePath <path-to-fdroid-build-evidence.json>
```

check status가 `ready`이고 summary의 `canCloseDocs=true`이면 runbook Step 5의 문서
closeout을 진행한다.
