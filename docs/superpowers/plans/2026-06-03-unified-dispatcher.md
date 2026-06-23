# Unified Shell/Ai Dispatcher 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**목표:** 입력을 분류해 Shell→`pipeline::execute` / Ai→AI 게이트웨이로 보내는 단일 오케스트레이터(`dispatch::run`)를 두고, TUI Submit과 신규 CLI `ai dispatch`가 이를 거치게 하여 TUI의 자연어 질의가 AI로 가도록 한다.

**Architecture:** 2레이어 — 순수 라우팅(`dispatch::dispatch`, 변경 없음) + 오케스트레이션(`dispatch::run`). I/O는 기존 `Executor`/`Confirmer`/`OutputSink`와 같은 결의 `AiResponder` 트레이트로 주입해 코어를 sync·테스트 가능하게 유지. 실제 AI는 `GatewayResponder`(lib 모듈)가 tokio 런타임을 `block_on`으로 감싼다.

**기술 스택:** Rust, anyhow, tokio(current-thread), 기존 모듈(`pipeline`/`gateway`/`aitask`/`context`/`intent`/`policy`).

**빌드·검증(WSL 단일라인 래퍼):**
```
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'
```
멀티라인 전달 금지(CRLF). `<cmd>`는 한 줄로.

---

## File Structure

- **Modify** `src/dispatch.rs` — `AiResponder`/`AiOutcome`/`Handled`/`Handlers`/`run` 추가 + 단위 테스트. (`crate::pipeline` 사용)
- **Create** `src/responder.rs` — `GatewayResponder`(실제 AI) + 순수 매핑 헬퍼 `finish` + 테스트.
- **Modify** `src/lib.rs` — `pub mod responder;` 등록.
- **Modify** `src/ui.rs` — Submit 경로를 `dispatch::run`으로 재배선 + 순수 렌더 헬퍼 `render` + 테스트.
- **Modify** `src/main.rs` — `Command::Dispatch` + `run_dispatch` 핸들러.
- **Modify** `docs/HISTORY.md`, `CHANGELOG.md`, `docs/TASK.md` — 기록.

---

## 작업 1: dispatch::run 오케스트레이터 + 트레이트/타입

**Files:**
- Modify: `src/dispatch.rs`

- [ ] **단계 1: 신규 타입·트레이트·`run` 추가**

`src/dispatch.rs` 상단 `use` 블록에 추가(기존 `use crate::intent...` 아래):

```rust
use crate::pipeline::{self, ExecConfig, ExecOutcome, OutputSink};
```

파일 끝의 `#[cfg(test)] mod tests` **앞**에 추가:

```rust
/// AI 핸들러 추상화(Executor/Confirmer/OutputSink와 같은 결의 심).
/// 컨텍스트(cwd 등)는 실제 구현이 내부에서 모은다.
pub trait AiResponder {
    fn respond(&mut self, prompt: &str, sink: &mut dyn OutputSink) -> anyhow::Result<AiOutcome>;
}

/// AI 응답 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiOutcome {
    /// 응답 성공(text는 sink에도 기록됨).
    Answered {
        text: String,
        input_tokens: usize,
        output_tokens: usize,
    },
    /// 마스킹 fail-closed 등으로 원격 전송 차단.
    Blocked(String),
    /// 장애·타임아웃·취소(§3-3: 셸을 막지 않음).
    Unavailable(String),
}

/// 통합 실행 결과.
#[derive(Debug, PartialEq, Eq)]
pub enum Handled {
    Empty,
    Shell(ExecOutcome),
    Ai(AiOutcome),
}

/// 주입 핸들러 묶음. sink는 셸/AI가 공유한다.
pub struct Handlers<'a> {
    pub executor: &'a dyn pipeline::Executor,
    pub confirmer: &'a mut dyn pipeline::Confirmer,
    pub ai: &'a mut dyn AiResponder,
    pub sink: &'a mut dyn OutputSink,
}

/// 입력을 분류해 셸 파이프라인 또는 AI 핸들러로 보낸다(설계 §3·§4).
pub fn run(
    input: &str,
    profile: &PolicyProfile,
    exec_cfg: &ExecConfig,
    h: &mut Handlers,
) -> anyhow::Result<Handled> {
    match dispatch(input, profile) {
        Route::Empty => Ok(Handled::Empty),
        Route::Shell { command, .. } => {
            let out = pipeline::execute(&command, exec_cfg, h.executor, h.confirmer, h.sink)?;
            Ok(Handled::Shell(out))
        }
        Route::Ai { prompt } => {
            let out = h.ai.respond(&prompt, h.sink)?;
            Ok(Handled::Ai(out))
        }
    }
}
```

