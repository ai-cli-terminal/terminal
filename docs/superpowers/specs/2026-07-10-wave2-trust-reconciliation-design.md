# Wave2 ↔ trust 재조정 설계 (develop 베이스 + trust 흡수)

- **작성일**: 2026-07-10
- **상태**: 설계(spec) — 리뷰 대기. 구현은 별도 세션(subagent-driven) 권장.
- **정본 브랜치**: `origin/develop` `c1ba562` (Wave2 split) / `origin/main` `96a8757` (trust monolithic)
- **관련**: [[terminal-project-state]] "남은 것 — Step 2 Wave2→main", Wave2 spec/plan `docs/superpowers/{specs,plans}/2026-07-09-wave2-*`

---

## 1. 목표 & 배경

`origin/develop`(Wave2 워크스페이스 리팩토링)과 `origin/main`(trust 채널 P3-1 + v0.4.0 릴리스)이 **하드하게 분기**했다. 양쪽 모두 이미 각자 랜딩 완료(관련 PR 전부 MERGED, 열린 작업 없음)이나 한 계보로 수렴하지 못한 상태다.

| | `src/main.rs` | `src/cli/` | trust 기능 | 분기 |
|---|---|---|---|---|
| `origin/main` `96a8757` | 4240줄 monolithic | 없음 | 있음 (#69~80) | develop보다 43 앞섬 |
| `origin/develop` `c1ba562` | 15줄 (`cli::dispatch::run()`) | 10모듈 split | 없음 | main보다 22 앞섬 |

**목표**: develop을 베이스로 trust를 흡수해, develop이 **Wave2 split 구조 + trust 기능**을 모두 갖추게 한다. 이후 필요 시 `develop → main` 릴리스 PR로 반영한다(글로벌 git-branch-flow: main 보호, develop 통합).

## 2. 전략 결정 — 전략 A (develop 베이스 + trust 흡수)

**채택**: `origin/develop`(Wave2 split, 검증 완료된 완성품)을 베이스로, `origin/main`의 trust 증분을 흡수한다.

**대안 B (기각)**: `origin/main`(4240줄 monolithic)을 베이스로 main.rs를 cli/로 재분할 = Wave2 결과물을 버리고 재작업. 노동량 과다 + `main` 직접 재작업으로 git-branch-flow 규칙 위반.

**근거**:
1. Wave2 split(10모듈, byte-identical move, WSL 5/5 PASS)은 이미 검증된 자산 → 재사용.
2. trust 증분은 국소적(핸들러 11개 + Command 확장)이라 cli/ 구조에 재배치하는 편이 4240줄 재분할보다 훨씬 작다.
3. develop 통합 → 릴리스 PR 경로가 규칙에 부합.

## 3. 충돌 범위 — 정밀 격리 결과

`git merge-tree --write-tree --name-only origin/develop origin/main` 실측: **충돌 파일은 단 2개**.

### 3-1. 자동 병합 (36파일, 검증만) — 수동 개입 불필요
- **trust 신규 모듈 4개**: `src/{trust,policy_d,skill_registry,binary_manifest}.rs` (develop에 없는 파일 → clean add)
- **trust가 수정한 lib 모듈**: `src/ai_router.rs`(+19)·`src/gated_runner.rs`(+20)·`src/shell_audit.rs`(+112)·`src/skill.rs`(+204) — Wave2가 건드리지 않아 clean merge
- **빌드/배포/문서**: `Cargo.toml`(+4, trust feature)·`package.json`·`scripts/install.{sh,ps1}`·`scripts/checks/*.mjs`·`.github/workflows/{ci,release}.yml`·`config.toml.example`·`docs/**` 전부 clean

### 3-2. 수동 해결 (2파일)
| 파일 | 충돌 원인 | 작업 규모 |
|---|---|---|
| `src/lib.rs` | Wave2가 도메인 클러스터로 재배열 vs trust가 `pub mod` 4개(+12줄) 추가 | 작음 |
| `src/main.rs` | Wave2가 15줄로 축소(→cli/) vs trust가 +1224줄 확장 | **핵심 — 아래 §4** |

## 4. `main.rs` → `cli/` 재배치 매핑 (핵심 작업)

trust가 v0.4.0 base(3041줄) 대비 `main.rs`에 추가한 CLI 표면을, develop의 cli/ 분할 구조에 재배치한다. **`main.rs` 자체는 develop의 15줄(`cli::dispatch::run()`)을 그대로 유지**한다.

### 4-1. Command enum 확장 → `src/cli/command.rs`
develop의 `cli/command.rs`에는 `Skill`·`Policy` variant가 **이미 존재**(v0.4.0 유래). trust는 이를 **확장**한다:

| trust 변경 | develop 현재 | 재배치 후 (cli/command.rs) |
|---|---|---|
| `Skill { query, action: Option<SkillAction> }` | `Skill { query }` | `action` 필드 추가 |
| `PolicyAction::Org { action: PolicyOrgAction }` | `PolicyAction`(Show/Set) | `Org` variant 추가 |
| `Release { action: ReleaseAction }` (신규) | 없음 | 새 Command variant 추가 |

**신규 action enum(모두 cli/command.rs)**: `SkillAction`(Enable/Disable/Enabled/Registry…)·`SkillRegistryAction`(Status/Update…)·`PolicyOrgAction`(Status)·`ReleaseAction`(Manifest)·`ReleaseManifestAction`(Status/Create/Sign/Verify).

### 4-2. dispatch arm → `src/cli/dispatch.rs`
develop `cli/dispatch.rs`의 `match cli.command`에 분기 추가/확장:
- `Command::Skill { query, action }` → `SkillAction` 분기(Enable/Disable/Enabled/Registry)
- `Command::Policy` → `PolicyAction::Org { action } → PolicyOrgAction::Status`
- `Command::Release { action }` → `ReleaseAction::Manifest → ReleaseManifestAction::{Status,Create,Sign,Verify}` (신규 arm)

### 4-3. 핸들러 함수 → **신설 `src/cli/trust.rs`**
trust 핸들러 11개를 새 모듈 `cli/trust.rs`에 몰아 배치(Wave2의 도메인별 모듈 원칙 계승):
`run_policy_org_status` · `run_skill_list` · `run_skill_enabled` · `run_skill_enable` · `run_skill_disable` · `run_skill_registry_status` · `run_skill_registry_update` · `run_release_manifest_create` · `run_release_manifest_sign` · `run_release_manifest_status` · `run_release_manifest_verify`.

- lib 함수 참조는 기존 경로 유지(`ai_terminal::{policy_d,skill_registry,binary_manifest,skill}::*`).
- `cli/mod.rs`에 `pub(crate) mod trust;` 추가.
- **feature gate 주의**: 이 모듈/핸들러/테스트 전체를 trust feature 조건으로 게이트(무피처 빌드에서 미참조 심볼·`use super::*` 고아 방지 — §6 교훈 ①).

### 4-4. 테스트 → `cli/trust.rs` 내부 `#[cfg(test)]`
main.rs `mod tests`의 trust 관련 테스트(`run_policy_org_status`/skill/registry/manifest 검증, `panic!("expected …")` 패턴)를 `cli/trust.rs`로 이동하고 `#[cfg(all(test, feature = "trust"))]`로 게이트.

## 5. 실행 단계 (구현 세션용 개요)

> 격리 워크트리에서 수행. 모든 git 명령 절대경로/`-C` 사용.

1. **통합 브랜치 분기**: `origin/develop`에서 `refactor/wave2-trust-reconcile`(가칭) 분기. 격리 워크트리 생성.
2. **머지 실행**: `git merge origin/main`(no-commit). 자동 병합 36파일 확인, 충돌 2파일 표면화.
3. **lib.rs 해결**: trust `pub mod` 4개를 Wave2 재배열 구조의 적절한 클러스터(주석 경계)에 삽입. rustfmt 클러스터 내부 알파벳 정렬 유지(§6 교훈 ②).
4. **main.rs 해결**: develop의 15줄 채택 + §4 매핑대로 cli/command.rs·cli/dispatch.rs 확장, cli/trust.rs 신설. (충돌 마커 제거 후 실제 재배치는 diff `v0.4.0..origin/main -- src/main.rs`를 소스로 사용.)
5. **자동 병합분 sanity**: trust 신규 모듈·lib 수정분이 cli/ 구조와 정합하는지(경로 참조) 확인.
6. **머지 커밋** 후 검증(§6).
7. **PR**: `refactor/wave2-trust-reconcile → develop`. CI green 확인 후 머지. (릴리스 `develop→main`은 별도 판단.)

## 6. 검증 계획 (WSL, [[terminal-build-env]])

**feature 조합을 CI만큼 넓게** 돌린다(§교훈 ④: verify가 CI보다 좁으면 cfg-combo 이슈 누출):
- **무피처**: `cargo build` · `cargo clippy --all-targets -- -D warnings` · `cargo test --all-targets`
- **trust**: `cargo build --features trust` · `cargo clippy --all-targets --features trust -- -D warnings` · `cargo test --features trust`
- **기타 조합**: `--features "storage tls remote"` 회귀 없음 확인
- `cargo fmt --all -- --check` (게이트)
- 판정은 파이프 없이 exit code로만(§교훈: `cmd | tail && echo OK`는 거짓양성).

**핵심 성공 기준**: trust 명령(`ai skill enable/disable/enabled`·`ai policy org status`·`ai release manifest {status,create,sign,verify}`)이 재배치 후에도 monolithic과 동일하게 동작(e2e). Wave2 명령 표면 회귀 0.

## 7. 리스크 & 함정 (누적 교훈 반영)

1. **feature-gate 비대칭**(교훈 ①/④): trust 심볼을 cli/에 옮길 때 무피처 빌드에서 unresolved/orphan import 발생 쉬움. cli/trust.rs 전체 + mod 선언 + 테스트를 trust로 일관 게이트. 무피처 `clippy --all-targets`로 반드시 확인.
2. **lib.rs rustfmt 클러스터**(교훈 ②): 도메인 클러스터 내부는 알파벳 정렬(주석이 경계). trust `pub mod` 삽입 위치가 알파벳순이어야 fmt 통과.
3. **main.rs 재배치 정확성**: 충돌 마커 기반이 아니라 `v0.4.0..origin/main -- src/main.rs` diff를 소스로 삼아 누락/중복 방지. Command/action/dispatch/handler/test 5요소 체크리스트로 대조.
4. **cwd 리셋**(subagent-scope): 워크트리 다수 → 모든 명령 절대경로. 인접 레포 스팟체크.
5. **서브에이전트 범위 잠금**: 재배치를 위임 시 Task별 정확한 명세 제공, 컨트롤러가 git으로 직접 재검증.

## 8. 산출물

- 통합 브랜치 `refactor/wave2-trust-reconcile`(develop 베이스) → **develop PR**.
- 결과: develop = Wave2 split + trust. main.rs 15줄 유지, trust는 `cli/trust.rs` + 신규 lib 모듈 4개로 재-홈.
- 백업(이 세션): SDD 원장 `scratchpad/sdd-backup/`, 분석 소스 `scratchpad/reconcile-analysis/`.

## 9. 다음 단계

이 spec 승인 후 → `writing-plans`로 구현 plan(Task 분해: 머지·lib.rs·command.rs·dispatch.rs·cli/trust.rs·테스트·검증) 작성 → subagent-driven 실행. **대규모 재배치라 별도 focused 세션 권장.**
