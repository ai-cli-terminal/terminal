# Wave 2 워크스페이스 리팩토링 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `src/main.rs`(3041줄)를 `src/cli/` 10모듈로 분할하고, `src/lib.rs`를 도메인 클러스터로 재배열하고, `docs/HANDOFF.md`를 슬리밍한다 — 전부 move-only(동작 보존).

**Architecture:** 바이너리 진입점 `main.rs`가 `mod cli;`를 선언하고 핸들러 그룹을 `src/cli/*.rs`로 verbatim 이동한다. 각 심볼은 `pub(crate)`로 가시성만 조정, 로직 불변. 각 함수의 `#[cfg(test)]` 테스트도 같은 모듈로 이동. Command enum·`main()` 디스패치까지 옮겨 main.rs를 ~50줄로 만든다.

**Tech Stack:** Rust(edition 2021), clap(derive), anyhow. 빌드·검증은 WSL(정본 `[[terminal-build-env]]`).

## Global Constraints

- **move-only**: 심볼/테스트는 **verbatim 이동**. 함수 본문·시그니처·로직·문자열 변경 금지. 가시성은 필요시 `pub(crate)`로만 확대.
- **`#[cfg(...)]` 보존**: cfg 게이팅·cfg 변형 함수 쌍(예: `run_remote_*`·`console_process_count`의 unix/windows/기타 변형)은 원본 그대로 이동.
- **`lib.rs` 모듈 경로 불변**: `pub mod X;` 이름·경로 변경 금지(`ai_terminal::X` 소비자 파손 방지).
- **테스트 수 baseline 불변**: 이동 후 `cargo test --features "storage tls remote"` 통과 테스트 수가 이동 전과 동일해야 한다. 이동 전 baseline을 먼저 기록한다.
- **검증(각 태스크 말미)**: WSL에서 무피처 `cargo build`, `cargo test --features "storage tls remote"`, `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`. 판정은 파이프 없이 `&& echo PASS || echo FAIL`. WSL 래퍼: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-release-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`(멀티라인 금지 → 스크립트 파일 경유).
- **커밋**: `git add -A` 금지(명시적 파일 add). 커밋 메시지 말미 `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.
- **base=develop**, 브랜치 `refactor/wave2-*`·`docs/wave2-*`. 2단계 PR 규칙.

---

## PR 1 — `refactor/wave2-main-split` (Task 0~10)

브랜치 `refactor/wave2-main-split`를 `origin/develop`에서 분기. `src/cli/mod.rs`를 만들고 `main.rs`에 `mod cli;` 선언 후, 모듈을 하나씩 추출한다. 각 태스크는 main.rs가 계속 컴파일되도록 이동 심볼을 `cli::<mod>::`로 참조하게 바꾼다.

### Task 0: baseline 기록 + cli 스캐폴딩

**Files:**
- Create: `src/cli/mod.rs`
- Modify: `src/main.rs` (상단에 `mod cli;` 추가)

- [ ] **Step 1: baseline 테스트 수 기록**

Run (WSL): `cargo test --features "storage tls remote" 2>&1 | grep -E 'test result: ok' | awk '{s+=$4} END{print s}'`
결과 숫자를 기록(예: `N`). 이후 모든 태스크에서 이 수가 유지돼야 한다.

- [ ] **Step 2: `src/cli/mod.rs` 생성 (빈 모듈 선언 골격)**

```rust
//! `ai` 바이너리의 CLI 표면 — 서브커맨드 정의·디스패치·핸들러.
//! main.rs에서 분리(move-only). 각 모듈은 자신의 테스트를 포함한다.
```

- [ ] **Step 3: `main.rs`에 `mod cli;` 선언**

`src/main.rs` 크레이트 문서(`//!`) 블록 바로 아래(첫 `use` 위)에 추가:
```rust
mod cli;
```

- [ ] **Step 4: 빌드 확인**

Run (WSL): `cargo build && echo PASS || echo FAIL`
Expected: PASS (빈 cli 모듈, 동작 불변)

- [ ] **Step 5: 커밋**

```bash
git add src/cli/mod.rs src/main.rs
git commit -m "refactor(cli): scaffold src/cli module for main.rs split"
```

### Task 1: `cli/command.rs` — clap 정의

**Files:**
- Create: `src/cli/command.rs`
- Modify: `src/main.rs` (심볼 제거·참조 갱신), `src/cli/mod.rs` (`pub mod command;`)