- [ ] **단계 2: 단위 테스트 추가(실패 확인용)**

`src/dispatch.rs`의 `#[cfg(test)] mod tests` 안, 기존 `use super::*;` 아래에 테스트 더블과 테스트를 추가:

```rust
    use crate::undo::UndoLimits;
    use std::cell::RefCell;
    use std::path::PathBuf;

    struct CollectSink(String);
    impl OutputSink for CollectSink {
        fn write(&mut self, c: &str) {
            self.0.push_str(c);
        }
    }

    struct MockExec {
        out: String,
        exit: i32,
        calls: RefCell<u32>,
    }
    impl pipeline::Executor for MockExec {
        fn run(&self, _cmd: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
            *self.calls.borrow_mut() += 1;
            sink.write(&self.out);
            Ok(self.exit)
        }
    }

    struct YesConfirm;
    impl pipeline::Confirmer for YesConfirm {
        fn confirm(&mut self, _: &pipeline::ConfirmRequest) -> bool {
            true
        }
    }

    struct MockAi {
        answer: String,
        calls: RefCell<u32>,
    }
    impl AiResponder for MockAi {
        fn respond(&mut self, _prompt: &str, sink: &mut dyn OutputSink) -> anyhow::Result<AiOutcome> {
            *self.calls.borrow_mut() += 1;
            sink.write(&self.answer);
            Ok(AiOutcome::Answered {
                text: self.answer.clone(),
                input_tokens: 1,
                output_tokens: 2,
            })
        }
    }

    fn undo_tmp() -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("ai_disp_{}_{}", std::process::id(), n))
    }

    fn run_with(input: &str, exec: &MockExec, ai: &mut MockAi, sink: &mut CollectSink) -> Handled {
        let prof = PolicyProfile::balanced();
        let undo = undo_tmp();
        let cfg = ExecConfig {
            profile: &prof,
            undo_dir: &undo,
            limits: UndoLimits::defaults(),
        };
        let mut conf = YesConfirm;
        let mut h = Handlers {
            executor: exec,
            confirmer: &mut conf,
            ai,
            sink,
        };
        run(input, &prof, &cfg, &mut h).unwrap()
    }

    #[test]
    fn empty_routes_to_handled_empty() {
        let exec = MockExec { out: String::new(), exit: 0, calls: RefCell::new(0) };
        let mut ai = MockAi { answer: "x".into(), calls: RefCell::new(0) };
        let mut sink = CollectSink(String::new());
        assert_eq!(run_with("   ", &exec, &mut ai, &mut sink), Handled::Empty);
        assert_eq!(*exec.calls.borrow(), 0);
        assert_eq!(*ai.calls.borrow(), 0);
    }

    #[test]
    fn shell_command_runs_through_pipeline() {
        let exec = MockExec { out: "hi\n".into(), exit: 0, calls: RefCell::new(0) };
        let mut ai = MockAi { answer: "x".into(), calls: RefCell::new(0) };
        let mut sink = CollectSink(String::new());
        let out = run_with("ls -al", &exec, &mut ai, &mut sink);
        assert_eq!(out, Handled::Shell(ExecOutcome::Ran { exit_code: 0, undo_id: None }));
        assert_eq!(*exec.calls.borrow(), 1);
        assert_eq!(sink.0, "hi\n");
    }

    #[test]
    fn critical_shell_is_blocked_not_executed() {
        let exec = MockExec { out: String::new(), exit: 0, calls: RefCell::new(0) };
        let mut ai = MockAi { answer: "x".into(), calls: RefCell::new(0) };
        let mut sink = CollectSink(String::new());
        let out = run_with("rm -rf /", &exec, &mut ai, &mut sink);
        assert!(matches!(out, Handled::Shell(ExecOutcome::Blocked { .. })), "{out:?}");
        assert_eq!(*exec.calls.borrow(), 0);
    }

    #[test]
    fn natural_language_routes_to_ai() {
        let exec = MockExec { out: String::new(), exit: 0, calls: RefCell::new(0) };
        let mut ai = MockAi { answer: "answer-text".into(), calls: RefCell::new(0) };
        let mut sink = CollectSink(String::new());
        let out = run_with("how do I undo a commit?", &exec, &mut ai, &mut sink);
        assert_eq!(
            out,
            Handled::Ai(AiOutcome::Answered {
                text: "answer-text".into(),
                input_tokens: 1,
                output_tokens: 2,
            })
        );
        assert_eq!(*ai.calls.borrow(), 1);
        assert_eq!(*exec.calls.borrow(), 0);
        assert_eq!(sink.0, "answer-text");
    }

    #[test]
    fn ai_inline_routes_to_ai() {
        let exec = MockExec { out: String::new(), exit: 0, calls: RefCell::new(0) };
        let mut ai = MockAi { answer: "a".into(), calls: RefCell::new(0) };
        let mut sink = CollectSink(String::new());
        let out = run_with("ai explain last-error", &exec, &mut ai, &mut sink);
        assert!(matches!(out, Handled::Ai(AiOutcome::Answered { .. })), "{out:?}");
        assert_eq!(*ai.calls.borrow(), 1);
    }
```

