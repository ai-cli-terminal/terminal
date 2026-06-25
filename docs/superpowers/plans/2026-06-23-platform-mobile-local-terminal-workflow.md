# 플랫폼 + 모바일 로컬 터미널 — 작업 흐름 계획

> **에이전트 작업자용:** 플랫폼 피벗 실행 계획으로 이 문서를 사용한다. 작업은 항목별로 진행하고, 각 slice가 완료될 때마다 `docs/TASK.md`를 동기화한다. `docs/HISTORY.md`는 동작이나 방향이 실제로 바뀐 경우에만 갱신한다.

**목표:** 2026-06-23 플랫폼 결정을 실행 가능한 작업으로 바꾼다. 제품 방향은 데스크톱과 모바일이 공유하는 `ash` 로컬 터미널이다. Android는 첫 모바일 로컬 터미널 타깃이고, iOS/iPadOS는 제품 형태 제약 때문에 연구 단계로 둔다. PWA/원격 승인은 모바일 제품 본체가 아니라 동반 기능으로 유지한다.

**정본 문서:**

- 매트릭스: `docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md`
- Android spike: `docs/superpowers/specs/2026-06-23-android-local-terminal-spike.md`
- Android external command strategy: `docs/superpowers/specs/2026-06-24-android-external-command-strategy.md`
- Termux opt-in bridge design: `docs/superpowers/specs/2026-06-25-termux-compatible-opt-in-bridge-design.md`
- 독립 셸: `docs/superpowers/specs/2026-06-05-independent-shell-s0-core-design.md`
- 로드맵: `docs/superpowers/specs/2026-06-05-phase3-roadmap-design.md`
- 백로그: `docs/TASK.md`
- 공통 개발 흐름: `docs/WORKFLOW.md`

---

## 0. 현재 진행 스냅샷

| 영역 | 상태 | 근거 | 다음 과제 |
|---|---|---|---|
| `ai` 릴리즈 라인 | 완료 | `Cargo.toml` 버전 `0.2.2`, Linux/Windows 릴리즈 문서와 스크립트 존재 | `ash`가 성장하는 동안 릴리즈 연속성 유지 |
| Phase 1/2 안전 코어 | 완료 | risk, policy, masking, preview, undo, usage, context, guardrails, provider, gateway, dispatch 모듈 | 성숙한 안전 경로를 `ash` 실행에 연결 |
| 원격 승인 기반 | 일부 완료 | M0~M1 슬라이스 4a 구현 완료: gate, Noise, validation, daemon substrate, framed transport | 실제 리스너, 페어링, 디바이스 등록, 게이트-디바이스 흐름, PWA 동반 기능 |
| `ash` / `shellcore` | 일부 완료 | `[[bin]] name = "ash"`, `src/bin/ash.rs`, `src/shellcore/*`; 값 모델, lexer/parser/engine, builtins, 외부 실행, REPL; S1a `where` 필터 존재 | 플랫폼 어댑터, 라인 에디터, 히스토리, 설정, AI/safety gate 결선 |
| 플랫폼 목표 매트릭스 | 완료 | 2026-06-23 매트릭스가 Linux/WSL/Windows/Git Bash/PowerShell/Android/iOS/PWA 타깃 정의 | 매트릭스를 구현 슬라이스로 전환 |
| Windows 네이티브 `ash.exe` | 진행 중 | Windows 실행 해석, argv spawn 계획, CI/local 스모크, exit code 보존, ConPTY 스모크, Git Bash/MSYS profile 계약 | line editor/history/config, AI/safety gate integration |
| Android 로컬 터미널 | 진행 중 | Kotlin/Compose skeleton, worker thread + stream/cancel JVM contract, Rust `MobileShell` pure core boundary, JNI bridge + instrumentation smoke, app-private workspace/cwd boundary, document import/export, full-ABI JNI packaging CI, shellcore-only MVP와 PM-3E/PM-3F 외부 명령 전략/bridge design, T0 probe substrate | Termux T0 real-device smoke, T1 helper-backed stream/cancel, imported file UX |
| iOS/iPadOS 로컬 터미널 | 연구 | 방향만 결정됨 | 자체 완결형 shellcore REPL, 파일 컨테이너, 정책-safe 명령 subset |
| PWA/mobile 동반 기능 | 개념 일부 | 원격 승인 mockup/design 계열 존재 | 로컬 터미널 대체가 아닌 승인/페어링/모니터링 역할 유지 |

