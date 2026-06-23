# Remote Approval M0 — 셸 인터셉트 제어점 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**목표:** armed 상태에서 위험 명령을 **실행 전** 셸 hook으로 가로채 로컬 위험도 게이트(§30-13)로 통과/차단하는 제어점을 bash·zsh에 in-repo로 착지한다(크립토·데몬·PWA는 M1+).

**Architecture:** 순수 결정 로직(`gate.rs`: `decide_gate` + armed 상태 파일) → 내부 `ai __gate "<cmd>"`(exit code 계약: 0=통과, 1=차단, fail-closed) → 셸 hook이 armed 파일 존재 시에만 `ai __gate`를 호출해 차단(bash: `extdebug`+`DEBUG` trap `return 1`, zsh: ZLE `accept-line` 위젯). 비-armed는 파일 stat만(hot-path 무시 가능).

**기술 스택:** Rust(기존 크레이트), clap 서브커맨드, 기존 `risk`/`policy`/`config`/`shell` 모듈, WSL bash/zsh + PTY e2e.

**검증 기반:** `docs/superpowers/specs/2026-06-04-remote-approval-m0-intercept-design.md`(WSL spike로 메커니즘 실증: bash extdebug·zsh ZLE 차단 ✅, IPC 0.117ms, 비-armed 0.02ms, fail-closed ✅).

---

### 작업 1: `gate.rs` — 게이트 결정 로직 (순수)

**Files:**
- Create: `src/gate.rs`
- Modify: `src/lib.rs` (모듈 등록)

- [ ] **단계 1: 모듈 등록**

`src/lib.rs`의 모듈 목록(알파벳 순, `gateway` 앞)에 추가:

```rust
pub mod gate;
```

- [ ] **단계 2: 실패하는 테스트 작성**

`src/gate.rs` 생성:

```rust
//! 원격 승인 게이트 결정 + armed 상태 (M0, §30-13 경계).
//!
//! armed 상태에서 명령을 **실행 전** 통과/차단 결정한다(순수, deterministic).
//! §30-13 정본 경계: Low/Medium 통과, High 기본 차단(opt-in 시 통과), Critical 항상 차단.
//! armed가 아니면 게이트는 개입하지 않는다(항상 통과). 크립토·원격 왕복은 M1+.

use crate::risk::{self, RiskLevel};

/// 게이트 결정.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Allow,
    Block { reason: String },
}

/// armed/allow_high 상태에서 명령의 통과/차단을 결정한다(순수).
pub fn decide_gate(command: &str, armed: bool, allow_high: bool) -> GateDecision {
    if !armed {
        return GateDecision::Allow;
    }
    let a = risk::assess(command);
    match a.level {
        RiskLevel::Low | RiskLevel::Medium => GateDecision::Allow,
        RiskLevel::High if allow_high => GateDecision::Allow,
        RiskLevel::High => GateDecision::Block {
            reason: format!("High 위험(score {}) — 원격 승인 opt-in 필요(§30-13)", a.score),
        },
        RiskLevel::Critical => GateDecision::Block {
            reason: format!(
                "Critical 위험(score {}) — 원격 승인 불가, 로컬 터미널에서 실행(§30-13)",
                a.score
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_armed_always_allows() {
        assert_eq!(decide_gate("rm -rf /", false, false), GateDecision::Allow);
    }

    #[test]
    fn armed_allows_low_and_medium() {
        assert_eq!(decide_gate("ls -al", true, false), GateDecision::Allow);
        // 재귀삭제(상대경로)는 Medium 수준 → 통과(원격 승인 대상이나 M0는 로컬 통과).
        assert_eq!(decide_gate("rm -rf build", true, false), GateDecision::Allow);
    }

    #[test]
    fn armed_blocks_critical_always() {
        match decide_gate("rm -rf /", true, true) {
            GateDecision::Block { reason } => assert!(reason.contains("Critical")),
            d => panic!("expected Block, got {d:?}"),
        }
    }

    #[test]
    fn armed_high_blocks_unless_optin() {
        let cmd = "sudo rm -rf /etc/nginx";
        match decide_gate(cmd, true, false) {
            GateDecision::Block { reason } => assert!(reason.contains("opt-in")),
            d => panic!("expected Block default, got {d:?}"),
        }
        assert_eq!(decide_gate(cmd, true, true), GateDecision::Allow);
    }
}
```

