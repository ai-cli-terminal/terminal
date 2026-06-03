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

/// `ai preview` 출력 문자열을 만든다.
fn format_preview(command: &str) -> String {
    use preview::PreviewPlan;
    match preview::classify_preview(command) {
        PreviewPlan::NotNeeded => "preview  : 불필요 (파일 변경 없음)\n".to_string(),
        PreviewPlan::DryRun(c) => format!("preview  : dry-run 제안\n  {c}\n"),
        PreviewPlan::TempCopyDiff => {
            "preview  : 임시 복사본에서 실행 후 diff (적용 전 변경 확인)\n".to_string()
        }
        PreviewPlan::ListTargets(targets) => {
            let mut s = format!("preview  : 대상 {}개\n", targets.len());
            for t in &targets {
                s.push_str(&format!("  - {t}\n"));
            }
            s
        }
        PreviewPlan::NotAvailable(reason) => {
            format!("preview  : 불가 — {reason}\n")
        }
    }
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
            let cap = ai_terminal::provider::Provider::mock().models[0].clone();
            let gw = match backend.as_str() {
                "ollama" => {
                    let b = ai_terminal::ollama::OllamaBackend::new(
                        ai_terminal::http::TcpTransport,
                        &ollama_url,
                        &model,
                    );
                    gateway::Gateway::new(Box::new(b), cap)
                }
                "openai" => {
                    let api_key = std::env::var("OPENAI_API_KEY").ok();
                    let b = ai_terminal::openai::OpenAiBackend::new(
                        ai_terminal::http::TcpTransport,
                        &openai_url,
                        &model,
                        api_key,
                    );
                    gateway::Gateway::new(Box::new(b), cap)
                }
                _ => gateway::Gateway::mock(),
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
                }) => {
                    println!("{text}");
                    println!("(tokens ~ in:{input_tokens} out:{output_tokens})");
                    #[cfg(feature = "storage")]
                    if let Ok(store) = ai_terminal::store::Store::open_default() {
                        let _ = store.record_usage(
                            "mock",
                            "mock-model",
                            input_tokens as i64,
                            output_tokens as i64,
                            0,
                            0.0,
                            None,
                        );
                    }
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
            ..
        }) => {
            #[cfg(feature = "storage")]
            if let Ok(store) = ai_terminal::store::Store::open_default() {
                let _ = store.record_usage(
                    "mock",
                    "mock-model",
                    input_tokens as i64,
                    output_tokens as i64,
                    0,
                    0.0,
                    None,
                );
            }
            // 답변 본문은 이미 sink(stdout)로 출력됨. 토큰 요약만 덧붙인다.
            println!("\n(tokens ~ in:{input_tokens} out:{output_tokens})");
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

#[cfg(feature = "storage")]
fn record_exec(command: &str, exit_code: i32, source: &str) {
    use ai_terminal::store::{NewCommand, NewSession, Store};
    let Ok(store) = Store::open_default() else {
        return;
    };
    let a = risk::assess(command);
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .ok();
    let _ = store.get_or_create_session(
        "sess-default",
        &NewSession {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "unknown".into()),
            hostname: std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".into()),
            cwd: cwd.clone().unwrap_or_default(),
            policy_profile: config::get_active_profile(),
        },
    );
    let _ = store.record_command(&NewCommand {
        session_id: "sess-default".into(),
        command_text: command.into(),
        source: source.into(),
        cwd,
        exit_code: Some(exit_code as i64),
        risk_level: Some(format!("{:?}", a.level)),
        risk_score: Some(a.score as i64),
        ai_generated: false,
        confirmed: true,
    });
    let _ = store.record_audit(
        "command_executed",
        Some(&format!("{:?}", a.level)),
        Some(&config::get_active_profile()),
        &format!("{{\"exit\":{exit_code}}}"),
    );
}

#[cfg(not(feature = "storage"))]
fn record_exec(_command: &str, _exit_code: i32, _source: &str) {}

/// 비-Ran 셸 결과의 audit 레코드. 순수 데이터 — storage/exit 비의존이라 단위 테스트 가능.
#[allow(dead_code)]
struct AuditRecord {
    event_type: &'static str,
    level: String,
    payload_json: String,
}

