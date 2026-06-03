# 설계: W2 실제 출력 스트리밍 + Ctrl+C 중단

> 날짜: 2026-06-03 · 핸드오프 백로그 ③ 중 W2 · 관련: W2 PTY Terminal Core, 중앙 실행 파이프라인

## 문제

`PtyExecutor::run`(`src/pipeline.rs`)은 `crate::pty::run_in_pty`를 호출한다 —
`reader.read_to_end`로 **명령 종료까지 블록**한 뒤 전체 출력을 한 번에 `sink.write`. 결과:
출력이 실시간으로 안 보이고(긴 명령일수록 체감 큼), 실행 중 중단도 불가.

W2 목표: PTY 출력을 도착 즉시 청크 단위로 `OutputSink`에 흘려보내고(라이브), 실행 중
Ctrl+C로 자식을 중단한다.

## 범위

- **포함**: 증분 스트리밍 + bounded 채널 backpressure + CLI Ctrl+C 중단(자식 kill). `PtyExecutor`
  제자리 교체로 `ai exec`/`ai dispatch`/TUI 3경로 자동 적용.
- **제외(비목표)**:
  - TUI 내 mid-exec 중단 — TUI 이벤트 루프가 exec 동안 블로킹되는 기존 한계(별도 아키텍처 후속).
  - stdout/stderr 분리(PTY 혼합 유지 — 기존과 동일).
  - 별도 backpressure 튜닝(bounded 채널 cap으로 충분).

## 아키텍처

### `src/pty.rs` — `run_in_pty_streaming` 추가(기존 `run_in_pty`·`PtySession` 유지)

```rust
/// PTY에서 `shell -c command`를 실행하며 출력을 청크 단위로 흘려보낸다.
/// Ctrl+C(SIGINT) 수신 시 자식을 kill하고 중단한다. 종료코드를 반환한다
/// (취소 시 130 = 128+SIGINT).
pub fn run_in_pty_streaming(
    shell: &str,
    command: &str,
    on_chunk: impl FnMut(&str),
) -> anyhow::Result<i32>
```

동작:
1. `run_in_pty`와 동일하게 PTY를 열고 `shell -c command`를 spawn, `drop(slave)`로 EOF 전달
   조건을 만든다. master reader를 clone한다.
2. **리더 스레드**(`std::thread::spawn`): 블로킹 `reader.read(&mut [0u8; 4096])` 루프.
   - `Ok(0)` 또는 `Err(_)` → break(EOF/에러).
   - `Ok(n)` → `tx.blocking_send(buf[..n].to_vec())`; 채널이 닫혀 있으면 break.
   `tx`는 **bounded `tokio::sync::mpsc::channel::<Vec<u8>>(64)`** — 소비가 느리면
   `blocking_send`가 막혀 **자연 backpressure**를 제공한다. 스레드 종료 시 `tx` drop으로 채널이 닫힌다.
3. **current-thread tokio 런타임**(`responder.rs`와 동일 패턴)에서 `block_on`:
   ```rust
   let cancelled = loop {
       tokio::select! {
           msg = rx.recv() => match msg {
               Some(bytes) => on_chunk(&String::from_utf8_lossy(&bytes)),
               None => break false, // EOF
           },
           _ = tokio::signal::ctrl_c() => {
               let _ = child.kill();
               break true; // 취소
           }
       }
   };
   ```
   `child`는 async 블록이 가변 차용하고 select 종료 후 아래 `wait`에서 다시 쓴다(차용은
   `block_on` 스코프에서 끝남). `on_chunk`도 async 블록이 가변 차용(각 청크에서 동기 호출,
   await 사이 없음).
4. `child.wait()`로 종료 상태를 얻고 리더 스레드를 `join`한다. 반환:
   `if cancelled { 130 } else { status.exit_code() as i32 }`.

런타임 중첩 없음: `run_exec`/TUI는 sync 컨텍스트, `run_dispatch` 셸 경로도 AI 런타임이
활성이 아닌 시점에 executor를 호출하므로 current-thread 런타임 생성이 안전하다.

### `src/pipeline.rs` — `PtyExecutor::run` 제자리 교체

```rust
impl Executor for PtyExecutor {
    fn run(&self, command: &str, sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
        crate::pty::run_in_pty_streaming(&self.shell, command, |c| sink.write(c))
    }
}
```
`Executor` 트레이트 시그니처 불변 → `run_exec`(main.rs)·`run_dispatch`·TUI(ui.rs)의 `PtyExecutor`
사용처가 그대로 라이브 스트리밍·중단을 얻는다. 기존 `MockExecutor`(테스트)는 무영향.

## Ctrl+C 동작

- **CLI(`ai exec`/`ai dispatch`)**: cooked 모드 → SIGINT가 시그널로 도착 → `ctrl_c()` 분기 발동
  → 자식 kill, exit 130.
- **TUI**: raw 모드 → Ctrl+C가 KeyEvent로 잡혀 프로세스 SIGINT는 미발생 → `ctrl_c()` 분기는
  발동하지 않고 스트리밍만 정상 동작(무해). TUI 내 중단은 비목표.

## 데이터 흐름

`PtyExecutor::run` → `run_in_pty_streaming` → (리더 스레드: PTY read → 채널) +
(메인: 채널 recv → `on_chunk` = `sink.write` → CLI=stdout 라이브 / TUI=히스토리 append).

## 테스트

- **단위**(`src/pty.rs`, `#[cfg(all(test, unix))]`):
  - 스트리밍 누적: `printf 'one\ntwo\n'` 류 명령 → `on_chunk`로 누적한 문자열이 기대 출력을 포함, 반환 exit 0.
  - 종료코드 전파: `exit 3` → 3.
  - (선택) 다중 청크: 작은 출력을 여러 번 내는 명령에서 `on_chunk` 1회 이상 호출 확인(누적 일치로 갈음).
- **e2e**(WSL): Ctrl+C 취소는 시그널 의존이라 단위테스트가 비결정적 → `ai exec "sleep 5"`
  실행 중 SIGINT 전송 시 즉시 종료하고 130을 반환하는지 확인(`cmd; rc` 측정은 `&&/||`로 — 이 환경
  `$?` 무력). 스트리밍 라이브성은 `ai exec`로 점진 출력 명령 실행해 출력이 즉시 나오는지 육안 확인.

## 비목표(재확인)

- TUI mid-exec 중단, stdout/stderr 분리, backpressure 파라미터화 — 모두 후속.
