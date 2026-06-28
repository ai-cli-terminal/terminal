# Windows GUI Terminal Pivot Design

Date: 2026-06-27

## Decision

Windows is no longer complete when `ash.exe` runs inside another terminal emulator.
The Windows product target is a standalone GUI terminal application:

- User-facing executable: `ai-terminal.exe`
- User-facing window: owned by `ai-terminal.exe`
- Terminal engine: xterm.js in a Tauri webview
- PTY backend: Rust + `portable-pty` over Windows ConPTY
- Shell runtime: bundled `ash.exe` launched as a child process inside the app PTY

`ash.exe` remains important, but it is now an internal runtime substrate for the GUI
app rather than the primary Windows UX.

## Non-Goals

- Do not treat Windows Terminal, PowerShell, cmd.exe, Git Bash, or MSYS windows as
  the Windows product surface.
- Do not mark PM-1 Windows complete from host-terminal `ash.exe` manual evidence.
- Do not build a marketing or landing screen before the terminal surface.

## Architecture

```text
ai-terminal.exe
  Tauri window
    xterm.js terminal viewport
      onData/onResize/events
  Rust backend commands
    TerminalSession manager
      portable-pty
        Windows ConPTY
          ash.exe child runtime
            shellcore + AI/safety/storage paths
```

The GUI app owns:

- window lifecycle
- PTY lifecycle
- resize propagation
- input/output streaming
- copy/paste/selection/scrollback behavior
- child cleanup on window close

`ash.exe` owns:

- structured shell evaluation
- Windows external command adapter
- safety gate
- AI routing
- usage/audit/storage behavior
- history/config behavior

## Completion Criteria

Windows PM-1 is complete only when a packaged `ai-terminal.exe` can be launched
directly and the following pass inside the app window without opening a separate
terminal:

- `ash` prompt appears
- normal commands stream output
- resize updates ConPTY dimensions
- Ctrl-C interrupts and Ctrl-D exits as expected
- copy/paste, selection, and scrollback work
- AI routing failures are fail-soft
- safety gate and audit paths still run
- app close kills child processes
- release artifact contains `ai-terminal.exe`, `ash.exe`, `ai.exe`, and checksums

## Status of Existing Windows Ash Work

Existing S1-S7 `ash.exe` work is retained as runtime substrate:

- config loading
- line editor/history
- safety gate/audit
- real AI provider routing
- usage recording
- MSYS bridge profile
- Windows execution adapter
- ConPTY smoke tests

The old Windows host-terminal manual plan remains useful as regression evidence
for the child runtime, not as the GUI product completion gate.