/// 비-Ran 결과를 audit 레코드로 변환한다. `Ran`은 `record_exec`가 처리하므로 `None`.
/// 명령은 W7 마스킹 후 payload에 담고, level은 Blocked의 carried 값 또는 재산출값을 쓴다.
fn shell_outcome_audit(
    command: &str,
    source: &str,
    outcome: &ai_terminal::pipeline::ExecOutcome,
) -> Option<AuditRecord> {
    use ai_terminal::pipeline::ExecOutcome;

    let (event_type, level, mut payload) = match outcome {
        ExecOutcome::Ran { .. } => return None,
        ExecOutcome::Blocked { level, factors } => (
            "command_blocked",
            format!("{level:?}"),
            serde_json::json!({ "factors": factors }),
        ),
        ExecOutcome::Declined => (
            "command_declined",
            format!("{:?}", risk::assess(command).level),
            serde_json::json!({}),
        ),
        ExecOutcome::BackupRefused(reason) => (
            "command_backup_refused",
            format!("{:?}", risk::assess(command).level),
            serde_json::json!({ "reason": reason }),
        ),
    };

    let masked = mask::Masker::baseline().mask(command).text;
    let map = payload
        .as_object_mut()
        .expect("audit payload must be a JSON object");
    map.insert("command".into(), serde_json::Value::String(masked));
    map.insert(
        "source".into(),
        serde_json::Value::String(source.to_owned()),
    );

    Some(AuditRecord {
        event_type,
        level,
        payload_json: payload.to_string(),
    })
}

/// audit 레코드를 영속화한다(storage feature). 실패는 조용히 무시(감사는 best-effort).
#[cfg(feature = "storage")]
fn record_outcome_audit(rec: &AuditRecord) {
    use ai_terminal::store::Store;
    let Ok(store) = Store::open_default() else {
        return;
    };
    let _ = store.record_audit(
        rec.event_type,
        Some(&rec.level),
        Some(&config::get_active_profile()),
        &rec.payload_json,
    );
}

#[cfg(not(feature = "storage"))]
fn record_outcome_audit(_rec: &AuditRecord) {}

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
        record_exec(command, *exit_code, source);
        std::process::exit(*exit_code);
    }

    // 비-Ran: audit 기록 후 안내 + exit 1.
    if let Some(rec) = shell_outcome_audit(command, source, &outcome) {
        record_outcome_audit(&rec);
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

/// `ai doctor` — 현재 환경/플랫폼 capability를 표시한다.
///
/// MVP에서는 정적 분석·preview·timeout 등 baseline guardrails를 모든 플랫폼에서
/// 보장하고, 동적 감시(seccomp/cgroups 등)는 플랫폼별로 다르다(§31.11).
fn run_doctor(show_guardrails: bool) -> anyhow::Result<()> {
    println!("AI Terminal doctor");
    println!("  version : {}", env!("CARGO_PKG_VERSION"));
    println!("  os      : {}", std::env::consts::OS);
    println!("  arch    : {}", std::env::consts::ARCH);

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
    fn shell_outcome_audit_ran_is_none() {
        use ai_terminal::pipeline::ExecOutcome;
        let out = ExecOutcome::Ran {
            exit_code: 0,
            undo_id: None,
        };
        assert!(shell_outcome_audit("ls -al", "exec", &out).is_none());
    }

    #[test]
    fn shell_outcome_audit_blocked_has_type_level_factors() {
        use ai_terminal::pipeline::ExecOutcome;
        use ai_terminal::risk::RiskLevel;
        let out = ExecOutcome::Blocked {
            level: RiskLevel::Critical,
            factors: vec!["재귀 삭제 (+30)".to_string()],
        };
        let rec = shell_outcome_audit("rm -rf /", "exec", &out).expect("blocked → Some");
        assert_eq!(rec.event_type, "command_blocked");
        assert_eq!(rec.level, "Critical");
        assert!(rec.payload_json.contains("\"factors\""));
        assert!(rec.payload_json.contains("재귀 삭제 (+30)"));
        assert!(rec.payload_json.contains("\"source\":\"exec\""));
        assert!(rec.payload_json.contains("\"command\""));
    }

    #[test]
    fn shell_outcome_audit_declined_reassesses_level() {
        use ai_terminal::pipeline::ExecOutcome;
        let rec = shell_outcome_audit("rm -rf /", "dispatch", &ExecOutcome::Declined)
            .expect("declined → Some");
        assert_eq!(rec.event_type, "command_declined");
        assert_eq!(rec.level, "Critical");
        assert!(rec.payload_json.contains("\"source\":\"dispatch\""));
    }

    #[test]
    fn shell_outcome_audit_backup_refused_has_reason() {
        use ai_terminal::pipeline::ExecOutcome;
        let out = ExecOutcome::BackupRefused("파일 크기 초과".to_string());
        let rec = shell_outcome_audit("rm /tmp/x", "exec", &out).expect("refused → Some");
        assert_eq!(rec.event_type, "command_backup_refused");
        assert!(rec.payload_json.contains("\"reason\":\"파일 크기 초과\""));
    }

    #[test]
    fn shell_outcome_audit_masks_secret_in_command() {
        use ai_terminal::pipeline::ExecOutcome;
        let token = "ghp_0123456789abcdef0123456789abcdef0123";
        let cmd = format!("echo {token}");
        let rec =
            shell_outcome_audit(&cmd, "exec", &ExecOutcome::Declined).expect("declined → Some");
        assert!(
            !rec.payload_json.contains(token),
            "원문 secret 이 payload 에 잔존하면 안 됨: {}",
            rec.payload_json
        );
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
        assert!(out.contains("대상"), "{out}");
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
}
