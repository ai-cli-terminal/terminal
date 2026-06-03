# Shell·Ai 단일 디스패처 통합 — 설계

- 날짜: 2026-06-03
- 상태: 승인됨 (브레인스토밍 완료)
- 범위: 그룹 C 후속 — "Shell/Ai 단일 dispatcher 통합" (풀 통합)
- 관련: `docs/superpowers/specs/2026-06-03-central-execution-pipeline-design.md`

## 1. 문제

`dispatch::dispatch(input, profile) -> Route{Empty, Shell{cmd,risk,decision}, Ai{prompt}}`
는 존재하지만, **이 `Route`를 받아 실제로 실행으로 보내는 단일 진입점이 없다.**
현재 실행 경로가 3갈래로 분산되어 있다:

- **TUI Submit**(`ui.rs`): 입력을 분류 없이 곧장 `pipeline::execute`로 보냄
  → TUI에서는 자연어 질의("how do I …?")도 셸 명령으로 PTY 실행되는 갭이 있다.
- **CLI `ai exec`**(`run_exec`): 곧장 `pipeline::execute`(셸 강제, 의도된 단축).
- **CLI `ai ask`**(`Command::Ask`): `gateway`로 AI 호출(AI 강제, 별도 경로).

`dispatch::dispatch`는 진단용 `ai route`(결과 출력만)에서만 소비된다.

## 2. 목표

입력을 분류해 **Shell→`pipeline::execute` / Ai→AI 게이트웨이**로 보내는 단일
오케스트레이터를 두고, 인터랙티브 경로(TUI Submit + 신규 CLI 일회성 명령)가
이를 거치게 한다. 특히 **TUI의 자연어 질의가 AI로 가도록** 갭을 닫는다.

비목표(이번 증분 제외): AI 백엔드 영구 config, CLI REPL, async 스트리밍(W2),
audit 기록 확장, `ai exec`/`ai ask`/`run_exec` 리팩터.

## 3. 아키텍처 (2레이어)

- **순수 라우팅** — 기존 `dispatch::dispatch(input, profile) -> Route` **변경 없음**.
- **오케스트레이션(신규)** — `dispatch::run(...)`이 `Route`를 받아 주입된 핸들러로
  실행한다. Empty→무동작, Shell→`pipeline::execute`, Ai→`AiResponder::respond`.

의존 방향: `dispatch` → `pipeline`(역방향 없음 → 순환 없음). I/O는 기존
`Executor`/`Confirmer`/`OutputSink`와 같은 결의 트레이트로 주입해 코어를 순수하게
유지한다(PTY·런타임·네트워크 없이 단위 테스트 가능).

## 4. 신규 타입 (`dispatch.rs`)

```rust
/// AI 핸들러 추상화(Executor/Confirmer/OutputSink와 같은 결의 심).
/// 컨텍스트(cwd 등)는 실제 구현이 내부에서 모은다.
pub trait AiResponder {
    fn respond(&mut self, prompt: &str, sink: &mut dyn pipeline::OutputSink)
        -> anyhow::Result<AiOutcome>;
}

/// AI 응답 결과.
pub enum AiOutcome {
    Answered { text: String, input_tokens: usize, output_tokens: usize },
    Blocked(String),     // 마스킹 fail-closed (gateway Blocked)
    Unavailable(String), // 장애·타임아웃·취소 (§3-3: 셸을 막지 않음)
}

/// 통합 실행 결과.
pub enum Handled {
    Empty,
    Shell(pipeline::ExecOutcome),
    Ai(AiOutcome),
}

/// 주입 핸들러 묶음. sink는 셸/AI가 공유(disjoint field 借用).
pub struct Handlers<'a> {
    pub executor: &'a dyn pipeline::Executor,
    pub confirmer: &'a mut dyn pipeline::Confirmer,
    pub ai: &'a mut dyn AiResponder,
    pub sink: &'a mut dyn pipeline::OutputSink,
}

pub fn run(
    input: &str,
    profile: &PolicyProfile,
    exec_cfg: &pipeline::ExecConfig,
    h: &mut Handlers,
) -> anyhow::Result<Handled>;
```

`run` 동작:

```text
match dispatch(input, profile) {
    Route::Empty            => Handled::Empty
    Route::Shell{command,..}=> Handled::Shell(pipeline::execute(&command, exec_cfg,
                                              h.executor, h.confirmer, h.sink)?)
    Route::Ai{prompt}       => Handled::Ai(h.ai.respond(&prompt, h.sink)?)
}
```

`pipeline::execute`가 내부에서 risk를 재산출하지만(self-contained) 수용한다.
`Route::Shell`의 risk/decision은 진단용 `ai route` 표시에만 쓰인다.

