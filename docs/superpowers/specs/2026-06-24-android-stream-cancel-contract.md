# Android stream/cancel contract

> 작성일: 2026-06-24
> 범위: PM-3 Android local terminal, output incremental stream/cancel contract.
> 관련 문서: `2026-06-23-android-local-terminal-spike.md`, `2026-06-23-platform-execution-contract.md`.

## 1. 목표

Android `ash`는 지금 `shellcore-only`라 한 줄 평가가 즉시 완료 결과를 반환한다. 하지만 PM-3E 이후 Termux-compatible 또는 bundled userland가 붙으면 output은 long-running stream이고 cancel은 사용자 기대의 핵심 동작이 된다.

따라서 지금부터 UI/worker 계약은 complete-result가 아니라 event stream 형태를 기준으로 둔다.

## 2. Event Model

Kotlin 계약 타입은 `ShellStreamEvent`다.

| Event | 의미 |
|---|---|
| `Started(input, state)` | worker가 command를 접수했고 UI가 running 상태로 전환할 수 있다. |
| `Stdout(text)` | stdout 또는 shellcore textual output chunk. |
| `Stderr(text)` | stderr 또는 failure message chunk. |
| `Finished(result)` | command가 정상/오류 result로 종료했다. `ShellEvalResult.state`가 다음 session state다. |
| `Cancelled(state)` | 사용자가 cancel했고 final result는 UI에 적용하지 않는다. |

현재 `NativeShellBridge.evalLine`은 complete-result bridge다. `ShellWorker.submitStreaming`은 이 결과를 다음 event sequence로 변환한다.

```text
Started
  -> Stdout? or Stderr?
  -> Finished
```

cancel이 completion 전에 들어오면:

```text
Started
  -> Cancelled
```

`shellcore-only` cancel은 cooperative UI contract다. 이미 JNI 호출에 들어간 Rust evaluation을 강제 중단하지 않는다. Long-running userland/PTY 단계에서는 adapter가 process/PTY interrupt를 실제로 수행해야 한다.

## 3. Threading Contract

- `ShellBridge.evalLine` 또는 future stream-capable bridge는 UI thread에서 실행하지 않는다.
- 모든 `ShellStreamEvent` delivery는 `ResultPoster` 뒤에서 UI-safe context로 post한다.
- `ShellWorker`는 기본 single-thread executor를 사용해 command ordering을 보존한다.
- UI는 `Finished` 또는 `Cancelled`를 terminal state transition의 final event로 취급한다.

## 4. Cancel Semantics

`ShellRunHandle.cancel()`은 idempotent다.

| 단계 | 동작 |
|---|---|
| queued before execution | future implementation should skip bridge execution and emit `Cancelled`. |
| during shellcore eval | current implementation suppresses final result and emits `Cancelled` after eval returns. |
| during process/userland | future adapter must send best-effort interrupt, then terminate after timeout if supported. |
| after final event | no-op. |

Cancellation does not roll back `ShellState` unless the future adapter can prove command state did not change. Current shellcore path emits the result state with `Cancelled` but UI does not apply `Finished`.

## 5. Future Adapter Requirements

Any Termux-compatible, bundled userland, PTY, or process-backed Android adapter must implement this event model before it reaches UI:

- output chunks must preserve order within a command.
- stdout and stderr must remain distinguishable if the underlying transport can distinguish them.
- exit code must arrive only in `Finished`.
- timeout and user cancel must produce `Cancelled` or an error `Finished`, not hang silently.
- command state updates must be explicit in the final event.

## 6. Tests

`ShellWorkerTest` fixes the current contract:

- bridge evaluation happens on worker thread.
- result callback is posted through `ResultPoster`.
- failure is converted to `ShellEvalResult(ok=false)`.
- streaming adapter emits `Started -> Stdout -> Finished`.
- cancel before completion emits `Started -> Cancelled` and suppresses `Finished`.
