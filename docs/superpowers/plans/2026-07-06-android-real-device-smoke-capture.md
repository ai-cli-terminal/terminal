# 2026-07-06 Android Real-Device Smoke Capture

## Purpose

Verify the completed Android local UX hardening on a real device after the
workspace document export, selected-file helper, and Termux shared staging
diagnostics slices. This is the next local follow-up while release follow-up
remains blocked on external MSI, Android signing secrets, and F-Droid
build/buildserver evidence.

## Status

Completed. Debug APK build, real-device Termux helper instrumentation, and
manual app UI smoke all passed on an authorized Android device. No source
changes were needed.

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
  "-Pandroid.testInstrumentationRunnerArguments.termuxRealDeviceSmoke=true" `
  "-Pandroid.testInstrumentationRunnerArguments.termuxBridgeStagingDir=/sdcard/Download/ash-termux-bridge"
```

## Evidence To Record

```text
Device: SM-F956N / R3CX60P3R5K / Android 16 / SDK 36
APK: android/app/build/outputs/apk/debug/app-debug.apk
Import: ai-terminal-reader-smoke.txt imported as text, 44 bytes
Open Last: reopened text preview, 44 bytes, 3 lines, 44 bytes read
Export Last: SAF save completed, exported 44 bytes
List Files: prepared command: ls
Find Last: prepared ls "." | where name == "ai-terminal-reader-smoke.txt" | first 1
Termux staging app-write: ok ash-termux-bridge
Termux staging helper-marker: ok with ASH_SHARED_STAGING_OK
External command enabled only after staging smoke: external / staging after both diagnostics
Screenshots / transcript capture path: artifacts/android-real-device-smoke/
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

2026-07-06 retry:

- `adb devices` detected `R3CX60P3R5K`.
- Device state: `unauthorized`.
- ADB server restart did not change the state.
- Next step: unlock the device and accept the `Allow USB debugging` prompt,
  then rerun `adb devices` and continue with install/grant.

2026-07-06 authorized device smoke:

- `adb devices` detected `R3CX60P3R5K` with state `device`.
- Device: `SM-F956N`, Android `16`, SDK `36`.
- `adb install -r android/app/build/outputs/apk/debug/app-debug.apk` passed.
- `adb shell pm grant dev.aiterminal.android com.termux.permission.RUN_COMMAND`
  passed.
- `adb shell am start -n dev.aiterminal.android/.MainActivity` launched the
  debug app before instrumentation.
- Initial unquoted instrumentation retry failed in PowerShell because Gradle
  parsed `.testInstrumentationRunnerArguments.termuxRealDeviceSmoke=true` as a
  task name.
- Quoted instrumentation command passed:

```powershell
gradle -p android :app:connectedDebugAndroidTest `
  "-Pandroid.testInstrumentationRunnerArguments.termuxRealDeviceSmoke=true" `
  "-Pandroid.testInstrumentationRunnerArguments.termuxBridgeStagingDir=/sdcard/Download/ash-termux-bridge"
```

- Result XML:
  `android/app/build/outputs/androidTest-results/connected/debug/TEST-SM-F956N - 16-_app-.xml`
- Result: `tests="4" failures="0" errors="0" skipped="0"`.
- Passed real-device helper cases:
  - `TermuxHelperRealDeviceSmokeTest.helperBootstrapAndEventFileSmokes`
  - `TermuxHelperRealDeviceSmokeTest.helperCancelSmoke`
- UTP log confirmed the runner args:
  - `termuxRealDeviceSmoke=true`
  - `termuxBridgeStagingDir=/sdcard/Download/ash-termux-bridge`
- Note: the Gradle connected test uninstalled `dev.aiterminal.android` after
  execution, so reinstall before continuing manual UI capture.
- Next step: reinstall the debug APK, then capture the manual app-private
  import/open/export and selected-file helper behavior.

2026-07-06 manual UI smoke:

- Reinstalled debug APK after connected test cleanup and relaunched
  `dev.aiterminal.android/.MainActivity`.
- Created `/sdcard/Download/ai-terminal-reader-smoke.txt` with two text lines
  for SAF import/export smoke.
- Initial screen showed `Open Last`, `Export Last`, and `Find Last` disabled
  before import.
- `Import` via Android DocumentsUI `다운로드` selected
  `ai-terminal-reader-smoke.txt`.
- Import transcript showed:
  - `imported ai-terminal-reader-smoke.txt (44 bytes, text)`
  - `preview ai-terminal-reader-smoke.txt (3 lines, 44 bytes read)`
  - sample text `hello from android reader smoke` and `second line`
- `Open Last` transcript showed:
  - `open ai-terminal-reader-smoke.txt (44 bytes, 3 lines, 44 bytes read)`
  - no app-private absolute path and no raw binary rendering
- `List Files` prepared input `ls` without auto-running it.
- `Find Last` prepared input
  `ls "." | where name == "ai-terminal-reader-smoke.txt" | first 1` without
  auto-running it or exposing an app-private absolute path.
- `Export Last` opened Android SAF save UI in `다운로드`, defaulted to
  `ai-terminal-reader-smoke.txt`, and returned to the app with:
  - `exported ai-terminal-reader-smoke.txt (44 bytes)`
- `Verify` for shared staging showed:
  - status `external / staging`
  - `termux: Termux shared staging ready: ash-termux-bridge`
  - `termux staging app-write: ok ash-termux-bridge`
  - `ASH_SHARED_STAGING_OK`
  - `termux staging helper-marker: ok`
  - `termux staging: ok`
- Captured local evidence under `artifacts/android-real-device-smoke/`
  (ignored smoke artifacts, not committed), including:
  - `ai-terminal-after-import-selected.png`
  - `ai-terminal-after-open-last.png`
  - `ai-terminal-after-find-last.png`
  - `ai-terminal-export-last-save-ui.png`
  - `ai-terminal-after-export-save-tap.png`
  - `ai-terminal-after-verify-staging.png`
- Result: pass.

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
