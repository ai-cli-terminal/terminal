This directory is the staging point for Tauri sidecar binaries.

The bundle config declares `bin/ash` and `bin/ai` as external binaries. Tauri
resolves target-specific files by appending the target triple and extension, for
example:

- `ash-x86_64-pc-windows-msvc.exe`
- `ai-x86_64-pc-windows-msvc.exe`
- `ash-x86_64-pc-windows-gnu.exe`
- `ai-x86_64-pc-windows-gnu.exe`

Packaging must build or copy the release `ash` and `ai` binaries here before
running `npm run tauri build`.
