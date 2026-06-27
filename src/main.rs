//! AI CLI 통합 리눅스 터미널 — `ai` 진입점 (스켈레톤).
//!
//! 설계 정본: `../document/`(v3.3). 본 골격은 M0 부트스트랩 단계의 최소 구현으로,
//! CLI 표면(§9, §31)을 잡아두고 이후 마일스톤(M1~M4)에서 채워 넣는다.
//!
//! 불변식(자세히는 `docs/RULES.md`):
//! - AI 기능 장애가 일반 셸 사용을 막아서는 안 된다(§3-3).
//! - AI 생성 명령은 자동 실행하지 않는다(§3-11).
//! - 로컬 정책/위험도 평가가 먼저 수행된다(§3-9).

use std::path::PathBuf;

use ai_terminal::config;
use ai_terminal::context;
use ai_terminal::dispatch;
use ai_terminal::explain;
use ai_terminal::gateway;
use ai_terminal::guardrails;
use ai_terminal::index;
use ai_terminal::intent;
use ai_terminal::mask;
use ai_terminal::mcp;
use ai_terminal::planner;
use ai_terminal::policy::PolicyProfile;
use ai_terminal::preview;
use ai_terminal::risk;
use ai_terminal::shell::{self, Shell};
use ai_terminal::skill;
use ai_terminal::ui;
use ai_terminal::undo;
#[cfg(feature = "storage")]
use ai_terminal::usage;
use ai_terminal::verify::{self, BinaryStatus};
use ai_terminal::verify_agent;
use clap::{Parser, Subcommand};

