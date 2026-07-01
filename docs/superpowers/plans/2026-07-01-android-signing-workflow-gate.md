# 2026-07-01 Android Signing Workflow Gate

## 목적

Release follow-up preflight가 GitHub repository secret 이름만 확인하면, 실제
release workflow가 다른 secret 이름을 참조하는 회귀를 놓칠 수 있다. 이번 작업은
secret 값은 절대 읽지 않으면서 `.github/workflows/release.yml`의 Android signing
secret reference와 repository secret name readiness를 함께 기록하게 만드는 것이다.

## 조사

- `.github/workflows/release.yml`의 Android job은 `Prepare Android signing
  keystore (optional)` 단계에서 네 GitHub secrets를 환경 변수로 매핑한다.
- 필요한 repository secret 이름은 다음 네 개다.
  - `AI_TERMINAL_ANDROID_KEYSTORE_BASE64`
  - `AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD`
  - `AI_TERMINAL_ANDROID_KEY_ALIAS`
  - `AI_TERMINAL_ANDROID_KEY_PASSWORD`
- `gh secret list --json name,updatedAt`는 secret 값 없이 이름과 갱신 시각만
  확인할 수 있다.

## 문서화 계획

| 대상 | Tutorial | How-to | Reference | Explanation |
|---|---|---|---|---|
| Android signing workflow gate | 없음 | runbook 보강 | preflight JSON 필드 | plan 설명 |

## 구현 범위

- `scripts/smoke-release-followup-preflight.ps1`
  - expected Android signing secret names를 한 곳에서 정의한다.
  - `.github/workflows/release.yml`가 네 `secrets.<NAME>` reference를 모두
    포함하는지 확인한다.
  - `androidSigningSecrets.workflow`에 workflow path, referenced names,
    missing references, status를 기록한다.
  - `androidSigningSecrets.presentDetails`에 present secret names와 `updatedAt`만
    기록한다.
  - secret 값, decoded keystore, password, alias value는 읽거나 쓰지 않는다.

## 검증

```powershell
npm run smoke:release-followup-preflight
```

현재 개발 host에서는 workflow references가 ready이고 repository secrets가 없어
전체 preflight는 `RELEASE_FOLLOWUP_PREFLIGHT_BLOCKED`가 정상이다.

확인할 JSON field:

```powershell
$json = Get-Content artifacts\release-followup-preflight\release-followup-preflight-evidence.json -Raw | ConvertFrom-Json
$json.androidSigningSecrets.workflow
$json.androidSigningSecrets.presentDetails
```

## 다음 작업

외부 secret 등록자가 네 `AI_TERMINAL_ANDROID_*` secret을 GitHub repository에
등록한 뒤 preflight를 다시 실행한다. `androidSigningSecrets.workflow.status=ready`와
`androidSigningSecrets.status=ready`가 같이 기록되면 Android signing secret-name
gate는 닫힌다. 실제 signed APK release evidence는 release workflow run과 asset
검증으로 따로 확인한다.