**Interfaces:**
- Produces: `pub(crate) struct Cli`, `pub(crate) enum Command`, `pub(crate) enum PolicyAction`, `pub(crate) enum RemoteAction`, `pub(crate) enum InitTarget`, `pub(crate) enum InitMode` — 이후 모든 태스크가 `cli::command::{Cli, Command, ...}`로 참조.

- [ ] **Step 1: `Cli`·`Command`·`PolicyAction`·`RemoteAction`·`InitTarget`·`InitMode`를 `cli/command.rs`로 verbatim 이동**

main.rs에서 이 6개 clap 타입(및 그 `#[derive(...)]`·doc)을 잘라 `src/cli/command.rs`로 옮긴다. `struct Cli`/`enum ...`의 pub 수준을 `pub(crate)`로. 필요한 `use`(clap `Parser`/`Subcommand`/`Args`, `std::path::PathBuf` 등)를 command.rs 상단에 추가. 파싱에 필요한 `ai_terminal::` 참조는 그대로.

- [ ] **Step 2: `cli/mod.rs`에 선언 추가**

```rust
pub mod command;
```

- [ ] **Step 3: main.rs 참조 갱신**

main.rs에서 `Cli::parse()`·`Command::*`·액션 enum 사용처를 `cli::command::{Cli, Command, PolicyAction, RemoteAction, InitTarget, InitMode}` import로 해소(`use cli::command::*;` 또는 개별 use).

- [ ] **Step 4: 관련 clap 파싱 테스트 이동**

main.rs `#[cfg(test)] mod tests`에서 **CLI 파싱 테스트**(`cli_parses_*`: doctor_with_guardrails, bare_invocation, risk_command_without_profile, explain_last_error_without_command, policy_show_and_set, ask, remote_daemon(+relay_transport), remote_devices/transport/relay_setup/pair/approval_url/approval_verify(+device_id), remote_arm/disarm_and_status, gate 등)를 `cli/command.rs`의 `#[cfg(test)] mod tests`로 이동. 테스트 내 참조를 모듈 경로에 맞게 조정(`super::*` + 필요한 `crate::cli::...`).

- [ ] **Step 5: 빌드·테스트·clippy 검증**

Run (WSL): `cargo build && cargo test --features "storage tls remote" && cargo clippy --all-targets --features "storage tls remote" -- -D warnings && echo PASS || echo FAIL`
Expected: PASS. 통과 테스트 수 = Task0 baseline `N`.

- [ ] **Step 6: 커밋**

```bash
git add src/cli/command.rs src/cli/mod.rs src/main.rs
git commit -m "refactor(cli): move clap Cli/Command definitions to cli/command.rs"
```

### Task 2: `cli/io.rs` — 출력 sink / confirmer

**Files:** Create `src/cli/io.rs`; Modify `src/main.rs`, `src/cli/mod.rs`.

**Interfaces:** Produces `pub(crate) struct StdoutSink`, `pub(crate) struct AutoYes`, `pub(crate) struct StdinConfirmer` (각 `impl ai_terminal::pipeline::{OutputSink, Confirmer}`).

- [ ] **Step 1:** `StdoutSink`·`AutoYes`·`StdinConfirmer`와 그 `impl` 블록을 `cli/io.rs`로 verbatim 이동, `pub(crate)`로. `use ai_terminal::pipeline::{OutputSink, Confirmer};` 등 필요한 use 추가.
- [ ] **Step 2:** `cli/mod.rs`에 `pub mod io;`.
- [ ] **Step 3:** main.rs 사용처를 `cli::io::{StdoutSink, AutoYes, StdinConfirmer}`로.
- [ ] **Step 4:** 검증 — `cargo build && cargo test --features "storage tls remote" && echo PASS || echo FAIL` (테스트 수 `N`).
- [ ] **Step 5:** 커밋 `git add src/cli/io.rs src/cli/mod.rs src/main.rs && git commit -m "refactor(cli): move output sink/confirmer to cli/io.rs"`.

### Task 3: `cli/hooks.rs` — init / 셸 훅

**Files:** Create `src/cli/hooks.rs`; Modify `src/main.rs`, `src/cli/mod.rs`.

**Interfaces:** Produces `pub(crate) struct InitPlan`, `pub(crate) fn plan_init_shell`, `pub(crate) fn resolve_shell`, `pub(crate) fn resolve_rc`, `pub(crate) fn record_hook_preexec`, `pub(crate) fn record_hook_precmd`, `pub(crate) fn record_hook_chpwd`.

