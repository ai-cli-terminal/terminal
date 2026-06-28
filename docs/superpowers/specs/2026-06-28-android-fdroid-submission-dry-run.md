# Android F-Droid Submission Dry Run — 2026-06-28

## Decision

Keep a fdroiddata-style submission draft in-tree at:

```text
android/fdroiddata/metadata/dev.aiterminal.android.yml
```

This file is not the authoritative upstream fdroiddata entry yet. It is the
reviewable draft to copy into a fdroiddata fork for `fdroid lint`,
`fdroid rewritemeta`, and buildserver testing.

## Why A Version Mirror Exists

The Android app derives `versionName` from root `VERSION` and computes
`versionCode` as `major * 10000 + minor * 100 + patch`. F-Droid update checks
use regex extraction and do not run Gradle or recompute dynamic version logic.

Therefore `android/fdroid-version.properties` mirrors the computed release
values:

```text
versionName=0.3.1
versionCode=301
```

`:app:verifyFdroidReleaseInputs` verifies this mirror against root `VERSION` and
the Gradle-computed Android `versionCode`.

## Draft Build Shape

The draft builds from source and lets F-Droid sign the APK:

```text
disable: Pending next Android release tag that includes F-Droid metadata and fdroid-version.properties
commit: TODO_NEXT_ANDROID_RELEASE_COMMIT
subdir: android
output: app/build/outputs/apk/release/app-release-unsigned.apk
prebuild: rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android
  x86_64-linux-android
build:
  - ANDROID_NDK_HOME="$$NDK$$" ./build-rust-jni.sh --profile release --no-rustup-target-install
  - ./gradlew :app:verifyFdroidReleaseInputs :app:assembleRelease :app:verifyNativeLibraries
ndk: 28.2.13676358
```

The build depends on Rust targets for Android. The draft prebuild step adds:

```text
aarch64-linux-android
armv7-linux-androideabi
i686-linux-android
x86_64-linux-android
```

## Current Limitations

- The draft has not been run inside fdroidserver/buildserver yet.
- The build block is intentionally disabled until a concrete release commit is
  selected for fdroiddata submission. Use
  `android/smoke-fdroid-release-activation.ps1 -Commit <hash>` to generate and
  lint the activated metadata.
- Reproducible binary matching against a developer-signed GitHub APK is blocked
  until a stable Android release signing key and certificate fingerprint exist.
- `Cargo.toml` declares `MIT OR Apache-2.0`; the fdroiddata draft uses
  `Apache-2.0` for the Android app entry because the APK includes AndroidX /
  Compose Apache-licensed dependencies.
- The source repository currently contains generated local JNI outputs under
  `android/app/src/main/jniLibs` in the working tree, but they are ignored and
  not part of the fdroiddata source build contract.

## Actual F-Droid Commands

In a fdroiddata fork:

```bash
cp /path/to/terminal/android/fdroiddata/metadata/dev.aiterminal.android.yml metadata/dev.aiterminal.android.yml
fdroid lint dev.aiterminal.android
fdroid rewritemeta dev.aiterminal.android
fdroid build dev.aiterminal.android:301
```

## Local Dry-Run Result

Local fdroidserver was installed into an isolated venv under
`artifacts/fdroid-dry-run/venv` with `virtualenv.pyz`; no system Python packages
were installed.

The reproducible local wrapper is:

```powershell
pwsh -NoProfile -ExecutionPolicy Bypass -File .\android\smoke-fdroid-metadata.ps1
```

The next-release activation preflight is:

```powershell
$commit = git rev-parse --verify <android-release-tag>^{commit}
pwsh -NoProfile -ExecutionPolicy Bypass -File .\android\smoke-fdroid-release-activation.ps1 -Commit $commit
```

This produces an activated metadata copy under
`artifacts/fdroid-activation-smoke/` without modifying the source draft. It
removes the temporary `disable`, replaces `TODO_NEXT_ANDROID_RELEASE_COMMIT`,
runs `fdroid rewritemeta`, and then lints the canonicalized metadata. Use
`-Apply` only when preparing the actual fdroiddata submission after the release
commit/tag exists.

The local fdroiddata dry-run directory is:

```text
artifacts/fdroid-dry-run/fdroiddata/
```

Because cloning the full fdroiddata repository was too slow in this Windows/WSL
workspace, the dry-run copied only the upstream `config/categories.yml` and its
referenced category icons. With that config in place:

```text
fdroid lint dev.aiterminal.android
fdroid rewritemeta dev.aiterminal.android
```

Both commands passed on 2026-06-28. `rewritemeta` produced no source diff after
the metadata draft was canonicalized. The only remaining local warning is:

```text
apksigner not found! Cannot sign or verify modern APKs
```

That warning is a dry-run environment limitation, not a metadata lint failure.
Actual build verification still needs a full fdroidserver/buildserver
environment with Android SDK build-tools on PATH.

`smoke-fdroid-release-activation.ps1` also passed on 2026-06-28 using the local
`HEAD` hash as a stand-in release commit. It produced a canonical activated
metadata copy and left the source draft disabled.

Official references checked on 2026-06-28:

- https://f-droid.org/docs/Build_Metadata_Reference/
- https://f-droid.org/docs/Submitting_to_F-Droid_Quick_Start_Guide/
- https://f-droid.org/docs/Update_Processing/
