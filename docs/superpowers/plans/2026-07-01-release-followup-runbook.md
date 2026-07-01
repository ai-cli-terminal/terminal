# 2026-07-01 Release Follow-up Runbook

## 목적

`scripts/smoke-release-followup-preflight.ps1`가 남은 blocker를 한 파일에 기록하게
됐지만, 실제 외부 환경에서 무엇을 실행해야 하는지 한 곳에 모은 operator 문서가
없었다. 이 작업은 MSI, Android signing, F-Droid build/buildserver evidence를 닫는
절차를 `docs/releases/release-followup-runbook.md`로 고정한다.

## 조사 결과

- Windows MSI는 현재 host에서 blocked가 정상이다. 필요한 것은 Windows-native
  Rust/MSVC/WiX host다.
- Android signing은 GitHub repository secret 이름 4개가 필요하다. 값은 preflight나
  evidence에 쓰지 않는다.
- F-Droid는 local metadata/input smoke가 green이어도 실제 `fdroid build` 또는
  buildserver evidence가 별도로 필요하다.
- 현재 fdroiddata draft는 `dev.aiterminal.android`, `versionName=0.3.3`,
  `versionCode=303`이다.

## 문서 범위

- `docs/releases/README.md`를 추가해 release 문서의 인덱스를 만든다.
- `docs/releases/release-followup-runbook.md`를 추가해 operator how-to를 제공한다.
- README, troubleshooting, handoff, history, task, remaining priority 문서에서 새
  runbook을 찾을 수 있게 연결한다.

## 완료 조건

- [x] Release follow-up runbook 추가.
- [x] Release docs index 추가.
- [x] README에서 release docs index가 1-click으로 reachable.
- [x] Troubleshooting/release follow-up 문서가 runbook을 가리킴.
- [x] `npm run smoke:release-followup-preflight`로 현재 blocker evidence 재확인.

## 검증

```powershell
npm run smoke:release-followup-preflight
git diff --check
```

## 다음 작업

1. Windows-native Rust/MSVC/WiX host에서 runbook의 MSI 절차 실행.
2. GitHub Android signing secrets 등록 후 runbook의 signing readiness 확인.
3. F-Droid build/buildserver evidence 확보 후 combined preflight에 evidence path 공급.
