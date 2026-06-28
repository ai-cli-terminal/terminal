# Windows GUI Terminal S0/S1 Plan

Date: 2026-06-27

## Objective

Convert Windows PM-1 from a host-terminal `ash.exe` verification effort into a
standalone GUI terminal implementation path for `ai-terminal.exe`.

## G0 Direction Lock

- [x] Update `docs/TASK.md` so PM-1 completion requires the GUI app.
- [x] Update `docs/HANDOFF.md` with the new next work sequence.
- [x] Update `docs/HISTORY.md` with the pivot decision.
- [x] Update `README.md` to distinguish current `ash.exe` runtime artifacts from
  the target GUI terminal.
- [x] Add pivot spec:
  `docs/superpowers/specs/2026-06-27-windows-gui-terminal-pivot-design.md`.

## G1 Desktop App Scaffold

- [x] Add `desktop/` Tauri v2 app skeleton.
- [x] Make the first viewport the terminal surface, not a landing page.
- [x] Add xterm.js terminal view with fit/resize behavior.
- [x] Add Tauri command/event bridge for terminal open/write/resize/kill.
- [x] Add Rust `TerminalSession` manager backed by `portable-pty`.
- [x] Resolve bundled or env-provided `ash.exe` as the GUI child runtime.

## G2 Follow-Up PTY Hardening

- [x] Add restart command and UI affordance after child exit.
- [x] Add explicit child cleanup command for app/window shutdown.
- [x] Add Windows packaging sidecar path policy for `ash.exe` and `ai.exe`.
- [x] Add repeatable sidecar staging and portable package/archive scripts with
  SHA256 manifests.
- [x] Add Windows GUI portable smoke script for process/window/child-cleanup
  evidence.
- [x] Add screenshot evidence to GUI smoke:
  `gui-smoke-screenshot.png` / `installed-gui-smoke-screenshot.png`.
- [x] Add command transcript evidence to GUI smoke:
  `print AI_TERMINAL_GUI_SMOKE_OK` is written through the app's normal
  Tauri `terminal_write` path and `gui-smoke-transcript.txt` must contain
  `AI_TERMINAL_GUI_SMOKE_OK` with no `error:`.
- [x] Add resize evidence to GUI smoke:
  `gui-smoke-resize-screenshot.png` plus before/target/after window bounds.
- [x] Add Ctrl-D exit evidence to GUI smoke:
  `gui-smoke-ctrl-d-screenshot.png` / `installed-gui-smoke-ctrl-d-screenshot.png`
  plus `ctrlD.ashExited=true`.
- [x] Add Ctrl-C recovery evidence to GUI smoke:
  `gui-smoke-ctrl-c-screenshot.png` / `installed-gui-smoke-ctrl-c-screenshot.png`
  plus `ctrlC.recovered=true` and `ctrlC.ashStillRunning=true`.
- [x] Add frontend terminal UX evidence to GUI smoke:
  `gui-smoke-frontend-evidence.json` / `installed-gui-smoke-frontend-evidence.json`
  plus `frontend.selection.selected=true`, `frontend.copy.copied=true`,
  `frontend.paste.dispatched=true`, and `frontend.scrollback.scrolled=true`.
- [x] Add ash integration evidence to GUI smoke:
  `gui-smoke-ash-integration-evidence.json` /
  `installed-gui-smoke-ash-integration-evidence.json` plus
  `ashIntegration.transcriptEvidence.aiRouted=true`,
  `ashIntegration.transcriptEvidence.safetyGateBlocked=true`,
  `ashIntegration.transcriptEvidence.externalCommandRan=true`,
  `ashIntegration.database.usagePersisted=true`,
  `ashIntegration.database.commandHistoryPersisted=true`, and
  `ashIntegration.database.auditBlockedPersisted=true`.
- [x] Run automated integration smoke on real Windows for package checksum,
  visible main window, `ash.exe` child process, unexpected external shell
  absence, visual prompt/input/output screenshot, transcript output, resize
  screenshot, frontend terminal UX evidence, GUI-internal AI/safety/storage/audit
  evidence, Ctrl-C recovery evidence, Ctrl-D exit evidence, and close cleanup.
- [x] Confirm terminal prompt/output inside the window on real Windows.
- [x] Verify resize on real Windows through automated smoke bounds + screenshot.
- [x] Verify copy-paste/selection/scrollback on real Windows through automated smoke.
- [x] Verify Ctrl-C recovery on real Windows through automated smoke.
- [x] Verify Ctrl-D exit on real Windows through automated smoke.

## G4 Windows Installer Packaging

- [x] Add repeatable WSL NSIS setup script:
  `desktop/scripts/setup-wsl-nsis.sh`.
- [x] Generate NSIS installer artifact:
  `desktop/src-tauri/target/x86_64-pc-windows-gnu/release/bundle/nsis/AI Terminal_0.1.0_x64-setup.exe`.
- [x] Generate installer SHA256 sidecar.
  Latest verified SHA256:
  `c90253b6b2f08a097114094ef84240c304ca8fe6c5aa73e7e1b9e194d1d776bd`.
- [x] Add and run installer install/launch/uninstall smoke:
  `scripts/smoke-nsis.ps1`.
- [ ] Revisit MSI on a Windows-native packaging host; Linux cross-host ignored
  MSI during bundling.

## Verification Plan

Local lightweight checks:

- `npm install`
- `npm run build`
- `cargo check --manifest-path desktop/src-tauri/Cargo.toml`
- `cargo check --manifest-path desktop/src-tauri/Cargo.toml --target x86_64-pc-windows-gnu`
- `npm run tauri -- build --target x86_64-pc-windows-gnu --no-bundle --ci`
- `npm run stage:windows-sidecars`
- `npm run package:windows-portable`
- `npm run setup:wsl-nsis`
- `npm run tauri -- build --target x86_64-pc-windows-gnu --ci --config src-tauri/tauri.windows.conf.example.json`
- Windows only: `pwsh scripts/smoke-gui.ps1`
- Windows only: `pwsh scripts/smoke-nsis.ps1`
- `git diff --check`

Full Windows checks require a Rust/Tauri toolchain:

- `npm run tauri build` with sidecar binaries staged
- manual launch of packaged `ai-terminal.exe`
