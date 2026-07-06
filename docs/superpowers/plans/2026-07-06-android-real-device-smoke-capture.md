# 2026-07-06 Android Real-Device Smoke Capture

## Purpose

Verify the completed Android local UX hardening on a real device after the
workspace document export, selected-file helper, and Termux shared staging
diagnostics slices. This is the next local follow-up while release follow-up
remains blocked on external MSI, Android signing secrets, and F-Droid
build/buildserver evidence.

## Status

Started. Debug APK build is green, but real-device smoke is blocked until an
Android device appears in `adb devices`. No source changes should be needed
unless the real-device smoke finds a regression.

## Scope

- Build and install the debug APK on a connected Android device.
- Capture import/open/export behavior for app-private workspace documents.
- Capture selected-file helpers:
  - `List Files` prepares `ls`;
  - `Find Last` prepares a workspace-relative `ls <dir> | where name == <file> | first 1`.
- Capture Termux shared staging diagnostics:
  - app-write ok;
  - helper-marker ok;
  - external commands remain disabled until both pass.
- Record device model/serial, build command, install command, and observed
  transcript snippets.

## Non-Goals

- Do not close external release blockers from this host.
- Do not require real signing secrets.
- Do not make Termux/shared staging the Android default.
- Do not add new product behavior during smoke capture unless a bug is found.

## Preconditions

- Android SDK available through `ANDROID_HOME` / `ANDROID_SDK_ROOT`.
- A real Android device is visible in `adb devices`.
- For Termux staging diagnostics:
  - Termux installed from a supported source;
  - `allow-external-apps=true` configured in Termux;
  - `com.termux.permission.RUN_COMMAND` granted to AI Terminal;
  - Termux storage permission granted;
  - shared staging directory such as `/sdcard/Download/ash-termux-bridge`.

## Commands

```powershell
$env:ANDROID_HOME="$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_SDK_ROOT=$env:ANDROID_HOME
gradle -p android :app:assembleDebug

$adb="$env:LOCALAPPDATA\Android\Sdk\platform-tools\adb.exe"
& $adb devices
& $adb install -r android/app/build/outputs/apk/debug/app-debug.apk
& $adb shell pm grant dev.aiterminal.android com.termux.permission.RUN_COMMAND
```

Optional helper real-device instrumentation:

```powershell
gradle -p android :app:connectedDebugAndroidTest `
  -Pandroid.testInstrumentationRunnerArguments.termuxRealDeviceSmoke=true `
  -Pandroid.testInstrumentationRunnerArguments.termuxBridgeStagingDir=/sdcard/Download/ash-termux-bridge
```

## Evidence To Record

```text
Device:
APK:
Import:
Open Last:
Export Last:
List Files:
Find Last:
Termux staging app-write:
Termux staging helper-marker:
External command enabled only after staging smoke:
Screenshots / transcript capture path:
```

## Attempt Log

2026-07-06 local host:

- `gradle -p android :app:assembleDebug` passed.
- Debug APK: `android/app/build/outputs/apk/debug/app-debug.apk`
- APK size: `11,952,502` bytes.
- APK SHA256:
  `7b2941d0ff2448f9b1d3cef8d2dad29ddf9c48331ba459f57d7317ce6d751c16`
- `adb devices` started the daemon successfully but returned no attached
  devices.
- Next step: connect/unlock a physical Android device with USB debugging
  enabled, rerun `adb devices`, then install the debug APK.

## Pass Criteria

- Import shows bounded text preview metadata or binary/non-UTF-8 summary.
- `Open Last` never renders raw binary bytes.
- `Export Last` writes the imported file to a user-selected SAF destination.
- `List Files` and `Find Last` prepare commands without auto-running them.
- `Find Last` does not expose app-private absolute paths.
- Termux external commands remain disabled until app-write and helper-marker
  diagnostics both pass.
- Failure copy points to the failing diagnostic instead of a generic smoke
  failure.