- [ ] **Step 1:** 위 심볼(`InitPlan` struct + 6 fn)을 `cli/hooks.rs`로 verbatim 이동, `pub(crate)`로. `InitTarget`/`InitMode`는 `cli::command`에서 `use`. `Shell` 등 `ai_terminal::` 참조 유지.
- [ ] **Step 2:** `cli/mod.rs`에 `pub mod hooks;`.
- [ ] **Step 3:** main.rs 디스패치의 호출처를 `cli::hooks::*`로.
- [ ] **Step 4:** 관련 테스트(`plan_init_shell`·hook 관련)가 있으면 `cli/hooks.rs` 테스트로 이동.
- [ ] **Step 5:** 검증 (테스트 수 `N`).
- [ ] **Step 6:** 커밋 `... -m "refactor(cli): move init/shell-hook handlers to cli/hooks.rs"`.

### Task 4: `cli/inspect.rs` — risk/preview/mask/explain formatter

**Files:** Create `src/cli/inspect.rs`; Modify `src/main.rs`, `src/cli/mod.rs`.

**Interfaces:** Produces `pub(crate) fn format_explain`, `run_explain`, `format_preview`, `format_mask`, `format_risk`, `describe_profile`, `resolve_profile`.

- [ ] **Step 1:** 위 7개 fn을 `cli/inspect.rs`로 verbatim 이동, `pub(crate)`. `PolicyProfile` 등 `ai_terminal::` 참조 유지.
- [ ] **Step 2:** `cli/mod.rs`에 `pub mod inspect;`.
- [ ] **Step 3:** main.rs 호출처를 `cli::inspect::*`로. `resolve_profile`이 다른 이동 모듈에서 쓰이면 그 모듈도 `cli::inspect::resolve_profile` 참조.
- [ ] **Step 4:** formatter 테스트(`format_risk_*`, `describe_profile_*`, `format_explain_*`, `format_preview_*`, `format_mask_*`, `resolve_profile_rejects_unknown`)를 `cli/inspect.rs` 테스트로 이동.
- [ ] **Step 5:** 검증 (테스트 수 `N`).
- [ ] **Step 6:** 커밋 `... -m "refactor(cli): move inspect/format handlers to cli/inspect.rs"`.

### Task 5: `cli/run.rs` — exec / dispatch 러너

**Files:** Create `src/cli/run.rs`; Modify `src/main.rs`, `src/cli/mod.rs`.

**Interfaces:** Produces `pub(crate) fn run_exec`, `run_dispatch`, `flush_stdout`, `finish_shell_outcome`, `cache_badge`.

- [ ] **Step 1:** 위 5개 fn을 `cli/run.rs`로 verbatim 이동, `pub(crate)`. `StdoutSink`/`StdinConfirmer`/`AutoYes`는 `cli::io`에서 use, `resolve_profile`은 `cli::inspect`에서 use. `ai_terminal::{pipeline, cache}` 참조 유지.
- [ ] **Step 2:** `cli/mod.rs`에 `pub mod run;`.
- [ ] **Step 3:** main.rs 디스패치 호출처를 `cli::run::*`로.
- [ ] **Step 4:** 관련 테스트 이동(있으면).
- [ ] **Step 5:** 검증 (테스트 수 `N`).
- [ ] **Step 6:** 커밋 `... -m "refactor(cli): move exec/dispatch runners to cli/run.rs"`.

### Task 6: `cli/gate.rs` — 게이트 / 데몬 전송

**Files:** Create `src/cli/gate.rs`; Modify `src/main.rs`, `src/cli/mod.rs`.

**Interfaces:** Produces `pub(crate) fn run_gate`, `run_gate_daemon`, `pub(crate) struct DaemonTransportSelection`, `pub(crate) fn resolve_daemon_transport_selection`, `resolve_relay_deployment_selection`.

- [ ] **Step 1:** 위 심볼을 `cli/gate.rs`로 verbatim 이동, `pub(crate)`. `#[cfg]` 변형 보존. `ai_terminal::{gate, daemon}` 참조 유지.
- [ ] **Step 2:** `cli/mod.rs`에 `pub mod gate;`.
- [ ] **Step 3:** main.rs 호출처를 `cli::gate::*`로.
- [ ] **Step 4:** 게이트/전송 테스트(`daemon_transport_selection_validates_relay_inputs`, `cli_parses_gate`가 command로 안 갔다면 여기로) 이동.
- [ ] **Step 5:** 검증 (테스트 수 `N`).
- [ ] **Step 6:** 커밋 `... -m "refactor(cli): move gate/daemon-transport handlers to cli/gate.rs"`.