**마지막 확인된 검증:** Android `gradle -p android :app:testDebugUnitTest :app:assembleDebugAndroidTest :app:assembleDebug` 통과, `git diff --check` 통과. 이 Windows 세션은 `cargo`가 PATH에 없어 Rust unit test와 Android Rust `.so` 실제 빌드는 실행하지 못했다.

---

## 1. 실행 순서

```text
PM-0 문서/백로그 동기화
  -> PM-1 공유 shellcore 플랫폼 경계
  -> PM-2 Windows 네이티브 ash.exe
  -> PM-3 Android 로컬 터미널 스파이크
  -> PM-4 iOS/iPadOS 연구 스파이크
  -> PM-5 RA/PWA 동반 기능 재사용
  -> PM-6 패키징과 공개 문서
```

RA/PWA 작업이 모바일 로컬 터미널 트랙을 막지 않게 한다. RA는 가치가 있지만, 모바일 제품의 본체는 이제 로컬 터미널이다.

---

## PM-0. 문서와 백로그 동기화

**목표:** 모든 계획 문서가 같은 방향을 말하게 한다.

**파일:** `README.md`, `docs/TASK.md`, `docs/WORKFLOW.md`, `docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md`, 이 계획 문서.

- [x] 플랫폼 매트릭스가 모바일을 PWA가 아닌 로컬 터미널로 정의한다.
- [x] TASK에 플랫폼 피벗 섹션이 있다.
- [x] WORKFLOW에 플랫폼/모바일 작업 흐름이 있다.
- [x] README가 목표 매트릭스와 이 workflow 계획을 노출한다.
- [x] 구현이 시작된 뒤 HISTORY 항목을 추가했다.

**검증:**

```powershell
rg -n "모바일 로컬 터미널|Android|iOS|PWA|platform-mobile-local-terminal-workflow" README.md docs
git diff --check
```

---

## PM-1. 공유 `shellcore` 플랫폼 경계

**목표:** 하나의 Rust 셸 언어 코어를 유지하면서 host별 실행을 분리한다.

### PM-1A — 코어 순수성 점검

**파일:** `src/shellcore/*`

- [x] `shellcore` 안의 desktop-only 의존성을 나열한다.
- [x] 순수 평가와 외부 process 실행을 분리한다.
- [x] `external::run`을 trait 기반 어댑터로 바꿀지, desktop runner 뒤 feature gate로 둘지 결정한다.
- [x] parser/evaluator/builtins가 OS process spawn 없이 동작함을 증명하는 `mobile-core`에 준하는 테스트를 추가한다.

점검 결과: `shellcore`의 desktop-only 결합은 `external.rs`의 `std::process::Command`, filesystem builtin(`cd`/`ls`), REPL process exit에 집중되어 있었다. Process spawn은 이제 `shellcore::external::ExternalRunner` 뒤에 있다. `Engine::pure()`는 `DisabledRunner`를 써서 모바일/PWA 임베딩이 literal, variable, table, `where`, `get`, `first`, `length`를 PATH lookup이나 OS spawn 없이 실행할 수 있게 한다. Filesystem builtin은 optional workspace root 경계 뒤에서만 host 파일을 보며, Android는 app-private workspace root를 `MobileShell` state로 넘긴다.

**완료 기준:** `shellcore`를 모바일 UI 코드에 임베드해도 PTY, desktop env, unrestricted process spawn을 실수로 요구하지 않는다.

### PM-1B — 플랫폼 실행 계약

**설계 산출물:** spec을 추가하거나 matrix를 갱신해 다음을 정의한다.