- [ ] **단계 3: 컴파일·테스트 실패 확인**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib dispatch:: 2>&1 | tail -20'`
기대: 컴파일 통과 후 5개 테스트 PASS (단계 1 구현이 이미 포함되므로 통과). 만약 컴파일 에러면 메시지대로 수정.

> 참고: 이 작업은 구현(단계 1)과 테스트(단계 2)를 함께 추가하므로 곧장 PASS를 목표로 한다. TDD 순서를 엄격히 보려면 단계 1의 `run` 본문을 `todo!()`로 두고 단계 3에서 FAIL 확인 → 단계 1 본문 채우기 순으로 진행해도 된다.

- [ ] **단계 4: fmt·clippy**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --all-targets -- -D warnings 2>&1 | tail -15'`
기대: 한글 체인 호출은 fmt가 자동정렬. clippy 경고 0.

- [ ] **단계 5: 커밋**

```bash
git add src/dispatch.rs
git commit -m "feat(dispatch): add run orchestrator with AiResponder injection"
```

---

## 작업 2: GatewayResponder (실제 AI 구현)

**Files:**
- Create: `src/responder.rs`
- Modify: `src/lib.rs`

- [ ] **단계 1: `lib.rs`에 모듈 등록**

`src/lib.rs`의 `pub mod provider;` 와 `pub mod pty;` 사이(알파벳 순)에 추가:

```rust
pub mod responder;
```

- [ ] **단계 2: `src/responder.rs` 작성(헬퍼 + 구조체)**

