# 비-Ran 명령 결과 audit 기록 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `run_exec`/`run_dispatch`의 `Blocked`/`Declined`/`BackupRefused` 결과를 마스킹된 명령과 함께 audit_events에 기록한다.

**Architecture:** 순수 매퍼(`shell_outcome_audit`)가 `ExecOutcome`을 audit 레코드로 변환(단위 테스트 가능)하고, 발산 I/O 헬퍼(`finish_shell_outcome`)가 audit 저장·stderr 출력·프로세스 종료를 담당한다. 두 CLI 호출자가 헬퍼를 공유해 기존 중복(Shell arm)을 제거한다. `pipeline.rs`는 storage-free 유지.

**Tech Stack:** Rust, `serde_json`(payload), `mask`(W7 마스킹), `risk`(level 재산출), rusqlite(`--features storage`).

설계 정본: `docs/superpowers/specs/2026-06-03-audit-non-ran-outcomes-design.md`

빌드/검증 래퍼(WSL): `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`

---

### Task 1: 순수 매퍼 `shell_outcome_audit` + `AuditRecord`

**Files:**
- Modify: `src/main.rs` (record_exec 근처, 약 1153행 뒤에 함수 추가; 테스트는 `#[cfg(test)] mod tests` 약 1188행)

- [ ] **Step 1: 실패 테스트 작성**

`src/main.rs`의 `mod tests` 안에 추가:

```rust
#[test]
fn shell_outcome_audit_ran_is_none() {
    use ai_terminal::pipeline::ExecOutcome;
    let out = ExecOutcome::Ran {
        exit_code: 0,
        undo_id: None,
    };
    assert!(shell_outcome_audit("ls -al", "exec", &out).is_none());
}

#[test]
fn shell_outcome_audit_blocked_has_type_level_factors() {
    use ai_terminal::pipeline::ExecOutcome;
    use ai_terminal::risk::RiskLevel;
    let out = ExecOutcome::Blocked {
        level: RiskLevel::Critical,
        factors: vec!["재귀 삭제 (+30)".to_string()],
    };
    let rec = shell_outcome_audit("rm -rf /", "exec", &out).expect("blocked → Some");
    assert_eq!(rec.event_type, "command_blocked");
    assert_eq!(rec.level, "Critical");
    assert!(rec.payload_json.contains("\"factors\""));
    assert!(rec.payload_json.contains("재귀 삭제 (+30)"));
    assert!(rec.payload_json.contains("\"source\":\"exec\""));
    assert!(rec.payload_json.contains("\"command\""));
}

#[test]
fn shell_outcome_audit_declined_reassesses_level() {
    use ai_terminal::pipeline::ExecOutcome;
    // rm -rf / 는 Critical 로 재산출되어야 한다(variant에 level 없음).
    let rec = shell_outcome_audit("rm -rf /", "dispatch", &ExecOutcome::Declined)
        .expect("declined → Some");
    assert_eq!(rec.event_type, "command_declined");
    assert_eq!(rec.level, "Critical");
    assert!(rec.payload_json.contains("\"source\":\"dispatch\""));
}

#[test]
fn shell_outcome_audit_backup_refused_has_reason() {
    use ai_terminal::pipeline::ExecOutcome;
    let out = ExecOutcome::BackupRefused("파일 크기 초과".to_string());
    let rec = shell_outcome_audit("rm /tmp/x", "exec", &out).expect("refused → Some");
    assert_eq!(rec.event_type, "command_backup_refused");
    assert!(rec.payload_json.contains("\"reason\":\"파일 크기 초과\""));
}

#[test]
fn shell_outcome_audit_masks_secret_in_command() {
    use ai_terminal::pipeline::ExecOutcome;
    let token = "ghp_0123456789abcdef0123456789abcdef0123";
    let cmd = format!("echo {token}");
    let rec = shell_outcome_audit(&cmd, "exec", &ExecOutcome::Declined)
        .expect("declined → Some");
    assert!(
        !rec.payload_json.contains(token),
        "원문 secret 이 payload 에 잔존하면 안 됨: {}",
        rec.payload_json
    );
}
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shell_outcome_audit'`
Expected: FAIL — `cannot find function shell_outcome_audit` / `cannot find type AuditRecord`.