/// AI CLI 통합 리눅스 터미널.
#[derive(Parser, Debug)]
#[command(
    name = "ai",
    version,
    about = "일반 셸 호환 + 안전한 AI 보조 터미널 (설계 v3.3)"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// 환경/플랫폼 진단을 표시한다 (§31.11 `ai doctor`).
    Doctor {
        /// 플랫폼별 guardrails capability matrix 출력 (§31.11).
        #[arg(long)]
        guardrails: bool,
    },
    /// 셸 명령의 위험도(0~100)를 평가해 등급·요인·정책 결정을 표시한다 (§31.4 `ai risk`).
    Risk {
        /// 평가할 명령 문자열. 예: `ai risk "rm -rf /"`
        command: String,
        /// 적용할 정책 프로파일(미지정 시 활성 프로파일).
        #[arg(long)]
        profile: Option<String>,
    },
    /// 정책 프로파일을 표시한다 (§31.3 `ai policy`).
    Policy {
        #[command(subcommand)]
        action: PolicyAction,
    },
    /// 셸 통합 hook 스크립트를 출력한다 (§31.1 `ai shell-hook <shell>`).
    ShellHook {
        /// bash | zsh
        shell: String,
    },
    /// 통합 초기화 (§31.1 `ai init shell`).
    Init {
        #[command(subcommand)]
        target: InitTarget,
    },
    /// 인터랙티브 TUI를 실행한다 (§5 Terminal UI). Esc/Ctrl-C로 종료.
    Tui {
        /// 표시할 정책 프로파일(미지정 시 활성 프로파일).
        #[arg(long)]
        profile: Option<String>,
    },
    /// 영속 PTY 셸을 띄운다 (Native Wrapper, cwd probe 동기화) (§30-1 FU-3). exit/Ctrl-D로 종료.
    Shell {},
    /// 원격 승인 게이트 arm/disarm/status (M0). armed 상태에서만 셸 인터셉트가 개입한다.
    Remote {
        #[command(subcommand)]
        action: RemoteAction,
    },
    /// 텍스트의 Secret/PII를 마스킹하고 원격 전송 가능 여부를 표시한다 (§31.8 `ai mask`).
    Mask {
        /// 마스킹할 텍스트(앞에 `-`가 있어도 허용).
        #[arg(allow_hyphen_values = true)]
        text: String,
    },
    /// 파일 변경 명령의 preview 전략을 표시한다 (§31.5 `ai preview`).
    Preview {
        /// 미리볼 명령 문자열.
        command: String,
    },
    /// AI 제안 명령을 종합 검증한다 (Phase 2 Verification Agent).
    Verify {
        /// 검증할 명령.
        command: String,
    },
    /// 최근 백업을 복구한다 (§31.6 `ai undo last`).
    Undo {
        /// 복구 대상(현재 `last`만 지원).
        #[arg(default_value = "last")]
        target: String,
    },
    /// 입력 의도(Shell/AiQuery/AiInline)를 분류한다 (Phase 2 Intent Classifier).
    Classify {
        /// 분류할 입력.
        input: String,
    },
    /// 입력을 셸/AI 경로로 분기한다 (Phase 2 Hybrid dispatcher).
    Route {
        /// 분기할 입력.
        input: String,
    },
    /// 입력을 분류해 셸 실행 또는 AI 응답으로 보낸다 (통합 디스패처, Phase 2).
    Dispatch {
        /// 분류·실행할 입력. 예: `ai dispatch "ls -al"` 또는 `ai dispatch "how do I list files?"`
        input: String,
        /// 셸 경로에서 확인 없이 자동 승인(Block은 우회 불가).
        #[arg(long)]
        yes: bool,
        /// 정책 프로파일(미지정 시 활성 프로파일).
        #[arg(long)]
        profile: Option<String>,
    },
    /// 셸 명령을 게이트(위험도·정책·preview·백업)를 거쳐 실행한다 (그룹 C `ai exec`).
    Exec {
        /// 실행할 명령 문자열. 예: `ai exec "rm -rf build"`
        command: String,
        /// 확인 프롬프트 없이 자동 승인(Block은 우회 불가).
        #[arg(long)]
        yes: bool,
        /// 정책 프로파일(미지정 시 활성 프로파일).
        #[arg(long)]
        profile: Option<String>,
    },
    /// 프로젝트 파일을 인덱싱해 키워드로 검색한다 (§25.2 Semantic File Index).
    Index {
        /// 검색 키워드.
        query: String,
        /// 인덱싱 루트(기본 현재 디렉터리).
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// 자연어 요청을 후보 명령 단계로 계획한다 (Phase 2 Tool Use Planner).
    Plan {
        /// 요청 문자열.
        request: String,
    },
    /// AI에게 질의한다 (Phase 2 Model Gateway).
    Ask {
        /// 질의 프롬프트.
        prompt: String,
        /// 백엔드: mock | ollama.
        #[arg(long, default_value = "mock")]
        backend: String,
        /// (ollama) 모델 이름.
        #[arg(long, default_value = "qwen2.5-coder")]
        model: String,
        /// (ollama) base URL.
        #[arg(long, default_value = "http://localhost:11434")]
        ollama_url: String,
        /// (openai) base URL(OpenAI 호환 엔드포인트, 평문 HTTP).
        #[arg(long, default_value = "http://localhost:8080")]
        openai_url: String,
    },
    /// 스킬을 발견·매칭해 표시한다 (§26 통합 스킬 관리).
    Skill {
        /// 키워드로 매칭(미지정 시 전체 나열).
        #[arg(long)]
        query: Option<String>,
    },
    /// 등록된 MCP 서버를 표시한다 (§27 통합 MCP 관리).
    Mcp {
        /// mcp.json 경로(미지정 시 ~/.config/ai-terminal/mcp.json).
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// 현재 세션 컨텍스트(cwd/shell/git 등)를 표시한다 (§31.10 `ai context`).
    Context {},
    /// 실패한 명령의 원인/해결책을 분석한다 (§4.3 `ai explain`).
    Explain {
        /// 실패한 명령 문자열. `--last-error` 사용 시 생략 가능.
        command: Option<String>,
        /// 종료 코드.
        #[arg(long, default_value_t = 1)]
        exit: i32,
        /// stderr 내용(있으면 분석에 사용).
        #[arg(long, default_value = "")]
        stderr: String,
        /// 저장소에 기록된 직전 실패 명령을 불러와 분석한다 (storage feature).
        #[arg(long)]
        last_error: bool,
    },
    /// 누적 사용량/예산 상태를 표시한다 (§31.7, storage feature).
    #[cfg(feature = "storage")]
    Usage {},
    /// 최근 명령 히스토리를 표시한다 (§31.2, storage feature).
    #[cfg(feature = "storage")]
    History {
        /// 표시 개수.
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// 내부: 셸 hook이 호출하는 게이트. armed 시 위험도 게이트(§30-13)로 통과/차단.
    /// exit 0=통과, 비0=차단(셸 hook이 명령 실행을 취소). 오류/불확실 시 fail-closed(차단).
    #[command(name = "__gate", hide = true)]
    Gate {
        /// 평가할 명령 문자열.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// 내부: 셸 hook이 호출하는 상태 보고 진입점. (storage feature 시 preexec 기록)
    #[command(name = "__hook", hide = true)]
    Hook {
        /// 이벤트 종류(preexec|precmd|chpwd|startup).
        event: String,
        /// key=value 인자들.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        rest: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum PolicyAction {
    /// 정책 프로파일의 주요 필드를 표시한다(미지정 시 활성 프로파일).
    Show {
        #[arg(long)]
        profile: Option<String>,
    },
    /// 활성 정책 프로파일을 설정(영속화)한다.
    Set {
        /// 설정할 프로파일(balanced|paranoid).
        profile: String,
    },
}

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
    /// 게이트 데몬을 포그라운드 실행한다(Unix 소켓, M1). hook이 여기에 질의한다.
    Daemon {},
}

#[derive(Subcommand, Debug)]
enum InitTarget {
    /// 셸 rc 파일에 통합 블록을 설치/제거한다 (§31.1).
    Shell {
        /// 대상 셸(미지정 시 $SHELL 추정, 기본 bash).
        #[arg(long)]
        shell: Option<String>,
        /// rc 파일 경로(미지정 시 홈의 기본 rc).
        #[arg(long)]
        rc: Option<PathBuf>,
        /// 변경 없이 미리보기만(파일 미수정).
        #[arg(long)]
        dry_run: bool,
        /// 적용 예정 diff 표시(파일 미수정).
        #[arg(long)]
        diff: bool,
        /// 삽입한 블록만 제거.
        #[arg(long)]
        uninstall: bool,
    },
}

/// `ai init shell`의 동작 모드.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitMode {
    Install,
    DryRun,
    Diff,
    Uninstall,
}

/// rc 수정 계획(파일 I/O와 분리해 테스트 가능하게).
#[derive(Debug, Clone, PartialEq, Eq)]
struct InitPlan {
    new_content: String,
    write: bool,
    message: String,
}

/// rc 내용·셸·모드로부터 수정 계획을 산출한다(순수 함수).
fn plan_init_shell(old: &str, shell: Shell, mode: InitMode, path: &str) -> InitPlan {
    match mode {
        InitMode::Install => {
            if shell::is_installed(old) {
                InitPlan {
                    new_content: old.to_string(),
                    write: false,
                    message: format!("이미 설치됨: {path}\n"),
                }
            } else {
                let new_content = shell::apply_install(old, shell);
                InitPlan {
                    new_content,
                    write: true,
                    message: format!("설치 완료: {path} ({} hook)\n", shell.as_str()),
                }
            }
        }
        InitMode::DryRun => {
            let new_content = shell::apply_install(old, shell);
            InitPlan {
                new_content,
                write: false,
                message: format!(
                    "[dry-run] {path} 를 수정하지 않았습니다. 변경 내용은 `--diff`로 확인하세요.\n"
                ),
            }
        }
        InitMode::Diff => {
            let new_content = shell::apply_install(old, shell);
            let message = shell::unified_diff(old, &new_content, path);
            InitPlan {
                new_content,
                write: false,
                message,
            }
        }
        InitMode::Uninstall => {
            if shell::is_installed(old) {
                let new_content = shell::apply_uninstall(old);
                InitPlan {
                    new_content,
                    write: true,
                    message: format!("제거 완료: {path} 에서 통합 블록 삭제\n"),
                }
            } else {
                InitPlan {
                    new_content: old.to_string(),
                    write: false,
                    message: format!("제거할 통합 블록이 없습니다: {path}\n"),
                }
            }
        }
    }
}

/// `--shell` 또는 `$SHELL`에서 셸을 결정한다(기본 bash).
fn resolve_shell(opt: Option<&str>) -> anyhow::Result<Shell> {
    if let Some(s) = opt {
        return Shell::parse(s).ok_or_else(|| anyhow::anyhow!("unsupported shell: {s} (bash|zsh)"));
    }
    if let Ok(env_shell) = std::env::var("SHELL") {
        if let Some(s) = Shell::parse(&env_shell) {
            return Ok(s);
        }
    }
    Ok(Shell::Bash)
}

/// rc 파일 경로를 결정한다(`--rc` 또는 `$HOME/<기본 rc>`).
fn resolve_rc(opt: Option<PathBuf>, shell: Shell) -> anyhow::Result<PathBuf> {
    if let Some(p) = opt {
        return Ok(p);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("HOME not set; use --rc <path>"))?;
    Ok(home.join(shell.default_rc_filename()))
}

/// 셸 hook의 `preexec` 이벤트를 받아 명령을 기본 세션에 기록한다(best-effort).
///
/// 위험도를 함께 산출해 저장한다. 재진입(자기 자신의 `ai __hook` 호출)은 건너뛴다.
#[cfg(feature = "storage")]
fn record_hook_preexec(rest: &[String]) -> anyhow::Result<()> {
    use ai_terminal::store::{NewCommand, NewSession, Store};

    let kv = |k: &str| -> Option<String> {
        let pre = format!("{k}=");
        rest.iter()
            .find_map(|s| s.strip_prefix(&pre))
            .map(String::from)
    };
    let cmd_text = kv("cmd").unwrap_or_default();
    if cmd_text.is_empty() || cmd_text.starts_with("ai __hook") {
        return Ok(());
    }

    let store = Store::open_default()?;
    let session_id = "sess-default";
    store.get_or_create_session(
        session_id,
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: kv("cwd").unwrap_or_default(),
            policy_profile: "balanced".into(),
        },
    )?;

    let a = risk::assess(&cmd_text);
    store.record_command(&NewCommand {
        session_id: session_id.into(),
        command_text: cmd_text,
        source: "shell".into(),
        cwd: kv("cwd"),
        exit_code: None,
        risk_level: Some(format!("{:?}", a.level)),
        risk_score: Some(a.score as i64),
        ai_generated: false,
        confirmed: true,
    })?;
    Ok(())
}

/// 셸 hook의 `precmd` 이벤트를 받아 직전 명령의 종료 코드를 반영한다(best-effort).
///
/// `preexec`에서 종료 코드 미정으로 기록된 명령에 실제 `$?`를 채운다(§31.1).
/// `last-error` 분석(`ai explain --last-error`)의 입력이 된다.
#[cfg(feature = "storage")]
fn record_hook_precmd(rest: &[String]) -> anyhow::Result<()> {
    use ai_terminal::store::Store;

    let exit = rest
        .iter()
        .find_map(|s| s.strip_prefix("exit="))
        .and_then(|s| s.parse::<i64>().ok());
    let Some(exit) = exit else { return Ok(()) };

    let store = Store::open_default()?;
    store.update_last_exit("sess-default", exit)?;
    Ok(())
}

/// 셸 hook의 `chpwd` 이벤트를 받아 세션 cwd와 git branch 컨텍스트를 갱신한다(§31.10).
///
/// `cd`/`git switch` 등으로 작업 디렉터리가 바뀌면 세션 cwd를 갱신하고,
/// 해당 경로의 git branch를 컨텍스트 스냅샷으로 남긴다(best-effort).
#[cfg(feature = "storage")]
fn record_hook_chpwd(rest: &[String]) -> anyhow::Result<()> {
    use ai_terminal::store::{NewContext, NewSession, Store};

    let cwd = rest.iter().find_map(|s| s.strip_prefix("cwd="));
    let Some(cwd) = cwd else { return Ok(()) };

    let store = Store::open_default()?;
    let session_id = "sess-default";
    store.get_or_create_session(
        session_id,
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: cwd.to_string(),
            policy_profile: "balanced".into(),
        },
    )?;

    let branch = ai_terminal::context::git_branch(std::path::Path::new(cwd));
    store.update_session_cwd(session_id, cwd)?;
    store.record_context_snapshot(&NewContext {
        session_id: session_id.into(),
        context_type: "chpwd".into(),
        cwd: Some(cwd.to_string()),
        git_branch: branch,
    })?;
    Ok(())
}