> **참고 (구현자):** 단계 2의 `armed_allows_low_and_medium`·`armed_high_blocks_unless_optin`은 실제 `risk::assess` 점수에 의존한다. 단계 4에서 테스트가 빨강이면(등급 가정이 어긋나면) 먼저 `cargo run -- risk "<cmd>"`로 실제 등급을 확인하고 **테스트의 예시 명령을 실제 등급에 맞는 것으로 교체**한 뒤 진행하라(로직이 아니라 픽스처를 맞춘다). Critical 확정 케이스 `rm -rf /`는 안정적이다.

- [ ] **단계 3: 실패 확인**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test gate:: 2>&1 | tail -20'`
기대: 컴파일은 되고 테스트 실행, 등급 가정 불일치 시 일부 FAIL.

- [ ] **단계 4: 등급 픽스처 보정 후 통과 확인**

필요 시 위 NOTE대로 명령 교체. 그 외 로직 변경 없음.
실행: 위와 동일.
기대: `test result: ok.` (gate 테스트 전부 통과)

- [ ] **단계 5: 커밋**

```
git add src/gate.rs src/lib.rs
git commit -F <msg>   # feat(remote): 게이트 결정 로직(decide_gate, §30-13 경계) — M0
```

---

### 작업 2: `gate.rs` — armed 상태 파일 (parse/render + I/O)

**Files:**
- Modify: `src/gate.rs`

- [ ] **단계 1: 실패하는 테스트 + 구현 추가**

`src/gate.rs`의 `use` 아래에 추가:

```rust
use std::path::{Path, PathBuf};

/// armed 상태(파일 존재 = armed). 내용으로 opt-in 플래그를 표현.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArmState {
    pub allow_high: bool,
}

/// armed 파일 경로: `<config_dir>/armed`. 셸 hook의 경로와 일치해야 한다
/// (`${XDG_CONFIG_HOME:-$HOME/.config}/ai-terminal/armed`).
pub fn armed_path() -> anyhow::Result<PathBuf> {
    Ok(crate::config::config_dir()?.join("armed"))
}

/// armed 파일 내용을 파싱(순수). `allow_high=true` 라인이 있으면 opt-in.
pub fn parse_arm_file(content: &str) -> ArmState {
    ArmState {
        allow_high: content.lines().any(|l| l.trim() == "allow_high=true"),
    }
}

/// armed 파일 내용을 생성(순수).
pub fn render_arm_file(allow_high: bool) -> String {
    format!("allow_high={allow_high}\n")
}

/// armed 상태를 읽는다. 파일이 없으면 `None`(=armed 아님).
pub fn load_arm_state(path: &Path) -> Option<ArmState> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(parse_arm_file(&content))
}

/// armed 파일을 기록(상위 디렉터리 생성).
pub fn arm_at(path: &Path, allow_high: bool) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, render_arm_file(allow_high))?;
    Ok(())
}

/// armed 파일을 제거(없으면 무해).
pub fn disarm_at(path: &Path) -> anyhow::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
```

`mod tests` 안에 추가:

```rust
    #[test]
    fn arm_file_roundtrip() {
        assert_eq!(parse_arm_file(&render_arm_file(true)).allow_high, true);
        assert_eq!(parse_arm_file(&render_arm_file(false)).allow_high, false);
        assert_eq!(parse_arm_file("garbage\n").allow_high, false);
    }

    #[test]
    fn arm_disarm_load_cycle() {
        let dir = std::env::temp_dir().join(format!("ra_gate_{}", std::process::id()));
        let path = dir.join("armed");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(load_arm_state(&path).is_none(), "초기엔 armed 아님");
        arm_at(&path, true).unwrap();
        assert_eq!(load_arm_state(&path).unwrap().allow_high, true);
        disarm_at(&path).unwrap();
        assert!(load_arm_state(&path).is_none(), "disarm 후 armed 아님");
        disarm_at(&path).unwrap(); // 재호출 무해
        let _ = std::fs::remove_dir_all(&dir);
    }
```

