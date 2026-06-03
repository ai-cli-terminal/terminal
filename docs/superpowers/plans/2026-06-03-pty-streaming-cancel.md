# W2 PTY 출력 스트리밍 + Ctrl+C 중단 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `PtyExecutor`가 PTY 출력을 청크 단위로 즉시 `OutputSink`에 스트리밍하고, CLI에서 Ctrl+C로 실행 중인 자식을 중단한다.

**Architecture:** `pty.rs`에 `run_in_pty_streaming`(리더 스레드 → bounded `tokio::mpsc` → current-thread 런타임의 `select!{ recv, ctrl_c }` 루프, 취소 시 child kill·exit 130)을 추가하고, `PtyExecutor::run`을 이 함수 호출로 제자리 교체한다. 트레이트 시그니처 불변이라 `ai exec`/`ai dispatch`/TUI 3경로가 자동 적용된다.

**Tech Stack:** Rust, portable-pty, tokio(`mpsc`/`signal`/current-thread 런타임 — 기존 dep).

설계 정본: `docs/superpowers/specs/2026-06-03-pty-streaming-cancel-design.md`

빌드/검증(WSL): `wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
**주의: 이 하니스에서 `$?`는 항상 0(측정 불가). 종료코드/성공은 `cmd && echo OK || echo FAIL` 또는 cargo `test result:` 텍스트로 판정.**

---

### Task 1: `run_in_pty_streaming` (`src/pty.rs`)

**Files:** Modify `src/pty.rs`(함수 추가 + `#[cfg(all(test, unix))]` 테스트)

- [ ] **Step 1: 실패 테스트 작성** — `src/pty.rs`의 `mod tests`에 추가:

```rust
    #[test]
    fn streaming_accumulates_full_output() {
        let mut acc = String::new();
        let code =
            run_in_pty_streaming("/bin/bash", "printf 'one\\ntwo\\n'", |c| acc.push_str(c)).unwrap();
        assert!(acc.contains("one"), "{acc:?}");
        assert!(acc.contains("two"), "{acc:?}");
        assert_eq!(code, 0);
    }

    #[test]
    fn streaming_propagates_nonzero_exit() {
        let code = run_in_pty_streaming("/bin/bash", "exit 3", |_| {}).unwrap();
        assert_eq!(code, 3);
    }
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib pty 2>&1 | tail -15'`
Expected: 컴파일 에러 — `run_in_pty_streaming` 미정의.

- [ ] **Step 3: 구현** — `run_in_pty` 함수 정의 바로 아래(약 50행)에 추가:

```rust
/// PTY에서 `shell -c command`를 실행하며 출력을 청크 단위로 흘려보낸다.
/// Ctrl+C(SIGINT) 수신 시 자식을 kill하고 중단한다. 종료코드를 반환한다
/// (취소 시 130 = 128+SIGINT).
///
/// 리더 스레드가 블로킹 read로 청크를 bounded 채널(cap 64)에 보내고(소비가 느리면
/// `blocking_send`가 막혀 backpressure), current-thread 런타임이 채널 수신과 ctrl_c를
/// `select`한다. `child`는 select 종료 후 `wait`에서 다시 쓰므로 async 블록이 가변 차용만 한다.
pub fn run_in_pty_streaming(
    shell: &str,
    command: &str,
    mut on_chunk: impl FnMut(&str),
) -> anyhow::Result<i32> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = CommandBuilder::new(shell);
    cmd.arg("-c");
    cmd.arg(command);
    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader()?;
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
    let reader_thread = std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let cancelled = runtime.block_on(async {
        loop {
            tokio::select! {
                msg = rx.recv() => match msg {
                    Some(bytes) => on_chunk(&String::from_utf8_lossy(&bytes)),
                    None => break false,
                },
                _ = tokio::signal::ctrl_c() => {
                    let _ = child.kill();
                    break true;
                }
            }
        }
    });

    let status = child.wait()?;
    let _ = reader_thread.join();
    Ok(if cancelled {
        130
    } else {
        status.exit_code() as i32
    })
}
```