## 5. 실제 AI 구현 `GatewayResponder` (신규 `src/responder.rs`, lib 모듈)

ui.rs(TUI)·main.rs 양쪽에서 쓰므로 **lib**에 둔다.

- 보유: `Gateway`, current-thread tokio 런타임(1회 생성), 요청 타임아웃.
- `respond(prompt, sink)`:
  1. `context::gather()`로 컨텍스트(`cwd=…`) 구성.
  2. `rt.block_on(ask_cancellable(prompt, ctx, timeout, cancel))`
     — Ctrl+C 취소(best-effort) + 타임아웃.
  3. 매핑:
     - `Ok(GatewayOutcome::Answered{text,in,out})` → sink.write(text) + `AiOutcome::Answered`
     - `Ok(GatewayOutcome::Blocked(r))` → `AiOutcome::Blocked(r)`
     - `Err(RequestError)` → `AiOutcome::Unavailable(e.to_string())`
- 기본 구성: `GatewayResponder::mock(Timeouts::defaults().request)` (백엔드 mock).

동기 `block_on`이므로 오케스트레이터는 sync 유지 — AI 동안 호출자가 블록되며
타임아웃이 상한이다. 블로킹 해소(스트리밍)는 후속 W2.

## 6. 배선

### TUI (`ui.rs` Submit)
직접 `pipeline::execute` 호출을 제거하고 `Handlers`를 구성해 `dispatch::run` 호출:
- `executor = PtyExecutor`, `confirmer = TuiDeny`, `ai = GatewayResponder::mock`,
  `sink = StringSink`.
- `Handled`를 출력 문자열로 포맷:
  - `Empty` → 무동작
  - `Shell(ExecOutcome)` → 기존 포맷(Ran/Blocked/Declined/BackupRefused) 유지
  - `Ai(Answered{text,..})` → text 추가
  - `Ai(Blocked(r))` → `[차단: {r}]`
  - `Ai(Unavailable(e))` → `[AI 사용 불가: {e}]`

### CLI 신규 `ai dispatch "<input>" [--yes] [--profile <name>]`
입력을 분류→실행까지 하는 일회성 명령(기존 `ai route`는 출력 전용으로 유지).
- `Handlers`: `PtyExecutor`, `AutoYes`(--yes)|`StdinConfirmer`, `GatewayResponder::mock`,
  `StdoutSink`.
- `Handled` 처리:
  - `Shell(ExecOutcome)` → `run_exec`와 동일(종료코드 전파·`record_exec`).
  - `Ai(Answered)` → text 출력 + 토큰 표시(`ai ask`와 동형).
  - `Ai(Blocked/Unavailable)` → 안내 출력, 정상 종료(셸 흐름 보호).

### 변경 없음
`ai exec` / `ai ask` / `run_exec` / `dispatch::dispatch` / `ai route`.

## 7. 에러·엣지

- AI 장애/타임아웃/취소 → `Unavailable`로 친절 고지, 셸 흐름 정상(§3-3 fail-open).
- 위험 셸: 기존 pipeline 게이트 그대로(TUI=`TuiDeny` 거부 안내, CLI=확인/`--yes`).
- 빈 입력 → `Handled::Empty`(무동작).

## 8. 테스트

- **단위(`dispatch::run`)**: mock `Executor`/`Confirmer`/`AiResponder` + 수집 `Sink`로
  라우팅 검증 — `"  "`→Empty, `"ls -al"`→Shell(Ran), 자연어→Ai(Answered),
  `"rm -rf /"`→Shell(Blocked), `"ai …"`→Ai. PTY·런타임·네트워크 불필요.
- **단위(`GatewayResponder`)**: mock 게이트웨이로 Answered가 sink에 기록되는지,
  Blocked/Unavailable 매핑 확인(런타임 1개 current-thread).
- **WSL e2e**: `ai dispatch "ls"`(셸 실행), `ai dispatch "how do I list files?"`
  (mock AI 응답) 확인. `cargo fmt`/`clippy`/`test`(default·storage·tls) green.

## 9. 변경 파일

- `src/dispatch.rs` — `AiResponder`/`AiOutcome`/`Handled`/`Handlers`/`run` 추가 + 테스트.
- `src/responder.rs` — 신규 `GatewayResponder` + 테스트. `lib.rs`에 모듈 등록.
- `src/ui.rs` — Submit 경로를 `dispatch::run`으로 재배선.
- `src/main.rs` — `Command::Dispatch` 추가 + 핸들러.
- `docs/HISTORY.md`/`CHANGELOG.md`/`TASK.md` — 기록.
