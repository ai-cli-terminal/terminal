# S3 — `ash` Line Editor 설계

> **작성일**: 2026-06-26
> **유형**: 단일 슬라이스 구현 spec (Windows ash 완성 로드맵 S3).
> **상위**: `2026-06-26-windows-ash-completion-scoping-design.md` (#1 line editor). 선행: S1(config), S2(safety gate).
> **참고**: reedline `/nushell/reedline` (crossterm 위에 빌드, 이미 타깃 게이트된 의존).

## 1. 목표

인터랙티브 `ash`에 **입력 편집·커서 이동·Ctrl-C(라인 취소)·Ctrl-D(EOF 종료)·in-session history(↑↓ 회상)**를 제공한다. Windows 콘솔/ConPTY와 unix 양쪽에서 동작한다. 현재 `repl::run`은 `stdin.read_line`만 쓴다(편집 없음).

라인 읽기를 **`LineReader` 트레이트로 추상화**해 reedline 기반 에디터를 데스크톱 호스트 계층에서 주입한다. `shellcore`는 reedline을 모른다(android/pure 빌드 불변).

## 2. 경계 제약 (핵심)

- `shellcore::repl`은 `LineReader` 트레이트와 `std`만 참조한다. reedline/crossterm 직접 참조 금지.
- reedline 기반 구현은 데스크톱 신규 모듈 `src/line_editor.rs`(`cfg(not(target_os="android"))`)에 둔다.
- `reedline`은 **`[target.'cfg(not(target_os="android"))'.dependencies]`** 에 추가한다(crossterm/ratatui/portable-pty 옆). android cdylib 빌드 불변(`cargo check --lib --target aarch64-linux-android` green).

## 3. 데이터 모델 + 트레이트 (shellcore)

`src/shellcore/repl.rs`에 추가한다(`ReplSettings` 옆 — 라인 읽기는 REPL의 관심사라 같은 파일):

```rust
/// 한 줄 읽기 결과.
pub enum ReadOutcome {
    Line(String),   // 제출된 한 줄(개행 제거)
    Eof,            // Ctrl-D / 입력 종료 → REPL 종료
    Interrupted,    // Ctrl-C → 현재 라인 취소, REPL 계속
}

/// 프롬프트를 표시하고 한 줄을 읽는다. 구현은 호스트가 주입한다.
pub trait LineReader {
    fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome>;
}
```

## 4. `StdinLineReader` (shellcore 기본, std만)

현행 동작을 트레이트로 옮긴 것. 임베드/비-TTY/테스트용.

```rust
pub struct StdinLineReader;
impl LineReader for StdinLineReader {
    fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome> {
        // 프롬프트 출력 + flush, stdin 한 줄 읽기.
        // n==0 → ReadOutcome::Eof. 그 외 → Line(trim_end). (편집/Ctrl-C 없음)
    }
}
```

- 테스트 가능하도록 읽기 로직 코어를 `read_outcome_from(reader: &mut impl BufRead, prompt) -> io::Result<ReadOutcome>` 같은 Read-심 함수로 분리(프롬프트 출력은 stdout, 읽기는 주입 reader).

## 5. `ReedlineReader` (데스크톱, src/line_editor.rs)

```rust
pub struct ReedlineReader {
    editor: reedline::Reedline,
}
impl ReedlineReader {
    /// 실패 시 호출측이 StdinLineReader로 폴백할 수 있게 Result 반환.
    pub fn new() -> anyhow::Result<Self>;   // Reedline::create() + 기본 in-session history
}
impl shellcore::...::LineReader for ReedlineReader {
    fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome> {
        let p = AshPrompt::new(prompt);
        match self.editor.read_line(&p) {
            Ok(Signal::Success(line)) => Ok(ReadOutcome::Line(line)),
            Ok(Signal::CtrlD) => Ok(ReadOutcome::Eof),
            Ok(Signal::CtrlC) => Ok(ReadOutcome::Interrupted),
            Ok(_) => Ok(ReadOutcome::Interrupted),  // HostCommand/ExternalBreak: 보수적으로 취소
            Err(e) => Err(e),
        }
    }
}
```

- **`AshPrompt`**: reedline `Prompt` 트레이트 구현. 전달받은 prompt 문자열(예: `~/proj〉 `)을 `render_prompt_left`(또는 indicator)에 넣고 나머지(right/multiline/history-search indicator)는 빈 문자열/기본값. cwd 계산은 계속 `repl`의 `make_prompt`가 하고 문자열만 넘긴다.
- **history**: S3는 reedline 기본 in-session(메모리) history로 ↑↓ 회상만. **파일 영속/민감명령 제외는 S4**.

## 6. 주입 + reader 선택 (repl + ash)

`repl::run` 시그니처 확장:
```rust
pub fn run(
    settings: ReplSettings,
    runner: Box<dyn ExternalRunner>,
    reader: Box<dyn LineReader>,
) -> Result<()> {
    // 루프:
    //   let prompt = make_prompt(&engine.cwd, home);
    //   match reader.read_line(&prompt)? {
    //       ReadOutcome::Line(l) => { eval_line(...) ... }
    //       ReadOutcome::Eof => break,
    //       ReadOutcome::Interrupted => continue,   // 라인 취소
    //   }
}
```
- `Interrupted`는 빈 라인처럼 계속(취소). `Line`이 빈 문자열이면 기존처럼 continue.

`ash.rs` — TTY 여부로 reader 선택, reedline 실패는 fail-soft:
```rust
let reader: Box<dyn LineReader> = if std::io::stdin().is_terminal() {
    match ReedlineReader::new() {
        Ok(r) => Box::new(r),
        Err(e) => { eprintln!("ash: 라인에디터 초기화 실패({e}) — 기본 입력 사용"); Box::new(StdinLineReader) }
    }
} else {
    Box::new(StdinLineReader)   // 파이프/스크립트/CI: 라인 기반
};
ash_repl::run(settings, runner, reader)
```

## 7. 의존성

`Cargo.toml`의 `[target.'cfg(not(target_os="android"))'.dependencies]`에 `reedline`(crossterm 0.28 호환 버전, 플랜에서 정확 핀) 추가.

## 8. 에러 처리

- reedline init 실패 → `StdinLineReader` 폴백(§6), 세션 비중단.
- `read_line` I/O 오류 → `repl::run`이 그대로 반환(기존 `?` 동작) → ash가 `ash: {e}` 출력 후 exit 1.
- 비-TTY에서 reedline은 쓰지 않는다(TTY 분기). 따라서 파이프/리다이렉트 입력은 항상 `StdinLineReader`.

## 9. 테스트

단위:
- `ReadOutcome` signal 매핑이 필요한 부분은 순수 헬퍼로(Signal→ReadOutcome) 분리해 테스트. (reedline 타입에 의존하므로 데스크톱 모듈에서, 가능한 범위)
- `StdinLineReader`의 `read_outcome_from`: 빈 입력→Eof, 한 줄→Line(trim), 여러 줄 순차.
- `AshPrompt`의 `render_prompt_left`(또는 indicator)가 주입 문자열을 반환.

e2e(WSL, 비-TTY 파이프):
- `printf 'echo hi\nexit\n' | ash` → 정상 실행(StdinLineReader 경로). S2 게이트 e2e(`rm -rf /` 차단)도 계속 통과.
- android 경계: `cargo check --lib --target aarch64-linux-android` green.

수동(인터랙티브, CI 스크립트 불가 — 별도 표기):
- 실제 터미널에서 편집(←→·backspace), ↑↓ history 회상, Ctrl-C(라인 취소·프롬프트 재표시), Ctrl-D(빈 라인에서 종료). Windows 콘솔 + ConPTY(WSL/Windows Terminal)에서 확인.

## 10. 수용 기준

1. 인터랙티브 `ash`(TTY)가 reedline 편집·↑↓ history·Ctrl-C(취소)·Ctrl-D(EOF)를 제공한다.
2. 비-TTY(파이프/스크립트)는 `StdinLineReader`로 라인 단위 동작 — S2 게이트 e2e 포함 기존 동작 불변.
3. reedline init 실패는 `StdinLineReader`로 fail-soft.
4. `shellcore`는 reedline/crossterm 미참조 — android cdylib 빌드 불변.
5. `cargo fmt --all -- --check`(실제 `cargo fmt --all` 후) · `cargo clippy --all-targets --features "storage tls remote" -D warnings` · `cargo test --features "storage tls remote"` green. CI windows build 잡이 Windows 컴파일 확인.

## 11. 비목표 (S3 밖)

- **파일 영속 history·민감명령 저장 제외(S4)**. S3는 in-session 메모리 history만.
- completions·syntax highlighting·multiline·키바인딩 커스터마이즈·vi 모드(후속/필요 시).
- AI 입력 분기(S5).