- [ ] **단계 2: 실패 확인**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test gate:: 2>&1 | tail -20'`
기대: 신규 두 테스트 포함 컴파일·통과(구현을 함께 넣었으므로 바로 green이면 OK — TDD red 단계가 짧음).

- [ ] **단계 3: 통과 확인**

실행: 위와 동일. 기대: `test result: ok.`

- [ ] **단계 4: 커밋**

```
git add src/gate.rs
git commit -F <msg>   # feat(remote): armed 상태 파일(parse/render/arm/disarm) — M0
```

---

### 작업 3: `ai __gate` 내부 커맨드 (exit code 계약 + fail-closed)

**Files:**
- Modify: `src/main.rs` (Command enum + main 디스패치)

- [ ] **단계 1: CLI 변형 추가**

`enum Command`의 `Hook` 변형 위(또는 근처)에 추가:

```rust
    /// 내부: 셸 hook이 호출하는 게이트. armed 시 위험도 게이트(§30-13)로 통과/차단.
    /// exit 0=통과, 비0=차단(셸 hook이 명령 실행을 취소). 오류/불확실 시 fail-closed(차단).
    #[command(name = "__gate", hide = true)]
    Gate {
        /// 평가할 명령 문자열.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
```

- [ ] **단계 2: 디스패치 추가**

`main()`의 `match cli.command` 안, `Some(Command::Hook { .. })` 근처에 추가:

```rust
        Some(Command::Gate { command }) => {
            let cmd = command.join(" ");
            let code = run_gate(&cmd);
            std::process::exit(code);
        }
```

`main.rs`에 헬퍼 추가(예: `run_doctor` 근처):

```rust
/// `ai __gate` 본체. armed 상태를 읽어 게이트 결정 → exit code 반환.
/// armed 파일/설정 접근 실패는 fail-closed(차단=1)로 처리한다(저하 경로, §DESIGN).
fn run_gate(command: &str) -> i32 {
    use ai_terminal::gate::{self, GateDecision};

    let path = match gate::armed_path() {
        Ok(p) => p,
        Err(_) => return 1, // 경로 불명 = fail-closed
    };
    let (armed, allow_high) = match gate::load_arm_state(&path) {
        Some(st) => (true, st.allow_high),
        None => (false, false),
    };
    match gate::decide_gate(command, armed, allow_high) {
        GateDecision::Allow => 0,
        GateDecision::Block { reason } => {
            eprintln!("AI 게이트 차단: {reason}");
            1
        }
    }
}
```

- [ ] **단계 3: 파싱 테스트 작성**

`main.rs`의 `#[cfg(test)] mod tests`에 추가:

```rust
    #[test]
    fn cli_parses_gate() {
        let cli = Cli::try_parse_from(["ai", "__gate", "rm", "-rf", "/"]).unwrap();
        match cli.command {
            Some(Command::Gate { command }) => assert_eq!(command.join(" "), "rm -rf /"),
            _ => panic!("expected gate"),
        }
    }
```

- [ ] **단계 4: 빌드·테스트·동작 확인**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test cli_parses_gate 2>&1 | tail -8'`
기대: PASS.

수동 동작 확인(armed 아님 → 통과 exit 0; Critical은 armed에서만 차단):

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo run -q -- __gate "rm -rf /" && echo GATE_ALLOW || echo GATE_BLOCK'`
기대: `GATE_ALLOW` (armed 아니므로 통과). **종료코드는 `&&/||`로만 검증**([[terminal-build-env]]).

- [ ] **단계 5: 커밋**

```
git add src/main.rs
git commit -F <msg>   # feat(remote): ai __gate 내부 커맨드(exit code 계약, fail-closed) — M0
```

---

### 작업 4: `ai remote arm/disarm/status` 커맨드

