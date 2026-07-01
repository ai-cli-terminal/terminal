# 2026-07-01 F-Droid Build Evidence Gate

## 목적

Release follow-up preflight가 F-Droid build/buildserver evidence 파일의 존재만
확인하면, 잘못된 앱이나 버전의 증거도 `ready`로 기록될 수 있다. 이번 작업은
runbook의 완료 기준과 자동 preflight 기준을 맞춰 `v0.3.3` F-Droid 후속 작업을
더 엄격하게 닫기 위한 것이다.

## 조사

- `docs/releases/release-followup-runbook.md`는 F-Droid evidence가
  `dev.aiterminal.android`, `versionName=0.3.3`, `versionCode=303`, build
  command, result status, output APK 또는 buildserver artifact를 기록해야 한다고
  요구한다.
- `scripts/smoke-release-followup-preflight.ps1`는 기존에
  `-FdroidBuildEvidencePath`가 가리키는 파일 존재만 확인했다.
- `android/fdroid-version.properties`가 현재 release identity의 source of truth다.
- `android/fdroiddata/metadata/dev.aiterminal.android.yml`도 같은 app id와
  version 값을 담고 있다.

## 문서화 계획

| 대상 | Tutorial | How-to | Reference | Explanation |
|---|---|---|---|---|
| F-Droid build evidence gate | 없음 | runbook 보강 | preflight JSON 필드 | plan 설명 |

## 구현 범위

- `scripts/smoke-release-followup-preflight.ps1`
  - `android/fdroid-version.properties`에서 expected `versionName`과
    `versionCode`를 읽는다.
  - F-Droid evidence 파일이 비어 있지 않고, expected app id/version/result/artifact
    marker를 포함하는지 확인한다.
  - blocked evidence에 `fdroidBuild.missing`, `fdroidBuild.checks`,
    `fdroidExpectations`를 기록한다.
- `docs/releases/release-followup-runbook.md`
  - acceptable JSON evidence 예시를 추가한다.
  - blocked 상태일 때 `fdroidBuild.missing`을 확인하도록 안내한다.

## 검증

```powershell
npm run smoke:release-followup-preflight
```

현재 개발 host에서는 MSI toolchain, GitHub Android signing secrets, F-Droid
build/buildserver evidence가 없어 `RELEASE_FOLLOWUP_PREFLIGHT_BLOCKED`가 정상이다.

추가 확인:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1 `
  -FdroidBuildEvidencePath <sample-success-evidence.json>
```

샘플 evidence가 expected app id/version/result/artifact marker를 모두 포함하면
`fdroidBuild.status`는 `ready`가 되어야 한다. 전체 preflight는 MSI와 Android
signing blocker가 남아 있으면 계속 `blocked`다.

## 다음 작업

외부 buildserver에서 실제 F-Droid build evidence를 만들 때, runbook의 JSON shape에
맞춰 evidence를 저장하고 combined preflight에 전달한다. preflight가
`fdroidBuild.status=ready`를 기록하면 F-Droid evidence 항목은 닫을 수 있다.
