# S6 — `ash` Git Bash/MSYS Bridge Runner 설계 (foundation + wiring)

> **작성일**: 2026-06-27
> **유형**: 단일 슬라이스 구현 spec (Windows ash 완성 로드맵 S6).
> **상위/정본**: `2026-06-23-platform-execution-contract.md` §3·§10, `2026-06-26-windows-ash-completion-scoping-design.md` S6. 선행: S2(gate), `shellcore::msys`(select_profile).

## 1. 목표

`AI_TERMINAL_WINDOWS_PROFILE=msys` opt-in(+`MSYSTEM` 존재) 시, `ash`의 외부 명령을 **MSYS POSIX host(`sh -lc "<cmd>"`)로 실행**한다. MSYS의 `sh`가 POSIX path 해석·tool discovery(MSYS `PATH`의 `/usr/bin`·`/mingw64/bin`)를 담당하므로, ash는 path 변환/tool 스캔을 재구현하지 않는다.

**검증 제약(중요)**: MSYS bridge **실행은 Windows+MSYS에서만** 동작한다. 본 슬라이스의 빌드/검증 환경은 WSL(Linux)이라 실행 자체는 테스트할 수 없다 → **순수 함수(bridge invocation·profile 선택)는 단위테스트, 실제 `sh` 실행은 Windows 수동 검증**.

## 2. 경계 제약

- `shellcore::msys`(android 컴파일됨)에 추가하는 함수는 **std만** 사용한다(데스크톱 모듈 미참조).
- 실행 결선은 데스크톱 `src/gated_runner.rs`의 `ArgvExecutor`에만 둔다(S2 게이트 경로). `shellcore::external`·`DesktopRunner`(임베드)·android cdylib 빌드는 불변.
- **native `.cmd/.ps1` adapter(winexec)와 MSYS bridge를 혼합하지 않는다**(계약 §10). 둘은 배타적 — 활성 profile 하나만.

## 3. 순수 함수 — `shellcore::msys` 추가

```rust
/// MSYS POSIX host 호출을 구성한다. sh가 PATH에서 POSIX tool을 찾고 path를 해석한다.
pub fn bridge_invocation(command: &str) -> (String, Vec<String>) {
    ("sh".to_string(), vec!["-lc".to_string(), command.to_string()])
}

/// 현재 활성 Windows 셸 profile. env(PROFILE_ENV/MSYSTEM/MSYSTEM_PREFIX)를 읽어
/// select_profile로 판정한다. Selected가 아니면(미지/MSYS밖) native로 안전 폴백.
pub fn active_profile() -> WindowsShellProfile {
    let p = std::env::var(PROFILE_ENV).ok();
    let msystem = std::env::var("MSYSTEM").ok();
    let prefix = std::env::var("MSYSTEM_PREFIX").ok();
    match select_profile(p.as_deref(), msystem.as_deref(), prefix.as_deref()) {
        ProfileSelection::Selected(profile) => profile,
        _ => WindowsShellProfile::NativeWindows,
    }
}
```
- `bridge_invocation`은 순수 — 단위테스트. `active_profile`은 env를 읽지만 비-Windows/android에서는 MSYSTEM 부재로 항상 native(무해).
- host는 `sh` 고정(YAGNI; `bash` 선택은 후속). cwd는 변환 없이 그대로 sh에 넘긴다(아래 §4).

## 4. 결선 — `gated_runner.rs` `ArgvExecutor`

`ArgvExecutor`에 분석/재구성된 **command string**을 보유시키고(S2의 `command_string` 재사용), `run`에서 active profile로 분기:

```rust
// ArgvExecutor { name, args, cwd, command: String }
fn run(&self, _command: &str, _sink: &mut dyn OutputSink) -> anyhow::Result<i32> {
    let code = match crate::shellcore::msys::active_profile() {
        WindowsShellProfile::MsysBridge => {
            let (prog, args) = crate::shellcore::msys::bridge_invocation(&self.command);
            crate::shellcore::external::spawn_inherit(&prog, &args, self.cwd)?
        }
        WindowsShellProfile::NativeWindows => {
            crate::shellcore::external::spawn_inherit(self.name, &self.args, self.cwd)?
        }
    };
    Ok(code.unwrap_or(-1))
}
```
- MSYS면 `spawn_inherit("sh", ["-lc", cmd], cwd)`(기존 함수 재사용, stdio 상속). native면 기존 argv 직접 실행.
- `GatedRunner::run`이 `ArgvExecutor`를 만들 때 `command: cmd.clone()`(이미 가진 `command_string`)을 넣는다.
- **S2 게이트(risk/policy/preview/undo/audit)는 불변** — 분석은 command string 기준, MSYS는 실행 수단만 sh 경유.

## 5. 에러 처리

- `sh` 미존재(MSYS 미설치인데 profile만 msys로 강제 + MSYSTEM 위조 등 비정상) → `spawn_inherit`의 NotFound → "command not found: sh" bail → REPL이 `error:`로 출력 후 지속(기존 동작). 정상 MSYS 환경에선 sh가 PATH에 있다.
- 비-Windows/MSYS밖 → `active_profile`이 native → 기존 경로(영향 없음).

## 6. 테스트

단위(`shellcore::msys`):
- `bridge_invocation("ls -al")` == `("sh", ["-lc", "ls -al"])`.
- `select_profile`(기존 4 테스트 유지). `active_profile`은 env 의존이라 직접 단위테스트는 생략(select_profile로 로직 커버; env 조작 테스트는 flaky라 비채택).

빌드/경계(WSL):
- `cargo build --bins`·`cargo check --lib --target aarch64-linux-android` green(msys 추가가 android 불변).
- 비-TTY e2e(native 경로): `echo hi`→셸, `rm -rf /`→차단 불변(MSYS 미활성이므로 native).

수동(Windows + Git Bash/MSYS, 이 환경 불가 — 별도 표기):
- Git Bash에서 `AI_TERMINAL_WINDOWS_PROFILE=msys ash.exe` 실행 후 `ls -al`·`grep x file`·`uname` 등 POSIX tool이 sh 경유로 동작, exit code 전파 확인. `AI_TERMINAL_WINDOWS_PROFILE` 미설정(native)에선 sh 경유 안 함 확인. 위험 명령은 S2 게이트로 차단됨 확인.

## 7. 수용 기준

1. MSYS profile 활성(`=msys` + `MSYSTEM`) 시 ash 외부 실행이 `sh -lc "<cmd>"` 경유.
2. native(기본/MSYS밖)에선 기존 argv 직접 실행(불변).
3. S2 게이트는 MSYS에서도 그대로 적용(분석은 command string).
4. `shellcore::msys` 추가는 std만 — android cdylib 빌드 불변. `gated_runner` 결선은 데스크톱.
5. `cargo fmt --all -- --check`(실제 `cargo fmt --all` 후) · `cargo clippy --all-targets --features "storage tls remote" -D warnings` · `cargo test --features "storage tls remote"` green.

## 8. 비목표

- 명시 path 변환(cygpath; sh가 POSIX path 해석 → 불필요), 명시 tool discovery(/usr/bin 스캔; sh PATH 의존), `bash`/host 셸 선택 config, MSYS PTY/signal 의미 별도 모델, `DesktopRunner`(임베드) bridge화, MSYS profile에서의 winexec 혼용.
