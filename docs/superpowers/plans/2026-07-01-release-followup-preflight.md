# 2026-07-01 Release Follow-up Preflight

## 목적

`v0.3.3` release body까지 보강된 뒤 남은 P1은 Windows MSI 재검토와 Android
signing/buildserver evidence다. 둘 다 현재 작업 머신만으로 완료할 수 있는 일이 아니라,
필요한 외부 상태가 준비됐는지 반복해서 확인할 수 있는 preflight가 필요하다.

## 범위

- Windows MSI readiness:
  - 기존 `scripts/smoke-msi-preflight.ps1`를 호출한다.
  - `-RunMsiBuild`로 native Rust/MSVC/WiX host에서 실제 MSI build까지 시도한다.
  - 실제 follow-up 완료는 generated `.msi` path와 SHA256 hash가 evidence에 있어야 한다.
- Android release signing readiness:
  - `.github/workflows/release.yml`가 expected secret names를 참조하는지 확인한다.
  - GitHub repository secret **이름**과 `updatedAt`만 확인한다.
  - secret 값은 읽거나 출력하거나 evidence에 저장하지 않는다.
- F-Droid build/buildserver readiness:
  - 실제 `fdroid build` 또는 buildserver 결과 evidence 파일이 있는지 확인한다.
  - local metadata/input smokes는 buildserver evidence를 대체하지 않는다.
- Optional local Android wiring:
  - `-RunAndroidLocalSmokes`로 throwaway signing과 F-Droid metadata smoke를 재실행할 수 있다.
  - 이 결과는 wiring evidence이지 실제 release signing/buildserver completion이 아니다.

## 새 스크립트

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1
```

기본 실행은 빠른 readiness check만 수행하고 다음 evidence를 기록한다.

```text
artifacts\release-followup-preflight\release-followup-preflight-evidence.json
```

현재 host에서는 정상적으로 `RELEASE_FOLLOWUP_PREFLIGHT_BLOCKED`가 나온다.
이유는 Windows MSI toolchain이 없고, GitHub Android signing secrets가 없으며,
F-Droid build/buildserver evidence가 아직 공급되지 않았기 때문이다.

## 완료 조건

- [x] Release follow-up preflight script 추가.
- [x] Default run이 현재 host의 blocker를 JSON으로 기록.
- [x] 기존 MSI preflight를 재사용.
- [x] MSI build completion은 `-RunMsiBuild`와 generated MSI/hash evidence를 요구.
- [x] Android signing secret 값은 읽거나 저장하지 않고 workflow reference와 secret name만 비교.
- [x] 문서와 handoff/priority 상태 갱신.

## 검증

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1
git diff --check
```

## 다음 작업

1. Windows-native Rust/MSVC/WiX host에서:

   ```powershell
   pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1 -RunMsiBuild
   ```

   `msi.checks.buildExitCodeZero`, `msi.checks.msiGenerated`,
   `msi.checks.msiSha256Recorded`가 모두 `true`여야 한다.

2. GitHub repository에 다음 secrets를 등록한다.

   ```text
   AI_TERMINAL_ANDROID_KEYSTORE_BASE64
   AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD
   AI_TERMINAL_ANDROID_KEY_ALIAS
   AI_TERMINAL_ANDROID_KEY_PASSWORD
   ```

   Preflight evidence의 `androidSigningSecrets.workflow.status`도 `ready`여야 한다.

3. F-Droid build/buildserver evidence를 확보한 뒤:

   ```powershell
   pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1 `
     -FdroidBuildEvidencePath <path-to-fdroid-build-evidence.json>
   ```

4. 위 세 조건이 모두 충족되면 `docs/superpowers/plans/2026-07-01-remaining-work-priority.md`
   에서 release follow-up P1을 완료로 내린다.
