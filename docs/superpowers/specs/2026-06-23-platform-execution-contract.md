# Platform Execution Contract — `ash` / `shellcore`

> **Date**: 2026-06-23
> **Scope**: PM-1B for the platform/mobile local terminal workflow.
> **Related**: `2026-06-23-platform-target-matrix-design.md`, `../plans/2026-06-23-platform-mobile-local-terminal-workflow.md`.

## 1. Boundary

`shellcore` owns pure language behavior:

- lexing, parsing, AST, value model, comparison ops
- pure pipeline evaluation such as literals, variables, records, lists, `where`, `get`, `first`, `length`
- REPL state that is safe to embed: cwd value, vars, exit request

Platform adapters own host execution:

- process spawn
- PTY/ConPTY
- PATH/PATHEXT lookup
- shell-specific invocation (`cmd.exe`, PowerShell, POSIX)
- filesystem/userland/network capabilities
- stream delivery and cancel/interrupt semantics

The code boundary is `shellcore::external::ExternalRunner`. `Engine::new()` uses the desktop process runner. `Engine::pure()` uses `DisabledRunner`, proving parser/evaluator/builtins can run without process spawn.

## 2. Command Resolution

| Target | Rule |
|---|---|
| Pure/mobile-core | Builtins only. Unknown commands fail with `external execution disabled` before PATH lookup. |
| Linux/WSL | Direct spawn by command name/path in current `cwd`, inheriting current env. PATH resolution is host OS behavior for now. |
| Windows native | Adapter resolves direct `.exe`, PATHEXT `.cmd/.bat`, and `.ps1` explicitly. `.cmd/.bat` go through `cmd.exe /d /c`; `.ps1` goes through PowerShell as an execution target, not `ash` grammar. |
| Android | Spike decides between `shellcore-only`, Termux-compatible userland, or bundled minimal userland. Pure core remains valid before that decision. |
| iOS/iPadOS | Research target starts as builtins/pure shellcore only. No arbitrary downloaded code or external userland promise. |
| PWA/WASM | Pure shellcore only. No native process execution. |

## 3. Arguments And Quoting

`shellcore` evaluates command arguments into `Value`, then host adapters receive `Vec<Value>`. Desktop currently coerces each value with `Value::coerce_string()` and passes argv directly to `std::process::Command`, so no intermediate shell quoting is applied.

Windows native uses three invocation kinds:

- Direct: `.exe`, `.com`, and explicit non-script files run via `std::process::Command`.
- Cmd script: `.cmd` and `.bat` run via `cmd.exe /d /c <script> ...args`.
- PowerShell script: `.ps1` runs via `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <script> ...args`.

The adapter preserves argv boundaries by passing arguments as process arguments after the resolved target. PowerShell syntax is not parsed as `ash` syntax; PowerShell is only a host for `.ps1` execution.

## 4. Cwd And Workspace

`Engine.cwd` is the logical working directory. Builtins such as `cd` and `ls` use it directly. External runners receive `cwd` in `ExternalCommand`.

Mobile adapters must map this to an app workspace:

- Android: app-private workspace or user-selected document tree.
- iOS/iPadOS: app container or document picker location only.
- PWA: virtual workspace only.

## 5. Env Policy

Desktop runner inherits the current process environment today. PM-2+ should narrow this through the existing context/env policy before `ash` becomes the safety-gated execution path.

Mobile and PWA adapters should default to an explicit allowlist. Secrets from the existing `context` and `mask` policy must not cross FFI, logs, or remote companion boundaries.

## 6. Streams And Exit Codes

Current desktop runner inherits stdout/stderr directly and returns `Value::Nothing`. A non-zero exit writes `[name: exit code]` and keeps the REPL alive.

Future platform adapters should expose a structured stream model:

- stdout chunks
- stderr chunks
- final exit code or signal/cancel reason
- capability-specific interrupt support

The REPL can keep rendering as text, while mobile UI and tests can consume structured events.

## 7. Capability Flags

| Capability | Meaning |
|---|---|
| `can_spawn` | Adapter can start host processes. |
| `has_pty` | Adapter can provide POSIX PTY behavior. |
| `has_conpty` | Adapter can provide Windows ConPTY behavior. |
| `has_userland` | Target has useful external commands beyond `shellcore` builtins. |
| `can_write_workspace` | Adapter can write files in the active workspace. |
| `can_network` | Adapter can open network connections directly. |

| Target | can_spawn | has_pty | has_conpty | has_userland | can_write_workspace | can_network |
|---|---:|---:|---:|---:|---:|---:|
| Pure/mobile-core | no | no | no | no | no | no |
| Linux/WSL desktop | yes | planned | no | yes | yes | yes |
| Windows native | planned | no | planned | yes | yes | yes |
| Git Bash/MSYS | planned | profile-dependent | no | yes | yes | yes |
| Android spike | TBD | TBD | no | TBD | yes | TBD |
| iOS/iPadOS research | no first | no | no | limited | yes, container only | TBD |
| PWA/WASM | no | no | no | no | virtual only | browser-limited |

## 8. PM-1 Result

PM-1 chooses the trait-backed adapter path over feature-gating `external::run`. The reason is product shape: desktop, Android, iOS, PWA, and remote-host targets differ in kind, not just compile-time availability. A runtime adapter lets one Rust `shellcore` expose a pure embedding mode while still allowing desktop `ash` to spawn commands.