참고: `tokio`는 이미 dep이고 features에 `mpsc`(`sync`)·`signal`·`rt`·`macros`가 활성(`Cargo.toml` 확인). `&String::from_utf8_lossy(&bytes)`는 `&Cow<str>`→`&str` deref 강제로 `on_chunk(&str)`에 전달된다.

- [ ] **Step 4: 테스트 통과 확인**

Run: `wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib pty 2>&1 | grep -E "test result|error\["'`
Expected: `test result: ok` — 신규 2개 포함 모두 통과.
(만약 `child`/`on_chunk` 차용 관련 컴파일 에러가 나면: async 블록이 `child`/`on_chunk`를 move하려 한 것이므로, `let child = &mut child;`/`let on_chunk = &mut on_chunk;` 재차용 후 블록 안에서 그 참조를 쓰도록 최소 조정. 단 위 코드는 두 변수가 select 이후/내부에서 &mut로만 쓰여 통상 컴파일된다.)

- [ ] **Step 5: fmt + 커밋**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add src/pty.rs && git commit -m "feat(pty): streaming PTY execution with Ctrl+C cancel

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

### Task 2: `PtyExecutor::run` 스트리밍 교체 (`src/pipeline.rs`)

**Files:** Modify `src/pipeline.rs`(`PtyExecutor::run` 본문 + 인접 doc 코멘트)

- [ ] **Step 1: 구현 교체**

`src/pipeline.rs`의 `impl Executor for PtyExecutor`(약 71행) 본문을 교체:

기존:
```rust
impl Executor for PtyExecutor {
    fn run(&self, command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
        let out = crate::pty::run_in_pty(&self.shell, command)?;
        sink.write(&out.output);
        Ok(out.exit_code as i32)
    }
}
```
교체:
```rust
impl Executor for PtyExecutor {
    fn run(&self, command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
        crate::pty::run_in_pty_streaming(&self.shell, command, |c| sink.write(c))
    }
}
```

그리고 `Executor` 트레이트 doc(약 19행) `/// 실행 추상화(W2 스트리밍 심). 지금은 동기 PtyExecutor, 후속에 스트리밍 impl.`을 다음으로 갱신:
```rust
/// 실행 추상화(W2 스트리밍 심). `PtyExecutor`는 PTY 출력을 청크 단위로 sink에 스트리밍한다.
```

- [ ] **Step 2: 빌드·clippy·fmt·테스트(기본 + storage)**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3 && cargo test 2>&1 | grep -E "test result|error\["'
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets --features storage -- -D warnings 2>&1 | tail -3 && cargo test --features storage 2>&1 | grep -E "test result|error\["'
```
Expected: clippy/fmt clean, 모든 `test result: ok`(기존 pipeline 테스트는 `MockExecutor` 사용이라 무영향).

- [ ] **Step 3: 커밋**

```
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add src/pipeline.rs && git commit -m "feat(pipeline): stream PtyExecutor output via run_in_pty_streaming

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

### Task 3: WSL e2e 검증 + 문서 갱신

**Files:** Modify `docs/TASK.md`, `docs/HISTORY.md`