### Task 7: `cli/remote.rs` — 원격 승인 명령

**Files:** Create `src/cli/remote.rs`; Modify `src/main.rs`, `src/cli/mod.rs`.

**Interfaces:** Produces `pub(crate) fn remote_pair_transport_addr`, `run_remote_devices`, `run_remote_transport`, `run_remote_relay_setup`, `run_remote_pair`, `run_remote_approval_url`, `run_remote_approval_verify`, `remote_approval_verify_device` — **각 함수의 cfg 변형 쌍(remote feature 유/무, unix/기타) 전부**.

- [ ] **Step 1:** 위 심볼과 **모든 `#[cfg(...)]` 변형 정의**를 `cli/remote.rs`로 verbatim 이동, `pub(crate)`. `ai_terminal::{remote, remote_transport, session, pairing, approval, device_registry, qr}` 참조 유지. cfg 대칭 보존(무피처 빌드에서 stub 변형이 남아야 함).
- [ ] **Step 2:** `cli/mod.rs`에 `pub mod remote;`.
- [ ] **Step 3:** main.rs 호출처를 `cli::remote::*`로.
- [ ] **Step 4:** 원격 파싱/동작 테스트 중 command로 안 간 것 이동.
- [ ] **Step 5:** 검증 — **무피처 `cargo build`(cfg 대칭 확인) + `--features remote` + `test --features "storage tls remote"`** (테스트 수 `N`).
- [ ] **Step 6:** 커밋 `... -m "refactor(cli): move remote approval command handlers to cli/remote.rs"`.

### Task 8: `cli/doctor.rs` — doctor / 설정 진단

**Files:** Create `src/cli/doctor.rs`; Modify `src/main.rs`, `src/cli/mod.rs`.

**Interfaces:** Produces `pub(crate) fn run_doctor`, `pub(crate) fn format_config_diagnostics`.

- [ ] **Step 1:** 두 fn을 `cli/doctor.rs`로 verbatim 이동, `pub(crate)`. `ai_terminal::config::LoadedConfig` 등 참조 유지.
- [ ] **Step 2:** `cli/mod.rs`에 `pub mod doctor;`.
- [ ] **Step 3:** main.rs 호출처를 `cli::doctor::*`로.
- [ ] **Step 4:** 검증 (테스트 수 `N`).
- [ ] **Step 5:** 커밋 `... -m "refactor(cli): move doctor/config-diagnostics to cli/doctor.rs"`.

### Task 9: `cli/shell_run.rs` — 영속 셸 / wrapper cwd

**Files:** Create `src/cli/shell_run.rs`; Modify `src/main.rs`, `src/cli/mod.rs`.

**Interfaces:** Produces `pub(crate) fn run_persistent_shell`, `pub(crate) fn sync_wrapper_cwd`.

- [ ] **Step 1:** 두 fn을 `cli/shell_run.rs`로 verbatim 이동, `pub(crate)`. `ai_terminal::{wrapper, shell}` 참조 유지.
- [ ] **Step 2:** `cli/mod.rs`에 `pub mod shell_run;`.
- [ ] **Step 3:** main.rs 호출처를 `cli::shell_run::*`로.
- [ ] **Step 4:** 검증 (테스트 수 `N`).
- [ ] **Step 5:** 커밋 `... -m "refactor(cli): move persistent-shell/wrapper-cwd to cli/shell_run.rs"`.

### Task 10: `cli/dispatch.rs` — `main()` → `run()`, main.rs 얇게

**Files:** Create `src/cli/dispatch.rs`; Modify `src/main.rs`, `src/cli/mod.rs`.

**Interfaces:** Produces `pub(crate) fn run() -> anyhow::Result<()>` (구 `main()` 본문). Consumes 모든 `cli::*` 핸들러.

- [ ] **Step 1:** 구 `main()` 본문 전체 + 유틸 `now_secs`·`console_process_count`(cfg 변형)·`is_double_click_launch`를 `cli/dispatch.rs`로 이동. `main()`을 `pub(crate) fn run() -> anyhow::Result<()>`로 이름만 변경(본문 verbatim). 내부 핸들러 호출은 이미 `cli::*` 경로.
- [ ] **Step 2:** `cli/mod.rs`에 `pub mod dispatch;`.
- [ ] **Step 3:** `main.rs`를 얇게 — 크레이트 문서 + `mod cli;` + 다음만 남긴다:

