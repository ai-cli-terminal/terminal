# 2026-07-01 MSI Build Evidence Gate

## 목적

Windows MSI follow-up은 toolchain 존재 확인만으로 닫으면 안 된다. 실제
`-RunBuild` 실행이 성공하고 `.msi` 파일과 SHA256 hash가 evidence에 남아야 release
follow-up을 닫을 수 있다. 이번 작업은 MSI preflight와 combined release follow-up
preflight의 완료 기준을 이 수준으로 맞춘다.

## 조사

- `scripts/smoke-msi-preflight.ps1`는 `cargo`, `rustc`, `cl`, `link`, `rc`,
  `node`, `npm`, WiX 계열 command를 확인한다.
- 기존 판정은 toolchain이 있으면 `status=ready`가 될 수 있었고, `-RunBuild` 실행
  결과가 실패하거나 `.msi`가 없더라도 build 결과를 별도 blocker로 내리지 않았다.
- `docs/releases/release-followup-runbook.md`는 MSI completion evidence로
  `status: ready`, generated `.msi` path, SHA256 hash를 요구한다.

## 문서화 계획

| 대상 | Tutorial | How-to | Reference | Explanation |
|---|---|---|---|---|
| MSI build evidence gate | 없음 | runbook 보강 | preflight JSON 필드 | plan 설명 |

## 구현 범위

- `scripts/smoke-msi-preflight.ps1`
  - `-RunBuild`가 요청되면 build exit code, generated `.msi`, SHA256 hash를
    `checks`와 `missing`에 반영한다.
  - build command 실패, missing MSI, missing hash 중 하나라도 있으면
    `status=blocked`다.
- `scripts/smoke-release-followup-preflight.ps1`
  - nested MSI `checks`와 `build` payload를 combined evidence에 포함한다.
  - MSI toolchain만 ready이고 `-RunMsiBuild`가 없으면 combined preflight는
    final `ready`가 되지 않는다.

## 검증

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-msi-preflight.ps1
npm run smoke:release-followup-preflight
```

현재 개발 host에서는 Rust/MSVC/WiX toolchain이 없어 `MSI_PREFLIGHT_BLOCKED`와
`RELEASE_FOLLOWUP_PREFLIGHT_BLOCKED`가 정상이다.

외부 Windows-native packaging host에서는 다음 명령으로 완료 evidence를 만든다.

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-followup-preflight.ps1 -RunMsiBuild
```

`msi.checks.buildExitCodeZero`, `msi.checks.msiGenerated`,
`msi.checks.msiSha256Recorded`가 모두 `true`여야 MSI follow-up을 닫을 수 있다.

## 다음 작업

Windows-native Rust/MSVC/WiX host에서 `-RunMsiBuild`를 실행한다. Android signing
secret names와 F-Droid build/buildserver evidence까지 준비된 상태라면 combined
preflight가 `RELEASE_FOLLOWUP_PREFLIGHT_READY`를 출력해야 한다.
