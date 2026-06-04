# WI-5 — TUI mid-exec 중단 + 라이브 스트리밍 (설계)

> **작성일**: 2026-06-04 · **정본**: §5(Terminal UI), §31.5(실행 UX), §16.2(Graceful Recovery).
> **계획**: `docs/superpowers/plans/2026-06-04-phase1-usability-gaps.md` WI-5.

## 문제

TUI(`ui::run`)는 Submit 시 `dispatch::run`을 **동기 블로킹** 실행하고 완료 후 출력을 일괄 append한다. 따라서 (1) 장기 명령의 출력이 **라이브로 표시되지 않고**, (2) 실행 중 **중단할 수 없다**. CLI엔 `run_in_pty_streaming`(ctrl_c 취소)이 있으나, raw-mode TUI에서 Ctrl+C는 KeyEvent라 SIGINT 기반 취소가 발동하지 않는다.

## 설계 결정

### 1) 명시적 취소 플래그 기반 PTY 스트리밍
- **추가(`pty.rs`)**: `run_in_pty_streaming_cancellable(shell, command, cancel: Arc<AtomicBool>, on_chunk)`.
  - spawn 후 `child.clone_killer()`로 killer를 얻어 **워처 스레드**가 `cancel`을 20ms 폴링 → set이면 kill. 리더 루프는 블로킹 read로 청크를 `on_chunk`에 흘려보낸다(kill 시 EOF로 종료).
  - 종료코드: 취소 시 130(=128+SIGINT), 아니면 자식 exit code.
  - **무출력 명령도 중단 가능**: 취소 감지가 출력과 무관한 워처 스레드에서 일어난다.

### 2) Shell만 워커 스레드, AI는 메인 스레드
- `GatewayResponder`(AI)는 `Box<dyn LlmBackend>`(Send 비보장)를 품어 워커로 옮기기 부적합 → **AI 경로는 메인 스레드 동기 유지**(요청 타임아웃이 상한).
- `dispatch::dispatch`(순수 분류기)로 메인에서 Shell/AI 분기. **Shell 경로만** `std::thread::scope` 워커에서 `pipeline::execute`(게이트+취소 가능 실행) 수행. 워커는 Send 한 것만(executor·confirmer·sink·cfg·command) 차용 → Send 문제 없음.

### 3) 메인 루프 = 채널(출력) + 키(중단) 폴링
- 워커는 `ChannelSink`(mpsc `Sender<String>`)로 청크를 보낸다. 메인은 `try_recv`로 청크를 받아 히스토리에 append하고 **매 틱 redraw**(라이브 표시), `event::poll(20ms)`로 키를 받아 Esc/Ctrl+C면 `cancel`을 set. 워커 완료(`is_finished`) 시 남은 청크를 drain하고 결과를 받는다.
- **이중 출력 방지**: 청크를 라이브로 이미 표시하므로, 완료 후엔 출력 본문을 다시 append하지 않고 **상태 꼬리(`render_tail`)** 만 append(exit note/차단/거부/취소). `render_output`(출력+꼬리 결합)은 AI 경로·기존 테스트용으로 유지.

## 범위

- **포함**: 취소 가능 PTY 스트리밍 + TUI Shell 라이브 스트리밍·중단 + `render_tail` + AI 경로 분리.
- **제외(후속)**: 인터랙티브 PTY(입력 전달·TUI 내 vim 등), 출력 스크롤백·ANSI 파싱, AI 중단(타임아웃이 상한).

## 수용 기준 (DoD, §5/§31.5)

1. `run_in_pty_streaming_cancellable`: 취소 set 시 자식 kill·exit 130·즉시 반환(WSL: `sleep 10` 200ms 후 취소 → 5s 내 130). 취소 없으면 출력 누적·정상 exit. (unix/WSL)
2. `render_tail`: Ran(0)→"" / Ran(N)→"[exit N]" / Blocked·Declined·BackupRefused·취소 각 안내. (단위)
3. TUI에서 장기 명령 출력 라이브 표시 + Esc/Ctrl+C로 중단(수동/WSL). hook·셸 비중단.
4. default·`--features storage` 빌드 모두 fmt/clippy(-D warnings)/test green.