**Files:**
- Modify: `src/main.rs` (Command enum + RemoteAction subcommand + 디스패치)

- [ ] **단계 1: CLI 추가**

`enum Command`에 추가:

```rust
    /// 원격 승인 게이트 arm/disarm/status (M0). armed 상태에서만 셸 인터셉트가 개입한다.
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },
```

`enum PolicyAction` 근처에 새 enum:

```rust
#[derive(Subcommand, Debug)]
enum RemoteAction {
    /// 게이트를 켠다. 이후 위험 명령이 인터셉트된다(§30-13 경계).
    Arm {
        /// High 위험 명령도 통과 허용(§30-13 opt-in 오버라이드).
        #[arg(long)]
        allow_high: bool,
    },
    /// 게이트를 끈다(인터셉트 미개입).
    Disarm {},
    /// 현재 armed 상태를 표시한다.
    Status {},
}
```

- [ ] **단계 2: 디스패치 추가**

`main()` match에 추가:

```rust
        Some(Command::Remote { action }) => {
            use ai_terminal::gate;
            let path = gate::armed_path()?;
            match action {
                RemoteAction::Arm { allow_high } => {
                    gate::arm_at(&path, allow_high)?;
                    println!(
                        "원격 게이트 armed{}.",
                        if allow_high { " (High opt-in 허용)" } else { "" }
                    );
                }
                RemoteAction::Disarm {} => {
                    gate::disarm_at(&path)?;
                    println!("원격 게이트 disarmed.");
                }
                RemoteAction::Status {} => match gate::load_arm_state(&path) {
                    Some(st) => println!(
                        "armed (allow_high={}). 위험 명령이 인터셉트됩니다.",
                        st.allow_high
                    ),
                    None => println!("disarmed. 인터셉트 미개입."),
                },
            }
            Ok(())
        }
```

- [ ] **단계 3: 파싱 테스트**

`mod tests`에 추가:

```rust
    #[test]
    fn cli_parses_remote_arm() {
        let cli = Cli::try_parse_from(["ai", "remote", "arm", "--allow-high"]).unwrap();
        match cli.command {
            Some(Command::Remote {
                action: RemoteAction::Arm { allow_high },
            }) => assert!(allow_high),
            _ => panic!("expected remote arm"),
        }
    }

    #[test]
    fn cli_parses_remote_disarm_and_status() {
        assert!(matches!(
            Cli::try_parse_from(["ai", "remote", "disarm"]).unwrap().command,
            Some(Command::Remote { action: RemoteAction::Disarm {} })
        ));
        assert!(matches!(
            Cli::try_parse_from(["ai", "remote", "status"]).unwrap().command,
            Some(Command::Remote { action: RemoteAction::Status {} })
        ));
    }
```

- [ ] **단계 4: 빌드·테스트·왕복 동작 확인**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test cli_parses_remote 2>&1 | tail -8'`
기대: PASS (2 tests).

armed 후 게이트가 Critical 차단하는지 e2e 동작 확인(임시 XDG로 격리):

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; export XDG_CONFIG_HOME=$(mktemp -d); cargo run -q -- remote arm; cargo run -q -- __gate "rm -rf /" && echo ALLOW || echo BLOCK; cargo run -q -- remote disarm; rm -rf "$XDG_CONFIG_HOME"'`
기대: `원격 게이트 armed.` 다음 줄 `BLOCK` (armed에서 Critical 차단).

- [ ] **단계 5: 커밋**

```
git add src/main.rs
git commit -F <msg>   # feat(remote): ai remote arm/disarm/status — M0
```

---

### 작업 5: bash hook에 인터셉트(extdebug + 병합 DEBUG trap) 추가

**Files:**
- Modify: `src/shell.rs` (`BASH_HOOK` 상수 + 단위 테스트)

- [ ] **단계 1: `BASH_HOOK` 교체**

