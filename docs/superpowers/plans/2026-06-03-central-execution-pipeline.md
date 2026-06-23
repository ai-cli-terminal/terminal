# 중앙 실행 파이프라인 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**목표:** 셸 명령을 위험도→정책→preview→undo 백업(W10)→실행(W2)→기록(W11/W12)의 단일 오케스트레이터로 묶고 `ai exec` CLI와 TUI에 연결한다.

**Architecture:** 순수 코어 `pipeline::execute`가 I/O를 3개 트레이트(Executor/Confirmer/OutputSink)로 주입받아 PTY 없이 단위 테스트 가능. 실제 실행은 `PtyExecutor`(`run_in_pty` 래핑), 스토리지 기록은 호출측(CLI)에서 `#[cfg(feature="storage")]`로 수행. 스트리밍/실제 diff/dispatcher 통합은 후속.

**기술 스택:** Rust, anyhow, 기존 모듈(risk/policy/preview/undo/pty/store). 빌드·검증은 WSL(메모리 `terminal-build-env`).

설계 정본: `docs/superpowers/specs/2026-06-03-central-execution-pipeline-design.md`.

---

## File Structure

- **Create** `src/pipeline.rs` — 오케스트레이터 + 트레이트(Executor/Confirmer/OutputSink) + 타입(ConfirmRequest/ExecConfig/ExecOutcome) + PtyExecutor + 백업 대상 산출 헬퍼. 단위 테스트 포함.
- **Modify** `src/lib.rs` — `pub mod pipeline;` 등록.
- **Modify** `src/main.rs` — `ai exec` 서브커맨드 + `run_exec` 핸들러 + StdoutSink/AutoYes/StdinConfirmer + `record_exec`(storage 게이트).
- **Modify** `src/ui.rs` — TUI Enter 경로를 `run_in_pty` 직접 호출에서 `pipeline::execute`로 재배선.

재사용(변경 없음): `risk`, `policy`, `preview`, `undo`, `pty`, `store`.

## 검증 환경 (메모리 `terminal-build-env`)

모든 cargo는 WSL에서 단일 라인으로 실행한다(멀티라인 금지):

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test'
```

이하 각 Task의 `실행:`은 위 래퍼의 `cargo ...` 부분만 적는다.

---

## 작업 1: pipeline 모듈 스캐폴드 + Allow 경로

**Files:**
- Create: `src/pipeline.rs`
- Modify: `src/lib.rs:26` (openai 다음 줄에 등록)

- [ ] **단계 1: 모듈을 등록한다**

`src/lib.rs`에서 `pub mod openai;`(26행)와 `pub mod planner;`(27행) 사이에 추가:

```rust
pub mod openai;
pub mod pipeline;
pub mod planner;
```

- [ ] **단계 2: 코어 타입·트레이트·Allow 경로만 가진 `src/pipeline.rs`를 작성한다**

```rust
//! 중앙 실행 파이프라인 (설계 §5/§16.2, 그룹 C 키스톤).
//!
//! 위험도 → 정책 게이트 → preview → undo 백업(W10) → 실행(W2)을 하나의
//! 오케스트레이터로 묶는다. I/O는 트레이트로 주입해 코어를 순수하게 유지한다
//! (PTY 없이 단위 테스트 가능). 스토리지 기록은 호출측(CLI)에서 수행한다.

use std::path::{Path, PathBuf};

use crate::policy::{Decision, PolicyProfile};
use crate::preview::{self, PreviewPlan};
use crate::risk::{self, RiskLevel};
use crate::undo::{self, BackupOutcome, UndoLimits};

/// 출력 싱크(W2 스트리밍 심). CLI=stdout, TUI=히스토리, 테스트=수집.
pub trait OutputSink {
    fn write(&mut self, chunk: &str);
}

/// 실행 추상화(W2 스트리밍 심). 지금은 동기 PtyExecutor, 후속에 스트리밍 impl.
pub trait Executor {
    /// 명령을 실행하고 출력을 sink로 흘려보낸 뒤 종료 코드를 반환한다.
    fn run(&self, command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32>;
}

/// 확인 게이트에 노출할 정보(감사/설명용, RULES §2).
pub struct ConfirmRequest {
    pub command: String,
    pub level: RiskLevel,
    pub decision: Decision,
    pub factors: Vec<String>,
    pub preview: PreviewPlan,
    pub backup_files: Vec<String>,
}

/// 확인 게이트 주입. CLI=stdin/--yes, TUI=모달, 테스트=스크립트.
pub trait Confirmer {
    fn confirm(&mut self, req: &ConfirmRequest) -> bool;
}

/// 파이프라인 설정.
pub struct ExecConfig<'a> {
    pub profile: &'a PolicyProfile,
    pub undo_dir: &'a Path,
    pub limits: UndoLimits,
}

