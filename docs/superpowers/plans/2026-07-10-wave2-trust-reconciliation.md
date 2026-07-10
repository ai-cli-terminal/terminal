# Wave2↔trust 재조정 구현 Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `origin/develop`(Wave2 split)에 `origin/main`의 trust 기능을 흡수해, develop이 `cli/` 분할 구조 + trust를 모두 갖게 한다.

**Architecture:** `origin/develop`에서 통합 브랜치를 만들고 `origin/main`을 머지한다. 충돌 2파일(`lib.rs`·`main.rs`)만 수동 해결 — `main.rs`는 develop의 15줄을 유지하고, trust가 monolithic `main.rs`에 넣었던 CLI 표면(~1224줄)은 develop의 `cli/` 모듈로 이식한다. trust 코드는 origin/main에서 이미 `#[cfg(feature="trust")]`로 게이트되어 있으므로 게이트를 그대로 옮긴다.

**Tech Stack:** Rust, clap(Subcommand derive), anyhow. 빌드/검증은 WSL([[terminal-build-env]]).

**정본 spec:** `docs/superpowers/specs/2026-07-10-wave2-trust-reconciliation-design.md` (PR #96)

## Global Constraints

- **main.rs는 develop의 15줄 유지**(`fn main() -> anyhow::Result<()> { cli::dispatch::run() }`). trust의 main.rs 증분은 전부 `cli/`로 이식하고 main.rs엔 남기지 않는다.
- **모든 trust 코드는 origin/main의 `#[cfg(feature="trust")]`/`#[cfg(not(feature="trust"))]` 게이트를 그대로 유지**한다. 무피처 빌드에서 orphan/unresolved 방지(교훈: Wave2 gate.rs). trust feature는 이미 `Cargo.toml`에 존재(머지로 흡수).
- **이식 소스**: `git show origin/main:src/main.rs` (전체) + `scratchpad/reconcile-analysis/trust-mainrs.diff` (v0.4.0 대비 trust 증분, 라인 참조). 코드를 재작성하지 말고 소스에서 **그대로 추출**한다.
- **참조 경로 규칙**: 완전경로 `ai_terminal::{policy_d,skill_registry,binary_manifest,trust}::*`는 그대로. `config::`/`skill::`/`PathBuf`는 각 cli 모듈 상단 `use ai_terminal::{config, skill}; use std::path::PathBuf;`로 해결. cli 헬퍼는 `use crate::cli::inspect::{...}`.
- **git**: 통합 브랜치 → **develop PR**(main 직접 금지). `git add`는 명시적 파일만(`git add -A` 금지). 절대경로/`-C` 사용(cwd 리셋 방지).
- **검증 판정**: 파이프 금지, `if cmd >log 2>&1; then PASS; else FAIL; fi` 또는 exit code 직접. WSL 래퍼는 [[terminal-build-env]].

---

## File Structure

| 파일 | 작업 | 책임 |
|---|---|---|
| `src/main.rs` | Modify(충돌해결) | develop 15줄 유지 |
| `src/lib.rs` | Modify(충돌해결) | trust `pub mod` 4개를 Wave2 클러스터에 통합 |
| `src/cli/mod.rs` | Modify | `#[cfg(feature="trust")] pub mod trust;`(+무피처용 stub 경로) 추가 |
| `src/cli/command.rs` | Modify | Command enum 확장(Skill에 `action`·`Release` 신규·`PolicyAction::Org`) + action enum 5개 |
| `src/cli/inspect.rs` | Modify | profile 해석 헬퍼 7개(effective/requested/describe/path_state) |
| `src/cli/trust.rs` | **Create** | trust 핸들러 11개 + 헬퍼(default_skill_discovery_paths 등) + 테스트 |
| `src/cli/dispatch.rs` | Modify | Risk/Tui/Verify/Route arm의 profile 해석 교체 + Policy(Org)·Skill·Release arm |
| `src/cli/run.rs` | Modify | run_exec/run_dispatch의 `resolve_profile`→`resolve_requested_profile` |
| (자동 병합) | — | `src/{trust,policy_d,skill_registry,binary_manifest}.rs`·Cargo.toml·CI·docs |

---

## Task 1: 격리 워크트리 + 머지 + 2충돌 해결 (무피처 그린 베이스)

**Files:**
- Create(worktree): `D:/workspace/terminal-project/terminal-reconcile-wt`
- Modify(충돌): `src/main.rs`, `src/lib.rs`

**Interfaces:**
- Produces: 통합 브랜치 `refactor/wave2-trust-reconcile`(develop 베이스, origin/main 머지됨). trust 신규 모듈 4개·lib 수정분·Cargo.toml trust feature가 워킹트리에 존재. main.rs=15줄, lib.rs=trust mod 통합.

- [ ] **Step 1: 격리 워크트리 생성 + 머지(no-commit)**

```bash
REPO=/d/workspace/terminal-project/terminal
WT=/d/workspace/terminal-project/terminal-reconcile-wt
git -C "$REPO" worktree add -b refactor/wave2-trust-reconcile "$WT" origin/develop
git -C "$WT" merge --no-commit --no-ff origin/main
```
Expected: `CONFLICT (content): Merge conflict in src/lib.rs` + `src/main.rs`. 나머지 자동 병합.

- [ ] **Step 2: 자동 병합 36파일 sanity (trust 신규 모듈·Cargo feature 존재 확인)**

```bash
WT=/d/workspace/terminal-project/terminal-reconcile-wt
ls "$WT/src/trust.rs" "$WT/src/policy_d.rs" "$WT/src/skill_registry.rs" "$WT/src/binary_manifest.rs"
grep -n 'trust' "$WT/Cargo.toml"
git -C "$WT" diff --name-only --diff-filter=U   # 충돌 파일이 정확히 lib.rs·main.rs 2개인지
```
Expected: 4개 파일 존재, Cargo.toml에 `trust = [...]` feature, 충돌 파일 2개.

- [ ] **Step 3: main.rs 충돌 해결 — develop 15줄 채택**

```bash
WT=/d/workspace/terminal-project/terminal-reconcile-wt
git -C "$WT" checkout --ours -- src/main.rs
git -C "$WT" add src/main.rs
```
확인: `src/main.rs`가 15줄(`mod cli; fn main() -> anyhow::Result<()> { cli::dispatch::run() }`)인지 `cat`으로 검증. (trust의 main.rs 증분은 Task 2에서 cli/로 이식.)

- [ ] **Step 4: lib.rs 충돌 해결 — trust `pub mod` 4개를 Wave2 클러스터에 통합**

`git -C "$WT" diff src/lib.rs`로 충돌 헝크 확인. Wave2가 도메인 클러스터로 재배열한 구조를 base로 하고, trust가 추가한 선언만 삽입:
```rust
#[cfg(feature = "trust")]
pub mod binary_manifest;
#[cfg(feature = "trust")]
pub mod policy_d;
#[cfg(feature = "trust")]
pub mod skill_registry;
#[cfg(feature = "trust")]
pub mod trust;
```
- **클러스터 내부 알파벳 순서 유지**(rustfmt `reorder_modules`가 주석 경계 내 알파벳 정렬 — 교훈 ②). 적절한 클러스터(예: 보안/정책 계열) 주석 아래 알파벳 위치에 삽입.
- 게이트는 origin/main lib.rs와 동일하게. `git show origin/main:src/lib.rs | grep -n 'trust\|policy_d\|skill_registry\|binary_manifest'`로 원본 게이트 확인.
```bash
git -C "$WT" add src/lib.rs
```

- [ ] **Step 5: 무피처 빌드/clippy/test 그린 (trust 코드 이식 전 베이스 확인)**

WSL에서(무피처 = trust 모듈 미컴파일):
```bash
# terminal-build-env WSL 래퍼로, 대상 워크트리 경로 주의: /mnt/d/workspace/terminal-project/terminal-reconcile-wt
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-reconcile-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal-reconcile; if cargo build >/tmp/b.log 2>&1 && cargo clippy --all-targets -- -D warnings >/tmp/c.log 2>&1 && cargo test --all-targets >/tmp/t.log 2>&1; then echo PASS; else echo FAIL; tail -30 /tmp/b.log /tmp/c.log /tmp/t.log; fi'
```
Expected: `PASS`. (develop 기능 회귀 0 — 이 시점 코드는 develop + 자동병합 lib 모듈, CLI 표면은 아직 develop 그대로.)

- [ ] **Step 6: 머지 상태 유지(커밋은 Task 4에서). 진행 체크포인트만.**

`git -C "$WT" status`로 충돌 해소(2파일 staged) 확인. **아직 커밋하지 않는다**(이식 후 한 번에).

---

## Task 2: trust CLI 표면을 cli/로 이식 (무피처+trust 그린)

> 원자적 컴파일 단위 — command enum 확장은 dispatch match를 non-exhaustive로 만들므로 command·dispatch·trust·inspect·run·mod를 함께 완성해야 빌드가 통과한다. 소스는 `origin/main:src/main.rs` + `trust-mainrs.diff`.

**Files:**
- Create: `src/cli/trust.rs`
- Modify: `src/cli/mod.rs`, `src/cli/command.rs`, `src/cli/inspect.rs`, `src/cli/dispatch.rs`, `src/cli/run.rs`

**Interfaces:**
- Consumes: Task 1의 병합 트리(trust lib 모듈 존재).
- Produces:
  - `cli/command.rs`: `Command::Skill{ query, action: Option<SkillAction> }`, `Command::Release{ action: ReleaseAction }`, `PolicyAction::Org{ action: PolicyOrgAction }`, enum `SkillAction`/`SkillRegistryAction`/`PolicyOrgAction`/`ReleaseAction`/`ReleaseManifestAction`(전부 `pub(crate)`).
  - `cli/inspect.rs`: `pub(crate) fn resolve_requested_profile(Option<String>) -> Result<PolicyProfile>`, `resolve_effective_profile(&str) -> Result<PolicyProfile>`, `describe_effective_policy(&str) -> Result<String>` + (trust) `resolve_effective_policy`/`describe_policy_source`/`path_state`.
  - `cli/trust.rs`: `pub(crate) fn run_policy_org_status`·`run_skill_list(Option<String>)`·`run_skill_enabled()`·`run_skill_enable(String,bool)`·`run_skill_disable(String)`·`run_skill_registry_status`·`run_skill_registry_update(PathBuf,PathBuf,bool)`·`run_release_manifest_create(PathBuf,Vec<PathBuf>,Option<String>)`·`run_release_manifest_status`·`run_release_manifest_verify(PathBuf,PathBuf,Option<String>,Option<PathBuf>)` + (trust) `run_release_manifest_sign(ReleaseManifestSignInput)`/`ReleaseManifestSignInput`.

- [ ] **Step 1: `cli/command.rs` — Command enum 확장 + action enum 5개**

소스 `trust-mainrs.diff` 라인 5-154. 대상 `cli/command.rs`:
- `Skill` variant(현 146행 `Skill { query }`)에 필드 추가:
```rust
    Skill {
        #[arg(long)]
        query: Option<String>,
        #[command(subcommand)]
        action: Option<SkillAction>,
    },
    /// 릴리스 바이너리 manifest 서명 상태를 진단한다 (P3 trust channel).
    Release {
        #[command(subcommand)]
        action: ReleaseAction,
    },
```
- `PolicyAction`(현 203행)에 `Org` variant 추가(diff 23-27).
- 파일 끝(다른 `#[derive(Subcommand)]` 블록 곁)에 `PolicyOrgAction`·`SkillAction`·`SkillRegistryAction`·`ReleaseAction`·`ReleaseManifestAction` 추가(diff 30-154). **enum 앞 가시성 `pub(crate)`** 부여(develop command.rs 관례: `pub(crate) enum`). `PathBuf`는 이미 `use std::path::PathBuf;` 존재.
- **게이트 없음**: 이 enum들은 무피처에서도 clap 표면으로 존재해야 함(핸들러만 게이트). origin/main도 enum엔 게이트 없음.

- [ ] **Step 2: `cli/inspect.rs` — profile 해석 헬퍼 7개 이식**

소스 `trust-mainrs.diff` 라인 161-228. 대상 `cli/inspect.rs`(기존 `resolve_profile`/`describe_profile` 곁). 그대로 이식하되 가시성 `pub(crate)`:
- `resolve_effective_policy`(trust), `resolve_effective_profile`(trust/non-trust 2버전), `resolve_requested_profile`(양쪽), `describe_policy_source`(trust), `describe_effective_policy`(trust/non-trust 2버전), `path_state`(trust).
- `resolve_profile`·`describe_profile`은 이미 이 모듈에 있으므로 내부 호출 그대로. `config::get_active_profile`는 `use ai_terminal::config;` 확인(inspect.rs 상단에 없으면 추가).
- `path_state`는 trust.rs만 쓰면 trust.rs로 옮겨도 됨 — **단순화를 위해 `path_state`는 `cli/trust.rs`에 두고**, inspect.rs엔 profile 해석만. (아래 Step 4에서 trust.rs가 자체 `path_state` 보유.)
- **최종 inspect.rs 추가분**: `resolve_effective_policy`·`resolve_effective_profile`(2)·`resolve_requested_profile`·`describe_policy_source`·`describe_effective_policy`(2). 전부 `pub(crate)`.

- [ ] **Step 3: `cli/mod.rs` — trust 모듈 선언**

```rust
pub mod trust;
```
`cli/mod.rs` 목록에 추가(알파벳/기존 순서 맞춰 `shell_run` 곁). trust.rs 내부가 feature 게이트를 담으므로 모듈 선언 자체는 무게이트로 두되, 내부 심볼이 전부 게이트면 무피처에서 빈 모듈이 됨(경고 없음 확인). 만약 무피처 dead_code 경고 시 `#[cfg(feature = "trust")]`를 일부 심볼에만 정밀 적용(origin/main 게이트 그대로면 안전).

- [ ] **Step 4: `cli/trust.rs` 신설 — 핸들러 11개 + 헬퍼**

소스 `trust-mainrs.diff` 라인 230-856(핸들러/헬퍼 정의부). 대상 새 파일 `cli/trust.rs`. 상단 import(핸들러는 enum이 아니라 **개별 필드**를 받으므로 command enum import 불필요 — enum 분해는 dispatch.rs 몫):
```rust
use std::path::PathBuf;

use ai_terminal::{config, skill};
```
이식 대상(게이트 포함 그대로):
- `path_state`(trust), `run_policy_org_status`(trust/non-trust 2버전)
- `run_skill_list`·`default_skill_discovery_paths`·`run_skill_enabled`·`validate_external_skill_enable_policy`(2)·`skill_enable_confirmation_matches`·`confirm_external_skill_enable`·`run_skill_enable`·`run_skill_disable`
- `run_skill_registry_status`(2)·`run_skill_registry_update`(2)
- `run_release_manifest_create`(2)·`ReleaseManifestSignInput`(trust)·`run_release_manifest_sign`(2)·`run_release_manifest_status`(2)·`run_release_manifest_verify`(2)
- 핸들러 가시성 `pub(crate)`. `skill::` 참조는 `use ai_terminal::skill` 로 해결(discover_with_source·match_skills·get_enabled_skills·enable_skill_name·disable_skill_name·record_skill_enable_audit·DiscoveredSkill·SkillSource·Skill 등 — 전부 skill 모듈 공개 심볼, 자동병합된 origin/main skill.rs에 존재).

- [ ] **Step 5: `cli/dispatch.rs` — arm 이식**

소스 `trust-mainrs.diff` 라인 861-1027(main() match 변경). 대상 `cli/dispatch.rs`:
- Risk arm(78행): `resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?` → `resolve_requested_profile(profile)?`
- Tui arm(101행): 동일 교체 → `resolve_requested_profile(profile)?`
- Verify arm(209행)·Route arm(245행): `resolve_profile(&config::get_active_profile())?` → `resolve_effective_profile(&config::get_active_profile())?`
- Policy Show(84행): `describe_profile(&p)` 경로 → `describe_effective_policy(&profile.unwrap_or_else(config::get_active_profile))?` 사용(diff 870-878).
- Policy Set(88행): trust-gated effective 재확인 블록 추가(diff 880-897).
- Policy match에 `PolicyAction::Org { action } => match action { PolicyOrgAction::Status => trust::run_policy_org_status() }`(diff 899-901) 추가.
- Skill arm(현 develop): `Some(Command::Skill { query, action }) => match action { ... }`로 재작성(diff 951-970), 핸들러는 `trust::run_skill_*`.
- Release arm 신규(diff 971-1027): `trust::run_release_manifest_*`. Sign은 `#[cfg(feature="trust")]`/`not` 분기 그대로(diff 990-1019).
- import 추가: `use crate::cli::inspect::{describe_effective_policy, resolve_effective_profile, resolve_requested_profile};` (기존 inspect import 확장), `use crate::cli::trust;`, `use crate::cli::command::{PolicyOrgAction, SkillAction, SkillRegistryAction, ReleaseAction, ReleaseManifestAction};` (command import 확장).

- [ ] **Step 6: `cli/run.rs` — run_exec/run_dispatch profile 교체**

소스 diff 1035, 1044. 대상 `cli/run.rs` 10·35행: `resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?` → `resolve_requested_profile(profile)?`. import에 `resolve_requested_profile` 추가(`use crate::cli::inspect::resolve_requested_profile;` — run.rs가 inspect를 이미 import하면 확장).

- [ ] **Step 7: 무피처 + trust 빌드/clippy 그린**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-reconcile-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal-reconcile; ok=1; for f in "" "trust"; do FEAT=""; [ -n "$f" ] && FEAT="--features $f"; if cargo build $FEAT >/tmp/b.log 2>&1 && cargo clippy --all-targets $FEAT -- -D warnings >/tmp/c.log 2>&1; then echo "PASS $f"; else echo "FAIL $f"; tail -40 /tmp/b.log /tmp/c.log; ok=0; fi; done; [ $ok -eq 1 ] && echo ALL_PASS || echo SOME_FAIL'
```
Expected: `PASS ` / `PASS trust` / `ALL_PASS`. 실패 시 참조 경로(`crate::cli::`)·게이트·enum 가시성 우선 점검.

---

## Task 3: trust CLI 파싱 테스트 이식 (trust test 그린)

**Files:** Modify `src/cli/trust.rs`(또는 `command.rs`) — `#[cfg(test)] mod tests`

**Interfaces:** Consumes Task 2의 enum/핸들러.

- [ ] **Step 1: 테스트 7개 이식**

소스 `trust-mainrs.diff` 라인 1053-1340의 `#[test]` 7개(`cli_parses_policy_org_status`·`cli_parses_skill_command`·`cli_parses_skill_registry_status`·`cli_parses_skill_enable_disable_and_enabled`·`skill_enable_confirmation_requires_exact_skill_name`·`cli_parses_skill_registry_update_and_revoke`·`cli_parses_release_manifest_status_and_verify`). 대상 `cli/trust.rs` 하단:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::command::Cli;
    use clap::Parser;
    // ... (diff에서 7개 테스트 그대로; Cli::try_parse_from 경로는 command::Cli)
}
```
- **게이트 주의**(교훈 ①/Wave2 gate.rs): 테스트가 trust 심볼(`skill_enable_confirmation_matches`)만 쓰면 무피처에서도 컴파일 가능. 그러나 `use super::*`가 무피처에서 orphan이면 `#[cfg(all(test, feature = "trust"))]`로 좁힌다. **원칙**: origin/main에서 각 테스트가 참조하는 심볼의 게이트를 따른다 — CLI 파싱 테스트(enum만)는 무피처 가능, 핸들러 참조 테스트는 trust 게이트.

