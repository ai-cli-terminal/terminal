# Platform + Mobile Local Terminal — Task Workflow Plan

> **For agentic workers:** Use this as the execution plan for the platform pivot. Work task-by-task, keep `docs/TASK.md` in sync after each completed slice, and update `docs/HISTORY.md` only when behavior or direction actually changes.

**Goal:** Turn the 2026-06-23 platform decision into executable work. The product direction is now a shared `ash` local terminal across desktop and mobile. Android becomes the first mobile local-terminal target. iOS/iPadOS stays in research because the viable product shape is constrained. PWA/remote approval remains a companion, not the mobile product body.

**Source of truth:**

- Matrix: `docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md`
- Independent shell: `docs/superpowers/specs/2026-06-05-independent-shell-s0-core-design.md`
- Roadmap: `docs/superpowers/specs/2026-06-05-phase3-roadmap-design.md`
- Backlog: `docs/TASK.md`
- Shared development flow: `docs/WORKFLOW.md`

---

## 0. Current Progress Snapshot

| Area | Status | Evidence | Next gap |
|---|---|---|---|
| `ai` release line | Done | `Cargo.toml` version `0.2.2`, Linux/Windows release docs and scripts exist | Keep release continuity while `ash` grows |
| Phase 1/2 safety core | Done | risk, policy, masking, preview, undo, usage, context, guardrails, provider, gateway, dispatch modules | Wire mature safety path into `ash` execution |
| Remote approval base | Partial | M0 through M1 slice 4a are implemented: gate, Noise, validation, daemon substrate, framed transport | Real listener, pairing, device registration, gate-to-device flow, PWA companion |
| `ash` / `shellcore` | Partial | `[[bin]] name = "ash"`, `src/bin/ash.rs`, `src/shellcore/*`; value model, lexer/parser/engine, builtins, external spawn, REPL; S1a `where` filtering exists | Platform adapters, line editor, history, config, AI/safety gate integration |
| Platform target matrix | Done | 2026-06-23 matrix defines Linux/WSL/Windows/Git Bash/PowerShell/Android/iOS/PWA targets | Convert matrix into implementation slices |
| Windows native `ash.exe` | Not started | `ai.exe` release exists; `ash.exe` product smoke not yet in CI/release | ConPTY/execution adapter, PATH/PATHEXT, `.ps1`/cmd handling |
| Android local terminal | Not started | Direction decided only | Rust core boundary, UI, worker process/thread, workspace/files, external command strategy |
| iOS/iPadOS local terminal | Research | Direction decided only | Self-contained shellcore REPL, file container, policy-safe command subset |
| PWA/mobile companion | Partial concept | Remote approval mockup/design lineage exists | Keep as approval/pairing/monitoring companion, not a local terminal substitute |

**Last known verification:** WSL `cargo test --features "storage tls remote"` passed with 342 tests, and `ash` smoke for `[{size: 50} {size: 200}] | where size > 100` returned only the `size 200` row.

---

## 1. Execution Order

```text
PM-0 docs/backlog sync
  -> PM-1 shared shellcore platform boundary
  -> PM-2 Windows native ash.exe
  -> PM-3 Android local terminal spike
  -> PM-4 iOS/iPadOS research spike
  -> PM-5 RA/PWA companion reuse
  -> PM-6 packaging and public docs
```

Do not let RA/PWA work block the mobile local-terminal track. RA is valuable, but the mobile product is now the local terminal.

---

## PM-0. Documentation and Backlog Sync

**Goal:** Make every planning surface say the same thing.

**Files:** `README.md`, `docs/TASK.md`, `docs/WORKFLOW.md`, `docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md`, this plan.

- [x] Platform matrix defines mobile as local terminal, not PWA.
- [x] TASK has a Platform Pivot section.
- [x] WORKFLOW has a platform/mobile task workflow.
- [x] README exposes both the target matrix and this workflow plan.
- [ ] HISTORY gets an entry when implementation begins, not for planning-only edits.