/// 실행 결과.
#[derive(Debug, PartialEq, Eq)]
pub enum ExecOutcome {
    /// 정책상 차단(실행 안 함).
    Blocked { level: RiskLevel, factors: Vec<String> },
    /// 사용자가 확인을 거부(실행 안 함).
    Declined,
    /// 백업이 상한 등으로 거부됨(위험 명령 중단).
    BackupRefused(String),
    /// 실행됨.
    Ran { exit_code: i32, undo_id: Option<String> },
}

/// PTY 기반 실행기(실제 셸 경로).
pub struct PtyExecutor {
    pub shell: String,
}

impl Executor for PtyExecutor {
    fn run(&self, command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
        let out = crate::pty::run_in_pty(&self.shell, command)?;
        sink.write(&out.output);
        Ok(out.exit_code as i32)
    }
}

/// 중앙 실행 파이프라인. 게이트를 통과한 경우에만 executor를 호출한다.
pub fn execute(
    command: &str,
    cfg: &ExecConfig,
    executor: &dyn Executor,
    confirmer: &mut dyn Confirmer,
    sink: &mut dyn OutputSink,
) -> anyhow::Result<ExecOutcome> {
    let _ = (command, cfg, confirmer); // 후속 단계에서 사용
    let exit_code = executor.run(command, sink)?;
    Ok(ExecOutcome::Ran {
        exit_code,
        undo_id: None,
    })
}

/// 백업 대상 파일을 산출한다. 삭제/덮어쓰기/in-place 편집 명령의 **기존 일반 파일**만.
/// 권한 변경(chmod/chown/chgrp)은 내용 백업이 무의미하므로 제외한다.
fn backup_targets(command: &str) -> Vec<PathBuf> {
    let toks: Vec<&str> = command.split_whitespace().collect();
    let prog = program_token(&toks);
    let in_place =
        matches!(prog, Some("sed") | Some("perl")) && toks.iter().any(|t| t.starts_with("-i"));
    let backupable = matches!(
        prog,
        Some("rm")
            | Some("unlink")
            | Some("shred")
            | Some("cp")
            | Some("mv")
            | Some("tee")
            | Some("touch")
    ) || in_place
        || command.contains('>');
    if !backupable {
        return Vec::new();
    }
    candidate_paths(&toks)
        .into_iter()
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect()
}

/// 선행 sudo/env/`VAR=` 를 건너뛴 프로그램 토큰.
fn program_token<'a>(toks: &[&'a str]) -> Option<&'a str> {
    for &t in toks {
        if matches!(t, "sudo" | "doas" | "env" | "nohup" | "nice") {
            continue;
        }
        if t.contains('=') && !t.starts_with('/') && !t.starts_with('.') {
            continue;
        }
        return Some(t);
    }
    None
}