- [ ] **Step 3: 매퍼 구현**

`src/main.rs`에서 `record_exec` 함수 정의 뒤(약 1156행, `#[cfg(not(feature = "storage"))] fn record_exec` 다음)에 추가:

```rust
/// 비-Ran 셸 결과의 audit 레코드. 순수 데이터 — storage/exit 비의존이라 단위 테스트 가능.
struct AuditRecord {
    event_type: &'static str,
    level: String,
    payload_json: String,
}

/// 비-Ran 결과를 audit 레코드로 변환한다. `Ran`은 `record_exec`가 처리하므로 `None`.
/// 명령은 W7 마스킹 후 payload에 담고, level은 Blocked의 carried 값 또는 재산출값을 쓴다.
fn shell_outcome_audit(
    command: &str,
    source: &str,
    outcome: &ai_terminal::pipeline::ExecOutcome,
) -> Option<AuditRecord> {
    use ai_terminal::pipeline::ExecOutcome;

    let (event_type, level, mut payload) = match outcome {
        ExecOutcome::Ran { .. } => return None,
        ExecOutcome::Blocked { level, factors } => (
            "command_blocked",
            format!("{level:?}"),
            serde_json::json!({ "factors": factors }),
        ),
        ExecOutcome::Declined => (
            "command_declined",
            format!("{:?}", risk::assess(command).level),
            serde_json::json!({}),
        ),
        ExecOutcome::BackupRefused(reason) => (
            "command_backup_refused",
            format!("{:?}", risk::assess(command).level),
            serde_json::json!({ "reason": reason }),
        ),
    };

    let masked = mask::Masker::baseline().mask(command).text;
    if let serde_json::Value::Object(map) = &mut payload {
        map.insert("command".into(), serde_json::Value::String(masked));
        map.insert(
            "source".into(),
            serde_json::Value::String(source.to_string()),
        );
    }

    Some(AuditRecord {
        event_type,
        level,
        payload_json: payload.to_string(),
    })
}
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shell_outcome_audit'`
Expected: PASS (5개). storage 미지정 기본 빌드여도 매퍼는 항상 컴파일됨.

참고: `serde_json::json!`은 `&Vec<String>`(factors)·`&String`(reason)을 직렬화한다. `risk::assess`/`mask`는 이미 `use` 되어 있다(main.rs 상단).

- [ ] **Step 5: fmt + 커밋**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
wsl.exe -- bash -lc 'cd /mnt/d/workspace/terminal-project/terminal; git add src/main.rs && git commit -m "feat(audit): pure mapper for non-Ran command outcomes

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

### Task 2: I/O 헬퍼 `finish_shell_outcome` + 호출자 재배선

**Files:**
- Modify: `src/main.rs` — 매퍼 뒤에 `record_outcome_audit`(storage-gated)·`finish_shell_outcome` 추가; `run_exec`(약 978행)·`run_dispatch`(약 1025행) 재배선.

- [ ] **Step 1: storage-gated audit writer + 발산 헬퍼 추가**

`shell_outcome_audit` 정의 뒤에 추가:

```rust
/// audit 레코드를 영속화한다(storage feature). 실패는 조용히 무시(감사는 best-effort).
#[cfg(feature = "storage")]
fn record_outcome_audit(rec: &AuditRecord) {
    use ai_terminal::store::Store;
    let Ok(store) = Store::open_default() else {
        return;
    };
    let _ = store.record_audit(
        rec.event_type,
        Some(&rec.level),
        Some(&config::get_active_profile()),
        &rec.payload_json,
    );
}

#[cfg(not(feature = "storage"))]
fn record_outcome_audit(_rec: &AuditRecord) {}

/// 셸 실행 결과를 마무리한다: audit 기록 + 사용자 안내 + 프로세스 종료(항상 발산).
/// `run_exec`·`run_dispatch`가 공유한다. `command`는 기록/안내에 쓸 명령 텍스트.
fn finish_shell_outcome(command: &str, source: &str, outcome: ai_terminal::pipeline::ExecOutcome) -> ! {
    use ai_terminal::pipeline::ExecOutcome;

    if let ExecOutcome::Ran { exit_code, undo_id } = &outcome {
        if let Some(id) = undo_id {
            eprintln!("(백업 생성: {id} — 되돌리려면 `ai undo last`)");
        }
        record_exec(command, *exit_code, source);
        std::process::exit(*exit_code);
    }

    // 비-Ran: audit 기록 후 안내 + exit 1.
    if let Some(rec) = shell_outcome_audit(command, source, &outcome) {
        record_outcome_audit(&rec);
    }
    match outcome {
        ExecOutcome::Blocked { level, factors } => {
            eprintln!("차단됨: 위험 등급 {level:?} (정책상 실행 불가)");
            for f in &factors {
                eprintln!("  - {f}");
            }
        }
        ExecOutcome::Declined => eprintln!("실행을 취소했습니다."),
        ExecOutcome::BackupRefused(r) => eprintln!("백업 거부로 실행 중단: {r}"),
        ExecOutcome::Ran { .. } => unreachable!("Ran 은 위에서 처리됨"),
    }
    std::process::exit(1);
}
```