**Verification:**

```powershell
rg -n "모바일 로컬 터미널|Android|iOS|PWA|platform-mobile-local-terminal-workflow" README.md docs
git diff --check
```

---

## PM-1. Shared `shellcore` Platform Boundary

**Goal:** Keep one Rust shell language core while separating host-specific execution.

### PM-1A — Core purity audit

**Files:** `src/shellcore/*`

- [x] List all desktop-only dependencies in `shellcore`.
- [x] Split pure evaluation from external process execution.
- [x] Decide whether `external::run` becomes a trait-backed adapter or is feature-gated behind a desktop runner.
- [x] Add a `mobile-core` test profile or equivalent that proves parser/evaluator/builtins work without spawning OS processes.

Audit result: `shellcore` desktop-only coupling was concentrated in `external.rs`
(`std::process::Command`) plus filesystem builtins (`cd`/`ls`) and REPL process exit.
Process spawn is now behind `shellcore::external::ExternalRunner`; `Engine::pure()`
uses `DisabledRunner` so mobile/PWA embedding can run literals, variables, tables,
`where`, `get`, `first`, and `length` without PATH lookup or OS spawn. Filesystem
builtins remain explicit workspace operations and need mobile workspace adapters in
PM-3/PM-4.

**DoD:** `shellcore` can be embedded by mobile UI code without accidentally requiring PTY, desktop env, or unrestricted process spawn.

### PM-1B — Platform execution contract

**Design output:** Add a spec or update the matrix with:

- command name resolution
- argv quoting
- cwd and workspace root
- env allowlist/denylist
- stdout/stderr stream model
- exit code model
- capability flags: `can_spawn`, `has_pty`, `has_conpty`, `has_userland`, `can_write_workspace`, `can_network`

**DoD:** Windows, Android, iOS, and PWA can each say which parts of the contract they implement.

Output: `docs/superpowers/specs/2026-06-23-platform-execution-contract.md`.

### PM-1C — Shared smoke tests

- [x] Add `ash` smoke fixtures that run on Linux/WSL.
- [ ] Add Windows `ash.exe` smoke once Windows adapter exists.
- [x] Add pure `shellcore` tests that do not call external commands.

**Baseline smoke:**

```bash
printf '[{size: 50} {size: 200}] | where size > 100\nexit\n' | cargo run --bin ash
```

Expected: only the `size 200` row is printed.

---

## PM-2. Windows Native `ash.exe`

**Goal:** Make Windows a first-class local terminal target, not only a host for `ai.exe`.

### PM-2A — Windows execution adapter

- [x] Define direct spawn vs `cmd.exe /c` vs PowerShell invocation rules.
- [x] Implement PATH/PATHEXT resolution for `.exe`, `.cmd`, `.bat`, and `.ps1`.
- [ ] Preserve exit codes exactly.
- [ ] Add quoting tests for spaces, quotes, backslashes, and PowerShell arguments.
- [x] Treat PowerShell as an execution target/host, not as `ash` grammar.

Progress: `src/shellcore/winexec.rs` now defines pure Windows resolution and
invocation classification. Windows `DesktopRunner` uses it to route direct
executables, `.cmd/.bat` scripts, and `.ps1` scripts through distinct hosts.
Linux/WSL keep the existing direct-spawn path.

**DoD:** `ash.exe` can run native Windows commands predictably without pretending to be PowerShell.

### PM-2B — ConPTY and terminal behavior

- [ ] Verify portable-pty ConPTY behavior with interactive programs.
- [ ] Record capability limitations in `ai doctor --guardrails` or equivalent platform output.
- [ ] Keep WSL and native Windows install docs separate.

### PM-2C — CI and release

- [ ] Add Windows `cargo build --bin ash`.
- [ ] Add Windows `ash.exe` smoke.
- [ ] Decide whether release assets include both `ai.exe` and `ash.exe`, or package them together.

---

## PM-3. Android Local Terminal Spike

**Goal:** Prove that Android can host the real mobile local terminal.

