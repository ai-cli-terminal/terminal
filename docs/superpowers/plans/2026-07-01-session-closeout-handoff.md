# 2026-07-01 Session Closeout Handoff

## 목적

이 문서는 2026-07-01 RA/PWA live companion과 v0.3.3 release follow-up
문서/도구 작업을 PR로 묶어 머지한 뒤 다음 세션이 바로 이어갈 수 있게 남기는
최종 이관 노트다. 정본 상태는 `docs/HANDOFF.md`,
`docs/TROUBLESHOOTING.md`, `docs/releases/release-followup-runbook.md`,
`docs/superpowers/plans/2026-07-01-remaining-work-priority.md`를 함께 본다.

## 이번 묶음의 완료 상태

- RA/PWA live companion은 local live loopback을 기본 product transport로 고정했고,
  browser/operator approve/reject evidence와 Monitor tab evidence까지 확보했다.
- v0.3.3 GitHub Release body는 Windows GUI assets, CLI/runtime assets,
  Android APK, checksum, known follow-up을 설명하도록 갱신했다.
- Release follow-up은 preflight, runbook, evidence gates, status command,
  status smoke, aggregate check command까지 갖췄다.
- `npm run check:release-followup`는 operator-facing 단일 진입점이다. 현재 host에서는
  외부 환경 blocker 때문에 blocked가 정상 결과다.

## 검증 기준

다음 명령은 이 handoff 시점의 핵심 회귀 확인이다.

```powershell
npm run check:release-followup
npm run check:release-followup -- -Json
npm run smoke:release-followup-status
git diff --check
```

`npm run check:release-followup -- -FailOnBlocked`는 현재 host에서 exit code `2`가
정상이다. 이 명령은 release follow-up blocker를 CI gate처럼 다룰 때 사용한다.

## 남은 작업 우선순위

1. Windows-native packaging host에서
   `scripts/smoke-release-followup-preflight.ps1 -RunMsiBuild`를 실행해 generated
   `.msi`와 SHA256 evidence를 확보한다.
2. GitHub repository에 실제 Android release signing secret names를 등록하고,
   `.github/workflows/release.yml` reference와 함께 preflight evidence를 ready로 만든다.
3. fdroiddata/buildserver 환경에서 `fdroid build dev.aiterminal.android:303`
   evidence를 만들고 app id/version/result/artifact marker를 preflight에 넘긴다.
4. 위 세 항목이 모두 ready가 되면 `npm run status:release-followup`에서
   `closeout.canCloseDocs=true`와 empty `closeout.blockedItems`를 확인한 뒤 follow-up
   문서를 완료 상태로 닫는다.
5. Release follow-up을 닫은 뒤 Relay/M2 transport 설계를 재개한다.

## 알려진 블로커

- 현재 개발 host는 Windows MSI에 필요한 Rust/Cargo, MSVC `cl`/`link`, Windows SDK
  `rc`, WiX tooling이 없어 MSI evidence가 blocked다.
- GitHub Android signing secret values는 repo에 없고, 문서와 evidence에는 secret names와
  `updatedAt`만 남겨야 한다.
- F-Droid build/buildserver evidence는 아직 외부 환경에서 만들어야 한다.
- gstack redaction scanner가 로컬에서 `../lib/redact-engine` module resolution 문제로
  실패할 수 있다. 그 경우 live-format secret fallback regex scan을 별도로 실행하고
  결과를 커밋/PR 설명에 남긴다.

## 다음 세션 시작 절차

```powershell
git status --short --branch
git log --oneline -5
npm run status:release-followup
```

외부 환경 follow-up을 진행할 세션이면
`docs/releases/release-followup-runbook.md`의 순서대로 실행한다. 구현 작업을 재개할
세션이면 `docs/superpowers/plans/2026-07-01-remaining-work-priority.md`의 우선순위를
먼저 확인한다.