- [ ] **Step 2: `run_exec` 재배선**

`run_exec`(약 978행)의 `use` 줄에서 `ExecOutcome`를 제거하고, outcome match 블록 전체(약 999~1022행)를 헬퍼 호출로 교체한다.

`use ai_terminal::pipeline::{self, ExecConfig, ExecOutcome};` → `use ai_terminal::pipeline::{self, ExecConfig};`

그리고:

```rust
    let outcome = pipeline::execute(command, &cfg, &executor, confirmer.as_mut(), &mut sink)?;
    flush_stdout();
    finish_shell_outcome(command, "exec", outcome)
```

(`finish_shell_outcome`는 `!`를 반환하므로 `anyhow::Result<()>` 꼬리 표현식으로 그대로 coerce 된다.)

- [ ] **Step 3: `run_dispatch` 재배선**

`run_dispatch`(약 1025행)의 `use` 줄에서 `ExecOutcome` 제거:

`use ai_terminal::pipeline::{self, ExecConfig, ExecOutcome};` → `use ai_terminal::pipeline::{self, ExecConfig};`

그리고 match 블록의 네 `Handled::Shell(...)` arm(약 1057~1079행)을 하나로 합친다:

```rust
    match handled {
        Handled::Empty => Ok(()),
        Handled::Shell(outcome) => finish_shell_outcome(input, "dispatch", outcome),
        Handled::Ai(AiOutcome::Answered {
            input_tokens,
            output_tokens,
            ..
        }) => {
            #[cfg(feature = "storage")]
            if let Ok(store) = ai_terminal::store::Store::open_default() {
                let _ = store.record_usage(
                    "mock",
                    "mock-model",
                    input_tokens as i64,
                    output_tokens as i64,
                    0,
                    0.0,
                    None,
                );
            }
            println!("\n(tokens ~ in:{input_tokens} out:{output_tokens})");
            Ok(())
        }
        Handled::Ai(AiOutcome::Blocked(r)) => {
            println!("[차단] 원격 전송 불가(fail-closed): {r}");
            Ok(())
        }
        Handled::Ai(AiOutcome::Unavailable(e)) => {
            println!("[AI 사용 불가] {e}");
            Ok(())
        }
    }
```

`Handled`·`AiOutcome`는 기존 `use ai_terminal::dispatch::{self, AiOutcome, Handled, Handlers};`로 이미 들어와 있다.

- [ ] **Step 4: 빌드·clippy·fmt·전체 테스트(기본 + storage)**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test'
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets --features storage -- -D warnings && cargo test --features storage'
```
Expected: clippy/fmt clean, 모든 테스트 PASS(기본 + storage). 기존 178/195+ 테스트 + 신규 5개.

- [ ] **Step 5: 커밋**

```bash
wsl.exe -- bash -lc 'cd /mnt/d/workspace/terminal-project/terminal; git add src/main.rs && git commit -m "feat(audit): record blocked/declined/backup-refused outcomes via shared finish helper

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

### Task 3: WSL e2e 검증 + 문서 갱신

**Files:**
- Modify: `docs/TASK.md`(백로그 LOW 항목 ① 완료 표기), `docs/HISTORY.md`(엔트리 추가)

- [ ] **Step 1: e2e — Blocked 가 audit 에 기록되는지 (storage)**