/// `ai explain` 출력 문자열을 만든다.
fn format_explain(command: &str, exit: i32, stderr: &str) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let e = explain::explain(&explain::ErrorContext {
        command: command.into(),
        exit_code: exit,
        stderr: stderr.into(),
        cwd,
    });
    let mut s = format!("원인     : {}\n", e.summary);
    if !e.suggestions.is_empty() {
        s.push_str("제안     :\n");
        for sug in &e.suggestions {
            s.push_str(&format!("  - {sug}\n"));
        }
    }
    s
}

/// `ai explain` 실행: 주어진 명령(또는 `--last-error`로 저장소의 직전 실패 명령)을 분석한다.
fn run_explain(
    command: Option<String>,
    exit: i32,
    stderr: String,
    last_error: bool,
) -> anyhow::Result<()> {
    if last_error {
        #[cfg(feature = "storage")]
        {
            let store = ai_terminal::store::Store::open_default()?;
            match store.last_error("sess-default")? {
                Some(row) => {
                    let ec = row.exit_code.unwrap_or(1) as i32;
                    print!("{}", format_explain(&row.command_text, ec, ""));
                }
                None => println!("최근 실패한 명령이 없습니다."),
            }
            return Ok(());
        }
        #[cfg(not(feature = "storage"))]
        anyhow::bail!("--last-error 는 storage feature 빌드에서만 사용할 수 있습니다.");
    }
    let command = command
        .ok_or_else(|| anyhow::anyhow!("분석할 명령을 지정하거나 --last-error 를 사용하세요."))?;
    print!("{}", format_explain(&command, exit, &stderr));
    Ok(())
}

/// `ai preview` 출력 문자열을 만든다(실제 diff/content-at-risk).
fn format_preview(command: &str) -> String {
    use preview::PreviewRender;
    let mut s = String::new();
    for r in preview::render_preview(command) {
        match r {
            PreviewRender::Diff(d) => {
                s.push_str("preview  : 변경 diff (적용 전)\n");
                s.push_str(&d);
                if !d.ends_with('\n') {
                    s.push('\n');
                }
            }
            PreviewRender::ContentAtRisk {
                path,
                lines,
                bytes,
                head,
            } => {
                s.push_str(&format!(
                    "preview  : 손실 예정 {path} ({lines}줄, {bytes} bytes)\n"
                ));
                for line in head.lines() {
                    s.push_str(&format!("  | {line}\n"));
                }
            }
            PreviewRender::Info(m) => {
                s.push_str(&format!("preview  : {m}\n"));
            }
        }
    }
    if s.is_empty() {
        s.push_str("preview  : (출력 없음)\n");
    }
    s
}

/// `ai mask` 출력 문자열을 만든다.
fn format_mask(input: &str) -> String {
    let out = mask::Masker::baseline().mask(input);
    let redacted = if out.redactions.is_empty() {
        "(none)".to_string()
    } else {
        out.redactions.join(", ")
    };
    let remote = if out.blocked {
        format!("BLOCKED ({})", out.block_reason.unwrap_or_default())
    } else {
        "eligible".to_string()
    };
    format!(
        "masked   : {}\nredacted : {}\nremote   : {}\n",
        out.text, redacted, remote
    )
}

/// `ai risk` 출력 문자열을 만든다. (stdout 분리로 테스트 가능하게)
fn format_risk(command: &str, profile: &PolicyProfile) -> String {
    let a = risk::assess(command);
    let decision = profile.decide(a.level);
    let binary = match verify::check_binary(command) {
        BinaryStatus::Found(_) => "found".to_string(),
        BinaryStatus::Builtin => "builtin".to_string(),
        BinaryStatus::Unknown => "UNKNOWN (hallucination?)".to_string(),
    };
    let mut out = format!(
        "command  : {command}\nrisk     : {:?} ({}/100)\npolicy   : {} -> {:?}\nbinary   : {}\n",
        a.level, a.score, profile.name, decision, binary
    );
    if !a.factors.is_empty() {
        out.push_str("factors  :\n");
        for f in &a.factors {
            out.push_str(&format!("  {:+4}  {}\n", f.delta, f.label));
        }
    }
    out
}

/// 정책 프로파일의 주요 필드를 표시한다 (§31.3).
fn describe_profile(p: &PolicyProfile) -> String {
    format!(
        "profile   : {}\n\
         confirm   : {:?}\n\
         block     : critical={} high={}\n\
         remote_ai : {}\n\
         sudo_ai   : {}\n\
         masking   : secrets={} pii={} fail_closed={}\n\
         preview   : {}\n\
         auto_exec : {}\n\
         healing   : {} (max {})\n",
        p.name,
        p.confirm_level,
        p.block_critical,
        p.block_high_risk,
        p.allow_remote_ai,
        p.allow_sudo_ai_commands,
        p.mask_secrets,
        p.mask_pii,
        p.block_on_masking_failure,
        p.preview_file_modifications,
        p.auto_execute,
        p.auto_healing,
        p.auto_healing_max_attempts
    )
}

/// 프로파일 이름을 조회하고, 없으면 명확한 오류를 반환한다.
fn resolve_profile(name: &str) -> anyhow::Result<PolicyProfile> {
    PolicyProfile::by_name(name)
        .ok_or_else(|| anyhow::anyhow!("unknown profile: {name} (balanced|paranoid)"))
}

/// `ai __gate` 본체. armed 상태를 읽어 게이트 결정 → exit code 반환.
/// armed면 데몬(Unix 소켓)에 질의하고, 데몬 도달 불가 시 로컬 `decide_gate`로 폴백한다
/// (데몬 다운은 보안 경계가 아니라 자기-가드레일 — DESIGN Threat Model). armed 경로
/// 접근 실패는 fail-closed(차단=1).
fn run_gate(command: &str) -> i32 {
    use ai_terminal::gate::{self, GateDecision};

    let path = match gate::armed_path() {
        Ok(p) => p,
        Err(_) => return 1, // 경로 불명 = fail-closed
    };
    let (armed, allow_high) = match gate::load_arm_state(&path) {
        Some(st) => (true, st.allow_high),
        None => return 0, // 비-armed: 게이트 미개입(hot-path)
    };

    // armed: 데몬에 질의(unix). 도달 불가 시 로컬 결정으로 폴백.
    #[cfg(unix)]
    {
        use ai_terminal::daemon;
        if let Ok(sock) = daemon::socket_path() {
            if let Ok(reply) = daemon::query(&sock, command) {
                if reply.is_allow() {
                    return 0;
                }
                eprintln!("AI 게이트 차단(데몬): {}", reply.reason);
                return 1;
            }
        }
    }

    match gate::decide_gate(command, armed, allow_high) {
        GateDecision::Allow => 0,
        GateDecision::Block { reason } => {
            eprintln!("AI 게이트 차단: {reason}");
            1
        }
    }
}