`src/shell.rs`의 `const BASH_HOOK` 전체를 아래로 교체. 기존 telemetry(preexec/precmd/chpwd)를 보존하면서, **단일 DEBUG trap**에 인터셉트를 병합하고 `extdebug`를 켠다. 재진입 가드(`__ai_in_trap`)로 trap 내부 호출의 자기-차단/무한루프를 막는다. armed 파일이 있을 때만 `ai __gate`를 호출(hot-path: 파일 stat만).

```rust
const BASH_HOOK: &str = r#"# ai-terminal bash hook
export AI_TERMINAL_HOOK=1
shopt -s extdebug
__ai_last_pwd=""
__ai_in_trap=""
__ai_armed_file="${XDG_CONFIG_HOME:-$HOME/.config}/ai-terminal/armed"
# 단일 DEBUG trap: 텔레메트리(preexec) + armed 인터셉트(실행 전 차단).
# 반환값이 명령 실행을 좌우하므로(extdebug) 차단 시에만 1, 그 외 0을 반환한다.
__ai_debug() {
  [ -n "$__ai_in_trap" ] && return 0
  [ -n "$COMP_LINE" ] && return 0
  __ai_in_trap=1
  command -v ai >/dev/null 2>&1 && ai __hook preexec "cmd=$BASH_COMMAND" "cwd=$PWD" >/dev/null 2>&1
  if [ -f "$__ai_armed_file" ]; then
    case "$BASH_COMMAND" in
      __ai_*|"$PROMPT_COMMAND") ;;
      *)
        if command -v ai >/dev/null 2>&1 && ! ai __gate "$BASH_COMMAND"; then
          __ai_in_trap=""
          return 1
        fi
        ;;
    esac
  fi
  __ai_in_trap=""
  return 0
}
# bash는 native chpwd가 없으므로 PWD 변화를 감지해 chpwd 이벤트를 에뮬레이트한다.
__ai_chpwd() {
  if [ "$PWD" != "$__ai_last_pwd" ]; then
    __ai_last_pwd="$PWD"
    command -v ai >/dev/null 2>&1 && ai __hook chpwd "cwd=$PWD" >/dev/null 2>&1 || true
  fi
}
__ai_precmd() {
  local __ai_ec=$?
  command -v ai >/dev/null 2>&1 && ai __hook precmd "exit=$__ai_ec" "cwd=$PWD" >/dev/null 2>&1 || true
  __ai_chpwd
  return $__ai_ec
}
trap '__ai_debug' DEBUG
case "$PROMPT_COMMAND" in
  *__ai_precmd*) ;;
  *) PROMPT_COMMAND="__ai_precmd${PROMPT_COMMAND:+; $PROMPT_COMMAND}" ;;
esac
"#;
```

- [ ] **단계 2: 단위 테스트 추가**

`shell.rs`의 `mod tests`에 추가:

```rust
    #[test]
    fn bash_hook_has_armed_intercept() {
        let h = hook_script(Shell::Bash);
        assert!(h.contains("extdebug"), "차단 위해 extdebug 필요: {h}");
        assert!(h.contains("__ai_armed_file"), "armed 파일 게이트 필요: {h}");
        assert!(h.contains("ai __gate"), "게이트 호출 필요: {h}");
        assert!(h.contains("__ai_in_trap"), "재진입 가드 필요: {h}");
    }
```

> 기존 `bash_hook_wires_preexec_and_precmd_safely`·`bash_hook_emulates_chpwd_on_cwd_change`는 새 본문에서도 만족한다(DEBUG·PROMPT_COMMAND·command -v ai·__ai_last_pwd·__hook chpwd·return $__ai_ec 모두 보존). 확인만 한다.

- [ ] **단계 3: 빌드·단위·문법 검증**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shell:: 2>&1 | tail -15'`
기대: `test result: ok.` — 특히 `generated_hooks_pass_syntax_check`(bash -n)와 신규 `bash_hook_has_armed_intercept` 통과.

- [ ] **단계 4: 커밋**

```
git add src/shell.rs
git commit -F <msg>   # feat(remote): bash hook 인터셉트(extdebug DEBUG trap, armed 게이트) — M0
```

---

