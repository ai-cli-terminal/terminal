# 2026-07-01 Release Follow-Up Status Command

## 목적

Release follow-up preflight evidence는 `closeout`까지 갖췄지만, 운영자가 매번
JSON을 직접 파싱해야 하면 다음 blocker를 놓치기 쉽다. 이번 작업은 evidence를
사람이 바로 읽을 수 있는 status command로 노출해 외부 MSI/signing/F-Droid 작업자가
현재 상태와 다음 action을 즉시 확인하게 만든다.

## 조사

- `scripts/smoke-release-followup-preflight.ps1`는 combined evidence를
  `artifacts/release-followup-preflight/release-followup-preflight-evidence.json`에
  기록한다.
- closeout 기준은 `closeout.canCloseDocs`, `closeout.blockedItems`,
  `closeout.releaseTagAction`, `closeout.assetAction`이다.
- `docs/releases/release-followup-runbook.md`의 Step 1과 Step 5는 현재 JSON 파싱
  명령을 직접 보여준다. 이 경로는 정확하지만 반복 운영에는 불편하다.

## 문서화 계획

| 대상 | Tutorial | How-to | Reference | Explanation |
|---|---|---|---|---|
| Release follow-up status command | 없음 | runbook Step 1/5에 사용법 추가 | npm/script options | 이 plan 문서 |

## 구현 범위

- `scripts/show-release-followup-status.ps1`
  - 기존 evidence가 있으면 읽고, 없거나 `-Refresh`가 있으면 preflight를 먼저 실행한다.
  - 기본 출력은 사람이 읽는 상태 요약이다.
  - `-Json`은 automation-friendly summary를 출력한다.
  - `-FailOnBlocked`는 `closeout.canCloseDocs=false`일 때 exit `2`로 종료한다.
  - `-RunMsiBuild`, `-RunAndroidLocalSmokes`, `-FdroidBuildEvidencePath`를 preflight로
    전달할 수 있다.
- `package.json`
  - `npm run status:release-followup`을 추가한다.
- release docs
  - runbook과 release docs index에서 status command를 discovery path로 노출한다.

## 검증

현재 host에서 expected status는 blocked다.

```powershell
npm run status:release-followup
npm run status:release-followup -- -Json
```

`-FailOnBlocked`는 현재 host에서 exit `2`가 정상이다.

```powershell
npm run status:release-followup -- -FailOnBlocked
```

## 다음 작업

외부 환경에서 MSI/Android signing/F-Droid evidence를 채운 뒤:

```powershell
npm run status:release-followup -- -Refresh -RunMsiBuild -FdroidBuildEvidencePath <path>
```

status가 `ready`이고 `Can close docs: True`이면 runbook Step 5의 문서 closeout을
진행한다.