```rust
//! 실제 AI 응답기(설계 §5). `dispatch::AiResponder`를 게이트웨이+런타임으로 구현한다.
//!
//! 동기 `block_on`으로 async 게이트웨이를 감싸 디스패처를 sync로 유지한다.
//! 실패·타임아웃·취소는 비치명적([`AiOutcome::Unavailable`])으로 흡수해 셸을 막지 않는다.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;

use crate::aitask::{self, RequestError, Timeouts};
use crate::context;
use crate::dispatch::{AiOutcome, AiResponder};
use crate::gateway::{Gateway, GatewayOutcome};
use crate::pipeline::OutputSink;

/// 게이트웨이 기반 AI 응답기.
pub struct GatewayResponder {
    gateway: Gateway,
    runtime: tokio::runtime::Runtime,
    timeout: Duration,
}

impl GatewayResponder {
    /// 주어진 게이트웨이·타임아웃으로 만든다(current-thread 런타임 1개 보유).
    pub fn new(gateway: Gateway, timeout: Duration) -> anyhow::Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        Ok(Self {
            gateway,
            runtime,
            timeout,
        })
    }

    /// mock(echo) 게이트웨이 + 기본 요청 타임아웃.
    pub fn mock() -> anyhow::Result<Self> {
        Self::new(Gateway::mock(), Timeouts::defaults().request)
    }
}

/// 게이트웨이 결과를 [`AiOutcome`]으로 매핑한다(성공 시 text를 sink로 흘림). 순수 함수.
fn finish(
    result: Result<GatewayOutcome, RequestError>,
    sink: &mut dyn OutputSink,
) -> AiOutcome {
    match result {
        Ok(GatewayOutcome::Answered {
            text,
            input_tokens,
            output_tokens,
        }) => {
            sink.write(&text);
            AiOutcome::Answered {
                text,
                input_tokens,
                output_tokens,
            }
        }
        Ok(GatewayOutcome::Blocked(reason)) => AiOutcome::Blocked(reason),
        Err(e) => AiOutcome::Unavailable(e.to_string()),
    }
}

impl AiResponder for GatewayResponder {
    fn respond(&mut self, prompt: &str, sink: &mut dyn OutputSink) -> anyhow::Result<AiOutcome> {
        let ctx = context::gather();
        let ctx_str = format!("cwd={}", ctx.cwd);
        let timeout = self.timeout;
        let gw = &self.gateway;
        let result = self.runtime.block_on(async {
            let cancel = Arc::new(Notify::new());
            aitask::cancel_on_ctrl_c(cancel.clone());
            gw.ask_cancellable(prompt, &ctx_str, timeout, cancel).await
        });
        Ok(finish(result, sink))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Sink(String);
    impl OutputSink for Sink {
        fn write(&mut self, c: &str) {
            self.0.push_str(c);
        }
    }

    #[test]
    fn finish_answered_writes_to_sink() {
        let mut sink = Sink(String::new());
        let out = finish(
            Ok(GatewayOutcome::Answered {
                text: "hello".into(),
                input_tokens: 3,
                output_tokens: 4,
            }),
            &mut sink,
        );
        assert_eq!(
            out,
            AiOutcome::Answered {
                text: "hello".into(),
                input_tokens: 3,
                output_tokens: 4
            }
        );
        assert_eq!(sink.0, "hello");
    }

    #[test]
    fn finish_blocked_maps_and_no_write() {
        let mut sink = Sink(String::new());
        let out = finish(Ok(GatewayOutcome::Blocked("masking".into())), &mut sink);
        assert_eq!(out, AiOutcome::Blocked("masking".into()));
        assert_eq!(sink.0, "");
    }

    #[test]
    fn finish_error_maps_to_unavailable() {
        let mut sink = Sink(String::new());
        let out = finish(Err(RequestError::Cancelled), &mut sink);
        assert!(matches!(out, AiOutcome::Unavailable(_)), "{out:?}");
        assert_eq!(sink.0, "");
    }

    #[test]
    fn mock_responder_answers_via_echo() {
        let mut r = GatewayResponder::mock().unwrap();
        let mut sink = Sink(String::new());
        let out = r.respond("ping", &mut sink).unwrap();
        // EchoBackend는 입력을 그대로 돌려주므로 Answered + sink에 내용이 있어야 한다.
        assert!(matches!(out, AiOutcome::Answered { .. }), "{out:?}");
        assert!(!sink.0.is_empty(), "echo answer should be written to sink");
    }
}
```

- [ ] **단계 3: 테스트 실행**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib responder:: 2>&1 | tail -20'`
기대: 4개 테스트 PASS. (`mock_responder_answers_via_echo`가 컨텍스트/런타임/echo 백엔드를 실제로 탄다.)

- [ ] **단계 4: fmt·clippy**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --all-targets -- -D warnings 2>&1 | tail -15'`
기대: 경고 0.

- [ ] **단계 5: 커밋**

```bash
git add src/responder.rs src/lib.rs
git commit -m "feat(responder): add GatewayResponder bridging async gateway to sync dispatcher"
```

---

## 작업 3: TUI Submit 재배선

**Files:**
- Modify: `src/ui.rs`

- [ ] **단계 1: 순수 렌더 헬퍼 추가**

`src/ui.rs`의 `pub fn run(profile: &str)` **앞**(예: `TuiDeny` impl 아래)에 추가:

```rust
/// 통합 실행 결과 + 누적 출력을 TUI 표시 문자열로 만든다(순수). `output`은 sink에
/// 누적된 셸/AI 출력.
fn render(handled: &crate::dispatch::Handled, 산출물: String) -> String {
    use crate::dispatch::{AiOutcome, Handled};
    use crate::pipeline::ExecOutcome;
    match handled {
        Handled::Empty => String::new(),
        Handled::Shell(ExecOutcome::Ran { exit_code, .. }) => {
            if *exit_code != 0 {
                format!("{output}[exit {exit_code}]\n")
            } else {
                output
            }
        }
        Handled::Shell(ExecOutcome::Blocked { level, .. }) => {
            format!("[차단됨: 위험 등급 {level:?} — 정책상 실행 불가]\n")
        }
        Handled::Shell(ExecOutcome::Declined) => {
            "[위험 명령 — 터미널에서 `ai exec --yes`로 실행하세요]\n".to_string()
        }
        Handled::Shell(ExecOutcome::BackupRefused(r)) => {
            format!("[백업 거부로 실행 중단: {r}]\n")
        }
        Handled::Ai(AiOutcome::Answered { .. }) => output,
        Handled::Ai(AiOutcome::Blocked(r)) => format!("{output}[차단: {r}]\n"),
        Handled::Ai(AiOutcome::Unavailable(e)) => format!("{output}[AI 사용 불가: {e}]\n"),
    }
}
```

- [ ] **단계 2: Submit 경로를 `dispatch::run`으로 교체**

`src/ui.rs`의 `Action::Submit(cmd) if !cmd.trim().is_empty() => { ... }` 블록 전체(현재 `let prof = ...` 부터 `state.append_output(&msg);` 직전까지)를 아래로 교체:

```rust
                    Action::Submit(cmd) if !cmd.trim().is_empty() => {
                        // 입력을 단일 디스패처로 보낸다(셸→pipeline / AI→gateway).
                        let prof = crate::policy::PolicyProfile::by_name(profile)
                            .unwrap_or_else(crate::policy::PolicyProfile::balanced);
                        let executor = crate::pipeline::PtyExecutor {
                            shell: shell.clone(),
                        };
                        let mut confirm = TuiDeny;
                        let mut ai: Box<dyn crate::dispatch::AiResponder> =
                            match crate::responder::GatewayResponder::mock() {
                                Ok(r) => Box::new(r),
                                Err(e) => {
                                    state.append_output(&format!("error: AI 런타임: {e}\n"));
                                    continue;
                                }
                            };
                        let mut buf = StringSink(String::new());
                        let msg = match crate::undo::default_undo_dir() {
                            Ok(dir) => {
                                let cfg = crate::pipeline::ExecConfig {
                                    profile: &prof,
                                    undo_dir: &dir,
                                    limits: crate::undo::UndoLimits::defaults(),
                                };
                                let mut h = crate::dispatch::Handlers {
                                    executor: &executor,
                                    confirmer: &mut confirm,
                                    ai: ai.as_mut(),
                                    sink: &mut buf,
                                };
                                match crate::dispatch::run(&cmd, &prof, &cfg, &mut h) {
                                    Ok(handled) => render(&handled, buf.0.clone()),
                                    Err(e) => format!("error: {e}\n"),
                                }
                            }
                            Err(e) => format!("error: undo 디렉터리: {e}\n"),
                        };
                        state.append_output(&msg);
                    }
```

> 주의: `continue`는 `loop` 안에서 동작한다(현재 Submit 처리가 `loop { match event::read() {...} }` 내부이므로 유효). `buf.0.clone()` — `render`가 String을 소유로 받기 때문(차용 충돌 회피).

- [ ] **단계 3: 렌더 헬퍼 테스트 추가**

`src/ui.rs`의 `#[cfg(test)] mod tests` 안에 추가:

```rust
    #[test]
    fn render_shell_ran_zero_passthrough() {
        use crate::dispatch::Handled;
        use crate::pipeline::ExecOutcome;
        let h = Handled::Shell(ExecOutcome::Ran { exit_code: 0, undo_id: None });
        assert_eq!(render(&h, "hi\n".into()), "hi\n");
    }

    #[test]
    fn render_shell_ran_nonzero_appends_exit() {
        use crate::dispatch::Handled;
        use crate::pipeline::ExecOutcome;
        let h = Handled::Shell(ExecOutcome::Ran { exit_code: 3, undo_id: None });
        assert_eq!(render(&h, "x".into()), "x[exit 3]\n");
    }

    #[test]
    fn render_ai_answered_passthrough() {
        use crate::dispatch::{AiOutcome, Handled};
        let h = Handled::Ai(AiOutcome::Answered {
            text: "ans".into(),
            input_tokens: 1,
            output_tokens: 1,
        });
        assert_eq!(render(&h, "ans".into()), "ans");
    }

    #[test]
    fn render_ai_unavailable_appends_note() {
        use crate::dispatch::{AiOutcome, Handled};
        let h = Handled::Ai(AiOutcome::Unavailable("timeout".into()));
        assert_eq!(render(&h, String::new()), "[AI 사용 불가: timeout]\n");
    }

    #[test]
    fn render_empty_is_blank() {
        use crate::dispatch::Handled;
        assert_eq!(render(&Handled::Empty, String::new()), "");
    }
```