### 작업 6: zsh hook에 인터셉트(ZLE accept-line 위젯) 추가

**Files:**
- Modify: `src/shell.rs` (`ZSH_HOOK` 상수 + 단위 테스트)

- [ ] **단계 1: `ZSH_HOOK`에 위젯 추가**

`const ZSH_HOOK`의 마지막 `add-zsh-hook ...` 줄들 뒤(닫는 `"#` 전)에 추가:

```rust
// preexec는 차단 불가 → ZLE accept-line 위젯으로 실행 전 차단(armed 시).
__ai_armed_file="${XDG_CONFIG_HOME:-$HOME/.config}/ai-terminal/armed"
__ai_accept_line() {
  if [[ -f "$__ai_armed_file" ]] && command -v ai >/dev/null 2>&1; then
    if ! ai __gate "$BUFFER"; then
      print -u2 "AI 게이트 차단: $BUFFER"
      BUFFER=""
      zle .accept-line
      return
    fi
  fi
  zle .accept-line
}
zle -N accept-line __ai_accept_line
```

> 위 블록을 기존 raw 문자열 안에 넣을 때 들여쓰기/줄바꿈을 그대로 유지한다(zsh는 들여쓰기 무관).

- [ ] **단계 2: 단위 테스트 추가**

`mod tests`에 추가:

```rust
    #[test]
    fn zsh_hook_has_armed_intercept() {
        let h = hook_script(Shell::Zsh);
        assert!(h.contains("zle -N accept-line"), "ZLE 위젯 필요: {h}");
        assert!(h.contains("__ai_armed_file"), "armed 파일 게이트 필요: {h}");
        assert!(h.contains("ai __gate"), "게이트 호출 필요: {h}");
    }
```

- [ ] **단계 3: 빌드·단위·문법 검증**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shell:: 2>&1 | tail -15'`
기대: `test result: ok.` — `generated_hooks_pass_syntax_check`(zsh -n, zsh 설치 시)와 신규 테스트 통과.

- [ ] **단계 4: 커밋**

```
git add src/shell.rs
git commit -F <msg>   # feat(remote): zsh hook 인터셉트(ZLE accept-line 위젯) — M0
```

---

### 작업 7: 대화형 e2e — 생성된 hook이 실제로 차단/통과 (unix)

**Files:**
- Modify: `tests/integration.rs` (또는 신규 `tests/intercept_e2e.rs`)

목표: **레포의 실제 `hook_script` 출력**을 rc로 써서 `bash -i`(zsh 설치 시 zsh도)를 PTY로 띄우고, armed에서 `rm -rf <dir>` 차단(대상 생존)·안전 명령 실행·disarmed 무개입을 파일시스템으로 증명한다. (spike를 in-repo 재현)

- [ ] **단계 1: e2e 테스트 작성**

`tests/intercept_e2e.rs` 생성. `ai_terminal::pty::PtySession`과 `ai_terminal::shell`을 사용. 빌드된 `ai` 바이너리를 PATH에 올려 hook의 `ai __gate`가 동작하게 한다(`CARGO_BIN_EXE_ai`).

