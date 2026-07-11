# Android AI 보조 이식 설계 (openai backend + mock transport)

- **작성일**: 2026-07-10
- **상태**: 설계(spec) — 리뷰 대기. 구현은 writing-plans→subagent-driven.
- **정본 브랜치**: develop (`b0fe6ba`+). 빌드·검증 [[terminal-build-env]].
- **관련**: PM-3 Android([[terminal-project-state]]), 데스크톱 ash AI(S5 `2026-06-27-windows-ash-s5-ai-integration`)

---

## 1. 목표 & 배경

Android(PM-3)는 **순수 shellcore 터미널 + Termux bridge**뿐이고, 데스크톱(`ai`/`ash`)의 **AI 보조(자연어→명령 제안)가 없다**. 실측: `src/mobile.rs` AI 참조 0, `android/*.kt` AI 없음, AI 스택(`ai_router`·`dispatch`·`gateway`·`http`·`intent`·`ollama`·`openai`·`provider`)이 전부 `#[cfg(not(target_os = "android"))]`로 Android 미컴파일.

**목표**: Android에 AI 보조를 이식한다 — 자연어 입력을 감지해 AI가 **명령을 제안**(자동 실행 안 함, §3-11)하고, 셸 명령은 기존 shellcore로 평가. provider는 openai backend까지 결선하되, 실 HTTP transport는 실기기 검증이 필요하므로 이번엔 MockTransport로 완결하고 실 android transport는 후속.

## 2. 현황 (실측)

| 항목 | 상태 |
|---|---|
| `MobileShell`(mobile.rs) | `eval_line`(순수 shellcore 평가)·`capabilities`·`state`만. AI/게이트 없음 |
| AI 스택 게이트 | `dispatch`·`intent`·`gateway`·`openai`·`provider`·`http`·`ai_router` 전부 `cfg(not android)` |
| 모듈 pure 여부 | **dispatch·intent·openai·provider = pure**(외부 crate 0 / anyhow만). gateway=`tokio::sync::Notify`(budget). http=`tokio::io/net`(실 I/O) |
| openai 구조 | `OpenAiBackend<T: HttpTransport>` — **HttpTransport 트레이트 추상 위**. `MockTransport` 존재(테스트) |
| async 구동 | gateway future는 "current-thread `block_on`"으로 구동(Send 불요) |
| shellcore | `repl.rs`에 `AiRouter` 트레이트(android 컴파일됨) — 데스크톱 ash가 `GatewayAiRouter` 주입 |

## 3. 설계 결정 (사용자 확정)

1. **AI는 mobile 계층**, shellcore는 pure 유지(AI 스택을 shellcore에 넣지 않음).
2. **openai backend까지 결선**, provider 선택은 config.
3. **HttpTransport는 MockTransport 이번**(cargo/gradle 검증) — 실 android transport(OkHttp JNI 권장)는 **후속 슬라이스**.
4. 명령 자동 실행 금지(§3-11): AI는 제안만, 사용자 확인 후 실행.

## 4. 아키텍처

### 4-1. AI 스택 android-compat 확장
`cfg(not android)`를 **android 포함**으로 넓힌다. 대부분 pure라 그대로 컴파일. 걸림돌 2개:
- **gateway `tokio::sync::Notify`**(budget 취소): android에서 `std::sync`(Condvar/AtomicBool) 대체 또는 budget 취소 경로를 android-무관하게 최소화. (budget은 데스크톱 storage 연동이므로 mobile에선 no-op 가능.)
- **http 실 I/O**(`TcpTransport`·`post_json_tls`): **android 미포함**(`cfg(not android)` 유지). `HttpTransport` 트레이트 자체는 pure(`async fn post_json`)라 android 포함. Android는 `MockTransport` 사용.
- **async runtime**: 데스크톱 tokio `block_on` 대신 android는 `futures::executor::block_on`(경량, 의존 최소) 검토 — mobile.rs 결선 지점에서 구동.

### 4-2. mobile.rs AI 라우팅 결선
`MobileShell::eval_line`을 확장:
1. 입력을 `dispatch::dispatch(input, profile)`로 **Shell / Ai / Empty** 분류
2. `Shell`이면 기존 shellcore 평가(불변)
3. `Ai`면 `gateway`(`OpenAiBackend<MockTransport>` 또는 config 기반) 호출 → **명령/설명 제안** 반환(실행 안 함)
4. `MobileEvalResult`(JSON)에 `ai` 종류·제안 필드 추가
- 라우팅 주입은 shellcore `AiRouter` 트레이트 패턴 재사용 검토(mobile용 구현) 또는 mobile.rs 자체 분기. **경계**: dispatch/gateway 참조는 mobile.rs에만, shellcore 불변.

### 4-3. Android UI (Kotlin)
- `TerminalViewModel`: 자연어 입력이 AI로 라우팅됨을 transcript에 구분 표시. `EntryKind`에 AI 계열(예: `AiSuggestion`) 추가.
- AI 제안은 실행 버튼/확인 UX(자동 실행 안 함). 상세는 plan.

## 5. 데이터 흐름
```
Android 입력 → JNI → mobile::eval_line_json
  → dispatch 분류
    ├ Shell → shellcore eval (기존)
    └ Ai → gateway(OpenAiBackend<MockTransport>) → 제안
  → MobileEvalResult(JSON: kind=shell|ai, 제안) → JNI → TerminalViewModel → Compose transcript
```

## 6. 테스트

- **cargo(무피처 + `--target aarch64-linux-android` check)**: dispatch 분류·gateway mock 응답·mobile AI 라우팅 단위테스트. **android 타깃 컴파일이 핵심 게이트**(AI 스택 android 확장이 깨지지 않는지).
- **gradle `:app:testDebugUnitTest`**: TerminalViewModel AI 표시 로직.
- **실 openai HTTPS 동작**: 실기기 수동검증 — **이 환경 밖(후속)**.
- 데스크톱 회귀 0: 무피처+`storage tls remote` 전 조합 green(AI 스택 cfg 확장이 데스크톱 빌드 불변).

## 7. 범위 밖 (후속 슬라이스)
- **실 android HttpTransport**(OkHttp JNI 콜백 or rustls NDK) + 실기기 openai 네트워크 검증
- ollama provider(로컬/원격)
- AI 응답에서 제안 명령을 안전 게이트 통과(별도 "Android 게이트 이식" 기능과 연계)

## 8. 리스크 & 완화
1. **gateway/http tokio 의존**: android 확장 시 tokio android 빌드 부담 → `Notify`를 std로 대체·실 I/O transport는 android 미포함(mock)으로 회피. **plan Task 1에서 `--target aarch64-linux-android` check 선행**해 실제 장애물 조기 확정.
2. **async runtime android**: `futures::block_on` 경량 선택(tokio-rt android 회피). 의존 추가 시 android cdylib 크기·빌드 확인.
3. **경계 침범**: dispatch/gateway를 shellcore에 넣지 않음(mobile.rs만). android cdylib 무조건 의존 주의(교훈: 데스크톱 전용 crate는 target-gate).
4. **§3-11**: AI 제안 자동 실행 금지 — UI 확인 게이트.

## 9. 다음 단계
spec 승인 후 → `writing-plans`로 구현 plan(Task: AI 스택 android 확장→mobile 결선→UI→검증). android 타깃 컴파일 장애물이 핵심이라 Task 1에서 조기 검증. **실 transport·실기기 검증은 별도 후속.**