/// `ai remote daemon` 본체. Unix 소켓 게이트 데몬을 포그라운드 실행한다(Ctrl-C 종료).
fn run_gate_daemon() -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use ai_terminal::daemon;
        let sock = daemon::socket_path()?;
        println!("원격 게이트 데몬 시작: {} (Ctrl-C 종료)", sock.display());
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(daemon::serve(&sock))
    }
    #[cfg(not(unix))]
    {
        println!("게이트 데몬은 Unix 전용입니다.");
        Ok(())
    }
}

/// 현재 콘솔에 attach된 프로세스 수(Windows). 비-Windows·감지 실패 시 None.
#[cfg(windows)]
fn console_process_count() -> Option<u32> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetConsoleProcessList(lpdwProcessList: *mut u32, dwProcessCount: u32) -> u32;
    }
    let mut buf = [0u32; 4];
    // SAFETY: 유효한 버퍼 포인터와 길이로 호출. 반환값은 콘솔에 attach된 프로세스 수(0=실패).
    let n = unsafe { GetConsoleProcessList(buf.as_mut_ptr(), buf.len() as u32) };
    if n == 0 {
        None
    } else {
        Some(n)
    }
}

#[cfg(not(windows))]
fn console_process_count() -> Option<u32> {
    None
}

/// 탐색기 더블클릭으로 자기 콘솔을 단독 점유해 실행됐는지 추정한다(순수).
/// attach 프로세스가 자기 자신 1개뿐이면 double-click; 터미널 실행은 부모 셸도 attach 되어
/// 2 이상. None(비-Windows/감지 실패)은 false(보수적 — 일시정지하지 않음).
fn is_double_click_launch(console_process_count: Option<u32>) -> bool {
    console_process_count == Some(1)
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Command::Doctor { guardrails }) => run_doctor(guardrails),
        Some(Command::Risk { command, profile }) => {
            let p = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
            print!("{}", format_risk(&command, &p));
            Ok(())
        }
        Some(Command::Policy { action }) => match action {
            PolicyAction::Show { profile } => {
                let p = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
                print!("{}", describe_profile(&p));
                Ok(())
            }
            PolicyAction::Set { profile } => {
                let p = resolve_profile(&profile)?;
                config::set_active_profile(p.name)?;
                println!("활성 정책 프로파일을 '{}'(으)로 설정했습니다.", p.name);
                Ok(())
            }
        },
        Some(Command::ShellHook { shell }) => {
            let sh = resolve_shell(Some(&shell))?;
            print!("{}", shell::hook_script(sh));
            Ok(())
        }
        Some(Command::Tui { profile }) => {
            let p = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
            ui::run(p.name)
        }
        Some(Command::Shell {}) => run_persistent_shell(),
        Some(Command::Remote { action }) => {
            use ai_terminal::gate;
            let path = gate::armed_path()?;
            match action {
                RemoteAction::Arm { allow_high } => {
                    gate::arm_at(&path, allow_high)?;
                    println!(
                        "원격 게이트 armed{}.",
                        if allow_high {
                            " (High opt-in 허용)"
                        } else {
                            ""
                        }
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
                RemoteAction::Daemon {} => run_gate_daemon()?,
            }
            Ok(())
        }
        Some(Command::Mask { text }) => {
            print!("{}", format_mask(&text));
            Ok(())
        }
        Some(Command::Preview { command }) => {
            print!("{}", format_preview(&command));
            Ok(())
        }
        Some(Command::Verify { command }) => {
            let profile = resolve_profile(&config::get_active_profile())?;
            let v = verify_agent::verify_command(&command, &profile);
            println!("binary   : {:?}", v.binary);
            println!("risk     : {:?} -> {:?}", v.risk, v.decision);
            println!("safe     : {}", v.safe_to_suggest);
            if !v.issues.is_empty() {
                println!("issues   :");
                for i in &v.issues {
                    println!("  - {i}");
                }
            }
            Ok(())
        }
        Some(Command::Classify { input }) => {
            println!("{:?}", intent::classify(&input));
            Ok(())
        }
        Some(Command::Plan { request }) => {
            for (i, step) in planner::plan(&request).steps.iter().enumerate() {
                let cmd = step.command.as_deref().unwrap_or("(AI 위임)");
                println!("{}. {} — {}", i + 1, step.description, cmd);
            }
            Ok(())
        }
        Some(Command::Index { query, root }) => {
            let idx = index::FileIndex::build(&root);
            let results = idx.search(&query, 10);
            if results.is_empty() {
                println!("(매칭 파일 없음, {} 파일 인덱싱)", idx.len());
            }
            for (path, score) in results {
                println!("  {score:>3}  {}", path.display());
            }
            Ok(())
        }
        Some(Command::Route { input }) => {
            let profile = resolve_profile(&config::get_active_profile())?;
            match dispatch::dispatch(&input, &profile) {
                dispatch::Route::Empty => println!("(빈 입력)"),
                dispatch::Route::Shell {
                    command,
                    risk,
                    decision,
                } => {
                    println!("route    : Shell");
                    println!("command  : {command}");
                    println!("risk     : {risk:?} -> {decision:?}");
                }
                dispatch::Route::Ai { prompt } => {
                    println!("route    : AI");
                    println!("prompt   : {prompt}");
                }
            }
            Ok(())
        }
        Some(Command::Ask {
            prompt,
            backend,
            model,
            ollama_url,
            openai_url,
        }) => {
            let ai_cfg = config::Ai {
                provider: backend.clone(),
                model: model.clone(),
                ollama_url: ollama_url.clone(),
                openai_url: openai_url.clone(),
            };
            let cap = ai_terminal::provider::Provider::mock().models[0].clone();
            let gw = match ai_cfg.provider.as_str() {
                "ollama" => {
                    let b = ai_terminal::ollama::OllamaBackend::new(
                        ai_terminal::http::TcpTransport,
                        &ai_cfg.ollama_url,
                        &ai_cfg.model,
                    );
                    gateway::Gateway::new(Box::new(b), cap)
                }
                "openai" => {
                    let api_key = std::env::var("OPENAI_API_KEY").ok();
                    let b = ai_terminal::openai::OpenAiBackend::new(
                        ai_terminal::http::TcpTransport,
                        &ai_cfg.openai_url,
                        &ai_cfg.model,
                        api_key,
                    );
                    gateway::Gateway::new(Box::new(b), cap)
                }
                _ => gateway::Gateway::mock(),
            };
            // 예산 게이트(§31.7) — storage가 있으면 누적 지출을 읽어 주입한다. 초과 시
            // 게이트웨이가 원격 백엔드 호출 전에 차단한다. default 빌드는 영속 지출을
            // 모르므로 미적용(현행 동작 보존).
            #[cfg(feature = "storage")]
            let gw = match ai_terminal::store::Store::open_default() {
                Ok(store) => {
                    let spent = store.total_cost(None).unwrap_or(0.0);
                    gw.with_budget(spent, usage::BudgetConfig::defaults())
                }
                Err(_) => gw,
            };
            let ctx = context::gather();
            // AI 호출을 타임아웃·Ctrl+C 취소와 함께 실행한다(§16.2, Graceful Recovery).
            let ctx_str = format!("cwd={}", ctx.cwd);
            let timeout = ai_terminal::aitask::Timeouts::defaults().request;
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let result = rt.block_on(async {
                let cancel = std::sync::Arc::new(tokio::sync::Notify::new());
                ai_terminal::aitask::cancel_on_ctrl_c(cancel.clone());
                gw.ask_cancellable(&prompt, &ctx_str, timeout, cancel).await
            });
            match result {
                Ok(gateway::GatewayOutcome::Answered {
                    text,
                    input_tokens,
                    output_tokens,
                    source,
                }) => {
                    println!("{text}");
                    let usage = ai_terminal::ai_usage::summarize(
                        &ai_cfg,
                        source,
                        input_tokens,
                        output_tokens,
                    );
                    let cost_badge = if usage.estimated { " estimated" } else { "" };
                    println!(
                        "(tokens ~ in:{input_tokens} out:{output_tokens} · cost ~ ${:.4}{cost_badge}){}",
                        usage.cost_usd,
                        cache_badge(source)
                    );
                    #[cfg(feature = "storage")]
                    let _ = ai_terminal::ai_usage::record(&usage, None);
                }
                Ok(gateway::GatewayOutcome::Blocked(reason)) => {
                    println!("[차단] 원격 전송 불가(fail-closed): {reason}");
                }
                Err(e) => {
                    // AI 장애는 셸로 전파되지 않는다(§3-3). 친절히 고지하고 정상 종료.
                    println!("[AI 사용 불가] {e}");
                }
            }
            Ok(())
        }
        Some(Command::Skill { query }) => {
            let mut paths = vec![PathBuf::from("./.ai-terminal/skills")];
            if let Ok(cd) = config::config_dir() {
                paths.push(cd.join("skills"));
            }
            let skills = skill::discover(&paths);
            let shown: Vec<&skill::Skill> = match &query {
                Some(q) => skill::match_skills(&skills, q, 5),
                None => skills.iter().collect(),
            };
            if shown.is_empty() {
                println!("(스킬 없음 — {:?})", paths);
            }
            for s in shown {
                println!("- {} — {}", s.name, s.description);
            }
            Ok(())
        }
        Some(Command::Mcp { config: cfg }) => {
            let path = match cfg {
                Some(p) => p,
                None => config::config_dir()?.join("mcp.json"),
            };
            match std::fs::read_to_string(&path) {
                Ok(json) => {
                    let servers = mcp::parse_servers(&json)?;
                    if servers.is_empty() {
                        println!("(등록된 MCP 서버 없음)");
                    }
                    for s in &servers {
                        println!("- {} : {} {}", s.name, s.command, s.args.join(" "));
                    }
                    println!("(auto_connect=false — 부작용 도구는 컨센트·감사 필요, §27)");
                }
                Err(_) => println!("mcp.json 없음: {}", path.display()),
            }
            Ok(())
        }
        Some(Command::Context {}) => {
            let c = context::gather();
            println!("cwd      : {}", c.cwd);
            println!("shell    : {}", c.shell);
            println!("user     : {}", c.user);
            println!("hostname : {}", c.hostname);
            println!("git      : {}", c.git_branch.as_deref().unwrap_or("-"));
            Ok(())
        }
        Some(Command::Explain {
            command,
            exit,
            stderr,
            last_error,
        }) => run_explain(command, exit, stderr, last_error),
        Some(Command::Undo { target }) => {
            if target != "last" {
                anyhow::bail!("지원하지 않는 undo 대상: {target} (last만 지원)");
            }
            let dir = undo::default_undo_dir()?;
            match undo::latest(&dir) {
                Some(id) => {
                    let n = undo::restore(&dir, &id)?;
                    println!("복구 완료: {n}개 파일 ({id})");
                }
                None => println!("복구할 백업이 없습니다."),
            }
            Ok(())
        }
        Some(Command::Init { target }) => match target {
            InitTarget::Shell {
                shell,
                rc,
                dry_run,
                diff,
                uninstall,
            } => {
                let sh = resolve_shell(shell.as_deref())?;
                let path = resolve_rc(rc, sh)?;
                let old = std::fs::read_to_string(&path).unwrap_or_default();
                let mode = if uninstall {
                    InitMode::Uninstall
                } else if diff {
                    InitMode::Diff
                } else if dry_run {
                    InitMode::DryRun
                } else {
                    InitMode::Install
                };
                let plan = plan_init_shell(&old, sh, mode, &path.display().to_string());
                if plan.write {
                    std::fs::write(&path, &plan.new_content)?;
                }
                print!("{}", plan.message);
                Ok(())
            }
        },
        #[cfg(feature = "storage")]
        Some(Command::Usage {}) => {
            let store = ai_terminal::store::Store::open_default()?;
            let spent = store.total_cost(None)?;
            let cfg = usage::BudgetConfig::defaults();
            let action = usage::evaluate(spent, cfg.session_usd, cfg.warn_pct, cfg.block_pct);
            println!("usage    : ${spent:.4} 사용");
            println!(
                "budget   : 세션 ${:.2} / 월 ${:.2} (경고 {}% / 차단 {}%)",
                cfg.session_usd, cfg.monthly_usd, cfg.warn_pct, cfg.block_pct
            );
            println!("status   : {action:?}");
            Ok(())
        }
        #[cfg(feature = "storage")]
        Some(Command::History { limit }) => {
            let store = ai_terminal::store::Store::open_default()?;
            let rows = store.recent_commands(limit)?;
            if rows.is_empty() {
                println!("(아직 기록된 명령이 없습니다)");
            }
            for r in rows.iter().rev() {
                let lvl = r.risk_level.as_deref().unwrap_or("-");
                println!("[{lvl:<8}] {}", r.command_text);
            }
            Ok(())
        }
        Some(Command::Exec {
            command,
            yes,
            profile,
        }) => run_exec(&command, yes, profile),
        Some(Command::Dispatch {
            input,
            yes,
            profile,
        }) => run_dispatch(&input, yes, profile),
        Some(Command::Gate { command }) => {
            let cmd = command.join(" ");
            let code = run_gate(&cmd);
            std::process::exit(code);
        }
        Some(Command::Hook { event, rest }) => {
            // hook 실패가 셸을 막지 않도록 항상 Ok 반환(best-effort).
            #[cfg(feature = "storage")]
            match event.as_str() {
                // preexec: 명령을 위험도와 함께 기록(종료 코드 미정).
                "preexec" => {
                    if let Err(e) = record_hook_preexec(&rest) {
                        tracing::debug!("hook preexec record failed (ignored): {e}");
                    }
                }
                // precmd: 직전 명령의 실제 종료 코드를 반영.
                "precmd" => {
                    if let Err(e) = record_hook_precmd(&rest) {
                        tracing::debug!("hook precmd record failed (ignored): {e}");
                    }
                }
                // chpwd: 작업 디렉터리 변경 → 세션 cwd + git branch 컨텍스트 스냅샷.
                "chpwd" => {
                    if let Err(e) = record_hook_chpwd(&rest) {
                        tracing::debug!("hook chpwd record failed (ignored): {e}");
                    }
                }
                _ => {}
            }
            tracing::trace!(event, ?rest, "shell hook event");
            Ok(())
        }
        None => {
            // TODO(M1): 인터랙티브 터미널(REPL/TUI) 진입. 현재는 사용법 안내.
            println!(
                "ai {} — `ai doctor` 로 환경 진단, `ai --help` 로 사용법 확인.",
                env!("CARGO_PKG_VERSION")
            );
            // 탐색기 더블클릭(자기 콘솔 단독 점유)이면 콘솔이 즉시 닫혀 안내를 못 본다.
            // 이 도구는 CLI다 — 사용법을 보여주고 Enter 입력까지 창을 유지한다(터미널 실행엔 무영향).
            if is_double_click_launch(console_process_count()) {
                use std::io::Write;
                println!();
                println!("이 프로그램은 명령줄(CLI) 도구입니다. 더블클릭이 아니라 터미널에서 실행하세요:");
                println!("  ai doctor                 # 환경 진단");
                println!("  ai risk \"rm -rf /tmp/x\"   # 위험도 평가");
                println!("  ai --help                 # 전체 명령");
                println!(
                    "설치(PATH 등록): scripts/install.ps1 · 문서: https://github.com/ai-cli-terminal/terminal"
                );
                print!("\n계속하려면 Enter 키를 누르세요... ");
                let _ = std::io::stdout().flush();
                let mut _line = String::new();
                let _ = std::io::stdin().read_line(&mut _line);
            }
            Ok(())
        }
    }
}

