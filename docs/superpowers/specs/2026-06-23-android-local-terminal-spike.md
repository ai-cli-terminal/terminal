# Android 로컬 터미널 스파이크 — PM-3

> **작성일**: 2026-06-23
> **범위**: 플랫폼/모바일 로컬 터미널 workflow의 PM-3A~PM-3C 첫 진입.
> **관련 문서**: `2026-06-23-platform-target-matrix-design.md`, `../plans/2026-06-23-platform-mobile-local-terminal-workflow.md`.

## 1. 목표

Android 목표는 PWA 승인 화면이 아니라 온디바이스 로컬 터미널이다. 첫 스파이크는 Android 앱 전체를 크게 만들지 않고, 모바일 UI가 Rust `shellcore`를 안전하게 호출할 수 있음을 증명한다.

첫 slice의 완료 기준:

- Android app shell 기본값은 Kotlin/Compose + Rust core binding이다.
- Rust core는 process spawn 없이 순수 `shellcore` 명령을 평가한다.
- 세션 상태는 `eval_line(input, session_state) -> output + updated_state` 형태로 이동할 수 있다.
- UI thread는 평가를 직접 실행하지 않고 worker thread/process를 통해 호출한다.
- 외부 명령 전략은 당장 Termux/bundled userland로 확대하지 않고 `shellcore-only` MVP를 기준선으로 둔다.

## 2. 앱 shell 결정

기본 app shell은 Kotlin/Compose다.

이유:

- Android UI, lifecycle, accessibility, IME, file picker는 Kotlin/Compose가 가장 직접적이다.
- Rust는 `shellcore`, safety/AI core, future local worker를 담당한다.
- WebView/PWA는 companion 또는 demo에 적합하지만, 로컬 터미널 본체의 기본 shell은 아니다.

대안은 후속 spike에서만 바꾼다. 바꿀 조건은 터미널 렌더링, PTY streaming, file/workspace UX에서 Kotlin/Compose가 명확히 불리하다는 증거가 있을 때다.

## 3. Rust Core Boundary

첫 Rust-side 계약은 `src/mobile.rs`다.

```text
MobileShell::new()
MobileShell::from_state(state)
MobileShell::eval_line(input) -> MobileEvalResult
MobileShell::state() -> MobileSessionState
```

`MobileShell`은 `Engine::pure()`를 사용한다. 따라서 외부 command spawn은 PATH lookup 전에 `external execution disabled`로 실패한다. 이 경계는 Android 앱이 아직 userland/process 전략을 정하지 않아도 list/record literal, variable, `where`, `length`, 순수 builtin을 실행할 수 있게 한다.

반환값은 두 가지 표면을 제공한다.

| 필드 | 용도 |
|---|---|
| `output_json` | Compose UI, tests, future typed bridge가 소비하는 구조화 결과 |
| `output_text` | 터미널 pane에 바로 표시할 human-readable 결과 |
| `state` | 다음 호출에 넘길 cwd/vars/exit state |
| `error` | panic이 FFI 경계를 넘지 않도록 문자열 오류로 반환 |

## 4. Worker Model

Android UI thread는 `MobileShell::eval_line`을 직접 호출하지 않는다.

첫 앱 spike의 구조:

```text
Compose screen
  -> ViewModel
  -> ShellWorker coroutine or dedicated thread
  -> Rust binding wrapping MobileShell
  -> result stream back to UI state
```

긴 평가와 future 외부 실행을 대비해 UI state는 append-only transcript와 current session state를 분리한다. cancel/interrupt는 `shellcore-only` 단계에서는 no-op 또는 cooperative cancel로 시작하고, PTY/userland가 붙는 시점에 structured event로 승격한다.

## 5. Workspace와 파일

첫 PM-3 slice는 파일 쓰기와 Android document tree를 구현하지 않는다. 다만 계약은 다음으로 고정한다.

- 기본 workspace는 app-private directory다.
- user-selected document tree는 import/export 또는 explicit workspace 선택으로만 다룬다.
- secret/path masking boundary는 desktop `context`/`mask` 정책을 재사용한다.
- 좁은 화면 status는 cwd basename, dirty/capability badge, active profile만 표시한다.

## 6. 외부 명령 전략

첫 결정은 `shellcore-only`다.

| 선택지 | 이번 판단 |
|---|---|
| `shellcore-only` | 채택. Android UI와 Rust core bridge를 가장 작게 검증한다. |
| Termux-compatible | 후속. 사용자가 설치한 userland와 연결할지 정책/UX 검토 필요. |
| bundled minimal userland | 후속. 배포 크기와 보안 업데이트 책임이 생긴다. |

따라서 이번 slice는 "Android가 완전 Linux 터미널을 제공한다"는 약속을 하지 않는다. "Android에서 `ash` 구조화 셸 코어를 로컬 평가한다"가 정확한 약속이다.

## 7. 다음 Slice

1. `src/mobile.rs`를 JNI 또는 UniFFI binding으로 감싼다.
2. 최소 Android app shell을 만든다: 입력 줄, 출력 transcript, session status.
3. worker thread에서 `eval_line`을 호출하고 UI thread에는 result만 전달한다.
4. app-private workspace root를 만들고 cwd 표시를 붙인다.
5. 외부 명령 전략 비교 spike를 별도 문서와 smoke로 진행한다.