- [ ] **Step 1: e2e — 스트리밍 출력·종료코드 (제어흐름 판정)**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build 2>&1 | tail -1; BIN=$HOME/targets/ai-terminal/debug/ai; echo "--- output ---"; $BIN exec "printf one; printf two" 2>&1 | tr -d "\r"; echo; echo "--- exit propagation ---"; $BIN exec "exit 4" && echo "EXIT0" || echo "NONZERO"'
```
Expected: 출력에 `onetwo` 포함, 마지막 줄 `NONZERO`(exit 4 → 비0). (`ai exec`가 Allow 등급 명령은 확인 없이 실행.)

- [ ] **Step 2: e2e — Ctrl+C 중단(자식 즉시 종료)**

`sleep 30`을 백그라운드 `ai exec`로 띄우고 1초 뒤 SIGINT → 30초 안 기다리고 즉시 끝나야 한다. 4초 워치독으로 행 방지:

```
wsl.exe -- bash -c 'BIN=$HOME/targets/ai-terminal/debug/ai; $BIN exec "sleep 30" >/dev/null 2>&1 & PID=$!; sleep 1; kill -INT $PID; ( sleep 5; kill -9 $PID 2>/dev/null; echo "WATCHDOG_KILLED" ) & W=$!; wait $PID 2>/dev/null; echo "child-ended-promptly"; kill $W 2>/dev/null'
```
Expected: `child-ended-promptly`가 (워치독 5초 전에) 빠르게 출력 → SIGINT가 자식을 중단함. `WATCHDOG_KILLED`가 보이면 중단 실패이므로 STOP·보고. (exit 130 정확값은 `$?` 측정 불가라 행동으로 갈음.)

- [ ] **Step 3: `docs/TASK.md` 갱신**

`### W2 PTY Terminal Core` 아래 항목 중 다음 줄:
```
- [x] 중앙 실행 파이프라인 연결: `ai exec` + TUI가 위험도·정책·preview·백업 게이트를 거쳐 실행(`src/pipeline.rs`). 비동기 출력 스트리밍/backpressure는 Executor 트레이트 뒤로 분리해 후속
```
의 끝 문장을 갱신(스트리밍 완료 반영):
```
- [x] 중앙 실행 파이프라인 연결: `ai exec` + TUI가 위험도·정책·preview·백업 게이트를 거쳐 실행(`src/pipeline.rs`). **출력 스트리밍 완료(2026-06-03)**: `run_in_pty_streaming`(리더 스레드→bounded mpsc→ctrl_c select)로 청크 라이브 스트리밍 + CLI Ctrl+C 중단(exit 130). TUI mid-exec 중단은 후속.
```

- [ ] **Step 4: `docs/HISTORY.md` 엔트리 추가**

먼저 `docs/HISTORY.md` 최신 엔트리 형식을 확인한 뒤, 최상단(newest-first)에 동일 스타일로 추가:
```markdown
## 2026-06-03 — W2 PTY 출력 스트리밍 + Ctrl+C 중단

- **pty**(`pty.rs`): `run_in_pty_streaming` 추가 — 리더 스레드가 PTY를 블로킹 read해 bounded `tokio::mpsc`(cap 64)로 보내고(backpressure), current-thread 런타임이 `select!{ recv, ctrl_c }`로 청크를 `on_chunk`에 흘리며 Ctrl+C 시 자식 kill·exit 130. 기존 `run_in_pty`/`PtySession` 유지.
- **pipeline**(`pipeline.rs`): `PtyExecutor::run`을 `run_in_pty_streaming(..|c| sink.write(c))`로 제자리 교체 → `ai exec`/`ai dispatch`/TUI 3경로가 라이브 스트리밍·CLI 중단 자동 적용. 트레이트 시그니처 불변.
- 검증: pty 단위 테스트(스트리밍 누적·종료코드 전파), WSL e2e(printf 라이브 출력·exit 전파·`sleep` SIGINT 즉시 중단). clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-pty-streaming-cancel-design.md`, `docs/superpowers/plans/2026-06-03-pty-streaming-cancel.md`.
```
(HISTORY.md 실제 스타일에 맞게 보정 — `## YYYY-MM-DD — title` + `-` 불릿.)

- [ ] **Step 5: 커밋**

```
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add docs/TASK.md docs/HISTORY.md && git commit -m "docs: record W2 PTY output streaming and cancel

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

## 완료 기준 (DoD)

- `run_in_pty_streaming`이 PTY 출력을 청크로 `on_chunk`에 흘리고 정확한 종료코드를 반환(취소 130).
- `PtyExecutor::run`이 이 함수를 사용 → `ai exec`/`ai dispatch`/TUI 라이브 스트리밍.
- 단위 테스트: 스트리밍 누적·종료코드 전파.
- WSL e2e: 라이브 출력·exit 전파·SIGINT 즉시 중단.
- clippy/fmt clean, 기본 + storage 전체 테스트 PASS.
- 문서(TASK/HISTORY) 갱신. 비목표(TUI 중단, stdout/stderr 분리)는 미포함.