struct StdoutSink;
impl ai_terminal::pipeline::OutputSink for StdoutSink {
    fn write(&mut self, chunk: &str) {
        print!("{chunk}");
    }
}

struct AutoYes;
impl ai_terminal::pipeline::Confirmer for AutoYes {
    fn confirm(&mut self, _: &ai_terminal::pipeline::ConfirmRequest) -> bool {
        true
    }
}

struct StdinConfirmer;
impl ai_terminal::pipeline::Confirmer for StdinConfirmer {
    fn confirm(&mut self, req: &ai_terminal::pipeline::ConfirmRequest) -> bool {
        use std::io::Write;
        eprintln!("위험 등급 {:?} 명령: {}", req.level, req.command);
        for f in &req.factors {
            eprintln!("  - {f}");
        }
        if !req.backup_files.is_empty() {
            eprintln!("  백업 대상: {}", req.backup_files.join(", "));
        }
        eprint!("실행할까요? [y/N] ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            return false;
        }
        matches!(line.trim(), "y" | "Y" | "yes")
    }
}

/// `ai shell` — 영속 PTY 셸(Native Wrapper, FU-3). 하나의 `PtySession`을 재사용해 `cd`가
/// 다음 명령에 유지되며(영속성), 각 명령 뒤 probe로 cwd를 동기화한다(§7.4). 라인 단위
/// REPL이며 입력 인터셉트·분류는 범위 외(라인 게이트는 `ai exec`/`ai tui`).
fn run_persistent_shell() -> anyhow::Result<()> {
    use std::io::{BufRead, Write};

    use ai_terminal::pty::PtySession;
    use ai_terminal::wrapper::{self, PROBE};

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    // 라인 에디터가 probe 마커(\x1f)를 가로채면 cwd 동기화가 멈추므로 셸별 안전 인자를 준다.
    let shell_args = wrapper::session_shell_args(&shell);
    let shell_arg_refs: Vec<&str> = shell_args.iter().map(String::as_str).collect();
    let mut session = PtySession::spawn(&shell, &shell_arg_refs)?;
    println!("ai shell — 영속 셸 (exit/quit/Ctrl-D 종료). cwd는 probe로 동기화됩니다.");

    let stdin = std::io::stdin();
    let mut last_cwd = String::new();
    loop {
        print!("ai> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break; // EOF(Ctrl-D)
        }
        let cmd = line.trim_end();
        if cmd == "exit" || cmd == "quit" {
            break;
        }
        if cmd.trim().is_empty() {
            continue;
        }

        session.write_input(&wrapper::probe_command(cmd))?;
        // probe 쌍(이번 명령의 cwd 방출)을 볼 때까지 출력을 모은다. 인터랙티브 echo로
        // 마커가 더 보일 수 있으나, 마지막 파싱 cwd가 실제값이다.
        let mut acc = String::new();
        for _ in 0..2000 {
            acc.push_str(&session.read_chunk()?);
            if acc.matches(PROBE).count() >= 2 {
                break;
            }
        }
        print!("{}", wrapper::strip_probes(&acc));
        let _ = std::io::stdout().flush();

        if let Some(cwd) = wrapper::parse_probe_cwds(&acc).into_iter().last() {
            if cwd != last_cwd && cwd.starts_with('/') {
                last_cwd = cwd.clone();
                #[cfg(feature = "storage")]
                sync_wrapper_cwd(&cwd);
            }
        }
    }
    let _ = session.kill();
    Ok(())
}