/// 플래그/숫자/옵션/리다이렉트 연산자를 제외한 경로 후보.
fn candidate_paths(toks: &[&str]) -> Vec<String> {
    let mut it = toks.iter().copied();
    for t in it.by_ref() {
        if matches!(t, "sudo" | "doas" | "env" | "nohup" | "nice") {
            continue;
        }
        if t.contains('=') && !t.starts_with('/') && !t.starts_with('.') {
            continue;
        }
        break;
    }
    it.filter(|t| {
        !t.starts_with('-')
            && !t.chars().all(|c| c.is_ascii_digit())
            && !t.contains('=')
            && !matches!(*t, ">" | ">>" | "|" | "&&" | ";")
    })
    .map(String::from)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockExecutor {
        calls: RefCell<u32>,
        산출물: String,
        exit: i32,
    }
    impl MockExecutor {
        fn new(산출물: &str, exit: i32) -> Self {
            Self {
                calls: RefCell::new(0),
                산출물: output.into(),
                exit,
            }
        }
    }
    impl Executor for MockExecutor {
        fn run(&self, _command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
            *self.calls.borrow_mut() += 1;
            sink.write(&self.output);
            Ok(self.exit)
        }
    }

    struct Sink(String);
    impl OutputSink for Sink {
        fn write(&mut self, c: &str) {
            self.0.push_str(c);
        }
    }

    struct Yes;
    impl Confirmer for Yes {
        fn confirm(&mut self, _: &ConfirmRequest) -> bool {
            true
        }
    }
    struct No;
    impl Confirmer for No {
        fn confirm(&mut self, _: &ConfirmRequest) -> bool {
            false
        }
    }

    fn tmp(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ai_pipe_{}_{}_{}", std::process::id(), tag, n));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn cfg<'a>(profile: &'a PolicyProfile, undo: &'a Path) -> ExecConfig<'a> {
        ExecConfig {
            profile,
            undo_dir: undo,
            limits: UndoLimits::defaults(),
        }
    }

    #[test]
    fn allow_command_runs_without_confirm() {
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("hi\n", 0);
        let mut sink = Sink(String::new());
        let mut conf = No; // Allow는 확인을 호출하지 않으므로 No여도 실행된다
        let out = execute("ls -al", &cfg(&prof, &undo), &exec, &mut conf, &mut sink).unwrap();
        assert_eq!(
            out,
            ExecOutcome::Ran {
                exit_code: 0,
                undo_id: None
            }
        );
        assert_eq!(*exec.calls.borrow(), 1);
        assert_eq!(sink.0, "hi\n");
    }
}
```

- [ ] **단계 3: 테스트가 통과(green)하는지 확인한다**

실행: `cargo test pipeline::`
기대: PASS (`allow_command_runs_without_confirm`).
주의: 이 단계에서 `backup_targets`/`program_token`/`candidate_paths`는 아직 미사용이라 dead_code 경고가 날 수 있으나 작업 2~4에서 사용된다. clippy는 작업 4 종료 후 실행.

- [ ] **단계 4: fmt 적용 후 커밋**

실행: `cargo fmt --all`
```bash
git add src/lib.rs src/pipeline.rs
git commit -m "feat(pipeline): scaffold execution pipeline core (Allow path)"
```

---

## 작업 2: 정책 차단(Block) 게이트

**Files:**
- Modify: `src/pipeline.rs` (`execute` 본문, tests)

- [ ] **단계 1: 실패 테스트를 추가한다**

`src/pipeline.rs`의 `mod tests`에 추가:

```rust
    #[test]
    fn critical_command_is_blocked() {
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 0);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let out = execute("rm -rf /", &cfg(&prof, &undo), &exec, &mut conf, &mut sink).unwrap();
        assert!(matches!(out, ExecOutcome::Blocked { .. }), "{out:?}");
        assert_eq!(*exec.calls.borrow(), 0, "blocked must not execute");
    }
```

- [ ] **단계 2: 실패를 확인한다**

실행: `cargo test pipeline::tests::critical_command_is_blocked`
기대: FAIL (현재 `execute`는 무조건 Ran 반환 → executor가 1회 호출되어 assert 실패).

- [ ] **단계 3: `execute`에 위험도·정책 평가와 Block 분기를 추가한다**

`execute` 본문을 다음으로 교체:

```rust
pub fn execute(
    command: &str,
    cfg: &ExecConfig,
    executor: &dyn Executor,
    confirmer: &mut dyn Confirmer,
    sink: &mut dyn OutputSink,
) -> anyhow::Result<ExecOutcome> {
    let _ = confirmer; // 작업 3에서 사용
    let assessment = risk::assess(command);
    let decision = cfg.profile.decide(assessment.level);
    let factors: Vec<String> = assessment
        .factors
        .iter()
        .map(|f| format!("{} ({:+})", f.label, f.delta))
        .collect();

    if decision == Decision::Block {
        return Ok(ExecOutcome::Blocked {
            level: assessment.level,
            factors,
        });
    }

    let exit_code = executor.run(command, sink)?;
    Ok(ExecOutcome::Ran {
        exit_code,
        undo_id: None,
    })
}
```

- [ ] **단계 4: 통과를 확인한다**

실행: `cargo test pipeline::`
기대: PASS (`allow_command_runs_without_confirm`, `critical_command_is_blocked`).

- [ ] **단계 5: fmt 후 커밋**

실행: `cargo fmt --all`
```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): block commands the policy refuses"
```

---

## 작업 3: 확인(Confirm/StrongConfirm) 게이트

**Files:**
- Modify: `src/pipeline.rs` (`execute` 본문, tests)

- [ ] **단계 1: 실패 테스트 2개를 추가한다**

`mod tests`에 추가:

```rust
    #[test]
    fn high_command_declined_when_confirmer_says_no() {
        // `sudo systemctl restart` → High → balanced: StrongConfirm
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 0);
        let mut sink = Sink(String::new());
        let mut conf = No;
        let out = execute(
            "sudo systemctl restart nginx",
            &cfg(&prof, &undo),
            &exec,
            &mut conf,
            &mut sink,
        )
        .unwrap();
        assert_eq!(out, ExecOutcome::Declined);
        assert_eq!(*exec.calls.borrow(), 0);
    }

    #[test]
    fn high_command_runs_when_confirmed() {
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("done\n", 0);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let out = execute(
            "sudo systemctl restart nginx",
            &cfg(&prof, &undo),
            &exec,
            &mut conf,
            &mut sink,
        )
        .unwrap();
        assert!(
            matches!(out, ExecOutcome::Ran { exit_code: 0, .. }),
            "{out:?}"
        );
        assert_eq!(*exec.calls.borrow(), 1);
    }
