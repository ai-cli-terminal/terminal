# 플랫폼 + 모바일 로컬 터미널 — 작업 흐름 계획

> **에이전트 작업자용:** 플랫폼 피벗 실행 계획으로 이 문서를 사용한다. 작업은 항목별로 진행하고, 각 slice가 완료될 때마다 `docs/TASK.md`를 동기화한다. `docs/HISTORY.md`는 동작이나 방향이 실제로 바뀐 경우에만 갱신한다.

**목표:** 2026-06-23 플랫폼 결정을 실행 가능한 작업으로 바꾼다. 제품 방향은 데스크톱과 모바일이 공유하는 `ash` 로컬 터미널이다. Android는 첫 모바일 로컬 터미널 타깃이고, iOS/iPadOS는 제품 형태 제약 때문에 연구 단계로 둔다. PWA/원격 승인은 모바일 제품 본체가 아니라 동반 기능으로 유지한다.

**정본 문서:**

- 매트릭스: `docs/superpowers/specs/2026-06-23-platform-target-matrix-design.md`
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
| Windows 네이티브 `ash.exe` | 진행 중 | Windows 실행 해석, argv spawn 계획, CI/local 스모크 추가 | exit code 보존, ConPTY 스모크, 릴리즈 산출물 정책 |
| Android 로컬 터미널 | 미착수 | 방향만 결정됨 | Rust core 경계, UI, worker process/thread, workspace/files, 외부 명령 전략 |
| iOS/iPadOS 로컬 터미널 | 연구 | 방향만 결정됨 | 자체 완결형 shellcore REPL, 파일 컨테이너, 정책-safe 명령 subset |
| PWA/mobile 동반 기능 | 개념 일부 | 원격 승인 mockup/design 계열 존재 | 로컬 터미널 대체가 아닌 승인/페어링/모니터링 역할 유지 |

**마지막 확인된 검증:** WSL `cargo test --features "storage tls remote"` 통과, `ash` smoke(`[ {size: 50} {size: 200} ] | where size > 100`)는 `size 200` 행만 반환.

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

점검 결과: `shellcore`의 desktop-only 결합은 `external.rs`의 `std::process::Command`, filesystem builtin(`cd`/`ls`), REPL process exit에 집중되어 있었다. Process spawn은 이제 `shellcore::external::ExternalRunner` 뒤에 있다. `Engine::pure()`는 `DisabledRunner`를 써서 모바일/PWA 임베딩이 literal, variable, table, `where`, `get`, `first`, `length`를 PATH lookup이나 OS spawn 없이 실행할 수 있게 한다. Filesystem builtin은 명시적 workspace 작업으로 남아 있으며, PM-3/PM-4에서 모바일 workspace 어댑터를 별도로 정해야 한다.

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
- [ ] exit code를 정확히 보존한다.
- [x] space, quote, backslash, PowerShell argument에 대한 quoting 테스트를 추가한다.
- [x] PowerShell을 `ash` grammar가 아니라 execution target/host로 취급한다.

진행: `src/shellcore/winexec.rs`가 순수 Windows resolution과 invocation classification을 정의한다. Windows `DesktopRunner`는 direct executable, `.cmd/.bat` script, `.ps1` script를 각기 다른 host로 라우팅한다. Spawn-plan 테스트는 space, quote, backslash, PowerShell-hosted `.ps1` script의 argv boundary를 고정한다. Linux/WSL은 기존 direct-spawn path를 유지한다.

**완료 기준:** `ash.exe`가 PowerShell인 척하지 않고도 Windows native 명령을 예측 가능하게 실행한다.

### PM-2B — ConPTY와 터미널 동작

- [ ] portable-pty ConPTY 동작을 interactive program으로 확인한다.
- [ ] capability 제한을 `ai doctor --guardrails` 또는 동등한 platform output에 기록한다.
- [ ] WSL과 native Windows 설치 문서를 분리해 유지한다.

### PM-2C — CI와 릴리즈

- [x] Windows `cargo build --bin ash`를 추가한다.
- [x] Windows `ash.exe` smoke를 추가한다.
- [ ] release asset에 `ai.exe`와 `ash.exe`를 함께 둘지, 별도 package로 둘지 결정한다.

---

## PM-3. Android 로컬 터미널 스파이크

**목표:** Android가 실제 모바일 로컬 터미널을 호스팅할 수 있음을 증명한다.

### PM-3A — 앱 shell 결정

- [ ] 다른 app shell이 정당화되지 않는 한 Kotlin/Compose + Rust FFI를 기본값으로 선택한다.
- [ ] 스파이크는 작게 유지한다: 한 화면, 입력 줄, 출력 pane, 로컬 workspace 선택기.
- [ ] process/userland 전략이 증명되기 전에는 Play Store 약속을 추가하지 않는다.

### PM-3B — Rust core 임베딩

- [ ] 최소 FFI 경계: `eval_line(input, session_state) -> output + updated_state`.
- [ ] 구조화 값을 JSON 또는 안정적인 typed bridge로 반환한다.
- [ ] panic이 FFI 경계를 넘어가지 않게 한다.
- [ ] list/record literal, `where`, variable, 가능한 경우 `cd`에 준하는 동작, error output을 테스트한다.

**완료 기준:** Android가 network나 desktop daemon 없이 순수 `shellcore` 명령을 로컬에서 평가할 수 있다.

### PM-3C — 터미널 UI와 worker model

- [ ] 평가/실행을 UI thread 밖에서 돌린다.
- [ ] shell worker를 thread로 둘지 process로 둘지 결정한다.
- [ ] output을 UI로 incremental stream한다.
- [ ] 긴 core operation에 대해 최소한 cancel/interrupt를 지원한다.

**완료 기준:** 터미널 세션이 바빠도 UI가 반응성을 유지한다.

### PM-3D — Workspace와 파일

- [ ] app-private workspace root를 정의한다.
- [ ] Android document API를 통한 import/export를 정의한다.
- [ ] desktop safety core의 secret/path masking boundary를 유지한다.
- [ ] 좁은 모바일 화면용 workspace/cwd 표시 모델을 추가한다.

### PM-3E — 외부 명령 전략

커밋 전 다음 세 접근을 비교한다.

| 선택지 | 의미 | 사용할 때 |
|---|---|---|
| `shellcore-only` | 구조화 셸만 제공하고 임의 OS process spawn 없음 | MVP 학습/증명 경로 |
| Termux-compatible | Termux/user-installed environment와 상호 운용 | userland 가치가 중요하고 정책이 허용할 때 |
| bundled minimal userland | 앱이 작은 명령 집합을 함께 제공 | 넓은 호환성보다 통제된 UX가 중요할 때 |

**완료 기준:** 한 선택지를 명시적 trade-off와 후속 구현 계획과 함께 선택한다.

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