/// probe로 관측한 cwd를 세션 컨텍스트에 동기화한다(§7.4, storage feature).
#[cfg(feature = "storage")]
fn sync_wrapper_cwd(cwd: &str) {
    use ai_terminal::store::{NewContext, NewSession, Store};
    let Ok(store) = Store::open_default() else {
        return;
    };
    let session_id = "sess-default";
    let _ = store.get_or_create_session(
        session_id,
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: cwd.to_string(),
            policy_profile: config::get_active_profile(),
        },
    );
    let branch = ai_terminal::context::git_branch(std::path::Path::new(cwd));
    let _ = store.update_session_cwd(session_id, cwd);
    let _ = store.record_context_snapshot(&NewContext {
        session_id: session_id.into(),
        context_type: "wrapper_probe".into(),
        cwd: Some(cwd.to_string()),
        git_branch: branch,
    });
}

fn run_exec(command: &str, yes: bool, profile: Option<String>) -> anyhow::Result<()> {
    use ai_terminal::pipeline::{self, ExecConfig};

    let prof = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let undo_dir = undo::default_undo_dir()?;
    let cfg = ExecConfig {
        profile: &prof,
        undo_dir: &undo_dir,
        limits: undo::UndoLimits::defaults(),
    };
    let executor = pipeline::PtyExecutor { shell };
    let mut sink = StdoutSink;
    let mut confirmer: Box<dyn pipeline::Confirmer> = if yes {
        Box::new(AutoYes)
    } else {
        Box::new(StdinConfirmer)
    };

    let outcome = pipeline::execute(command, &cfg, &executor, confirmer.as_mut(), &mut sink)?;
    flush_stdout();
    finish_shell_outcome(command, "exec", outcome)
}

fn run_dispatch(input: &str, yes: bool, profile: Option<String>) -> anyhow::Result<()> {
    use ai_terminal::dispatch::{self, AiOutcome, Handled, Handlers};
    use ai_terminal::pipeline::{self, ExecConfig};

    let prof = resolve_profile(&profile.unwrap_or_else(config::get_active_profile))?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let undo_dir = undo::default_undo_dir()?;
    let cfg = ExecConfig {
        profile: &prof,
        undo_dir: &undo_dir,
        limits: undo::UndoLimits::defaults(),
    };
    let executor = pipeline::PtyExecutor { shell };
    let mut confirmer: Box<dyn pipeline::Confirmer> = if yes {
        Box::new(AutoYes)
    } else {
        Box::new(StdinConfirmer)
    };
    let mut ai = ai_terminal::responder::GatewayResponder::mock()?;
    let mut sink = StdoutSink;

    let mut h = Handlers {
        executor: &executor,
        confirmer: confirmer.as_mut(),
        ai: &mut ai,
        sink: &mut sink,
    };
    let handled = dispatch::run(input, &prof, &cfg, &mut h)?;
    flush_stdout();

    match handled {
        Handled::Empty => Ok(()),
        Handled::Shell(outcome) => finish_shell_outcome(input, "dispatch", outcome),
        Handled::Ai(AiOutcome::Answered {
            input_tokens,
            output_tokens,
            source,
            ..
        }) => {
            let usage = ai_terminal::ai_usage::summarize(
                &config::Ai {
                    provider: "mock".into(),
                    model: "mock-model".into(),
                    ..Default::default()
                },
                source,
                input_tokens,
                output_tokens,
            );
            #[cfg(feature = "storage")]
            let _ = ai_terminal::ai_usage::record(&usage, None);
            // 답변 본문은 이미 sink(stdout)로 출력됨. 토큰 요약만 덧붙인다.
            let cost_badge = if usage.estimated { " estimated" } else { "" };
            println!(
                "\n(tokens ~ in:{input_tokens} out:{output_tokens} · cost ~ ${:.4}{cost_badge}){}",
                usage.cost_usd,
                cache_badge(source)
            );
            Ok(())
        }
        Handled::Ai(AiOutcome::Blocked(r)) => {
            println!("[차단] 원격 전송 불가(fail-closed): {r}");
            Ok(())
        }
        Handled::Ai(AiOutcome::Unavailable(e)) => {
            println!("[AI 사용 불가] {e}");
            Ok(())
        }
    }
}

