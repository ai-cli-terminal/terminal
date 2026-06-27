# S2 후속 — `ash` Gate Audit 기록 설계

> **작성일**: 2026-06-27
> **유형**: 단일 슬라이스 구현 spec (S2 안전 게이트의 후속 — audit 기록).
> **상위/정본**: `2026-06-26-windows-ash-completion-scoping-design.md` S2 후속, `2026-06-23-platform-execution-contract.md`. 선행: S2(GatedRunner).
> **참고**: `ai exec`/`ai dispatch`의 기록(`src/main.rs` `record_exec`/`shell_outcome_audit`/`record_outcome_audit`), `src/store.rs`.

## 1. 목표

`ash`의 외부 실행 게이트 결과를 **storage에 기록**한다(`ai exec` 패리티). 현재 `GatedRunner`는 아무것도 기록하지 않는다.

- **Ran** → 실행된 명령을 `commands` 테이블에 기록(+`command_executed` audit).
- **Blocked / Declined / BackupRefused** → `audit_events`에 이벤트 기록(`command_blocked`/`command_declined`/`command_backup_refused`).
- 기록은 **storage feature 게이트**(default 빌드는 no-op). 기록 실패는 **best-effort로 조용히 무시**(셸 비중단).

**env 실행 좁히기는 비목표**: 데스크톱 셸은 자식에게 full env를 물려줘야 도구(`gh`/`aws` 등)가 동작한다. secret이 AI/remote로 새는 §5 우려는 이미 context 정책(`context::gather`는 raw env 미포함)+mask로 충족된다.

## 2. DRY — 공유 lib 모듈

현재 audit 로직은 `src/main.rs`(`ai` bin) 안에 있어 lib의 `GatedRunner`가 못 쓴다. 이를 **lib 모듈 `src/shell_audit.rs`로 이동**해 `ai exec`와 `ash`가 공유한다(중복 제거).

`src/shell_audit.rs` (`cfg(not(target_os = "android"))` — `pipeline::ExecOutcome` 등 데스크톱 의존):

```rust
pub struct AuditRecord {
    pub event_type: &'static str,
    pub level: String,
    pub payload_json: String,
}

/// 비-Ran ExecOutcome → audit 레코드(순수). Ran은 None(별도 command 기록).
pub fn shell_outcome_audit(
    command: &str,
    source: &str,
    outcome: &crate::pipeline::ExecOutcome,
) -> Option<AuditRecord>;   // main.rs에서 이동(로직 동일: blocked/declined/backup_refused, 명령 마스킹)

/// audit 레코드 영속화(best-effort). storage 미빌드는 no-op.
#[cfg(feature = "storage")]
pub fn record_outcome_audit(rec: &AuditRecord);
#[cfg(not(feature = "storage"))]
pub fn record_outcome_audit(_rec: &AuditRecord) {}

/// 실행된 명령을 commands + command_executed audit으로 기록. storage 미빌드는 no-op.
#[cfg(feature = "storage")]
pub fn record_ran_command(command: &str, exit_code: i32, source: &str);  // main.rs record_exec 이동
#[cfg(not(feature = "storage"))]
pub fn record_ran_command(_command: &str, _exit_code: i32, _source: &str) {}
```
- `lib.rs`에 `#[cfg(not(target_os="android"))] pub mod shell_audit;` 등록.
- 기존 `record_exec`/`shell_outcome_audit`/`record_outcome_audit`/`struct AuditRecord`는 main.rs에서 **삭제**하고 `shell_audit::*`로 호출 교체(동작 불변). `record_exec`→`record_ran_command`로 이름만 변경.

## 3. `GatedRunner` 결선

`GatedRunner::run`에서 `pipeline::execute`가 돌려준 `outcome`을 outcome_message로 매핑하기 **전에** 기록:

```rust
let outcome = pipeline::execute(&cmd, &cfg, &executor, &mut confirmer, &mut sink)?;
match &outcome {
    ExecOutcome::Ran { exit_code, .. } => {
        crate::shell_audit::record_ran_command(&cmd, *exit_code, "ash");
    }
    other => {
        if let Some(rec) = crate::shell_audit::shell_outcome_audit(&cmd, "ash", other) {
            crate::shell_audit::record_outcome_audit(&rec);
        }
    }
}
let (msg, value) = outcome_message(&outcome, command.name);
...
```
- source 라벨은 **`"ash"`**(`ai exec`의 `exec`/`dispatch`와 구분).
- `cmd`는 GatedRunner가 이미 가진 재구성 명령 문자열.

## 4. 에러 처리

- 기록은 best-effort: `Store::open_default()` 실패·INSERT 실패는 조용히 무시(기존 패턴, `let _ =`). 게이트/실행/REPL에 영향 없음.
- default(C-free) 빌드: 기록 함수가 no-op → ash는 동일하게 동작(기록만 없음).

## 5. 경계·테스트

- `shell_audit`는 데스크톱 모듈(android 제외). `shellcore`·android cdylib 빌드 불변(`cargo check --lib --target aarch64-linux-android` green).
- 단위: `shell_outcome_audit` 매퍼(Ran→None, Blocked/Declined/BackupRefused→해당 event_type·마스킹된 command·source 포함). (main.rs에서 이동한 테스트 + ExecOutcome variant 커버)
- e2e(WSL, **storage feature**): `cargo run --features storage --bin ash`로 `rm -rf /`(차단)·`echo hi`(실행) 후 `python3` 표준 sqlite3로 `audit_events`/`commands` 행 확인(차단=command_blocked, 실행=command_executed/commands). default 빌드는 기록 no-op 확인.
- `ai exec` 회귀 없음: 기존 `ai exec`/`ai dispatch` 기록이 lib 공유본으로도 동일 동작(전체 테스트 green).

## 6. 수용 기준

1. ash 게이트 결과가 storage에 기록된다(Ran→commands+command_executed, 비-Ran→해당 audit event). source="ash".
2. 기록은 storage 게이트·best-effort(default no-op, 실패 무시, 셸 비중단).
3. audit 로직이 lib `shell_audit`로 단일화되고 `ai exec`도 그것을 쓴다(중복 제거, 동작 불변).
4. `shellcore`/android cdylib 빌드 불변.
5. `cargo fmt --all -- --check`(실제 `cargo fmt --all` 후) · `cargo clippy --all-targets --features "storage tls remote" -D warnings` · `cargo test --features "storage tls remote"` green. default 빌드도 `cargo build`/`cargo test` green.

## 7. 비목표

- env 실행 좁히기(해로움), AI usage 기록(별개 S5 후속), `command_executed` payload를 serde_json·source 포함으로 통일(기존 불일치 — 별도), 마스킹 정책 변경.