- [ ] **단계 4: 테스트·fmt·clippy**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib ui:: 2>&1 | tail -20 && cargo fmt --all && cargo clippy --all-targets -- -D warnings 2>&1 | tail -15'`
기대: ui 렌더 테스트 5개 PASS, clippy 경고 0.

- [ ] **단계 5: 커밋**

```bash
git add src/ui.rs
git commit -m "feat(ui): route TUI submit through unified dispatcher (AI for queries)"
```

---

## 작업 4: CLI `ai dispatch` 명령

**Files:**
- Modify: `src/main.rs`

- [ ] **단계 1: 서브커맨드 추가**

`src/main.rs`의 `enum Command`에서 `Route { input: String }` 정의 **아래**에 추가:

```rust
    /// 입력을 분류해 셸 실행 또는 AI 응답으로 보낸다 (통합 디스패처, Phase 2).
    Dispatch {
        /// 분류·실행할 입력. 예: `ai dispatch "ls -al"` 또는 `ai dispatch "how do I list files?"`
        input: String,
        /// 셸 경로에서 확인 없이 자동 승인(Block은 우회 불가).
        #[arg(long)]
        yes: bool,
        /// 정책 프로파일(미지정 시 활성 프로파일).
        #[arg(long)]
        profile: Option<String>,
    },
```

- [ ] **단계 2: 매치 암 추가**

`src/main.rs`의 `Some(Command::Exec { ... }) => run_exec(...)` 암 **아래**에 추가:

```rust
        Some(Command::Dispatch {
            input,
            yes,
            profile,
        }) => run_dispatch(&input, yes, profile),
```

- [ ] **단계 3: `run_dispatch` 핸들러 작성**

`src/main.rs`의 `fn run_exec(...)` **아래**에 추가:

```rust
fn run_dispatch(input: &str, yes: bool, profile: Option<String>) -> anyhow::Result<()> {
    use ai_terminal::dispatch::{self, AiOutcome, Handled, Handlers};
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
    let mut confirmer: Box<dyn pipeline::Confirmer> = if yes {
        Box::new(AutoYes)
    } else {
        Box::new(StdinConfirmer)
    };
    let mut ai = ai_terminal::responder::GatewayResponder::mock()?;
    let mut sink = StdoutSink;

    let mut h = Handlers {
        executor: &executor,
        confirmer: confirmer.as_mut(),
        ai: &mut ai,
        sink: &mut sink,
    };
    let handled = dispatch::run(input, &prof, &cfg, &mut h)?;
    flush_stdout();

    match handled {
        Handled::Empty => Ok(()),
        Handled::Shell(ExecOutcome::Blocked { level, factors }) => {
            eprintln!("차단됨: 위험 등급 {level:?} (정책상 실행 불가)");
            for f in &factors {
                eprintln!("  - {f}");
            }
            std::process::exit(1);
        }
        Handled::Shell(ExecOutcome::Declined) => {
            eprintln!("실행을 취소했습니다.");
            std::process::exit(1);
        }
        Handled::Shell(ExecOutcome::BackupRefused(r)) => {
            eprintln!("백업 거부로 실행 중단: {r}");
            std::process::exit(1);
        }
        Handled::Shell(ExecOutcome::Ran { exit_code, undo_id }) => {
            if let Some(id) = undo_id {
                eprintln!("(백업 생성: {id} — 되돌리려면 `ai undo last`)");
            }
            record_exec(input, exit_code);
            std::process::exit(exit_code);
        }
        Handled::Ai(AiOutcome::Answered {
            input_tokens,
            output_tokens,
            ..
        }) => {
            // 답변 본문은 이미 sink(stdout)로 출력됨. 토큰 요약만 덧붙인다.
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
}
```