fn flush_stdout() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
}

/// 셸 실행 결과를 마무리한다: audit 기록 + 사용자 안내 + 프로세스 종료(항상 발산).
/// `run_exec`·`run_dispatch`가 공유한다. `command`는 기록/안내에 쓸 명령 텍스트.
fn finish_shell_outcome(
    command: &str,
    source: &str,
    outcome: ai_terminal::pipeline::ExecOutcome,
) -> ! {
    use ai_terminal::pipeline::ExecOutcome;

    if let ExecOutcome::Ran { exit_code, undo_id } = &outcome {
        if let Some(id) = undo_id {
            eprintln!("(백업 생성: {id} — 되돌리려면 `ai undo last`)");
        }
        ai_terminal::shell_audit::record_ran_command(command, *exit_code, source);
        std::process::exit(*exit_code);
    }

    // 비-Ran: audit 기록 후 안내 + exit 1.
    if let Some(rec) = ai_terminal::shell_audit::shell_outcome_audit(command, source, &outcome) {
        ai_terminal::shell_audit::record_outcome_audit(&rec);
    }
    match outcome {
        ExecOutcome::Blocked { level, factors } => {
            eprintln!("차단됨: 위험 등급 {level:?} (정책상 실행 불가)");
            for f in &factors {
                eprintln!("  - {f}");
            }
        }
        ExecOutcome::Declined => eprintln!("실행을 취소했습니다."),
        ExecOutcome::BackupRefused(r) => eprintln!("백업 거부로 실행 중단: {r}"),
        ExecOutcome::Ran { .. } => unreachable!("Ran 은 위에서 처리됨"),
    }
    std::process::exit(1);
}

/// 캐시 출처 배지(Backend는 무배지). `ai ask`·`ai dispatch` 공용.
fn cache_badge(source: ai_terminal::cache::CacheSource) -> &'static str {
    use ai_terminal::cache::CacheSource;
    match source {
        CacheSource::Backend => "",
        CacheSource::Exact => " [cache: exact]",
        CacheSource::Semantic => " [cache: semantic ~근사]",
    }
}

/// `ai doctor`용 config 진단 텍스트(순수 포매터).
fn format_config_diagnostics(loaded: &config::LoadedConfig) -> String {
    use std::fmt::Write as _;
    let source = match &loaded.source {
        config::ConfigSource::File(p) => format!("file: {}", p.display()),
        config::ConfigSource::Default => "default (no file)".to_string(),
    };
    let shell = loaded
        .config
        .general
        .default_shell
        .as_deref()
        .unwrap_or("<unset>");
    let mut out = String::new();
    let _ = writeln!(out, "config: {source}");
    let _ = writeln!(
        out,
        "  general.history_limit = {}",
        loaded.config.general.history_limit
    );
    let _ = write!(out, "  general.default_shell = {shell}");
    if let Some(w) = &loaded.warning {
        let _ = write!(out, "\n  warning: {w}");
    }
    out
}

