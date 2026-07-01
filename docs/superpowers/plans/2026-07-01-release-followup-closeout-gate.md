# 2026-07-01 Release Follow-Up Closeout Gate

## 목적

Release follow-up preflight가 MSI, Android signing, F-Droid evidence를 각각
검사하더라도 문서 마감 기준이 명시적으로 남지 않으면 다음 세션이 blocker를 해석해야
한다. 이번 작업은 combined evidence에 `closeout` 객체를 추가해 후속 문서를 언제
닫아도 되는지 기계적으로 판단할 수 있게 만든다.

## 조사

- `scripts/smoke-release-followup-preflight.ps1`는 이미 세 release follow-up
  항목을 검사한다.
  - `msi`
  - `androidSigningSecrets`
  - `fdroidBuild`
- `status=ready`는 전체 blocker가 없다는 결론이지만, 문서 closeout 단계에서 필요한
  evidence 목록과 tag/asset 처리 방침은 별도 필드로 드러나 있지 않았다.
- `docs/releases/release-followup-runbook.md`는 release tag와 기존 asset을 유지하라고
  설명하지만, preflight evidence에는 그 결정을 직접 기록하지 않았다.

## 문서화 계획

| 대상 | Tutorial | How-to | Reference | Explanation |
|---|---|---|---|---|
| Release follow-up closeout gate | 없음 | runbook Step 5 보강 | preflight `closeout` JSON | 이 plan 문서 |

## 구현 범위

- `scripts/smoke-release-followup-preflight.ps1`
  - evidence에 `closeout.requiredEvidence`를 기록한다.
  - `closeout.readyItems`와 `closeout.blockedItems`를 기록한다.
  - `closeout.canCloseDocs=true`는 전체 `status=ready`이고 blocked item이 없을 때만
    설정한다.
  - `releaseTagAction=unchanged`, `assetAction=unchanged`를 기록해 기존 v0.3.3
    tag/assets를 별도 결정 없이 바꾸지 않도록 한다.
- `docs/releases/release-followup-runbook.md`
  - Step 5 completion condition을 `closeout.canCloseDocs=true`,
    `closeout.blockedItems=[]` 기준으로 보강한다.
- handoff/priority/troubleshooting 문서
  - 다음 세션이 `closeout` 필드를 먼저 확인하도록 갱신한다.

## 검증

현재 개발 host에서는 외부 MSI/secret/F-Droid evidence가 없으므로 blocked가 정상이다.

```powershell
npm run smoke:release-followup-preflight
```

확인할 JSON field:

```powershell
$json = Get-Content artifacts\release-followup-preflight\release-followup-preflight-evidence.json -Raw | ConvertFrom-Json
$json.closeout
```

기대값:

- `closeout.canCloseDocs=false`
- `closeout.blockedItems`에 `msi`, `androidSigningSecrets`, `fdroidBuild`가 남아 있음
- `releaseTagAction=unchanged`
- `assetAction=unchanged`

## 다음 작업

외부 Windows-native packaging host, GitHub repository secrets, F-Droid build/buildserver
환경에서 각 evidence를 준비한 뒤 combined preflight를 다시 실행한다. 그 결과
`status=ready`와 `closeout.canCloseDocs=true`가 같이 기록되면 후속 문서를 완료 상태로
정리한다.