> `record_exec`는 `#[cfg(feature = "storage")]` 함수다. 기존 `run_exec`도 동일하게 호출하므로 동일한 cfg 환경에서 컴파일된다(스토리지 미사용 빌드에서 `record_exec`는 no-op 스텁이 존재해야 함 — 기존 `run_exec`가 이미 그렇게 호출하고 있으니 그대로 따른다).

- [ ] **단계 4: 빌드(전 feature)**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build 2>&1 | tail -15 && cargo build --features storage 2>&1 | tail -15'`
기대: 두 빌드 모두 성공. 만약 `record_exec`가 default 빌드에서 미정의면, 기존 `run_exec`가 어떻게 처리하는지 보고 동일 패턴(스텁/cfg) 적용.

- [ ] **단계 5: fmt·clippy·커밋**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --all-targets --features storage -- -D warnings 2>&1 | tail -15'`
기대: 경고 0.

```bash
git add src/main.rs
git commit -m "feat(cli): add 'ai dispatch' one-shot unified routing command"
```

---

## 작업 5: 전체 검증 + e2e + 문서

**Files:**
- Modify: `docs/HISTORY.md`, `CHANGELOG.md`, `docs/TASK.md`

- [ ] **단계 1: 전 feature 테스트**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test 2>&1 | tail -8 && cargo test --features storage 2>&1 | tail -8 && cargo test --features "storage tls" 2>&1 | tail -8'`
기대: 세 구성 모두 전부 PASS(기존 178/195 + 신규 ~14개).

- [ ] **단계 2: WSL e2e — 셸 경로**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo run -q -- dispatch "echo hello-from-dispatch"; echo "exit=$?"'`
기대: `hello-from-dispatch` 출력 + `exit=0`.

- [ ] **단계 3: WSL e2e — AI 경로(mock)**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo run -q -- dispatch "how do I list files?"; echo "exit=$?"'`
기대: mock(echo) AI 응답 본문 + `(tokens ~ in:.. out:..)` + `exit=0`(AI 경로는 셸을 막지 않으므로 정상 종료).

- [ ] **단계 4: WSL e2e — 위험 셸 차단**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo run -q -- dispatch "rm -rf /"; echo "exit=$?"'`
기대: `차단됨: 위험 등급 Critical ...` + `exit=1`.

- [ ] **단계 5: 문서 기록**

`CHANGELOG.md` 최상단 미릴리즈 섹션에 한 줄 추가:
```markdown
- 통합 디스패처: `dispatch::run`으로 셸/AI 경로 일원화. TUI 자연어 질의가 AI로 라우팅되고, CLI `ai dispatch "<input>"` 추가.
```

`docs/HISTORY.md`에 작업 요약 항목 추가(기존 형식 따름): 날짜 2026-06-03, "Shell/Ai 단일 디스패처 통합" — `dispatch::run`+`AiResponder`+`GatewayResponder`, TUI 재배선, `ai dispatch` 신규, 테스트 수 갱신.

`docs/TASK.md`에서 "Shell/Ai dispatcher 통합" 백로그 항목을 완료로 표시.

- [ ] **단계 6: 최종 fmt·clippy·커밋**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --all-targets --features "storage tls" -- -D warnings 2>&1 | tail -10'`
기대: 경고 0.

```bash
git add docs/HISTORY.md CHANGELOG.md docs/TASK.md
git commit -m "docs: record unified dispatcher integration"
```

---

## 완료 기준

- `dispatch::run`이 Empty/Shell/Ai를 올바르게 라우팅(단위 테스트 5).
- `GatewayResponder`가 게이트웨이 결과를 AiOutcome으로 매핑(단위 테스트 4).
- TUI Submit이 디스패처를 거치고 자연어 질의가 AI(mock)로 감(렌더 테스트 5 + e2e).
- `ai dispatch`가 셸/AI/차단을 올바른 종료코드로 처리(e2e 3).
- 전 feature(`default`/`storage`/`storage tls`) 테스트·clippy·fmt green.
- `ai exec`/`ai ask`/`run_exec`/`dispatch::dispatch`/`ai route` 동작 불변.
