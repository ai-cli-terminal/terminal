# 플랫폼 실행 계약 — `ash` / `shellcore`

> **작성일**: 2026-06-23
> **범위**: 플랫폼/모바일 로컬 터미널 workflow의 PM-1B.
> **관련 문서**: `2026-06-23-platform-target-matrix-design.md`, `../plans/2026-06-23-platform-mobile-local-terminal-workflow.md`.

## 1. 경계

`shellcore`는 순수 언어 동작을 책임진다.

- lexing, parsing, AST, value model, comparison op
- literal, variable, record, list, `where`, `get`, `first`, `length` 같은 순수 파이프라인 평가
- 임베드해도 안전한 REPL 상태: cwd 값, vars, exit request

플랫폼 어댑터는 host 실행을 책임진다.

- process spawn
- PTY/ConPTY
- PATH/PATHEXT lookup
- shell별 호출 방식(`cmd.exe`, PowerShell, POSIX)
- filesystem/userland/network capability
- stream 전달과 cancel/interrupt 의미

코드 경계는 `shellcore::external::ExternalRunner`다. `Engine::new()`는 desktop process runner를 사용한다. `Engine::pure()`는 `DisabledRunner`를 사용해 parser/evaluator/builtin이 process spawn 없이 동작함을 증명한다.

## 2. 명령 해석

| 대상 | 규칙 |
|---|---|
| Pure/mobile-core | builtin만 허용한다. 알 수 없는 명령은 PATH lookup 전에 `external execution disabled`로 실패한다. |
| Linux/WSL | 현재 `cwd`에서 명령 이름/경로를 직접 spawn하고 현재 env를 상속한다. PATH resolution은 당분간 host OS 동작을 따른다. |
| Windows 네이티브 | 어댑터가 직접 실행 `.exe`, PATHEXT `.cmd/.bat`, `.ps1`을 명시적으로 해석한다. `.cmd/.bat`는 `cmd.exe /d /c`를 거치고, `.ps1`는 `ash` grammar가 아니라 PowerShell 실행 대상으로 실행한다. |
| Android | 스파이크에서 `shellcore-only`, Termux 호환 userland, bundled minimal userland 중 선택한다. 그 결정 전에도 pure core는 유효해야 한다. |
| iOS/iPadOS | 연구 타깃은 builtin/pure shellcore만으로 시작한다. 임의 downloaded code나 외부 userland를 약속하지 않는다. |
| PWA/WASM | pure shellcore만 제공한다. native process execution은 없다. |

## 3. 인자와 quoting

`shellcore`는 command argument를 `Value`로 평가한 뒤 host adapter에 `Vec<Value>`를 넘긴다. Desktop runner는 현재 각 값을 `Value::coerce_string()`으로 바꿔 `std::process::Command`에 argv로 직접 전달한다. 따라서 중간 shell quoting 문자열을 만들지 않는다.

Windows 네이티브는 세 가지 호출 방식을 사용한다.

- 직접 실행: `.exe`, `.com`, 명시적 non-script file은 `std::process::Command`로 직접 실행한다.
- Cmd script: `.cmd`, `.bat`는 `cmd.exe /d /c <script> ...args`로 실행한다.
- PowerShell script: `.ps1`는 `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <script> ...args`로 실행한다.

어댑터는 resolved target 뒤에 process argument를 별도로 전달해 argv boundary를 보존한다. PowerShell 문법은 `ash` 문법으로 해석하지 않는다. PowerShell은 `.ps1` 실행 host일 뿐이다.

## 4. Cwd와 워크스페이스

`Engine.cwd`는 논리 working directory다. `cd`, `ls` 같은 builtin은 이 값을 직접 사용한다. External runner는 `ExternalCommand` 안의 `cwd`를 받는다.

모바일 어댑터는 이 값을 앱 workspace에 매핑해야 한다.