- [ ] **Step 2: test 그린 (무피처 + trust)**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-reconcile-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal-reconcile; if cargo test --all-targets >/tmp/t0.log 2>&1 && cargo test --all-targets --features trust >/tmp/t1.log 2>&1; then echo PASS; else echo FAIL; tail -40 /tmp/t0.log /tmp/t1.log; fi'
```
Expected: `PASS`. trust CLI 파싱 테스트가 실행됨(`cli_parses_*`).

---

## Task 4: 전체 WSL 검증 + fmt + 머지 커밋

**Files:** 전체.

- [ ] **Step 1: 전 feature 조합 + fmt (CI 조합만큼 넓게)**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-reconcile-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal-reconcile; ok=1; cargo fmt --all -- --check >/tmp/fmt.log 2>&1 || { echo FMT_FAIL; tail /tmp/fmt.log; ok=0; }; for f in "" "trust" "storage tls remote" "storage tls remote trust"; do FEAT=""; [ -n "$f" ] && FEAT="--features $f"; if cargo clippy --all-targets $FEAT -- -D warnings >/tmp/c.log 2>&1 && cargo test $FEAT >/tmp/t.log 2>&1; then echo "PASS [$f]"; else echo "FAIL [$f]"; tail -30 /tmp/c.log /tmp/t.log; ok=0; fi; done; [ $ok -eq 1 ] && echo ALL_PASS || echo SOME_FAIL'
```
Expected: `ALL_PASS`. fmt 위반 시 `cargo fmt --all` 실행 후 재확인(교훈: `--check`만으론 부족).