```

- [ ] **단계 2: 실패를 확인한다**

실행: `cargo test pipeline::tests::high_command_declined_when_confirmer_says_no`
기대: FAIL (현재 확인 게이트가 없어 executor가 호출됨 → Declined 아님).

- [ ] **단계 3: `execute`에 preview·확인 게이트를 추가한다**

`execute`의 Block 분기와 `executor.run` 호출 **사이**에 삽입:

```rust
    let plan = preview::classify_preview(command);
    let targets = backup_targets(command);
    let backup_files: Vec<String> = targets.iter().map(|p| p.display().to_string()).collect();

    if matches!(decision, Decision::Confirm | Decision::StrongConfirm) {
        let req = ConfirmRequest {
            command: command.to_string(),
            level: assessment.level,
            decision,
            factors: factors.clone(),
            preview: plan,
            backup_files: backup_files.clone(),
        };
        if !confirmer.confirm(&req) {
            return Ok(ExecOutcome::Declined);
        }
    }
```

그리고 `execute` 첫 줄의 `let _ = confirmer;` 를 삭제한다. (`targets`/`backup_files`/`plan`은 작업 4에서 백업에 쓰인다 — 지금은 `let _ = &targets;` 같은 임시 억제 없이 두면 unused 경고가 날 수 있으므로, 작업 4를 곧바로 이어서 진행한다. 경고만 발생하고 컴파일·테스트는 통과한다.)

- [ ] **단계 4: 통과를 확인한다**

실행: `cargo test pipeline::`
기대: PASS (4개 테스트).

- [ ] **단계 5: fmt 후 커밋**

실행: `cargo fmt --all`
```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): confirm gate for Confirm/StrongConfirm commands"
```

---

## 작업 4: undo 백업 자동 트리거(W10) + 종료코드 전파

**Files:**
- Modify: `src/pipeline.rs` (`execute` 본문, tests)

- [ ] **단계 1: 실패 테스트 3개를 추가한다**

`mod tests`에 추가:

```rust
    #[test]
    fn deletion_backs_up_existing_file_before_running() {
        let prof = PolicyProfile::balanced();
        let work = tmp("w");
        std::fs::create_dir_all(&work).unwrap();
        let f = work.join("data.txt");
        std::fs::write(&f, "original").unwrap();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 0);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let cmd = format!("rm {}", f.display());
        let out = execute(&cmd, &cfg(&prof, &undo), &exec, &mut conf, &mut sink).unwrap();
        let id = match out {
            ExecOutcome::Ran {
                undo_id: Some(id), ..
            } => id,
            other => panic!("expected Ran with undo_id, got {other:?}"),
        };
        // 백업으로 복구 가능해야 한다
        std::fs::write(&f, "changed").unwrap();
        undo::restore(&undo, &id).unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "original");
    }

    #[test]
    fn backup_refused_aborts_execution() {
        let prof = PolicyProfile::balanced();
        let work = tmp("w");
        std::fs::create_dir_all(&work).unwrap();
        let f = work.join("data.txt");
        std::fs::write(&f, vec![0u8; 1024]).unwrap();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 0);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let limits = UndoLimits {
            max_file_size_mb: 0,
            ..UndoLimits::defaults()
        };
        let config = ExecConfig {
            profile: &prof,
            undo_dir: &undo,
            limits,
        };
        let cmd = format!("rm {}", f.display());
        let out = execute(&cmd, &config, &exec, &mut conf, &mut sink).unwrap();
        assert!(matches!(out, ExecOutcome::BackupRefused(_)), "{out:?}");
        assert_eq!(*exec.calls.borrow(), 0, "refused backup must not execute");
    }

    #[test]
    fn exit_code_is_propagated() {
        let prof = PolicyProfile::balanced();
        let undo = tmp("u");
        let exec = MockExecutor::new("", 3);
        let mut sink = Sink(String::new());
        let mut conf = Yes;
        let out = execute("ls", &cfg(&prof, &undo), &exec, &mut conf, &mut sink).unwrap();
        assert_eq!(
            out,
            ExecOutcome::Ran {
                exit_code: 3,
                undo_id: None
            }
        );
    }