/// `ai doctor` — 현재 환경/플랫폼 capability를 표시한다.
///
/// MVP에서는 정적 분석·preview·timeout 등 baseline guardrails를 모든 플랫폼에서
/// 보장하고, 동적 감시(seccomp/cgroups 등)는 플랫폼별로 다르다(§31.11).
fn run_doctor(show_guardrails: bool) -> anyhow::Result<()> {
    println!("AI Terminal doctor");
    println!("  version : {}", env!("CARGO_PKG_VERSION"));
    println!("{}", format_config_diagnostics(&config::load()));
    println!("  os      : {}", std::env::consts::OS);
    println!("  arch    : {}", std::env::consts::ARCH);

    // 통합 모드(§30-1): hook 마커가 현재 셸에 있으면 hook, 아니면 wrapper fallback.
    let hook_on = shell::hook_active(|k| std::env::var(k).ok());
    let mode = shell::resolve_integration_mode(shell::ConfiguredMode::Auto, hook_on);
    match mode {
        shell::IntegrationMode::Hook => println!("  shell   : hook 통합 활성"),
        shell::IntegrationMode::Wrapper => {
            println!("  shell   : wrapper fallback (hook 미감지)");
            println!(
                "            명령을 `ai exec \"<cmd>\"`로 실행하면 컨텍스트가 기록됩니다. hook 설치: `ai init shell`."
            );
        }
    }

    if show_guardrails {
        let platform = guardrails::detect();
        println!("\nplatform : {platform:?}  (정본 §31.11)");
        println!("baseline guardrails (모든 플랫폼):");
        for g in guardrails::baseline() {
            println!("  - {g}");
        }
        println!("platform-specific (동적 감시):");
        for c in guardrails::capabilities(platform) {
            println!("  - {:<28} {:?}", c.name, c.support);
        }
        if guardrails::dynamic_monitoring_limited(platform) {
            println!(
                "\n[!] 동적 감시가 제한되는 플랫폼입니다. High 이상 명령 확인을 강화합니다(§31.11)."
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_click_launch_only_when_sole_console_process() {
        // 자기 콘솔 단독 점유(1) = 더블클릭. 터미널 실행(부모 셸 attach, 2+)·감지 실패(None)는 아님.
        assert!(is_double_click_launch(Some(1)));
        assert!(!is_double_click_launch(Some(2)));
        assert!(!is_double_click_launch(Some(5)));
        assert!(!is_double_click_launch(None));
    }

    #[test]
    fn cli_parses_doctor_with_guardrails() {
        let cli = Cli::try_parse_from(["ai", "doctor", "--guardrails"]).unwrap();
        match cli.command {
            Some(Command::Doctor { guardrails }) => assert!(guardrails),
            _ => panic!("expected doctor subcommand"),
        }
    }

    #[test]
    fn cli_parses_bare_invocation() {
        let cli = Cli::try_parse_from(["ai"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn cli_parses_risk_command_without_profile() {
        let cli = Cli::try_parse_from(["ai", "risk", "rm -rf /"]).unwrap();
        match cli.command {
            Some(Command::Risk { command, profile }) => {
                assert_eq!(command, "rm -rf /");
                assert_eq!(profile, None);
            }
            _ => panic!("expected risk subcommand"),
        }
    }

    #[test]
    fn cli_parses_explain_last_error_without_command() {
        let cli = Cli::try_parse_from(["ai", "explain", "--last-error"]).unwrap();
        match cli.command {
            Some(Command::Explain {
                command,
                last_error,
                ..
            }) => {
                assert!(last_error);
                assert!(command.is_none(), "--last-error 는 명령 생략을 허용한다");
            }
            _ => panic!("expected explain subcommand"),
        }
    }

    #[test]
    fn cli_parses_policy_show_and_set() {
        let show = Cli::try_parse_from(["ai", "policy", "show", "--profile", "paranoid"]).unwrap();
        match show.command {
            Some(Command::Policy {
                action: PolicyAction::Show { profile },
            }) => assert_eq!(profile.as_deref(), Some("paranoid")),
            _ => panic!("expected policy show"),
        }
        let set = Cli::try_parse_from(["ai", "policy", "set", "paranoid"]).unwrap();
        match set.command {
            Some(Command::Policy {
                action: PolicyAction::Set { profile },
            }) => assert_eq!(profile, "paranoid"),
            _ => panic!("expected policy set"),
        }
    }

    #[test]
    fn format_risk_reports_score_level_and_decision() {
        let out = format_risk("rm -rf /", &PolicyProfile::balanced());
        assert!(out.contains("Critical"), "should report level: {out}");
        assert!(out.contains("100"), "should report score: {out}");
        assert!(out.contains("Block"), "balanced must Block critical: {out}");
    }

    #[test]
    fn describe_profile_shows_remote_ai_setting() {
        let out = describe_profile(&PolicyProfile::paranoid());
        assert!(out.contains("paranoid"), "{out}");
        assert!(out.to_lowercase().contains("remote_ai"), "{out}");
    }

    #[test]
    fn format_risk_flags_unknown_binary() {
        let out = format_risk("definitely_not_real_xyz123 foo", &PolicyProfile::balanced());
        assert!(
            out.contains("UNKNOWN"),
            "unknown binary should be flagged: {out}"
        );
    }

    #[test]
    fn format_risk_marks_builtin() {
        let out = format_risk("cd /tmp", &PolicyProfile::balanced());
        assert!(out.contains("builtin"), "{out}");
    }

    #[test]
    fn format_explain_reports_cause_and_suggestions() {
        let out = format_explain("frob", 127, "command not found");
        assert!(out.contains("찾을 수 없"), "{out}");
        assert!(out.contains("제안"), "{out}");
    }

    #[test]
    fn format_preview_lists_delete_targets() {
        let out = format_preview("rm -rf ./build");
        assert!(out.contains("preview  :"), "{out}");
        assert!(out.contains("./build"), "{out}");
    }

    #[test]
    fn format_preview_flags_not_available() {
        let out = format_preview("sudo systemctl restart nginx");
        assert!(out.contains("불가"), "{out}");
    }

    #[test]
    fn format_mask_redacts_and_blocks() {
        let out = format_mask("-----BEGIN RSA PRIVATE KEY-----");
        assert!(out.contains("PRIVATE_KEY_REDACTED"), "{out}");
        assert!(out.contains("BLOCKED"), "{out}");
    }

    #[test]
    fn format_mask_eligible_for_clean_text() {
        let out = format_mask("ls -al");
        assert!(out.contains("eligible"), "{out}");
        assert!(out.contains("(none)"), "{out}");
    }

    #[test]
    fn resolve_profile_rejects_unknown() {
        assert!(resolve_profile("nonexistent").is_err());
        assert!(resolve_profile("balanced").is_ok());
    }

    #[test]
    fn cli_parses_ask() {
        let cli = Cli::try_parse_from(["ai", "ask", "what time is it"]).unwrap();
        match cli.command {
            Some(Command::Ask { prompt, .. }) => assert_eq!(prompt, "what time is it"),
            _ => panic!("expected ask"),
        }
    }

    #[test]
    fn cli_parses_remote_daemon() {
        assert!(matches!(
            Cli::try_parse_from(["ai", "remote", "daemon"])
                .unwrap()
                .command,
            Some(Command::Remote {
                action: RemoteAction::Daemon {}
            })
        ));
    }

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
            Cli::try_parse_from(["ai", "remote", "disarm"])
                .unwrap()
                .command,
            Some(Command::Remote {
                action: RemoteAction::Disarm {}
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["ai", "remote", "status"])
                .unwrap()
                .command,
            Some(Command::Remote {
                action: RemoteAction::Status {}
            })
        ));
    }

    #[test]
    fn cli_parses_gate() {
        let cli = Cli::try_parse_from(["ai", "__gate", "rm", "-rf", "/"]).unwrap();
        match cli.command {
            Some(Command::Gate { command }) => assert_eq!(command.join(" "), "rm -rf /"),
            _ => panic!("expected gate"),
        }
    }

    #[test]
    fn cli_parses_shell_hook() {
        let cli = Cli::try_parse_from(["ai", "shell-hook", "zsh"]).unwrap();
        match cli.command {
            Some(Command::ShellHook { shell }) => assert_eq!(shell, "zsh"),
            _ => panic!("expected shell-hook"),
        }
    }

    #[test]
    fn cli_parses_init_shell_dry_run() {
        let cli = Cli::try_parse_from(["ai", "init", "shell", "--dry-run"]).unwrap();
        match cli.command {
            Some(Command::Init {
                target: InitTarget::Shell { dry_run, .. },
            }) => assert!(dry_run),
            _ => panic!("expected init shell"),
        }
    }

    #[test]
    fn plan_dry_run_never_writes() {
        let p = plan_init_shell(
            "export X=1\n",
            Shell::Bash,
            InitMode::DryRun,
            "/tmp/.bashrc",
        );
        assert!(!p.write, "dry-run must not write");
        assert!(p.message.contains("dry-run"));
    }

    #[test]
    fn plan_diff_shows_diff_without_writing() {
        let p = plan_init_shell("export X=1\n", Shell::Bash, InitMode::Diff, "/tmp/.bashrc");
        assert!(!p.write, "diff is preview-only");
        assert!(
            p.message.contains('+'),
            "should show added lines: {}",
            p.message
        );
    }

    #[test]
    fn plan_install_writes_block_then_idempotent() {
        let p = plan_init_shell(
            "export X=1\n",
            Shell::Bash,
            InitMode::Install,
            "/tmp/.bashrc",
        );
        assert!(p.write);
        assert!(shell::is_installed(&p.new_content));
        let again = plan_init_shell(
            &p.new_content,
            Shell::Bash,
            InitMode::Install,
            "/tmp/.bashrc",
        );
        assert!(!again.write, "second install must be a no-op");
    }

    #[test]
    fn plan_uninstall_writes_removal() {
        let installed = shell::apply_install("export X=1\n", Shell::Zsh);
        let p = plan_init_shell(&installed, Shell::Zsh, InitMode::Uninstall, "/tmp/.zshrc");
        assert!(p.write);
        assert!(!shell::is_installed(&p.new_content));
    }

    #[test]
    fn parses_exec_command() {
        let cli = Cli::parse_from(["ai", "exec", "rm -rf build", "--yes"]);
        match cli.command {
            Some(Command::Exec {
                command,
                yes,
                profile,
            }) => {
                assert_eq!(command, "rm -rf build");
                assert!(yes);
                assert!(profile.is_none());
            }
            other => panic!("expected Exec, got {other:?}"),
        }
    }

    #[test]
    fn parses_dispatch_command() {
        let cli = Cli::parse_from(["ai", "dispatch", "ls -al", "--yes"]);
        match cli.command {
            Some(Command::Dispatch {
                input,
                yes,
                profile,
            }) => {
                assert_eq!(input, "ls -al");
                assert!(yes);
                assert!(profile.is_none());
            }
            other => panic!("expected Dispatch, got {other:?}"),
        }
    }

    #[test]
    fn cache_badge_labels() {
        use ai_terminal::cache::CacheSource;
        assert_eq!(cache_badge(CacheSource::Backend), "");
        assert!(cache_badge(CacheSource::Exact).contains("exact"));
        assert!(cache_badge(CacheSource::Semantic).contains("semantic"));
    }

    #[test]
    fn config_diagnostics_show_file_source_and_values() {
        let loaded = ai_terminal::config::LoadedConfig {
            config: ai_terminal::config::Config {
                general: ai_terminal::config::General {
                    default_shell: Some("/bin/bash".to_string()),
                    history_limit: 123,
                },
                ai: Default::default(),
            },
            source: ai_terminal::config::ConfigSource::File(std::path::PathBuf::from(
                "/cfg/config.toml",
            )),
            warning: None,
        };
        let out = format_config_diagnostics(&loaded);
        assert!(out.contains("file: /cfg/config.toml"), "{out}");
        assert!(out.contains("general.history_limit = 123"), "{out}");
        assert!(out.contains("general.default_shell = /bin/bash"), "{out}");
    }

    #[test]
    fn config_diagnostics_show_default_and_warning() {
        let loaded = ai_terminal::config::LoadedConfig {
            config: ai_terminal::config::Config::default(),
            source: ai_terminal::config::ConfigSource::Default,
            warning: Some("boom".to_string()),
        };
        let out = format_config_diagnostics(&loaded);
        assert!(out.contains("default (no file)"), "{out}");
        assert!(out.contains("general.default_shell = <unset>"), "{out}");
        assert!(out.contains("warning: boom"), "{out}");
    }
}