### PM-3A — App shell decision

- [ ] Choose spike shell: Kotlin/Compose + Rust FFI by default unless another app shell is justified.
- [ ] Keep the spike small: one screen, input line, output pane, local workspace selector.
- [ ] Do not add Play Store promises until the process/userland strategy is proven.

### PM-3B — Rust core embedding

- [ ] Expose a minimal FFI boundary: `eval_line(input, session_state) -> output + updated_state`.
- [ ] Return structured values as JSON or a stable typed bridge.
- [ ] Keep panics from crossing the FFI boundary.
- [ ] Test with list/record literals, `where`, variables, `cd` equivalent if present, and error output.

**DoD:** Android can evaluate pure `shellcore` commands locally without network or desktop daemon.

### PM-3C — Terminal UI and worker model

- [ ] Run evaluation/execution off the UI thread.
- [ ] Decide thread vs process for the shell worker.
- [ ] Stream output back to the UI incrementally.
- [ ] Support cancel/interrupt at least for long-running core operations.

**DoD:** The UI stays responsive while the terminal session is busy.

### PM-3D — Workspace and files

- [ ] Define app-private workspace root.
- [ ] Define import/export through Android document APIs.
- [ ] Keep secret/path masking boundaries from desktop safety core.
- [ ] Add a visible workspace/cwd model for narrow mobile screens.

### PM-3E — External command strategy

Compare three approaches before committing:

| Option | Meaning | Use when |
|---|---|---|
| `shellcore-only` | structured shell, no arbitrary OS process spawn | MVP learning/proof path |
| Termux-compatible | interop with Termux/user-installed environment | userland value matters, policy allows |
| bundled minimal userland | app ships a small command set | controlled UX beats broad compatibility |

**DoD:** One option is selected with explicit trade-offs and a follow-up implementation plan.

---

## PM-4. iOS/iPadOS Research Spike

**Goal:** Determine the policy-safe shape of an iOS local terminal without overpromising Linux behavior.

- [ ] Build a self-contained `shellcore` REPL prototype.
- [ ] Do not download or execute code that changes app behavior.
- [ ] Keep files inside the app container or user-selected document locations.
- [ ] Define an allowed command subset: pure structured shell commands first.
- [ ] TestFlight first; App Store wording only after policy review.

**DoD:** A research note states whether iOS can ship as a constrained local structured terminal, and what it cannot honestly claim.

---

## PM-5. RA/PWA Companion Reuse

**Goal:** Reuse remote approval as a companion for desktop and mobile `ash`, not as the mobile terminal itself.

- [ ] Complete RA-1 through RA-4 on desktop daemon/listener/pairing/gate flow.
- [ ] Keep RA-5 PWA as approval/pairing/monitoring companion.
- [ ] Let Android/iOS local terminal use the same device identity model only after local terminal spike succeeds.
- [ ] Do not require a phone companion to run local Android `ash`.

**DoD:** A user can understand the distinction:

```text
Mobile ash app = local terminal on the phone/tablet.
PWA companion  = approve, pair, monitor, or demo.
```

---

## PM-6. Packaging and Public Docs

- [ ] Decide product names and binaries: `ai`, `ash`, mobile app name.
- [ ] Split README tables into current support vs target matrix.
- [ ] Add install instructions for `ash` once release artifacts exist.
- [ ] Add mobile status language: Android spike, iOS research, PWA companion.
- [ ] Add migration note from `../document/` v3.3 to terminal repo platform pivot.

---

## Final Verification Checklist

- [ ] `git diff --check`
- [ ] `cargo test shellcore`
- [ ] `cargo test --features "storage tls remote"`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`
- [ ] `ash` smoke on WSL/Linux
- [ ] Windows `ash.exe` smoke once adapter exists
- [ ] Android pure `shellcore` spike result documented
- [ ] iOS policy/research result documented

Update `docs/TASK.md` after each completed PM slice, then add `docs/HISTORY.md` entries for implementation changes.
