# Windows dev runtime workbench design

Date: 2026-06-29

## Decision

AI Terminal will evolve from a single bundled `ash` GUI terminal into a Windows
developer workbench that can manage WSL2 Ubuntu, Docker, and the AI CLI tools
`codex`, `claude`, and `gemini`.

The first implementation slice is UI-only:

- top ribbon bar
- tabs
- split panes
- pane-level runtime selector with `ash`, `Ubuntu`, `Docker`, `Codex`, `Claude`,
  and `Gemini`

The first slice must not install or mutate WSL, Docker, apt packages, npm
packages, or user credentials. Runtime execution and install/update automation
come in later slices.

## Runtime Model

| Runtime | First slice behavior | Later execution target |
|---|---|---|
| `ash` | existing bundled PTY session remains live | current `ash.exe` sidecar |
| `Ubuntu` | selectable pane runtime placeholder | managed WSL2 Ubuntu distro |
| `Docker` | selectable pane runtime placeholder | managed Docker Engine/Desktop plus image-first app installs |
| `Codex` | selectable pane runtime placeholder | CLI inside managed Ubuntu by default |
| `Claude` | selectable pane runtime placeholder | CLI inside managed Ubuntu by default |
| `Gemini` | selectable pane runtime placeholder | CLI inside managed Ubuntu by default |

## Installation Policy

1. App startup checks runtime status.
2. Missing/outdated runtimes are reported in the ribbon or settings surface.
3. Automatic install/update is allowed only after a clear UI approval unless the
   user has enabled unattended updates.
4. App installation prefers Docker images for internal apps.
5. Ubuntu apt is the second-level package manager for base runtime dependencies.
6. AI CLI credentials and auth sessions are never copied between tools.

## Future Slices

### S1: UI Shell

- Add ribbon, tabs, split panes, and pane-level runtime selection.
- Preserve existing `scripts/smoke-gui.ps1` behavior for the first `ash` pane.
- Do not execute non-`ash` runtime selections yet.

### S2: Runtime Inventory

- Add backend commands that report:
  - WSL availability
  - managed Ubuntu distro status
  - Docker availability/version
  - `codex`, `claude`, `gemini` availability/version
- UI startup renders these states without mutating the machine.
- Status probes are read-only: `wsl.exe --status`, `wsl.exe --list --verbose`,
  `docker --version`, and each AI CLI `--version`.
- The first implementation reports host PATH availability for AI CLIs. Installing
  or updating them inside managed Ubuntu remains S5.

### S3: WSL2 Ubuntu Manager

- Install or import a managed Ubuntu distro.
- Store the distro name in app config.
- Open pane sessions with `wsl.exe -d <managed-distro> -- bash -lc ...`.
- First executable S3 slice uses `AI_TERMINAL_UBUNTU_DISTRO` when set, otherwise
  the default `Ubuntu` distro. If that exact distro is not present, any installed
  Ubuntu-family WSL distro can be used for the Ubuntu pane.
- The live pane can be switched to `Ubuntu` and restarted into
  `wsl.exe -d <distro> --exec bash -l`.
- The ribbon install action starts `wsl.exe --install -d <distro>` only after an
  explicit click. It does not run silently on app startup.
- Import workflows and apt update/upgrade orchestration remain follow-up work
  after the first Ubuntu pane execution path is verified.

### S4: Docker Manager

- Detect Docker Desktop/Engine.
- Install or guide installation when missing.
- Prefer Docker image installs for internal apps.
- Expose image pull/update/status logs in panes.
- First executable S4 slice uses `AI_TERMINAL_DOCKER_IMAGE` when set, otherwise
  the default `ubuntu:24.04` image.
- Runtime inventory reports Docker CLI, Docker Engine reachability, and managed
  image presence. The Docker chip is `ready` only when the Engine is reachable
  and the managed image exists locally.
- The ribbon install action starts
  `winget install --exact --id Docker.DockerDesktop` only after an explicit
  click. It does not run silently on app startup.
- The ribbon pull action runs `docker pull <managed-image>` on explicit click.
- The live pane can be switched to `Docker` and restarted into
  `docker run --rm -it <managed-image> bash -l`.
- Internal app catalogs, Compose stacks, and per-app image policies remain
  follow-up work after the first Docker runtime path is verified.

### S5: AI CLI Installer

- Install/update `codex`, `claude`, and `gemini` inside the managed Ubuntu
  runtime.
- Run startup update checks.
- Let panes switch directly into each CLI.

## Acceptance Criteria for S1

1. The app shows a top ribbon bar with tab, split, and runtime controls.
2. The app shows a tab strip and at least one active tab.
3. Split H and Split V create a visible placeholder pane without breaking the
   existing live `ash` pane.
4. New Tab creates a tab-level placeholder workspace.
5. The runtime selector updates the active pane state for all six runtimes.
6. The first `ash` pane still starts with the existing Tauri `terminal_open`
   path and remains compatible with the current GUI smoke hooks.

## Out of Scope for S1

- Installing WSL2, Ubuntu, Docker, or AI CLIs.
- Running non-`ash` panes.
- Persisting tabs/panes across restarts.
- Credential management for AI CLI tools.