- command name resolution
- argv quoting
- cwd와 workspace root
- env allowlist/denylist
- stdout/stderr stream model
- exit code model
- capability flags: `can_spawn`, `has_pty`, `has_conpty`, `has_userland`, `can_write_workspace`, `can_network`

**완료 기준:** Windows, Android, iOS, PWA가 각자 실행 계약의 어느 부분을 구현하는지 말할 수 있다.

산출물: `docs/superpowers/specs/2026-06-23-platform-execution-contract.md`.

### PM-1C — 공유 smoke 테스트

- [x] Linux/WSL에서 실행되는 `ash` smoke fixture를 추가한다.
- [x] Windows adapter가 생긴 뒤 Windows `ash.exe` smoke를 추가한다.
- [x] 외부 명령을 호출하지 않는 순수 `shellcore` 테스트를 추가한다.

**기준 smoke:**

```bash
printf '[{size: 50} {size: 200}] | where size > 100\nexit\n' | cargo run --bin ash
```

기대 결과: `size 200` 행만 출력된다.

Windows CI/local smoke도 같은 `ash.exe` 구조화 명령을 실행하고, Windows adapter를 거친 `.cmd`/`.ps1` 외부 실행을 확인한다.

---

## PM-2. Windows Native `ash.exe`

**목표:** Windows를 `ai.exe` 호스트만이 아니라 1급 로컬 터미널 타깃으로 만든다.

### PM-2A — Windows 실행 adapter

- [x] direct spawn, `cmd.exe /c`, PowerShell invocation 규칙을 정의한다.
- [x] `.exe`, `.cmd`, `.bat`, `.ps1`에 대한 PATH/PATHEXT resolution을 구현한다.
- [x] exit code를 정확히 보존한다(`.cmd` exit 7, `.ps1` exit 9 smoke).
- [x] space, quote, backslash, PowerShell argument에 대한 quoting 테스트를 추가한다.
- [x] PowerShell을 `ash` grammar가 아니라 execution target/host로 취급한다.

진행: `src/shellcore/winexec.rs`가 순수 Windows resolution과 invocation classification을 정의한다. Windows `DesktopRunner`는 direct executable, `.cmd/.bat` script, `.ps1` script를 각기 다른 host로 라우팅한다. Spawn-plan 테스트는 space, quote, backslash, PowerShell-hosted `.ps1` script의 argv boundary를 고정한다. Linux/WSL은 기존 direct-spawn path를 유지한다.

**완료 기준:** `ash.exe`가 PowerShell인 척하지 않고도 Windows native 명령을 예측 가능하게 실행한다.

### PM-2B — ConPTY와 터미널 동작

- [x] portable-pty ConPTY 동작을 interactive program으로 확인한다.
- [x] capability 제한을 `ai doctor --guardrails` 또는 동등한 platform output에 기록한다.
- [x] WSL과 native Windows 설치 문서를 분리해 유지한다.

진행: Windows 전용 `pty` 단위 smoke가 `cmd.exe`를 ConPTY 세션으로 띄우고 `CONPTY_OK` marker round-trip을 확인한다. Windows CI와 `scripts/smoke.ps1`은 이 테스트를 실행한다. `guardrails::Platform::Windows`와 `Windows ConPTY` capability가 추가되어 `ai doctor --guardrails`가 Windows를 `Other`로 뭉개지 않고, Linux 동적 감시 제한도 함께 표시한다. 설치 안내는 `docs/INSTALL.md`로 분리했다.

### PM-2C — Git Bash/MSYS profile

- [x] MSYS bridge를 Windows native와 섞지 않는 profile 계약을 정의한다.
- [x] path conversion은 MSYS bridge에서만 수행한다고 명시한다.
- [x] POSIX tool discovery(`/usr/bin`, `/mingw64/bin` 등)는 명시 opt-in profile에서만 수행한다고 명시한다.
- [x] `shellcore::msys` 순수 profile selection 테스트를 추가한다.

진행: `AI_TERMINAL_WINDOWS_PROFILE` 계약을 추가했다. 기본값은 `native`이며, Git Bash/MSYS 환경에서도 자동 bridge로 들어가지 않는다. `msys` profile은 `MSYSTEM` 또는 `MSYSTEM_PREFIX`가 있는 환경에서만 선택 가능하다. 실제 MSYS bridge runner와 smoke는 후속 구현 대상이다.