```rust
//! M0 인터셉트 대화형 e2e — 생성 hook이 armed에서 위험 명령을 실행 전 차단함을 증명.
//! unix 전용(대화형 셸·PTY). zsh 미설치 시 zsh 케이스는 건너뛴다.
#![cfg(unix)]

use std::io::Write;
use std::time::{Duration, Instant};

use ai_terminal::pty::PtySession;
use ai_terminal::shell::{self, Shell};

/// 셸별 e2e: armed 시 rm -rf 차단(대상 생존), 안전 명령 실행.
fn run_case(shell: Shell, bin: &str) -> Option<(bool, bool)> {
    // bin 존재 확인(zsh 미설치 스킵).
    if which(bin).is_none() {
        eprintln!("skip {bin}: not installed");
        return None;
    }
    let tmp = std::env::temp_dir().join(format!("ra_e2e_{}_{}", bin, std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    // 격리된 XDG_CONFIG_HOME + armed 파일.
    let xdg = tmp.join("cfg");
    std::fs::create_dir_all(xdg.join("ai-terminal")).unwrap();
    std::fs::write(xdg.join("ai-terminal").join("armed"), "allow_high=false\n").unwrap();

    // rc 파일 = 생성된 hook.
    let rc = tmp.join("rc");
    std::fs::write(&rc, shell::hook_script(shell)).unwrap();

    let target = tmp.join("target");
    std::fs::create_dir_all(&target).unwrap();
    let safe = tmp.join("safe");

    // ai 바이너리를 PATH 앞에 추가.
    let ai_bin = env!("CARGO_BIN_EXE_ai");
    let ai_dir = std::path::Path::new(ai_bin).parent().unwrap();
    let path_env = format!("{}:{}", ai_dir.display(), std::env::var("PATH").unwrap_or_default());

    // 환경을 셸에 전달하기 위해 export 라인을 먼저 보낸다.
    let argv: Vec<&str> = match shell {
        Shell::Bash => vec!["--norc", "--noprofile", "-i"],
        Shell::Zsh => vec!["-f", "-i"],
    };
    let mut s = PtySession::spawn(bin, &argv).unwrap();
    // 무한 행 방지 워치독(회귀 안전장치).
    let mut killer = s.killer();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(8));
        let _ = killer.kill();
    });

    let send = |s: &mut PtySession, line: String| {
        s.write_input(&line).unwrap();
        std::thread::sleep(Duration::from_millis(250));
    };
    // PATH/XDG 주입 후 hook source.
    send(&mut s, format!("export PATH='{}'\n", path_env));
    send(&mut s, format!("export XDG_CONFIG_HOME='{}'\n", xdg.display()));
    send(&mut s, format!("source '{}'\n", rc.display()));
    // 차단 대상 + 안전 명령.
    send(&mut s, format!("rm -rf '{}'\n", target.display()));
    send(&mut s, format!("echo hi > '{}'\n", safe.display()));
    // 출력 드레인(최대 2s).
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if s.read_chunk().is_err() {
            break;
        }
    }
    let _ = s.kill();

    let target_survived = target.is_dir();
    let safe_executed = safe.is_file();
    let _ = std::fs::remove_dir_all(&tmp);
    Some((target_survived, safe_executed))
}

/// 최소 which: PATH에서 실행 파일을 찾는다.
fn which(bin: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|d| {
        let p = d.join(bin);
        if p.is_file() {
            Some(p)
        } else {
            None
        }
    })
}

#[test]
fn bash_armed_blocks_rm_rf_allows_safe() {
    if let Some((survived, safe)) = run_case(Shell::Bash, "bash") {
        assert!(survived, "armed bash: rm -rf 가 차단되어 대상이 살아남아야 함");
        assert!(safe, "armed bash: 안전 명령은 실행되어야 함");
    }
}

#[test]
fn zsh_armed_blocks_rm_rf_allows_safe() {
    if let Some((survived, safe)) = run_case(Shell::Zsh, "zsh") {
        assert!(survived, "armed zsh: rm -rf 가 차단되어 대상이 살아남아야 함");
        assert!(safe, "armed zsh: 안전 명령은 실행되어야 함");
    }
}
```

> **참고 (구현자):** `rm -rf '<dir>'`는 risk 등급이 Critical이 아닐 수 있다(절대경로·시스템경로가 Critical 트리거). e2e의 목적은 "차단 동작" 증명이므로, **armed에서 확실히 Block 되는 명령**을 써야 한다. 작업 1 NOTE처럼 `cargo run -- risk "rm -rf '<tmp>/target'"`로 등급을 확인하고, Block 등급(High opt-in 꺼짐 또는 Critical)이 나오는 명령으로 맞춘다. 안 되면 확실한 Critical(`rm -rf /` 대신 부수효과 없는) 대체가 어렵다 → 대상 디렉터리를 `/tmp` 아래 두되 **게이트가 Block하도록 `allow_high=false` + High 트리거(예: `sudo` 포함)** 명령으로 바꾸거나, 테스트용으로 게이트 임계를 명확히 만족시키는 명령을 선택한다. 핵심: 등급은 `risk`로 실측해 픽스처를 맞춘다(로직 변경 금지).