```

- [ ] **단계 2: 실패를 확인한다**

실행: `cargo test pipeline::tests::deletion_backs_up_existing_file_before_running`
기대: FAIL (현재 `undo_id`가 항상 None → `expected Ran with undo_id` panic).

- [ ] **단계 3: `execute`에 백업 트리거를 추가한다**

확인 게이트 블록과 `executor.run` 호출 **사이**에 삽입:

```rust
    let mut undo_id = None;
    if !targets.is_empty() {
        match undo::create_backup(cfg.undo_dir, &targets, &cfg.limits)? {
            BackupOutcome::Created(id) => undo_id = Some(id),
            BackupOutcome::Refused(reason) => return Ok(ExecOutcome::BackupRefused(reason)),
        }
    }
```

그리고 마지막 `Ok(ExecOutcome::Ran { exit_code, undo_id: None })` 를 다음으로 교체:

```rust
    let exit_code = executor.run(command, sink)?;
    Ok(ExecOutcome::Ran { exit_code, undo_id })
```

- [ ] **단계 4: 통과를 확인한다**

실행: `cargo test pipeline::`
기대: PASS (7개 테스트).

- [ ] **단계 5: clippy + fmt clean 확인**

실행: `cargo clippy --all-targets -- -D warnings`
기대: 경고 0 (backup_targets/program_token/candidate_paths가 모두 사용됨).
실행: `cargo clippy --all-targets --features storage -- -D warnings`
기대: 경고 0.
실행: `cargo fmt --all -- --check`
기대: 차이 없음.

- [ ] **단계 6: 커밋**

```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): auto-backup destructive targets before exec (W10)"
```

---

## 작업 5: `ai exec` CLI 커맨드

**Files:**
- Modify: `src/main.rs` (Command enum, dispatch match, 헬퍼 struct, `run_exec`, `record_exec`, tests)

- [ ] **단계 1: 실패할 파싱 테스트를 추가한다**

`src/main.rs`의 `#[cfg(test)] mod tests`에 추가:

```rust
        #[test]
        fn parses_exec_command() {
            let cli = Cli::parse_from(["ai", "exec", "rm -rf build", "--yes"]);
            match cli.command {
                Some(Command::Exec {
                    command,
                    yes,
                    profile,
                }) => {
                    assert_eq!(command, "rm -rf build");
                    assert!(yes);
                    assert!(profile.is_none());
                }
                other => panic!("expected Exec, got {other:?}"),
            }
        }
```

- [ ] **단계 2: 실패(컴파일 에러)를 확인한다**

실행: `cargo test parses_exec_command`
기대: FAIL — `Command::Exec` 변형이 없어 컴파일 에러.

- [ ] **단계 3: Command enum에 `Exec` 변형을 추가한다**

`src/main.rs`의 `enum Command { ... }` 안(예: `Route` 변형 근처)에 추가:

```rust
    /// 셸 명령을 게이트(위험도·정책·preview·백업)를 거쳐 실행한다 (그룹 C `ai exec`).
    Exec {
        /// 실행할 명령 문자열. 예: `ai exec "rm -rf build"`
        command: String,
        /// 확인 프롬프트 없이 자동 승인(Block은 우회 불가).
        #[arg(long)]
        yes: bool,
        /// 정책 프로파일(미지정 시 활성 프로파일).
        #[arg(long)]
        profile: Option<String>,
    },
```

- [ ] **단계 4: 파싱 테스트 통과를 확인한다**

실행: `cargo test parses_exec_command`
기대: PASS.

- [ ] **단계 5: 핸들러·주입 구현체·기록을 추가한다**

`src/main.rs`의 dispatch `match`(다른 `Some(Command::...)` 들 사이)에 추가:

```rust
        Some(Command::Exec {
            command,
            yes,
            profile,
        }) => run_exec(&command, yes, profile),
```

그리고 `fn main` **밖**(파일 하단 헬퍼 영역)에 추가:

```rust
struct StdoutSink;
impl ai_terminal::pipeline::OutputSink for StdoutSink {
    fn write(&mut self, chunk: &str) {
        print!("{chunk}");
    }
}

struct AutoYes;
impl ai_terminal::pipeline::Confirmer for AutoYes {
    fn confirm(&mut self, _: &ai_terminal::pipeline::ConfirmRequest) -> bool {
        true
    }
}

struct StdinConfirmer;
impl ai_terminal::pipeline::Confirmer for StdinConfirmer {
    fn confirm(&mut self, req: &ai_terminal::pipeline::ConfirmRequest) -> bool {
        use std::io::Write;
        eprintln!("위험 등급 {:?} 명령: {}", req.level, req.command);
        for f in &req.factors {
            eprintln!("  - {f}");
        }
        if !req.backup_files.is_empty() {
            eprintln!("  백업 대상: {}", req.backup_files.join(", "));
        }
        eprint!("실행할까요? [y/N] ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            return false;
        }
        matches!(line.trim(), "y" | "Y" | "yes")
    }
}

fn run_exec(command: &str, yes: bool, profile: Option<String>) -> anyhow::Result<()> {
    use ai_terminal::pipeline::{self, ExecConfig, ExecOutcome};

    let prof = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let undo_dir = undo::default_undo_dir()?;
    let cfg = ExecConfig {
        profile: &prof,
        undo_dir: &undo_dir,
        limits: undo::UndoLimits::defaults(),
    };
    let executor = pipeline::PtyExecutor { shell };
    let mut sink = StdoutSink;
    let mut confirmer: Box<dyn pipeline::Confirmer> = if yes {
        Box::new(AutoYes)
    } else {
        Box::new(StdinConfirmer)
    };

    let outcome = pipeline::execute(command, &cfg, &executor, confirmer.as_mut(), &mut sink)?;
    flush_stdout();
    match outcome {
        ExecOutcome::Blocked { level, factors } => {
            eprintln!("차단됨: 위험 등급 {level:?} (정책상 실행 불가)");
            for f in &factors {
                eprintln!("  - {f}");
            }
            std::process::exit(1);
        }
        ExecOutcome::Declined => {
            eprintln!("실행을 취소했습니다.");
            std::process::exit(1);
        }
        ExecOutcome::BackupRefused(r) => {
            eprintln!("백업 거부로 실행 중단: {r}");
            std::process::exit(1);
        }
        ExecOutcome::Ran { exit_code, undo_id } => {
            if let Some(id) = undo_id {
                eprintln!("(백업 생성: {id} — 되돌리려면 `ai undo last`)");
            }
            record_exec(command, exit_code);
            std::process::exit(exit_code);
        }
    }
}

fn flush_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

#[cfg(feature = "storage")]
fn record_exec(command: &str, exit_code: i32) {
    use ai_terminal::store::{NewCommand, NewSession, Store};
    let Ok(store) = Store::open_default() else {
        return;
    };
    let a = risk::assess(command);
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .ok();
    let _ = store.get_or_create_session(
        "sess-default",
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: cwd.clone().unwrap_or_default(),
            policy_profile: config::get_active_profile(),
        },
    );
    let _ = store.record_command(&NewCommand {
        session_id: "sess-default".into(),
        command_text: command.into(),
        source: "exec".into(),
        cwd,
        exit_code: Some(exit_code as i64),
        risk_level: Some(format!("{:?}", a.level)),
        risk_score: Some(a.score as i64),
        ai_generated: false,
        confirmed: true,
    });
    let _ = store.record_audit(
        "command_executed",
        Some(&format!("{:?}", a.level)),
        Some(&config::get_active_profile()),
        &format!("{{\"exit\":{exit_code}}}"),
    );
}

#[cfg(not(feature = "storage"))]
fn record_exec(_command: &str, _exit_code: i32) {}
```

- [ ] **단계 6: 전체 빌드·테스트·lint를 확인한다**

실행: `cargo test`
기대: PASS (기존 + `parses_exec_command`).
실행: `cargo test --features storage`
기대: PASS.
실행: `cargo clippy --all-targets -- -D warnings` 및 `cargo clippy --all-targets --features storage -- -D warnings`
기대: 경고 0.
실행: `cargo fmt --all -- --check`
기대: 차이 없음.

- [ ] **단계 7: 커밋**

```bash
git add src/main.rs
git commit -m "feat(cli): add `ai exec` running commands through the pipeline"
```

---

## 작업 6: TUI Enter 경로 재배선

**Files:**
- Modify: `src/ui.rs` (`run` 함수, 보조 struct)

- [ ] **단계 1: 보조 구현체를 모듈 상단에 추가한다**

`src/ui.rs`의 `pub fn run` **위**(파일 상단 모듈 스코프)에 추가:

```rust
/// TUI 출력 싱크: pipeline 출력을 문자열로 모은다.
struct StringSink(String);
impl crate::pipeline::OutputSink for StringSink {
    fn write(&mut self, c: &str) {
        self.0.push_str(c);
    }
}

/// TUI 확인기: 이번 증분은 확인이 필요한(위험) 명령을 거부하고 안내한다.
/// Allow 등급 명령은 pipeline이 확인을 호출하지 않으므로 그대로 실행된다.
struct TuiDeny;
impl crate::pipeline::Confirmer for TuiDeny {
    fn confirm(&mut self, _: &crate::pipeline::ConfirmRequest) -> bool {
        false
    }
}
```