### PM-2D — CI와 릴리즈

- [x] Windows `cargo build --bin ash`를 추가한다.
- [x] Windows `ash.exe` smoke를 추가한다.
- [x] release asset에 `ai`와 `ash`를 별도 바이너리 asset으로 함께 배포한다(v0.2.4, 각 checksum 포함).

---

## PM-3. Android 로컬 터미널 스파이크

**목표:** Android가 실제 모바일 로컬 터미널을 호스팅할 수 있음을 증명한다.

### PM-3A — 앱 shell 결정

- [x] 다른 app shell이 정당화되지 않는 한 Kotlin/Compose + Rust FFI를 기본값으로 선택한다.
- [x] 스파이크는 작게 유지한다: 한 화면, 입력 줄, 출력 pane, 로컬 workspace 선택기.
- [x] process/userland 전략이 증명되기 전에는 Play Store 약속을 추가하지 않는다.

진행: `docs/superpowers/specs/2026-06-23-android-local-terminal-spike.md`에서 Kotlin/Compose + Rust core binding을 기본값으로 고정했다. 첫 약속은 "완전 Linux 터미널"이 아니라 Android에서 로컬 `ash` 구조화 셸 코어를 평가하는 것이다.

### PM-3B — Rust core 임베딩

- [x] 최소 FFI 경계: `eval_line(input, session_state) -> output + updated_state`.
- [x] 구조화 값을 JSON 또는 안정적인 typed bridge로 반환한다.
- [x] panic이 FFI 경계를 넘어가지 않게 한다.
- [x] list/record literal, `where`, variable, error output을 테스트한다.

**완료 기준:** Android가 network나 desktop daemon 없이 순수 `shellcore` 명령을 로컬에서 평가할 수 있다.

진행: `src/mobile.rs`의 `MobileShell`이 `Engine::pure()`를 감싼다. `MobileEvalResult`는 `output_json`, `output_text`, `error`, updated `state`를 반환한다. `src/mobile_jni.rs`와 Android `NativeShellBridge`가 이 계약을 JSON-in/JSON-out JNI 호출로 연결했다. 외부 command spawn은 PATH lookup 전에 `external execution disabled`로 실패한다. `workspace_root`도 state에 포함되어 filesystem builtin은 app-private workspace 밖을 거부한다. `android/build-rust-jni.ps1`과 CI는 `arm64-v8a`, `armeabi-v7a`, `x86`, `x86_64` 전체 ABI `.so` 빌드와 Gradle `verifyNativeLibraries` 검증을 수행한다. CI emulator smoke는 실제 `NativeShellBridge`가 packaged `.so`를 로드하고 Rust `MobileShell`까지 왕복하는지 `connectedDebugAndroidTest`로 검증한다.

### PM-3C — 터미널 UI와 worker model

- [x] 평가/실행을 UI thread 밖에서 돌린다.
- [x] shell worker를 thread로 둘지 process로 둘지 결정한다.
- [ ] output을 UI로 incremental stream한다.
- [ ] 긴 core operation에 대해 최소한 cancel/interrupt를 지원한다.

**완료 기준:** 터미널 세션이 바빠도 UI가 반응성을 유지한다.

진행: `android/` skeleton을 추가했다. `TerminalViewModel`은 Compose state를 소유하고, `ShellWorker`는 single-thread executor에서 `NativeShellBridge.evalLine`을 호출한 뒤 main thread로 결과를 post한다. 이번 slice는 thread worker를 선택했다. JVM unit test는 bridge 평가가 worker thread에서 일어나고 result callback이 `ResultPoster` 뒤로 post되는 계약, bridge failure가 error result로 변환되는 계약을 고정한다. `ShellStreamEvent`/`ShellRunHandle` 계약은 complete-result bridge를 `Started -> Stdout/Stderr -> Finished` event stream으로 어댑트하고, completion 전 cancel은 `Started -> Cancelled`로 final result 적용을 막는다. 별도 process는 실제 native userland/PTY가 붙는 시점에 재평가한다.