```rust
fn main() -> anyhow::Result<()> {
    cli::dispatch::run()
}
```

main.rs의 잔여 `use`·헬퍼가 모두 옮겨졌는지 확인(남은 참조 0).

- [ ] **Step 4:** `double_click_launch_only_when_sole_console_process` 등 dispatch 유틸 테스트를 `cli/dispatch.rs` 테스트로 이동.
- [ ] **Step 5:** 전체 검증 — 무피처 build + `test --features "storage tls remote"`(수 `N`) + clippy + `check --lib --target aarch64-linux-android`. `main.rs` 줄수 ~50 확인.
- [ ] **Step 6:** 커밋 `... -m "refactor(cli): move dispatch to cli/dispatch.rs, slim main.rs to entry point"`.

**→ PR 1 완료: push `refactor/wave2-main-split`, base=develop PR. whole-branch diff가 순수 이동인지 리뷰.**

---

## PR 2 — `refactor/wave2-lib-domains` (Task 11)

브랜치 `refactor/wave2-lib-domains`를 (PR1 머지 후) `origin/develop`에서 분기.

### Task 11: `lib.rs` 도메인 클러스터 재배열

**Files:** Modify `src/lib.rs`.

- [ ] **Step 1:** `pub mod` 선언들을 스펙 §5 클러스터 순서로 재배열하고 각 그룹 위에 `// === <도메인> ===` 주석 추가. **`pub mod X;` 이름·경로·`#[cfg(...)]`는 각 줄 그대로**(이동만, 텍스트 불변). 크레이트 `//!` doc의 "현재 구현된 모듈" 목록도 최신 클러스터로 갱신.
- [ ] **Step 2:** 경로 불변 확인 — `git diff`가 **줄 재배열·주석 추가만**인지(모듈명/cfg 변경 0) 검토.
- [ ] **Step 3:** 검증 — 무피처 build + `test --features "storage tls remote"`(수 `N`) + clippy + android check. 경로 불변이라 소비자 무영향.
- [ ] **Step 4:** 커밋 `git add src/lib.rs && git commit -m "refactor(lib): group module declarations by domain (path-preserving)"`.

**→ PR 2 완료: push, base=develop PR.**

---

## PR 3 — `docs/wave2-handoff-slim` (Task 12)

브랜치 `docs/wave2-handoff-slim`를 `origin/develop`에서 분기. 이 설계/계획 문서도 이 PR에 포함(이미 커밋됨이면 cherry-pick/포함).

### Task 12: `HANDOFF.md` 슬리밍 + 설계·계획 문서

**Files:** Modify `docs/HANDOFF.md`; (포함) `docs/superpowers/specs/2026-07-09-wave2-workspace-refactoring-design.md`, `docs/superpowers/plans/2026-07-09-wave2-workspace-refactoring.md`.

- [ ] **Step 1:** `docs/HANDOFF.md`에서 stale·중복 제거 — 중복 `## 1.` 중 "현재 상태 — v0.3.3 릴리스 완료"·`## 2. v0.3.3 릴리스 산출물 검증`·오래된 세션 closeout 섹션 삭제. 섹션 번호 단일 증가로 재정리.
- [ ] **Step 2:** **유지·압축** — 최신 재개점(§0), 빌드·검증 환경 메모(요약), 다음 작업 우선순위, 비목표, 정본 포인터(TASK/WORKFLOW/HISTORY/CHANGELOG). 목표 ~180줄.
- [ ] **Step 3:** 링크·경로 유효성 확인(삭제 섹션을 참조하는 앵커 없음).
- [ ] **Step 4:** (문서 전용이라 빌드 무관) `git add docs/HANDOFF.md docs/superpowers/specs/2026-07-09-wave2-workspace-refactoring-design.md docs/superpowers/plans/2026-07-09-wave2-workspace-refactoring.md && git commit -m "docs: slim HANDOFF.md and add Wave2 design/plan"`.

**→ PR 3 완료: push, base=develop PR.**

---

## 최종 (전 PR 머지 후)

- develop 통합 검증(WSL 매트릭스 green, 테스트 수 `N` 유지) → 트러블슈팅 기록(`docs/superpowers/2026-07-09-wave2-execution-troubleshooting.md`, Wave1 패턴).
- **후속(별도)**: Wave2 develop→main 반영 후 trust 스택(#69~80)을 새 main 위로 재rebase(설계 §3·§8).
