# Android F-Droid Reproducibility Prep — 2026-06-28

## Decision

Use one universal APK for the F-Droid/direct-release candidate. Do not introduce
ABI split APKs until there is a concrete size or update-channel need.

## Versioning

Android `versionName` is derived from the repository `VERSION` file.
`versionCode` is computed as:

```text
major * 10000 + minor * 100 + patch
```

For `VERSION=0.3.1`, Android uses:

```text
versionName = 0.3.1
versionCode = 301
```

The matching metadata changelog is:

```text
android/fastlane/metadata/android/en-US/changelogs/301.txt
```

For fdroiddata update checks, the same version values are mirrored in:

```text
android/fdroid-version.properties
```

This mirror exists because fdroidserver extracts update values with regex and
does not run Gradle to recompute the semver-derived `versionCode`.

Fastlane phone screenshots are checked in under:

```text
android/fastlane/metadata/android/en-US/images/phoneScreenshots/
```

Current screenshots were captured from the `Medium_Phone` emulator after
installing and launching the debug APK:

```text
01-home.png        139,284 bytes  sha256 6d68275038ef081bfb6247255c0a00579a7eb88677f97933a852d0bf70ae3ec3
02-run-result.png  143,973 bytes  sha256 24c26c9bf96bb1a2b3e51c08181cde7968f0bc2a9a075738ae3d7ae3d6461c8c
```

`02-run-result.png` includes the default shellcore query result (`# size` /
`200`), verified through the Android UI hierarchy after tapping `Run`.

## Build Contract

F-Droid/direct release builds should use the checked-in Gradle wrapper and the
same toolchain pins already used by CI:

- Android API 35
- build-tools 35.0.0
- NDK r28c, `28.2.13676358`
- Rust stable
- Linux-host NDK for `android/build-rust-jni.sh`

Expected commands:

```bash
export ANDROID_NDK_HOME="$HOME/.local/opt/android-ndk-r28c-linux/android-ndk-r28c"
bash android/build-rust-jni.sh --profile release
cd android
./gradlew :app:testDebugUnitTest :app:verifyFdroidReleaseInputs :app:assembleRelease :app:verifyNativeLibraries
```

## Signing

GitHub direct-release signing is opt-in through:

- `AI_TERMINAL_ANDROID_KEYSTORE_BASE64`
- `AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD`
- `AI_TERMINAL_ANDROID_KEY_ALIAS`
- `AI_TERMINAL_ANDROID_KEY_PASSWORD`

Local Gradle signing uses file-based equivalents:

- `AI_TERMINAL_ANDROID_KEYSTORE`
- `AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD`
- `AI_TERMINAL_ANDROID_KEY_ALIAS`
- `AI_TERMINAL_ANDROID_KEY_PASSWORD`

F-Droid should build from source and apply its own signing path. Do not commit
release keystores, generated `.so` files, or APK outputs.

Local signing wiring can be validated without real release secrets:

```powershell
android/smoke-release-signing.ps1
```

The smoke uses a throwaway keystore under `artifacts/android-signing-smoke`,
builds `app-release.apk`, verifies it with `apksigner`, and writes evidence to
`artifacts/android-signing-smoke/android-signing-smoke-evidence.json`.

Latest local result:

```text
app-release.apk
size: 8,950,386 bytes
sha256: 245844e4cc684c24868158be6edbb8443a7e9b310054f668cf2274dfa0da492f
apksigner: v2 verified, 1 signer
```

GitHub secret-shaped signing values can be validated through the same base64
decode path used by `.github/workflows/release.yml`:

```powershell
android/smoke-github-signing-secrets.ps1 -UseThrowawayKeystore
```

Without `-UseThrowawayKeystore`, the script expects:

- `AI_TERMINAL_ANDROID_KEYSTORE_BASE64`
- `AI_TERMINAL_ANDROID_KEYSTORE_PASSWORD`
- `AI_TERMINAL_ANDROID_KEY_ALIAS`
- `AI_TERMINAL_ANDROID_KEY_PASSWORD`

The throwaway GitHub-secret preflight writes run-specific APK hash and
certificate fingerprint evidence to:

```text
artifacts/android-github-signing-preflight/android-github-signing-preflight-evidence.json
```

Expected verifier result: `apksigner --print-certs` reports v2 verification and
one signer.

## Verification

`verifyFdroidReleaseInputs` checks that:

- root `VERSION` parses as semver-like `MAJOR.MINOR.PATCH`
- computed Android `versionCode` is positive
- title, short description, full description, and matching changelog exist and
  are non-empty
- repository license files `LICENSE-MIT` and `LICENSE-APACHE` exist and are
  non-empty, matching `Cargo.toml`'s `MIT OR Apache-2.0` declaration
- `android/fdroid-version.properties` matches root `VERSION` and the
  Gradle-computed Android `versionCode`
- the fdroiddata draft at
  `android/fdroiddata/metadata/dev.aiterminal.android.yml` references the
  expected repo, tag, NDK, JNI build command, Gradle release gate, and output APK
- at least two non-empty Fastlane phone screenshot PNGs are present

`verifyNativeLibraries` remains the ABI packaging gate for the universal APK.