- [ ] **단계 2: e2e 실행(빌드 후)**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --test intercept_e2e -- --test-threads=1 --nocapture 2>&1 | tail -25'`
기대: `bash_armed_blocks_rm_rf_allows_safe ... ok` (+ zsh ok 또는 skip). 8s 워치독으로 무한 행 없음.

- [ ] **단계 3: 커밋**

```
git add tests/intercept_e2e.rs
git commit -F <msg>   # test(remote): M0 인터셉트 대화형 e2e(bash/zsh 차단·통과) — WSL
```

---

### 작업 8: 문서 동기화 + 전체 검증 + 통합 커밋/푸시

**Files:**
- Modify: `docs/HISTORY.md`, `docs/TASK.md`

- [ ] **단계 1: HISTORY.md 엔트리 추가**

`docs/HISTORY.md` 최상단(`---` 다음)에 M0 완료 엔트리 추가: 배경(인터셉트 제어점 = 최대 feasibility 위험), 검증(spike + in-repo e2e), 구현(gate.rs·ai __gate·ai remote arm/disarm/status·bash extdebug/zsh ZLE hook), §30-13 경계, fail-closed, M1+ 제외 범위.

- [ ] **단계 2: TASK.md 갱신**

FU-4 항목을 `[~] M0 완료`로 표기하고 다음 단계(M0.5 와이어 프로토콜 / M1 데몬+PWA)를 인계에 명시.

- [ ] **단계 3: 전체 feature-gate 검증**

실행: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo test 2>&1 | grep -E "test result|error\[" && cargo test --features "storage tls" 2>&1 | grep -E "test result|error\[" && cargo clippy --all-targets --features "storage tls" 2>&1 | grep -E "warning|error" | tail -5; echo DONE'`
기대: 모든 `test result: ok.`, clippy 경고/에러 없음.

- [ ] **단계 4: 문서 커밋 + push (사용자 확인 후)**

```
git add docs/HISTORY.md docs/TASK.md
git commit -F <msg>   # docs(remote): M0 인터셉트 완료 기록 + 인계
# push는 사용자 확인 후 main FF
```

---

## Self-Review

- **스펙 커버리지**: 완료 기준 #1(hook 설치·문법·idempotent) → 작업 5/6 + 기존 install 머신 재사용. #2/#3(bash/zsh 대화형 차단) → 작업 7. #4(비-armed 무개입) → 작업 7 disarmed 경로 + hook의 armed-file stat(작업 5/6 설계). #5(게이트 등급 경계 + fail-closed) → 작업 1/3. #6(빌드 green) → 작업 8. ✅
- **제외 범위 명시**: 크립토·데몬·소켓 서버·PWA·페어링·TTL/heartbeat/viz·replay/nonce·revoke·TOCTOU = M1+ (작업 본문에 없음, 의도적).
- **타입 일관성**: `GateDecision`(Allow/Block{reason}), `ArmState{allow_high}`, `decide_gate(command, armed, allow_high)`, `armed_path/load_arm_state/arm_at/disarm_at`, CLI `Command::Gate{command:Vec<String>}`·`Command::Remote{action:RemoteAction}` — 모든 작업에서 일관.
- **리스크**: 셸 hook 정확성은 단위 테스트로 완전 검증 불가 → `bash -n`/`zsh -n`(기존 테스트 자동 확장) + 작업 7 대화형 e2e + spike 사전 실증으로 3중 방어. risk 등급 가정은 작업 1/7 NOTE로 실측 보정(로직 불변).

## 실행 핸드오프

계획 저장: `docs/superpowers/plans/2026-06-04-remote-approval-m0-intercept.md`. 사용자가 "구현 진행"을 이미 지시 → **인라인 실행(executing-plans)**으로 이 세션에서 작업 1→8 TDD 실행, Task마다 커밋, 작업 8 push 전 확인.
