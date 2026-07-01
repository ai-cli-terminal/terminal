# 2026-07-01 v0.3.3 Release Body

## 목적

`v0.3.3` GitHub Release의 body가 비어 있어 사용자가 어떤 자산을 받아야 하는지
알기 어렵다. 태그와 업로드된 자산은 그대로 두고, release body만 보강해 Windows GUI,
CLI/runtime, Android unsigned APK의 역할을 명확히 설명한다.

## 현재 확인

- Repository: `https://github.com/ai-cli-terminal/terminal.git`
- Default branch: `main`
- Release URL: `https://github.com/ai-cli-terminal/terminal/releases/tag/v0.3.3`
- Release state: published, not draft, not prerelease
- Before change: `body` was empty
- Uploaded assets:
  - `AI.Terminal_0.3.3_x64-setup.exe`
  - `ai-terminal-windows-x86_64-pc-windows-msvc.zip`
  - `ai-windows-x86_64.exe`
  - `ash-windows-x86_64.exe`
  - `ai-linux-x86_64`
  - `ash-linux-x86_64`
  - `ai-terminal-android-universal-unsigned.apk`
  - matching `.sha256` files for all listed assets

## 결정

- Release tag and assets are left untouched.
- Body copy lives in `docs/releases/v0.3.3-release-body.md` and is applied to
  GitHub with `gh release edit v0.3.3 --notes-file ...`.
- The body explicitly says:
  - Windows GUI users should use the installer or portable package.
  - `ai-windows-x86_64.exe` is a CLI helper, not the GUI app.
  - Android APK is unsigned and not the final signed/F-Droid evidence.
  - SHA256 files are provided, while signed binary trust-channel verification is
    still future work.

## 완료 조건

- [x] Verify current GitHub release body and asset names.
- [x] Add release body source file in the repo.
- [x] Apply body to the published GitHub release without changing tag/assets.
- [x] Verify GitHub release body is no longer empty.
- [x] Update `docs/HANDOFF.md`, `docs/HISTORY.md`, `docs/TASK.md`,
  `docs/TROUBLESHOOTING.md`, and
  `docs/superpowers/plans/2026-07-01-remaining-work-priority.md`.

## 검증

```powershell
gh release view v0.3.3 --repo ai-cli-terminal/terminal --json body,tagName,url
git diff --check
```

## 다음 작업

1. Windows MSI packaging evidence on a Windows-native Rust/MSVC/WiX host.
2. Android real signing secrets plus F-Droid build/buildserver evidence.
3. RA/PWA relay/M2 planning after release follow-up blockers are resolved or
   explicitly deferred.
