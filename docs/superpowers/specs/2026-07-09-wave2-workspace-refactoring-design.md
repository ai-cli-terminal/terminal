# Wave 2 워크스페이스 리팩토링 설계 (2026-07-09)

Wave 1(거대파일 3분할: desktop `main.ts`/`src-tauri main.rs`, `daemon.rs`→`src/daemon/`,
`remote_transport.rs`→`src/remote_transport/`)에 이은 후속. 바이너리 진입점 `src/main.rs`
분할과 `src/lib.rs` 도메인 정리, `docs/HANDOFF.md` 슬리밍을 다룬다.

## 1. 목표

- **`src/main.rs`(3041줄) 분할** → `src/cli/` 하위 모듈. 진입점은 얇게(~50줄).
- **`src/lib.rs`(125줄) 도메인 정리** — 평면 `pub mod` 나열을 도메인 클러스터 + 섹션 주석으로.
- **`docs/HANDOFF.md`(504줄) 슬리밍** — 중복·stale 섹션 제거, 재개 가이드+우선순위만.

전부 **move-only(동작 보존)**. 심볼 이동·재배열만, 로직/동작/공개 API 변경 없음.

## 2. 비목표

- 로직·동작 변경, 기능 추가, 성능 최적화.
- 무관한 리팩토링.
- `lib.rs` 모듈 **경로 변경**(`ai_terminal::X`) — 소비자 대량 파손 유발. 이름·경로 불변 유지.
- trust 스택(#69~80) 통합 — 별도. Wave2 랜딩 후 trust rebase(§8).

## 3. trust 스택과의 관계 (제약)

trust 스택(codex/trust-*, #69~80)은 새 main(`fcb6627`) 위로 rebase돼 있고, `src/main.rs`를
**+1224 additive**(새 서브커맨드 핸들러: `PolicyOrgAction`·`SkillAction`·`SkillRegistryAction`·
`ReleaseAction`·`ReleaseManifestAction` enum + `run_skill_*`·`run_release_manifest_*`·
`run_policy_org_status` 등)로 수정한다. 삭제는 25줄(Command enum·`main()`에 variant/arm 삽입).

Wave2는 **공격적 분할**(사용자 결정) — `Command` enum과 `main()` 디스패치도 `src/cli/`로 옮긴다.
이러면 trust의 다음 rebase 충돌이 `cli/command.rs`(enum variant 추가)·`cli/dispatch.rs`(match arm
추가)로 **국한**된다. trust 변경이 순수 additive라 재적용 가능하며, trust는 우리가 통제하는
rebase 스택이므로 감수한다. **순서: Wave2를 develop→(추후 main) 먼저, 그 뒤 trust를 새 main 위로
재rebase**(bottom #69부터 `--update-refs`, 문서 충돌은 `-X ours`).

## 4. 설계 1 — `src/main.rs` → `src/cli/`

`main.rs`가 `mod cli;`를 선언하고, 핸들러 그룹을 `src/cli/*.rs`로 추출한다. 각 모듈은 자신의
`#[cfg(test)]` 테스트를 함께 가져간다(main.rs 하단 ~764줄 테스트 분배).

| 모듈 | 이동 심볼 |
|---|---|
| `src/cli/command.rs` | `Cli`, `Command`, `PolicyAction`, `RemoteAction`, `InitTarget`, `InitMode` (clap 정의) |
| `src/cli/dispatch.rs` | `main()` 본문 → `pub fn run()`(Command 매치→핸들러 호출), `now_secs`·`console_process_count`·`is_double_click_launch` |
| `src/cli/hooks.rs` | `InitPlan`, `plan_init_shell`, `resolve_shell`, `resolve_rc`, `record_hook_{preexec,precmd,chpwd}` |
| `src/cli/inspect.rs` | `format_explain`, `run_explain`, `format_preview`, `format_mask`, `format_risk`, `describe_profile`, `resolve_profile` |
| `src/cli/run.rs` | `run_exec`, `run_dispatch`, `flush_stdout`, `finish_shell_outcome`, `cache_badge` |
| `src/cli/gate.rs` | `run_gate`, `run_gate_daemon`, `DaemonTransportSelection`, `resolve_daemon_transport_selection`, `resolve_relay_deployment_selection` |
| `src/cli/remote.rs` | `remote_pair_transport_addr`, `run_remote_{devices,transport,relay_setup,pair}`, `run_remote_approval_{url,verify}`, `remote_approval_verify_device` (cfg 변형 쌍 전부) |
| `src/cli/doctor.rs` | `run_doctor`, `format_config_diagnostics` |
| `src/cli/io.rs` | `StdoutSink`, `AutoYes`, `StdinConfirmer` (pipeline `OutputSink`/`Confirmer` impl) |
| `src/cli/shell_run.rs` | `run_persistent_shell`, `sync_wrapper_cwd` |
| **`src/main.rs`(유지)** | 크레이트 문서, `mod cli;`, `fn main() -> anyhow::Result<()> { cli::dispatch::run() }` (~50줄) |

**규율**:
- `#[cfg(...)]` 게이팅·cfg 변형 함수(예: `run_remote_*`의 unix/비-unix, `console_process_count`의
  windows/기타 쌍)는 **그대로** 이동. 무피처·remote·android 조합에서 심볼 해소 불변 확인.
- 모듈 간 가시성: 추출된 함수는 `pub(crate)` 또는 `pub`(모듈 내), `cli/dispatch.rs`가 호출.
- 테스트는 대상 함수와 같은 모듈의 `#[cfg(test)] mod tests`로 이동(예: `cli_parses_remote_*`→
  `cli/command.rs` 또는 `cli/remote.rs`, `format_risk_*`→`cli/inspect.rs`).
- 순수 이동 입증: 이동 전후 `main.rs` 줄수 감소 + 각 모듈 심볼 카운트 대조(Wave1 troubleshooting 패턴).

## 5. 설계 2 — `src/lib.rs` 도메인 정리

~53개 `pub mod`를 알파벳순 → **도메인 클러스터 + 섹션 주석**으로 재배열. `pub mod X;` 이름·경로·
`#[cfg]`는 **불변**(비파괴). 클러스터(안):

- `// === 보안 코어 ===` risk·policy·mask·preview·diff·undo·sandbox·guardrails
- `// === AI / 게이트웨이 ===` gateway·intent·cache·dispatch·ollama·openai·provider·responder·planner·aitask·verify·verify_agent·tokenwin·http
- `// === 컨텍스트 / 인덱스 ===` context·index·explain·cmdparse
- `// === 셸 ===` shellcore·shell·pty·wrapper·line_editor·gated_runner·shell_audit
- `// === 원격 승인(RA) ===` gate·approval·daemon·remote·remote_transport·session·pairing·device_registry·qr
- `// === 저장 / 사용량 ===` store·lock·usage·ai_usage
- `// === skill / mcp ===` skill·mcp
- `// === 모바일 ===` mobile·mobile_ffi·mobile_jni
- `// === config / UI ===` config·ui

크레이트 doc(`//!`)의 "현재 구현된 모듈" 목록도 최신화. **경로 불변**이라 소비자·`main.rs`·테스트
무영향.

## 6. 설계 3 — `docs/HANDOFF.md` 슬리밍

504줄 → ~180줄. 현재 섹션 번호가 중복·누적(`## 1.`이 두 개: "2026-07-02 closeout"과 "현재 상태 —
v0.3.3", `## 0.`/`## 0.1.` 등)이라 정리한다.

- **제거**: v0.3.3 구 상태(§1 하단)·v0.3.3 산출물 검증(§2)·오래된 세션 closeout 등 stale.
- **유지·압축**: 최신 재개점(§0), 빌드·검증 환경 메모(WSL/feature gate 요약 — 정본은
  `[[terminal-build-env]]` 메모리/`docs/`), 다음 작업 우선순위, 비목표.
- 섹션 번호 재정리(단일 증가). 정본 포인터(`docs/TASK.md`·`WORKFLOW.md`·`HISTORY.md`·`CHANGELOG.md`)는 유지.

## 7. 검증

Wave1과 동일 매트릭스. 각 PR·최종:

- WSL cargo: `fmt --all -- --check` / `clippy --all-targets --features "storage tls remote" -- -D warnings` /
  `test --features "storage tls remote"`(baseline 테스트 수 동일 확인) / `build`(무피처) /
  `check --lib --target aarch64-linux-android`. 판정은 파이프 없이 `&& echo PASS || echo FAIL`.
- Android: 해당 없음(main.rs/lib.rs는 desktop 게이트, android는 lib만·`cfg(not(android))`) — 단
  `cargo check --lib --target aarch64-linux-android`로 android 타깃 심볼 해소 재확인.
- **move-only 입증**: 이동 심볼/테스트 카운트 대조, `git diff` 순수 이동 검토, 최종 whole-branch 리뷰.

## 8. PR 구조

Wave1처럼 **base=develop**(2단계 PR 규칙), 브랜치 `refactor/wave2-*`·`docs/wave2-*`. subagent-driven
가능(SDD File Handoffs). 3 PR:

1. `refactor/wave2-main-split` — `src/main.rs`→`src/cli/` 10모듈 + main.rs 얇게(이 PR이 핵심·최대).
2. `refactor/wave2-lib-domains` — `src/lib.rs` 도메인 재배열.
3. `docs/wave2-handoff-slim` — `docs/HANDOFF.md` 슬리밍 + 이 설계/계획 문서.

Wave2 랜딩(develop→main) 후 **trust 스택 재rebase**(§3)가 후속.

## 9. 리스크 · 롤백

| 리스크 | 대응 |
|---|---|
| move 중 로직 변경(회귀) | 매트릭스 test 수 baseline 동일 + whole-branch diff가 순수 이동인지 리뷰 |
| cfg 비대칭(무피처 빌드 unresolved) | 무피처 `cargo check` + `--features remote`·android 양쪽(Wave1 교훈: 모듈 재수출 cfg 대칭) |
| 테스트 이동 누락 | 이동 전후 `#[test]` 총수 대조 |
| trust 재rebase 충돌 확대 | §3대로 additive라 `-X ours`(문서)+수동(command/dispatch) 재적용, trust는 통제 스택 |
| 롤백 | 각 PR 미머지 시 브랜치 폐기. develop 머지 후엔 revert PR. base develop tip 재확인 후 분기 |
