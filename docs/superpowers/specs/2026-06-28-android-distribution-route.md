# Android distribution route decision — 2026-06-28

## Decision

Ship Android experiments through direct APK/GitHub Release first, then prepare an F-Droid submission once release signing, metadata, and reproducible-build constraints are ready. Do not target Google Play for the Termux-enabled build until a policy review is complete.

## Rationale

- The Android app's value is local terminal behavior plus an explicit Termux bridge. That bridge can run user-entered commands through another app and shared storage. This is appropriate for direct APK and F-Droid-style power-user distribution, but it needs a stricter Play policy review before store submission.
- Google Play's Device and Network Abuse policy prohibits unauthorized device/network interference and flags apps or SDKs that download executable code outside Google Play. It also says runtime-loaded interpreted languages must not allow potential policy violations. Source checked 2026-06-28: https://support.google.com/googleplay/android-developer/answer/16559646
- Google Checks can be used before any Play attempt to scan for data safety, permission, and restricted API issues, but its results do not guarantee Play approval. Source checked 2026-06-28: https://developers.google.com/checks/guide/app-compliance/getting-started/play-policies
- F-Droid's path fits the current project better because it expects public source, FOSS dependencies, command-line builds, metadata, and review of binary blobs/anti-features. Source checked 2026-06-28: https://f-droid.org/docs/Inclusion_Policy/ and https://f-droid.org/en/docs/Submitting_to_F-Droid_Quick_Start_Guide/

## Product Boundary

- APK/F-Droid candidate: core shell UI, document import/export, read-only import reopen, Termux bridge behind explicit install/staging/smoke gates.
- Play candidate, if pursued later: likely a separate core-only or reduced bridge flavor unless policy review confirms the Termux flow is acceptable.
- No automatic Termux installation, no package-manager bootstrap, and no downloaded executable payloads from inside the app.

## Follow-Up Checklist

- Add release signing guidance outside the repo secrets.
- Add Android release metadata: app name, short/full descriptions, screenshots, changelog, license references.
- Initial Fastlane/F-Droid-style metadata is in `android/fastlane/metadata/android/en-US`.
- F-Droid/direct-release candidate uses one universal APK for now. Android `versionName` comes from root `VERSION`; `versionCode = major * 10000 + minor * 100 + patch`.
- Run `:app:verifyFdroidReleaseInputs` to check metadata, repository license files, and the matching changelog before packaging.
- Run `:app:assembleRelease` once signing/config is ready, plus `:app:testDebugUnitTest` before every Android PR.

## Local Packaging Probe

2026-06-28 initial probe:

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
.\gradlew.bat :app:assembleRelease :app:verifyNativeLibraries
```

Result: release resource/manifest/native packaging tasks started, then `:app:verifyNativeLibraries` failed because `app/src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86,x86_64}/libai_terminal.so` was missing.

Current host constraint:

- Windows has Android SDK/NDK `28.2.13676358`, but only the `windows-x86_64` NDK prebuilt is installed.
- WSL has Rust/Cargo, but no Linux-host Android NDK prebuilt.
- Windows has no `cargo`/`rustup` on PATH, so `android/build-rust-jni.ps1` cannot run in the Windows host shell yet.

Unblocked in the same session:

```bash
export ANDROID_NDK_HOME="$HOME/.local/opt/android-ndk-r28c-linux/android-ndk-r28c"
bash android/build-rust-jni.sh --profile release
```

The WSL path used NDK r28c Linux prebuilt downloaded to user-local storage and staged all four ABI files:

- `arm64-v8a/libai_terminal.so`
- `armeabi-v7a/libai_terminal.so`
- `x86/libai_terminal.so`
- `x86_64/libai_terminal.so`

After staging, this command passed:

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
.\gradlew.bat :app:assembleRelease :app:verifyNativeLibraries
```

Artifact after version alignment: `android/app/build/outputs/apk/release/app-release-unsigned.apk`, 8,942,194 bytes, SHA256 `21b82a2b68f25d143346244d1f0a7129c68ca6d96dfa5921462fb44700ba2aeb`.

APK manifest metadata:

```text
package: name='dev.aiterminal.android' versionCode='301' versionName='0.3.1'
```

Signing status: unsigned. `apksigner verify --verbose` reports `DOES NOT VERIFY` / missing `META-INF/MANIFEST.MF`, as expected for the unsigned release artifact. Signed releases are opt-in via `AI_TERMINAL_ANDROID_KEYSTORE`, `AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD`, `AI_TERMINAL_ANDROID_KEY_ALIAS`, and `AI_TERMINAL_ANDROID_KEY_PASSWORD`.

## Release Automation

The tag-triggered GitHub Release workflow now has an `android` job:

1. Install Android platform/build-tools/NDK r28c (`28.2.13676358`) on Ubuntu.
2. Build all four JNI ABIs with `bash android/build-rust-jni.sh --profile release`.
3. Optionally materialize a release keystore from GitHub secrets.
4. Run the checked-in Gradle wrapper: `cd android && ./gradlew :app:testDebugUnitTest :app:verifyFdroidReleaseInputs :app:assembleRelease :app:verifyNativeLibraries`.
5. Upload `ai-terminal-android-universal.apk` when signed, otherwise `ai-terminal-android-universal-unsigned.apk`, plus SHA256.

Expected GitHub secrets for signed output:

- `AI_TERMINAL_ANDROID_KEYSTORE_BASE64`
- `AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD`
- `AI_TERMINAL_ANDROID_KEY_ALIAS`
- `AI_TERMINAL_ANDROID_KEY_PASSWORD`

If any signing secret is missing, the release job intentionally falls back to the unsigned APK asset.

CI uses the same Android toolchain pin for the JNI packaging job: Gradle wrapper, Android API 35, build-tools 35.0.0, and NDK r28c.