- [ ] **Step 2: e2e sanity — trust 명령 동작 (선택, trust feature 빌드)**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal-reconcile-wt; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal-reconcile; cargo run --features trust --bin ai -- policy org status && cargo run --features trust --bin ai -- release manifest status && cargo run --features trust --bin ai -- skill enabled'
```
Expected: 각 명령이 monolithic과 동일한 상태 출력(absent/unavailable 등, panic 없음).

- [ ] **Step 3: 머지 커밋**

```bash
WT=/d/workspace/terminal-project/terminal-reconcile-wt
git -C "$WT" add src/cli/mod.rs src/cli/command.rs src/cli/inspect.rs src/cli/trust.rs src/cli/dispatch.rs src/cli/run.rs src/main.rs src/lib.rs
git -C "$WT" status   # 자동병합분은 이미 머지 인덱스에 있음. 명시 파일만 추가 확인.
git -C "$WT" commit --no-edit   # 머지 커밋 메시지 유지 or -m 로 재조정 설명
```
- `git add -A` 금지. `.superpowers/` 등 untracked 제외 확인.

---

## Task 5: develop PR

- [ ] **Step 1: push + PR**

```bash
WT=/d/workspace/terminal-project/terminal-reconcile-wt
git -C "$WT" push -u origin refactor/wave2-trust-reconcile
cd "$WT" && gh pr create --base develop --head refactor/wave2-trust-reconcile \
  --title "refactor(cli): Wave2↔trust 재조정 — trust를 cli/ 구조로 흡수" \
  --body "spec docs/superpowers/specs/2026-07-10-wave2-trust-reconciliation-design.md (#96). origin/main trust를 develop cli/로 이식: cli/trust.rs 신설·command/dispatch/inspect/run 확장. 충돌 2파일(lib.rs·main.rs)만 수동, 36파일 자동병합. WSL 전 feature 조합 green."
```

- [ ] **Step 2: CI green 확인 후 (사용자 판단으로) develop 머지**

CI 4잡(fmt·clippy 무피처+feature·test·android) green 확인. 머지는 사용자 승인.

---

## Self-Review 체크리스트 (실행자용)

- [ ] main.rs 15줄 유지, trust 증분 0 잔존(`grep -c 'run_skill\|run_release_manifest\|run_policy_org' src/main.rs` = 0)
- [ ] 무피처 `clippy --all-targets` orphan/dead_code 0 (게이트 정확)
- [ ] trust 명령 표면 완전(`ai skill enable/disable/enabled/registry`·`ai policy org status`·`ai release manifest {status,create,sign,verify}`)
- [ ] Wave2 명령 회귀 0(기존 doctor/risk/policy/tui/remote/... 동작)
- [ ] enum/헬퍼 시그니처가 Task 간 일치(`ReleaseManifestSignInput` 필드 9개, `run_skill_registry_update(PathBuf,PathBuf,bool)`)