`store::data_dir`는 `$XDG_DATA_HOME/ai-terminal`(없으면 `$HOME/.local/share/ai-terminal`)를 쓴다. `XDG_DATA_HOME=$D`로 격리하면 db는 `$D/ai-terminal/ai-terminal.db`에 생긴다. 바이너리는 `$HOME/targets/ai-terminal/debug/ai`(Cargo.toml `[[bin]] name = "ai"`). 한 줄로 실행(WSL 멀티라인 금지):

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build --features storage; BIN=$HOME/targets/ai-terminal/debug/ai; D=$(mktemp -d); XDG_DATA_HOME=$D $BIN exec "rm -rf /"; echo "exit=$?"; python3 -c "import sqlite3; c=sqlite3.connect(\"$D/ai-terminal/ai-terminal.db\"); print(list(c.execute(\"select event_type,risk_level,payload_json from audit_events\")))"'
```
Expected: `exit=1`, 그리고 audit_events에 `('command_blocked','Critical','{...\"command\"...}')` 행 출력. 마스킹 무관(`rm -rf /`에 secret 없음).

- [ ] **Step 2: e2e — Declined 가 기록되는지**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; BIN=$HOME/targets/ai-terminal/debug/ai; D=$(mktemp -d); echo n | XDG_DATA_HOME=$D $BIN exec "sudo systemctl restart nginx"; echo "exit=$?"; python3 -c "import sqlite3; c=sqlite3.connect(\"$D/ai-terminal/ai-terminal.db\"); print(list(c.execute(\"select event_type,risk_level from audit_events\")))"'
```
Expected: `exit=1`, audit_events에 `('command_declined','High')` 행.

- [ ] **Step 3: TASK.md 갱신**

`docs/TASK.md`의 "남은 백로그(LOW)" 줄(약 20행)에서 audit 항목을 완료로 표기:

기존:
```
  - **남은 백로그(LOW)**: ① Blocked/Declined/BackupRefused 시도 audit 미기록(현재 Ran만). W2 실제 async 스트리밍·W9 실제 diff·gateway 시맨틱 캐시 2차 조회도 후속. (Shell/Ai dispatcher 통합은 완료 — 위 참조.)
```
변경:
```
  - **남은 백로그(LOW)**: ✅ Blocked/Declined/BackupRefused audit 기록 완료(2026-06-03, `command_blocked`/`command_declined`/`command_backup_refused`, 마스킹된 명령 포함). W2 실제 async 스트리밍·W9 실제 diff·gateway 시맨틱 캐시 2차 조회는 후속.
```

- [ ] **Step 4: HISTORY.md 엔트리 추가**

`docs/HISTORY.md` 최상단(또는 날짜 섹션 규칙에 맞는 위치)에 추가:

```markdown
- 2026-06-03: 비-Ran 명령 결과 audit 기록 — `run_exec`/`run_dispatch`의 Blocked/Declined/BackupRefused를 `shell_outcome_audit`(순수 매퍼) + `finish_shell_outcome`(공용 발산 헬퍼)로 audit_events에 기록. 마스킹된 명령 포함, event_type 결과별 분리. Shell arm 중복 제거. 설계/계획: `docs/superpowers/{specs,plans}/2026-06-03-audit-non-ran-outcomes*`.
```

(HISTORY.md의 기존 엔트리 형식을 먼저 확인해 맞춘다.)

- [ ] **Step 5: 커밋**

```bash
wsl.exe -- bash -lc 'cd /mnt/d/workspace/terminal-project/terminal; git add docs/TASK.md docs/HISTORY.md && git commit -m "docs: mark non-Ran audit logging complete

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

## 완료 기준 (DoD)

- `Blocked`/`Declined`/`BackupRefused` 모두 audit_events에 각각의 event_type으로 기록(마스킹된 command + level + source).
- 단위 테스트 5개 통과(Ran→None, 각 비-Ran 타입/level, BackupRefused reason, 마스킹 무유출).
- clippy/fmt clean, 기본 + storage 전체 테스트 PASS.
- WSL e2e로 command_blocked·command_declined 행 실제 확인.
- run_exec/run_dispatch Shell arm 중복 제거(공용 헬퍼).
- 문서(TASK/HISTORY) 갱신.
