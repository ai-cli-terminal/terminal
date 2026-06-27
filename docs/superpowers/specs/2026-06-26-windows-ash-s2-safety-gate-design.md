# S2(코어) — `ash` 실행 경로 안전 게이트 결선 설계

> **작성일**: 2026-06-26
> **유형**: 단일 슬라이스 구현 spec (Windows ash 완성 로드맵 S2 코어).
> **상위**: `2026-06-26-windows-ash-completion-scoping-design.md` (#4 안전 게이트). 선행: S1(config) — `2026-06-26-windows-ash-s1-config-loading-design.md`.
> **정본**: 중앙 실행 파이프라인 `src/pipeline.rs`, 플랫폼 실행 계약 `2026-06-23-platform-execution-contract.md`.

## 1. 목표

`ash`의 외부 명령 실행을 **안전 게이트 뒤로** 보낸다. 현재 `shellcore::external::DesktopRunner`는 risk/policy/preview/undo를 우회하고 `std::process`로 바로 spawn한다. S2 코어는 ash 외부 실행을 `pipeline::execute`(risk→policy(Block)→preview→confirm→undo 백업→실행) 경유로 바꾼다. **실행 자체는 argv 직접 spawn을 유지**(셸 경유 안 함, 계약 §3)하고, 게이트 분석만 재구성한 명령 문자열로 한다.

**비목표(S2 후속 sub-slice로 이연)**: audit/usage 기록(storage-gated, `ai exec` 패리티), env 정책 좁히기(계약 §5). preview 렌더는 기존 수준 재사용(신규 정교화 안 함).

## 2. 경계 제약 (핵심)

게이트 로직(`pipeline`/`policy`/`risk`/`preview`/`undo`/`config`)은 데스크톱 모듈(`cfg(not(target_os="android"))`)이다. `shellcore`는 android/pure에도 컴파일되므로:

- **게이트 runner는 데스크톱 호스트 계층의 신규 모듈 `src/gated_runner.rs`**(`cfg(not(target_os="android"))`)에 둔다. 이 모듈이 `shellcore::external::ExternalRunner` trait을 구현한다.
- `shellcore::external`은 `DesktopRunner`/`DisabledRunner`를 그대로 유지한다(임베드/모바일/테스트용). shellcore가 데스크톱 게이트 모듈을 참조하지 않는다.
- 검증: `cargo check --lib --target aarch64-linux-android` green 유지.

## 3. 타깃 개선 — `spawn_inherit` 추출

`pipeline::Executor::run`은 exit code(`i32`)를 반환해야 한다. 현재 `shellcore::external`의 `run_desktop_command`는 spawn 후 `Value::Nothing`을 반환하고 비0이면 `[name: exit N]`을 **직접 출력**한다. 게이트 경로가 exit code를 파이프라인에 넘기려면 출력 없이 코드만 주는 spawn 함수가 필요하다.

`shellcore::external`에 다음을 추가(plain `std::process`/`winexec`만 사용 — 경계 안전, android 컴파일됨):

```rust
/// argv를 cwd/현재 env로 stdio 상속 spawn하고 exit code를 반환한다.
/// 출력하지 않는다(호출측이 결과를 처리). NotFound는 "command not found"로 bail.
pub fn spawn_inherit(name: &str, args: &[String], cwd: &Path) -> anyhow::Result<i32>;
```

기존 `run_desktop_command`(windows/non-windows 양쪽)를 `spawn_inherit`로 재구성하고, `DesktopRunner::run`은 `spawn_inherit` 호출 후 **기존 동작 보존**(비0이면 `[name: exit N]` 출력, `Value::Nothing` 반환, NotFound bail). 즉 DesktopRunner의 외부 동작은 불변, 내부만 공유 함수로.

## 4. `GatedRunner` (src/gated_runner.rs)

```rust
pub struct GatedRunner {
    profile: PolicyProfile,
    undo_dir: PathBuf,
    limits: UndoLimits,
    confirmer_is_tty: bool,   // 생성 시 stdin TTY 여부 캡처
}

impl GatedRunner {
    /// config의 활성 profile + 기본 undo dir/limits로 구성. 실패는 fail-soft(balanced/임시).
    pub fn from_environment() -> Self;
}

impl shellcore::external::ExternalRunner for GatedRunner {
    fn capabilities(&self) -> ExecutionCapabilities { ExecutionCapabilities::desktop_process() }
    fn run(&self, command: ExternalCommand<'_>) -> anyhow::Result<Value>;
}
```

`run` 흐름:
1. `let cmd = command_string(command.name, command.args);` (분석용 문자열 재구성).
2. `ExecConfig { profile: &self.profile, undo_dir: &self.undo_dir, limits: self.limits }`.
3. argv `Executor`(아래)와 `StdinConfirmer`, no-op sink 구성.
4. `let outcome = pipeline::execute(&cmd, &cfg, &executor, &mut confirmer, &mut sink)?;`
5. `outcome_message(&outcome, command.name)`로 매핑 → stderr 출력(있으면) + `Value` 반환.

`from_environment`:
- `let name = config::get_active_profile();`
- `let profile = PolicyProfile::by_name(&name).unwrap_or_else(PolicyProfile::balanced);`
- `let undo_dir = undo::default_undo_dir().unwrap_or_else(|_| std::env::temp_dir().join("ai-terminal-undo"));`
- `let limits = UndoLimits::defaults();`
- `confirmer_is_tty = std::io::stdin().is_terminal();`

## 5. 컴포넌트 (작고 순수하게 분리)

### 5.1 `command_string(name: &str, args: &[Value]) -> String`
각 `Value`를 `coerce_string()`으로 바꿔 `name` 뒤에 공백으로 join. 순수 함수. **한계 주석**: 공백/특수문자가 든 인자는 분석 토크나이즈가 부정확할 수 있으나, 안전 분석은 더 보수적으로 기우는 쪽이라 수용. 실행은 이 문자열을 쓰지 않는다(argv 직접).

### 5.2 argv `Executor`
```rust
struct ArgvExecutor<'a> { name: &'a str, args: Vec<String>, cwd: &'a Path }
impl pipeline::Executor for ArgvExecutor<'_> {
    fn run(&self, _command: &str, _sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
        shellcore::external::spawn_inherit(self.name, &self.args, self.cwd)
    }
}
```
- `_command`(파이프라인이 넘기는 문자열)와 `_sink`는 무시한다. **원본 argv로 직접 spawn**, stdio 상속(현행 ash 동작 유지). args는 `command.args`를 `coerce_string()`한 것.

### 5.3 `StdinConfirmer`
```rust
struct StdinConfirmer { is_tty: bool }
impl pipeline::Confirmer for StdinConfirmer {
    fn confirm(&mut self, req: &ConfirmRequest) -> bool;
}
```
- 비-TTY(`!is_tty`) → **즉시 false(fail-closed)**, stdin 미소비. (파이프/스크립트 입력이 답으로 오인되는 것 방지)
- TTY → 명령·등급·factors·backup_files + preview 렌더를 stderr/stdout에 표시하고 `[y/N]` 프롬프트. stdin 한 줄 읽어 `y`/`yes`(대소문자 무시)만 true, 그 외/EOF는 false.
- preview 렌더: 확인 표시는 `req`의 command·level·factors·backup_files를 출력하고, `req.preview`(PreviewPlan) 요약을 덧붙인다. 상세 PreviewPlan 렌더 함수가 `main.rs`의 `format_preview`에만 있으면 **`preview` 모듈로 공개 함수를 추출**해 양쪽이 공유한다(타깃 개선). 신규 렌더 디자인은 만들지 않음.
- 단위테스트를 위해 입력 판정 코어를 `decide_confirm(answer: &str) -> bool`(순수)로 분리.

### 5.4 `outcome_message(outcome: &ExecOutcome, name: &str) -> (Option<String>, Value)`
순수 매퍼:
- `Blocked{level, factors}` → `(Some("ash: 정책상 차단됨 [{level}] — {factors join}"), Value::Nothing)`.
- `Declined` → `(Some("ash: 취소됨"), Value::Nothing)`.
- `BackupRefused(reason)` → `(Some(format!("ash: 백업 거부({reason}) — 실행 중단")), Value::Nothing)`.
- `Ran{exit_code, ..}` → exit 0이면 `(None, Value::Nothing)`, 비0이면 `(Some("[{name}: exit {exit_code}]"), Value::Nothing)`.

## 6. 주입 — `repl::run` + `ash.rs`

`shellcore::repl::run` 시그니처 확장(S1의 `ReplSettings` 주입에 runner 추가):
```rust
pub fn run(settings: ReplSettings, runner: Box<dyn external::ExternalRunner>) -> Result<()> {
    let mut engine = Engine::with_external_runner(runner);
    apply_settings(&mut engine, &settings);
    // ... 기존 루프 ...
}
```
- shellcore는 trait 객체만 받는다(데스크톱 게이트 모름 — 경계 유지).

`src/bin/ash.rs`:
```rust
let runner: Box<dyn ai_terminal::shellcore::external::ExternalRunner> =
    Box::new(ai_terminal::gated_runner::GatedRunner::from_environment());
ai_terminal::shellcore::repl::run(settings, runner)
```

## 7. 에러 처리 원칙

- **fail-soft**: config/undo dir 해석 실패가 ash를 종료시키지 않는다(기본 profile/임시 undo dir). 게이트 비-Ran 결과는 메시지 후 REPL 지속.
- `pipeline::execute`가 `Err`를 반환(드묾: undo 백업 I/O 오류 등)하면 `GatedRunner::run`이 그 에러를 그대로 반환 → REPL이 `error: ...` 출력 후 지속(기존 eval 오류 처리와 동일).

## 8. 테스트

단위:
- `command_string`: `("rm", ["-rf", "/x"]) → "rm -rf /x"`; 인자 없는 경우; 공백 인자 한계 케이스 문서화 테스트.
- `decide_confirm`: `"y"`/`"Y"`/`"yes"`→true; `""`/`"n"`/`"no"`/`"maybe"`→false.
- `outcome_message`: 4개 variant 각각 (메시지 유무 + Value::Nothing).

순수 3종(`command_string`/`decide_confirm`/`outcome_message`)이 게이트의 신규 로직 전부다. 게이트 오케스트레이션 자체(`pipeline::execute`)는 이미 `pipeline.rs` 테스트로 검증됨 — `GatedRunner::run`의 배선(재구성→execute→매핑)은 thin하므로 **별도 mock 단위테스트를 만들지 않고 e2e로 커버**(중복 회피).

e2e(WSL, ash 바이너리):
- 안전 명령(`echo hi` 또는 `ls`) → 실행·출력.
- `rm -rf /` → 차단(미실행), "정책상 차단" 메시지.
- High 명령 + 비-TTY stdin → fail-closed(미실행).
- rm 기존파일 → 백업 생성 후 실행, `ai undo last`로 복구 가능.
- android 경계: `cargo check --lib --target aarch64-linux-android` green.

## 9. 수용 기준

1. ash 외부 명령이 `pipeline::execute` 게이트(risk→policy→preview→confirm→undo→argv 실행)를 통과한다.
2. Critical=차단, High=확인(비-TTY는 fail-closed), 파일 변경은 undo 백업 후 실행.
3. 실행은 argv 직접 spawn 유지(셸 미경유), stdio 상속.
4. `shellcore`는 데스크톱 게이트 모듈 미참조 — android cdylib 빌드 불변.
5. `DesktopRunner` 외부 동작 불변(`spawn_inherit` 추출 후에도).
6. `cargo fmt --all -- --check`(실제 `cargo fmt --all` 적용 후) · `cargo clippy --all-targets --features "storage tls remote" -D warnings` · `cargo test --features "storage tls remote"` green.

## 10. 후속(S2 sub-slice)

- **S2b audit/usage**: 게이트 결과를 storage에 기록(`ai exec`의 `record_exec`/`shell_outcome_audit` 패리티, storage feature gate).
- **S2c env 정책 좁히기**: 상속 env를 context/mask 정책으로 좁힘(계약 §5).