- [ ] **단계 2: Enter(Submit) 처리부를 pipeline 호출로 교체한다**

`src/ui.rs:156-163`의 `Action::Submit(cmd) ...` arm 전체를 다음으로 교체:

```rust
                    Action::Submit(cmd) if !cmd.trim().is_empty() => {
                        // 제출된 명령을 중앙 실행 파이프라인(위험도·정책·백업·실행)으로 보낸다.
                        let prof = crate::policy::PolicyProfile::by_name(profile)
                            .unwrap_or_else(crate::policy::PolicyProfile::balanced);
                        let mut buf = StringSink(String::new());
                        let mut confirm = TuiDeny;
                        let executor = crate::pipeline::PtyExecutor {
                            shell: shell.clone(),
                        };
                        let msg = match crate::undo::default_undo_dir() {
                            Ok(dir) => {
                                let cfg = crate::pipeline::ExecConfig {
                                    profile: &prof,
                                    undo_dir: &dir,
                                    limits: crate::undo::UndoLimits::defaults(),
                                };
                                match crate::pipeline::execute(
                                    &cmd, &cfg, &executor, &mut confirm, &mut buf,
                                ) {
                                    Ok(crate::pipeline::ExecOutcome::Ran { exit_code, .. }) => {
                                        if exit_code != 0 {
                                            buf.0.push_str(&format!("[exit {exit_code}]\n"));
                                        }
                                        buf.0
                                    }
                                    Ok(crate::pipeline::ExecOutcome::Blocked { level, .. }) => {
                                        format!("[차단됨: 위험 등급 {level:?} — 정책상 실행 불가]\n")
                                    }
                                    Ok(crate::pipeline::ExecOutcome::Declined) => {
                                        "[위험 명령 — 터미널에서 `ai exec --yes`로 실행하세요]\n"
                                            .to_string()
                                    }
                                    Ok(crate::pipeline::ExecOutcome::BackupRefused(r)) => {
                                        format!("[백업 거부로 실행 중단: {r}]\n")
                                    }
                                    Err(e) => format!("error: {e}\n"),
                                }
                            }
                            Err(e) => format!("error: undo 디렉터리: {e}\n"),
                        };
                        state.append_output(&msg);
                    }
```

참고: 기존 `let shell = std::env::var("SHELL")...`(142행)는 그대로 두고 `shell.clone()`으로 사용한다. 이 변경으로 `crate::pty::run_in_pty` 직접 호출은 제거된다(pipeline 내부 `PtyExecutor`가 대체).

- [ ] **단계 3: 빌드·테스트·lint를 확인한다**

실행: `cargo test`
기대: PASS (기존 ui 테스트 `append_output_adds_lines_to_history` 등 유지).
실행: `cargo test --features storage`
기대: PASS.
실행: `cargo clippy --all-targets -- -D warnings` 및 `--features storage`
기대: 경고 0.
실행: `cargo fmt --all -- --check`
기대: 차이 없음.

- [ ] **단계 4: 커밋**

```bash
git add src/ui.rs
git commit -m "feat(tui): route Enter through the execution pipeline (gates + backup)"
```

---

## 작업 7: WSL e2e 검증 + 문서 갱신

**Files:**
- Modify: `docs/TASK.md` (W2·W10 항목), `docs/HISTORY.md` (신규 항목)

- [ ] **단계 1: WSL에서 `ai exec` 라운드트립을 검증한다**

다음을 단일 라인으로 실행(스크립트 파일 권장 — 멀티라인 금지):

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build --features storage && D=$(mktemp -d) && echo orig > $D/a.txt && SHELL=/bin/bash ./'"$CARGO_TARGET_DIR"'/debug/ai exec "rm '"$D"'/a.txt" --yes; ls $D'
```

기대: `a.txt` 삭제됨 + stderr에 `(백업 생성: undo_... )` 표시. 이어서 `ai undo last`로 복구되는지 확인:

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; ./'"$CARGO_TARGET_DIR"'/debug/ai undo last; cat $D/a.txt 2>/dev/null'
```

기대: 복구 안내 출력. (주의: 셸 변수 `$D`는 호출 간 유지되지 않으므로 실제로는 한 줄로 묶어 검증하거나 고정 경로 사용. 핵심은 "삭제 전 백업 → undo 복구" 동작 확인.)