### PM-3D — Workspace와 파일

- [x] app-private workspace root를 정의한다.
- [x] Android document API를 통한 import/export 경계를 정의한다.
- [x] desktop safety core의 secret/path masking boundary를 유지한다.
- [x] 좁은 모바일 화면용 workspace/cwd 표시 모델을 추가한다.

진행: Android 앱은 `Context.filesDir/ash-workspace`를 기본 workspace root로 만들고, `ShellState.cwd`와 JNI `workspace_root`를 이 경로로 초기화한다. Rust `Engine`은 optional workspace root를 갖고, 모바일 `cd`/`ls`는 root 밖 경로를 `workspace boundary` 오류로 거부한다. 상태바는 전체 경로 대신 workspace/cwd basename과 `core / private` capability만 표시한다. Document tree는 직접 mount하지 않으며, Android Storage Access Framework picker로 선택한 파일을 app-private workspace 안으로 복사(import)하고 transcript를 사용자가 고른 URI로 쓴다(export). workspace root 밖 파일은 명시 import/export 복사 경로로만 다룬다.

### PM-3D2 — Android Rust `.so` 전체 ABI/CI 패키징

- [x] `arm64-v8a`, `armeabi-v7a`, `x86`, `x86_64` 대상 빌드를 자동화한다.
- [x] `android/build-rust-jni.ps1`를 CI에서도 실행 가능한 형태로 다듬는다.
- [x] `jniLibs` 패키징 후 native smoke를 추가한다.

진행: build script의 기본 Rust target은 네 ABI 전체이며, Windows/Linux/macOS NDK host tag와 linker 이름을 감지한다. CI의 `android JNI packaging` job은 Android SDK/NDK 설치, Rust JNI build, Gradle `:app:verifyNativeLibraries`, JVM unit test, x86_64 emulator `:app:connectedDebugAndroidTest`, `:app:assembleDebug`를 순서대로 실행한다. Native smoke는 packaging smoke와 live JNI instrumentation smoke를 모두 포함한다.

### PM-3E — 외부 명령 전략

다음 세 접근을 비교했다.

| 선택지 | 의미 | 사용할 때 |
|---|---|---|
| `shellcore-only` | 구조화 셸만 제공하고 임의 OS process spawn 없음 | 이번 PM-3 첫 slice 기준선 |
| Termux-compatible | 사용자가 설치한 Termux-compatible runtime과 명시 bridge로 상호 운용 | 다음 spike. userland 가치가 중요하고 explicit opt-in UX가 성립할 때 |
| bundled minimal userland | 앱이 작은 명령 집합을 함께 제공 | Termux bridge가 불충분하고, 보안 업데이트/라이선스/ABI 패키징 책임을 CI로 감당할 수 있을 때 |

**완료 기준:** 한 선택지를 명시적 trade-off와 후속 구현 계획과 함께 선택한다.

진행: `docs/superpowers/specs/2026-06-24-android-external-command-strategy.md`에서 비교를 완료했다. Android MVP는 계속 `shellcore-only`다. 다음 구현 후보는 Termux-compatible opt-in bridge이며, 다른 앱 UID/샌드박스 경계를 direct PATH처럼 취급하지 않는다. Bundled minimal userland는 4 ABI 패키징, CVE update, 라이선스, binary provenance, 배포 크기 책임 때문에 보류한다.

PM-3F 설계: `docs/superpowers/specs/2026-06-25-termux-compatible-opt-in-bridge-design.md`에서 bridge를 T0 `RUN_COMMAND` completion probe와 T1 helper-backed stream/cancel protocol로 나눴다. T0는 permission/setup과 final stdout/stderr/exit만 검증한다. 실제 incremental stream, cancel, workspace staging은 T1 helper가 job id, NDJSON event log, cancel token을 관리할 때만 ready로 표시한다.