- Android: app-private workspace 또는 user-selected document tree.
- iOS/iPadOS: app container 또는 document picker location만 사용.
- PWA: virtual workspace만 사용.

## 5. Env 정책

Desktop runner는 현재 process environment를 상속한다. `ash`가 안전 게이트를 거치는 실행 경로가 되기 전, PM-2+에서 기존 context/env policy를 통해 이를 좁혀야 한다.

Mobile/PWA 어댑터는 명시적 allowlist를 기본값으로 삼아야 한다. 기존 `context`와 `mask` 정책의 secret은 FFI, log, remote companion 경계를 넘어가면 안 된다.

## 6. 스트림과 Exit Code

현재 desktop runner는 stdout/stderr를 직접 상속하고 `Value::Nothing`을 반환한다. Non-zero exit은 `[name: exit code]`를 출력하고 REPL은 계속 유지한다.

향후 platform 어댑터는 구조화 stream model을 노출해야 한다.

- stdout chunk
- stderr chunk
- final exit code 또는 signal/cancel reason
- capability별 interrupt 지원 여부

REPL은 계속 text로 렌더링할 수 있고, 모바일 UI와 테스트는 구조화 event를 소비할 수 있다.

## 7. Capability 플래그

| Capability | 의미 |
|---|---|
| `can_spawn` | adapter가 host process를 시작할 수 있다. |
| `has_pty` | adapter가 POSIX PTY 동작을 제공할 수 있다. |
| `has_conpty` | adapter가 Windows ConPTY 동작을 제공할 수 있다. |
| `has_userland` | target이 `shellcore` builtin 밖의 유용한 외부 명령을 갖고 있다. |
| `can_write_workspace` | adapter가 active workspace에 file write를 할 수 있다. |
| `can_network` | adapter가 직접 network connection을 열 수 있다. |

| 대상 | can_spawn | has_pty | has_conpty | has_userland | can_write_workspace | can_network |
|---|---:|---:|---:|---:|---:|---:|
| Pure/mobile-core | 아니오 | 아니오 | 아니오 | 아니오 | 아니오 | 아니오 |
| Linux/WSL desktop | 예 | 예 | 아니오 | 예 | 예 | 예 |
| Windows 네이티브 | 예 | 아니오 | 예 | 예 | 예 | 예 |
| Git Bash/MSYS | 계획됨 | profile-dependent | 아니오 | 예 | 예 | 예 |
| Android spike | TBD | TBD | 아니오 | TBD | 예 | TBD |
| iOS/iPadOS research | 처음에는 아니오 | 아니오 | 아니오 | 제한적 | 예, container만 | TBD |
| PWA/WASM | 아니오 | 아니오 | 아니오 | 아니오 | virtual만 | browser-limited |

## 8. PM-1 결과

PM-1은 `external::run`을 feature gate 뒤에 두는 방식이 아니라 trait 기반 어댑터 방식을 선택했다. 이유는 제품 형태다. Desktop, Android, iOS, PWA, remote-host target은 단순 compile-time availability가 아니라 실행 모델 자체가 다르다. Runtime adapter를 쓰면 하나의 Rust `shellcore`가 pure embedding mode를 제공하면서도 desktop `ash`는 명령을 spawn할 수 있다.

## 9. PM-2B Windows ConPTY 검증

Windows 네이티브의 `has_conpty`는 `portable-pty`의 Windows backend 위에서 검증한다. CI는 `cmd.exe`를 ConPTY interactive program으로 띄우고, 입력으로 `echo CONPTY_OK`와 `exit`를 보낸 뒤 marker가 PTY output으로 돌아오는지 확인한다.

이 검증은 Windows native 터미널 transport가 살아 있음을 보장하지만, Linux 전용 동적 감시(seccomp/fanotify/cgroups)를 Windows에서 지원한다는 뜻은 아니다. 그 제한은 `ai doctor --guardrails`의 platform-specific matrix에 별도 표시한다.
