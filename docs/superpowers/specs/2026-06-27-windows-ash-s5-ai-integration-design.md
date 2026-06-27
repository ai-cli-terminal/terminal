# S5 — `ash` AI Integration 설계

> **작성일**: 2026-06-27
> **유형**: 단일 슬라이스 구현 spec (Windows ash 완성 로드맵 S5).
> **상위**: `2026-06-26-windows-ash-completion-scoping-design.md` (#5 AI 통합). 선행: S1(config), S2(gate), S3(editor).
> **참고**: `src/intent.rs`(classify), `src/dispatch.rs`(Route), `src/responder.rs`(GatewayResponder), `src/gateway.rs`.

## 1. 목표

`ash` REPL이 **자연어 입력을 AI로, 셸 명령은 그대로 셸로** 라우팅한다.

- `intent::classify`(기존, deterministic)로 자동 분류: `ai <prompt>`/`?`/의문사/한글 요청마커 → AI, 그 외 → 셸.
- AI 경로는 `GatewayResponder`(기존)로 응답하고 **timeout/cancel·백엔드 장애를 fail-soft**(`AiOutcome::Unavailable`)로 흡수 — **셸 세션을 절대 막지 않는다**.
- 셸 경로는 기존 `eval_line`(shellcore, S2 게이트 포함) 그대로.

S5는 **라우팅 결선**이 핵심이다. AI 백엔드는 **mock(echo)** 를 쓴다(현 `ai dispatch`와 동일). 실 provider(ollama/openai)는 `[ai]` config 모델링과 함께 **후속**.

## 2. 경계 제약

- AI 라우팅(intent/dispatch/gateway/responder)은 데스크톱 모듈이다. `shellcore::repl`은 이를 참조하지 않고 **`AiRouter` 트레이트만** 안다.
- reedline 기반 구현이 아니라 라우팅이므로 신규 데스크톱 모듈 `src/ai_router.rs`(`cfg(not(target_os="android"))`)에 둔다.
- `shellcore`·android cdylib 빌드 불변(`cargo check --lib --target aarch64-linux-android` green).

## 3. `AiRouter` 트레이트 (shellcore)

`src/shellcore/repl.rs`:
```rust
/// 입력이 AI 질의면 처리(응답 출력)하고 true, 셸이면 false를 반환한다.
pub trait AiRouter {
    fn try_handle(&mut self, input: &str) -> bool;
}

/// 기본 라우터: 항상 false(모든 입력을 셸로). 임베드/비-AI/테스트용. std만.
pub struct NoAiRouter;
impl AiRouter for NoAiRouter {
    fn try_handle(&mut self, _input: &str) -> bool { false }
}
```

## 4. `repl::run` 변경

시그니처에 router 주입(S1~S4 패턴):
```rust
pub fn run(
    settings: ReplSettings,
    runner: Box<dyn ExternalRunner>,
    reader: Box<dyn LineReader>,
    router: Box<dyn AiRouter>,
) -> Result<()>;
```
루프의 `ReadOutcome::Line(line)` 처리:
```rust
if line.is_empty() { continue; }
if router.try_handle(&line) { continue; }   // AI가 처리 → 다음 프롬프트
// 아니면 기존 셸 경로
match eval_line(&line, &mut engine) { ... }
```
- `try_handle`가 false면(셸/빈입력) 기존 `eval_line`로 폴백. AI가 처리하면 셸 eval을 건너뛴다.

## 5. `GatewayAiRouter` (데스크톱, src/ai_router.rs)

```rust
pub struct GatewayAiRouter {
    responder: crate::responder::GatewayResponder,
    profile: crate::policy::PolicyProfile,
}
impl GatewayAiRouter {
    /// mock(echo) 게이트웨이로 구성. config 활성 profile 사용.
    pub fn from_environment() -> anyhow::Result<Self>;
}
impl crate::shellcore::repl::AiRouter for GatewayAiRouter {
    fn try_handle(&mut self, input: &str) -> bool { ... }
}
```

`try_handle`:
1. `dispatch::dispatch(input, &self.profile)` → `Route::Ai { prompt }` 가 아니면(`Empty`/`Shell`) **false 반환**.
2. AI면: `StdoutSink`(아래)로 `self.responder.respond(&prompt, &mut sink)` 호출.
3. 결과 매핑(모두 출력 후 **true** 반환):
   - `Ok(AiOutcome::Answered { .. })` → text는 이미 sink로 출력됨. 끝에 개행 보장.
   - `Ok(AiOutcome::Blocked(r))` → `eprintln!("ash: AI 차단됨: {r}")`.
   - `Ok(AiOutcome::Unavailable(r))` → `eprintln!("ash: AI 사용 불가: {r}")`.
   - `Err(e)` → `eprintln!("ash: AI 오류: {e}")`.
   어느 경우든 **REPL은 지속**(셸 안 막음).

`from_environment`:
- `responder = GatewayResponder::mock()?` (echo 백엔드 — 현 `ai dispatch`와 동일).
- `profile = PolicyProfile::by_name(&config::get_active_profile()).unwrap_or_else(PolicyProfile::balanced)`.

`StdoutSink`(ai_router.rs): `impl pipeline::OutputSink { fn write(&mut self, c) { print!("{c}"); stdout flush } }`.

## 6. ash 결선

`src/bin/ash.rs`: router를 만들어 주입. 실패 시 `NoAiRouter` 폴백(순수 셸로 계속):
```rust
let router: Box<dyn ...AiRouter> = match ai_terminal::ai_router::GatewayAiRouter::from_environment() {
    Ok(r) => Box::new(r),
    Err(_) => Box::new(ai_terminal::shellcore::repl::NoAiRouter),
};
ai_terminal::shellcore::repl::run(settings, runner, reader, router)
```
TTY/비-TTY 무관하게 router는 동일(분류는 입력 텍스트 기반). reader 선택(S3)은 그대로.

## 7. "AI 제안 명령은 게이트 통과"

gateway는 **텍스트 답변**만 반환한다(auto_execute=false, §3-11 — AI 생성 명령 자동 실행 금지). 사용자가 제안받은 명령을 직접 입력해 실행하면 그것은 일반 셸 경로이므로 **S2 안전 게이트가 자동 적용**된다. 자동 실행 결선은 비목표(만족됨).

## 8. 에러 처리 (fail-soft 전면)

- AI timeout/cancel(Ctrl+C)/백엔드 장애 → `GatewayResponder`가 `AiOutcome::Unavailable`로 흡수(기존 동작). 라우터가 메시지 출력 후 true 반환, REPL 지속.
- `from_environment` 실패 → ash가 `NoAiRouter` 폴백(AI 없이 순수 셸).
- AI 처리 중 어떤 오류도 셸 세션을 종료시키지 않는다.

## 9. 테스트

단위:
- `NoAiRouter::try_handle`는 항상 false(`"ls -al"`, `"how do I X?"` 모두 false).
- `GatewayAiRouter`(mock responder)로: `try_handle("how do I undo a commit?")`→true(처리), `try_handle("ls -al")`→false(셸), `try_handle("ai explain x")`→true. (intent classify·dispatch는 이미 테스트됨; 여기선 라우터 분기·반환값 검증)

e2e(WSL, 비-TTY 파이프):
- `printf 'how do I list files?\nexit\n' | ash` → mock echo 답변 출력(AI 경로).
- `printf 'echo hi\nexit\n' | ash` → `hi`(셸 경로 유지).
- `printf 'rm -rf /\nexit\n' | ash` → 정책상 차단(S2 게이트 유지).
- android 경계: `cargo check --lib --target aarch64-linux-android` green.

## 10. 수용 기준

1. ash가 자연어(의문사/`?`/한글마커/`ai ` 접두)를 AI로 라우팅해 응답(mock)을 출력한다.
2. 셸 명령은 기존 `eval_line` 경로(S2 게이트 포함) 그대로.
3. AI 실패/타임아웃/취소는 fail-soft(메시지 + REPL 지속), 셸 비중단.
4. `shellcore`(NoAiRouter)는 std만 — android cdylib 빌드 불변.
5. `cargo fmt --all -- --check`(실제 `cargo fmt --all` 후) · `cargo clippy --all-targets --features "storage tls remote" -D warnings` · `cargo test --features "storage tls remote"` green.

## 11. 비목표 (S5 밖)

- **실 provider(ollama/openai) 결선 + `[ai]` config 모델링**(provider/model/timeout/budget) — 후속 슬라이스. S5는 mock.
- AI usage/audit 기록(storage, S2 audit과 함께 이연).
- AI 출력 스트리밍(responder는 sync block_on), classifier 미세조정(`where`/`who`/`help` 명령 충돌), 캐시 배지 표기.
- `repl::run` 4개 주입 파라미터 번들링(현재는 개별 — YAGNI).