T0 substrate 진행: Android manifest package visibility/permission, `AndroidTermuxBridge`, PendingIntent result service, `Probe Termux` UI, pure result decoding tests를 추가했다. 실제 Termux 설치 기기(`SM_F956N / R3CX60P3R5K`)에서 `allow-external-apps`, stdout/stderr, non-zero exit smoke까지 통과했다.

후속:

- [x] Termux-compatible bridge design spike를 작성한다.
- [x] T0 `RUN_COMMAND` probe substrate를 구현한다: availability, permission, echo probe, result receiver.
- [x] T0 real-device smoke를 실행한다: `allow-external-apps`, `pwd`, stderr, non-zero exit.
- [x] T1 helper protocol substrate를 구현한다: argv request JSON, NDJSON event-to-`ShellStreamEvent` mapping.
- [x] T1 helper event file polling과 cancel file-backed `ShellRunHandle.cancel()` 계약을 구현한다.
- [ ] T1 helper protocol을 구현한다: long-running stdout, cancel token, large output, workspace staging.
- [ ] bridge output을 `ShellStreamEvent`와 `ShellRunHandle.cancel()` 계약에 맞춘다.
- [ ] imported file UX와 bridge workspace sharing 모델을 연결한다.

---

## PM-4. iOS/iPadOS 연구 스파이크

**목표:** Linux 동작을 과장하지 않으면서 iOS 로컬 터미널의 정책-safe 형태를 판단한다.

- [ ] self-contained `shellcore` REPL prototype을 만든다.
- [ ] 앱 동작을 바꾸는 code를 download/execute하지 않는다.
- [ ] 파일은 app container 또는 사용자가 선택한 document location 안에 둔다.
- [ ] 허용 명령 subset을 정의한다: 순수 구조화 셸 명령 우선.
- [ ] App Store 문구는 policy review 뒤에 쓰고, 먼저 TestFlight로 검증한다.

**완료 기준:** iOS가 제한적 로컬 구조화 터미널로 출시 가능한지, 그리고 정직하게 약속할 수 없는 것이 무엇인지 연구 노트에 남긴다.

---

## PM-5. RA/PWA Companion 재사용

**목표:** remote approval을 모바일 터미널 본체가 아니라 desktop/mobile `ash`의 companion으로 재사용한다.

- [ ] RA-1~RA-4를 desktop daemon/listener/pairing/gate flow 기준으로 완료한다.
- [ ] RA-5 PWA를 approval/pairing/monitoring companion으로 유지한다.
- [ ] Android/iOS 로컬 터미널이 성공한 뒤에만 같은 device identity model을 사용하게 한다.
- [ ] 로컬 Android `ash`를 실행하는 데 phone companion을 요구하지 않는다.

**완료 기준:** 사용자가 다음 차이를 이해할 수 있다.

```text
Mobile ash 앱 = phone/tablet 위 로컬 터미널.
PWA companion  = 승인, 페어링, 모니터링, 데모.
```

---

## PM-6. 패키징과 공개 문서

- [ ] 제품명과 바이너리 이름을 결정한다: `ai`, `ash`, mobile app name.
- [ ] README table을 현재 지원 범위와 목표 매트릭스로 분리한다.
- [ ] release artifact가 생기면 `ash` 설치 안내를 추가한다.
- [ ] 모바일 상태 문구를 추가한다: Android spike, iOS research, PWA companion.
- [ ] `../document/` v3.3에서 terminal repo 플랫폼 피벗으로 넘어온 migration note를 추가한다.

---

## 최종 검증 체크리스트

- [ ] `git diff --check`
- [ ] `cargo test shellcore`
- [ ] `cargo test --features "storage tls remote"`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --features "storage tls remote" -- -D warnings`
- [ ] WSL/Linux `ash` smoke
- [ ] Windows adapter가 있는 경우 Windows `ash.exe` smoke
- [ ] Android pure `shellcore` spike 결과 문서화
- [ ] iOS policy/research 결과 문서화

각 PM slice 완료 뒤 `docs/TASK.md`를 갱신하고, 구현 변경이 있으면 `docs/HISTORY.md` 항목을 추가한다.