- [ ] **단계 2: Block/Declined 경로를 확인한다**

```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; ./'"$CARGO_TARGET_DIR"'/debug/ai exec "rm -rf /" --yes; echo "exit=$?"'
```

기대: `차단됨: 위험 등급 Critical ...` + `exit=1` (실행되지 않음).

- [ ] **단계 3: `docs/TASK.md`를 갱신한다**

W2 항목(33행 부근)의 미완 줄을 갱신:

```markdown
- [x] 중앙 실행 파이프라인 연결: `ai exec` + TUI가 위험도·정책·preview·백업 게이트를 거쳐 실행(`src/pipeline.rs`). 비동기 출력 스트리밍/backpressure는 Executor 트레이트 뒤로 분리해 후속
```

W10 항목(95행)의 미완 줄을 갱신:

```markdown
- [x] 명령 실행 파이프라인에 백업 자동 트리거 연결(`pipeline::execute` → 삭제/덮어쓰기 시 `undo::create_backup`, Refused 시 실행 중단)
```

- [ ] **단계 4: `docs/HISTORY.md`에 항목을 추가한다**

최상단(`---` 다음, 가장 최근 항목 위)에 추가:

```markdown
## 2026-06-03 — 그룹 C: 중앙 실행 파이프라인 (W10/W11/W2 키스톤)

- **pipeline**(`pipeline.rs`, 신규): `execute`가 위험도→정책(Block/Confirm)→preview→undo 백업(W10 자동 트리거, Refused 시 실행 중단)→실행→결과를 묶는다. I/O는 `Executor`/`Confirmer`/`OutputSink` 트레이트로 주입(PTY 없이 단위 테스트). `PtyExecutor`가 `run_in_pty` 래핑 — 청크 sink 모양이 W2 스트리밍을 수용(후속에 impl 교체).
- **CLI**(`main.rs`): `ai exec "<cmd>" [--yes] [--profile]` — stdin y/N 확인(`--yes`로 생략, Block은 우회 불가), 종료코드 전파. storage 시 명령+종료코드+audit 기록.
- **TUI**(`ui.rs`): Enter가 `run_in_pty` 직접 호출 대신 `pipeline::execute`를 거친다. 이번 증분은 위험(확인 필요) 명령을 거부+안내, Allow 명령은 실행.
- **백업 범위**: 삭제(rm/unlink/shred)·덮어쓰기/in-place(sed -i, `>`, cp/mv/tee/touch)의 기존 일반 파일만. 권한 변경(chmod/chown)은 내용 백업 무의미로 제외(한계 고지). W11은 셸 경로 토큰비용 없음 → AI 경로 기존 기록 재사용.
- 검증: TDD(pipeline 7: Allow/Block/Declined/Confirmed/백업생성/백업거부중단/종료코드), `ai exec` WSL e2e(rm 백업→undo 복구, `rm -rf /` 차단 exit 1). storage/default 통과, clippy(default+storage)·fmt clean.
- **후속**: W2 실제 async 스트리밍, W9 실제 diff, Shell/Ai 단일 dispatcher 통합, TUI 인라인 확인 모달.
```

- [ ] **단계 5: 문서 커밋**

```bash
git add docs/TASK.md docs/HISTORY.md
git commit -m "docs: record central execution pipeline (group C keystone)"
```

---

## Finalization (실행 종료 후, TDD 루프 밖)

- **master→main 원격 정렬**(사용자 결정): 구현을 push할 때 `git push -u origin main`, GitHub 기본 브랜치를 main으로 변경(`gh repo edit --default-branch main`), 원격 `master` 삭제(`git push origin --delete master`). 열린 PR 없음 확인 후 진행.
- PR 생성 시 CI(`push: [main]` + PR)가 fmt/clippy/test/audit를 default·storage·tls로 검증.

## 자기검토 메모

- **스펙 커버리지**: 단계 순서(§4)=작업 2~4, 백업 범위(§4-4)=작업 4 `backup_targets`, 진입 표면(§6)=작업 5(CLI)·작업 6(TUI), 결과 타입(§5)=작업 1, 테스트(§7)=작업 1~4 + 작업 7 e2e. W11 위치(셸 무비용)=설계·HISTORY에 명시.
- **타입 일관성**: `ExecConfig`/`ExecOutcome`/`ConfirmRequest`/`execute` 시그니처가 작업 1 정의와 5·6 호출에서 동일. `record_command`/`record_audit`/`create_backup`/`default_undo_dir` 시그니처는 기존 소스와 대조 완료.
- **플레이스홀더**: 없음(모든 코드 블록 실제 내용).
