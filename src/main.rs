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
        #[command(subcommand)]
        action: Option<SkillAction>,
    },
    /// 릴리스 바이너리 manifest 서명 상태를 진단한다 (P3 trust channel).
    Release {
        #[command(subcommand)]
        action: ReleaseAction,
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
    /// 조직 정책 진단.
    Org {
        #[command(subcommand)]
        action: PolicyOrgAction,
    },
}

#[derive(Subcommand, Debug)]
enum PolicyOrgAction {
    /// signed policy.d 파일 세트와 검증 상태를 표시한다.
    Status,
}

#[derive(Subcommand, Debug)]
enum SkillAction {
    /// 외부(user config) 스킬을 명시적으로 활성화한다.
    Enable {
        /// 활성화할 스킬 이름.
        name: String,
        /// 확인 프롬프트를 생략한다(자동화용).
        #[arg(long)]
        yes: bool,
    },
    /// 외부(user config) 스킬 활성화를 해제한다.
    Disable {
        /// 비활성화할 스킬 이름.
        name: String,
    },
    /// 명시적으로 활성화된 외부 스킬 이름을 표시한다.
    Enabled,
    /// 조직 스킬 레지스트리 진단.
    Registry {
        #[command(subcommand)]
        action: SkillRegistryAction,
    },
}

#[derive(Subcommand, Debug)]
enum SkillRegistryAction {
    /// signed organization skill registry 파일 세트와 검증 상태를 표시한다.
    Status,
    /// signed registry payload와 manifest를 검증 후 활성 registry로 설치한다.
    Update {
        /// 설치할 signed registry JSON 파일.
        #[arg(long)]
        registry: PathBuf,
        /// registry payload를 바인딩한 signed manifest JSON 파일.
        #[arg(long)]
        manifest: PathBuf,
    },
    /// revoked entry가 포함된 signed registry snapshot을 검증 후 설치한다.
    Revoke {
        /// 설치할 signed registry JSON 파일.
        #[arg(long)]
        registry: PathBuf,
        /// registry payload를 바인딩한 signed manifest JSON 파일.
        #[arg(long)]
        manifest: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum ReleaseAction {
    /// signed binary release manifest 진단.
    Manifest {
        #[command(subcommand)]
        action: ReleaseManifestAction,
    },
}

#[derive(Subcommand, Debug)]
enum ReleaseManifestAction {
    /// active signed binary manifest 파일 세트와 검증 상태를 표시한다.
    Status,
    /// signed binary manifest payload와 manifest를 검증한다.
    Verify {
        /// 검증할 binary-manifest JSON 파일.
        #[arg(long)]
        payload: PathBuf,
        /// binary-manifest payload를 바인딩한 signed manifest JSON 파일.
        #[arg(long)]
        manifest: PathBuf,
        /// 선택: manifest 안의 특정 artifact 이름.
        #[arg(long)]
        name: Option<String>,
        /// 선택: hash를 계산해 signed manifest entry와 대조할 artifact 파일.
        #[arg(long)]
        artifact: Option<PathBuf>,
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
    Daemon {
        /// 원격 승인에 사용할 등록 디바이스 id. 미지정 시 등록 디바이스가 정확히 1개여야 한다.
        #[arg(long)]
        device_id: Option<String>,
        /// 요청할 companion transport mode. 기본값은 live-loopback이며 relay는 setup JSON 발급까지만 수행한다.
        #[arg(long, default_value = "live-loopback")]
        transport: String,
        /// --transport relay에서 사용할 self-hosted WebSocket relay endpoint URL.
        #[arg(long)]
        relay_endpoint_url: Option<String>,
        /// --transport relay에서 발급할 setup deployment mode(self-hosted|private-network).
        #[arg(long, default_value = "self-hosted")]
        relay_deployment_mode: String,
        /// --relay-deployment-mode private-network에서 요구되는 private network 이름.
        #[arg(long)]
        private_network_name: Option<String>,
        /// --transport relay에서 발급할 relay session ticket TTL(초).
        #[arg(long, default_value_t = 300)]
        relay_ttl_seconds: u64,
    },
    /// 등록된 원격 승인 디바이스를 나열한다(RA-2/RA-5 운영 helper).
    Devices {},
    /// 원격 companion transport mode와 후보 상태를 표시한다(Relay/M2 kickoff).
    Transport {},
    /// self-hosted WebSocket relay setup JSON을 발급한다(Relay/M2 runtime issuer).
    RelaySetup {
        /// self-hosted WebSocket relay endpoint URL. Production은 wss://, local smoke는 localhost ws://만 허용한다.
        #[arg(long)]
        relay_endpoint_url: String,
        /// 발급할 setup deployment mode(self-hosted|private-network).
        #[arg(long, default_value = "self-hosted")]
        relay_deployment_mode: String,
        /// --relay-deployment-mode private-network에서 요구되는 private network 이름.
        #[arg(long)]
        private_network_name: Option<String>,
        /// relay ticket을 발급할 등록 디바이스 id. 미지정 시 등록 디바이스가 정확히 1개여야 한다.
        #[arg(long)]
        device_id: Option<String>,
        /// relay session ticket TTL(초).
        #[arg(long, default_value_t = 300)]
        ttl_seconds: u64,
    },
    /// 디바이스 페어링을 시작하거나 완료한다(RA-2).
    Pair {
        /// 등록할 디바이스 id. 지정하면 complete 모드로 동작한다.
        #[arg(long)]
        device_id: Option<String>,
        /// 6자리 페어링 코드. complete 모드에서 필요하다.
        #[arg(long)]
        code: Option<String>,
        /// 디바이스 Noise static public key(hex). complete 모드에서 필요하다.
        #[arg(long)]
        noise_pubkey_hex: Option<String>,
        /// 디바이스 Ed25519 approval public key(hex, 32 bytes). complete 모드에서 필요하다.
        #[arg(long)]
        approval_pubkey_hex: Option<String>,
        /// start 모드에서 pairing code TTL(초).
        #[arg(long, default_value_t = 300)]
        ttl_seconds: u64,
        /// start 모드에서 payload를 붙일 PWA URL. 지정하면 스캔 가능한 PWA QR을 함께 출력한다.
        #[arg(long)]
        pwa_url: Option<String>,
    },
    /// 승인 요청 JSON을 PWA/URL/QR payload로 변환한다(RA-5 bridge helper).
    ApprovalUrl {
        /// ApprovalRequestMsg JSON.
        #[arg(long)]
        request_json: String,
        /// payload를 붙일 PWA URL. 지정하면 PWA-opening URL/QR도 출력한다.
        #[arg(long)]
        pwa_url: Option<String>,
    },
    /// PWA가 만든 ApprovalResponseMsg JSON을 등록 approval pubkey 기준으로 검증한다.
    ApprovalVerify {
        /// 원본 ApprovalRequestMsg JSON.
        #[arg(long)]
        request_json: String,
        /// PWA가 생성한 ApprovalResponseMsg JSON.
        #[arg(long)]
        response_json: String,
        /// 등록된 디바이스 id. 지정하면 remote-devices.json의 approval key/epoch로 검증한다.
        #[arg(long)]
        device_id: Option<String>,
        /// 디바이스 Ed25519 approval public key(hex, 32 bytes). device-id 없이 직접 검증할 때 사용한다.
        #[arg(long)]
        approval_pubkey_hex: Option<String>,
        /// 현재 디바이스 epoch. 직접 pubkey 검증 모드에서 미지정 시 요청의 device_epoch를 사용한다.
        #[arg(long)]
        device_epoch: Option<u64>,
        /// 검증 기준 Unix time seconds. 미지정 시 현재 시간을 사용한다.
        #[arg(long)]
        now: Option<u64>,
        /// 실행 직전 context hash. 미지정 시 요청의 context_hash를 사용한다.
        #[arg(long)]
        context_hash: Option<String>,
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

#[cfg(feature = "trust")]
fn resolve_effective_policy(name: &str) -> anyhow::Result<ai_terminal::policy_d::EffectivePolicy> {
    let user_profile = resolve_profile(name)?;
    ai_terminal::policy_d::resolve_effective_profile(user_profile).map_err(anyhow::Error::from)
}

#[cfg(feature = "trust")]
fn resolve_effective_profile(name: &str) -> anyhow::Result<PolicyProfile> {
    Ok(resolve_effective_policy(name)?.profile)
}

#[cfg(not(feature = "trust"))]
fn resolve_effective_profile(name: &str) -> anyhow::Result<PolicyProfile> {
    resolve_profile(name)
}

fn resolve_requested_profile(profile: Option<String>) -> anyhow::Result<PolicyProfile> {
    resolve_effective_profile(&profile.unwrap_or_else(config::get_active_profile))
}

#[cfg(feature = "trust")]
fn describe_policy_source(source: &ai_terminal::policy_d::PolicySource) -> String {
    match source {
        ai_terminal::policy_d::PolicySource::UserActiveProfile { profile } => {
            format!("source    : user active profile ({profile})\n")
        }
        ai_terminal::policy_d::PolicySource::OrganizationPolicy {
            subject,
            version,
            manifest_id,
            policy_path,
        } => format!(
            "source    : organization policy\n\
             subject   : {subject}\n\
             version   : {version}\n\
             manifest  : {manifest_id}\n\
             path      : {}\n",
            policy_path.display()
        ),
    }
}

#[cfg(feature = "trust")]
fn describe_effective_policy(name: &str) -> anyhow::Result<String> {
    let effective = resolve_effective_policy(name)?;
    let mut out = describe_profile(&effective.profile);
    out.push_str(&describe_policy_source(&effective.source));
    Ok(out)
}

#[cfg(not(feature = "trust"))]
fn describe_effective_policy(name: &str) -> anyhow::Result<String> {
    Ok(describe_profile(&resolve_profile(name)?))
}

#[cfg(feature = "trust")]
fn path_state(path: &std::path::Path, check_readonly: bool) -> String {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            if check_readonly {
                format!("exists readonly={}", metadata.permissions().readonly())
            } else {
                "exists".to_string()
            }
        }
        Err(_) => "missing".to_string(),
    }
}

#[cfg(feature = "trust")]
fn run_policy_org_status() -> anyhow::Result<()> {
    let paths = ai_terminal::policy_d::default_policy_d_paths().map_err(anyhow::Error::from)?;
    println!("organization_policy :");
    println!(
        "  policy   : {} ({})",
        paths.policy_path.display(),
        path_state(&paths.policy_path, true)
    );
    println!(
        "  manifest : {} ({})",
        paths.manifest_path.display(),
        path_state(&paths.manifest_path, false)
    );
    println!(
        "  anchor   : {} ({})",
        paths.anchor_path.display(),
        path_state(&paths.anchor_path, false)
    );
    println!("  anchor_source: {}", paths.anchor_source.as_str());
    println!("  subject  : {}", paths.expected_subject);

    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    match ai_terminal::policy_d::load_default_organization_policy(now) {
        Ok(Some(loaded)) => {
            println!("status    : active");
            println!("runtime   : organization policy overrides user active profile");
            println!("profile   : {}", loaded.verified.profile.name);
            println!(
                "skill_external_sources: {}",
                loaded.verified.external_skill_sources.as_str()
            );
            println!("key_id    : {}", loaded.verified.manifest.key_id);
            println!(
                "manifest  : {}",
                loaded.verified.manifest.manifest.manifest_id
            );
            println!("version   : {}", loaded.verified.manifest.manifest.version);
            println!(
                "issued_at : {}",
                loaded.verified.manifest.manifest.issued_at_unix
            );
            println!(
                "expires_at: {}",
                loaded.verified.manifest.manifest.expires_at_unix
            );
        }
        Ok(None) => {
            println!("status    : absent");
            println!("runtime   : user active profile");
            println!("profile   : {}", config::get_active_profile());
            println!("skill_external_sources: user-enabled");
        }
        Err(e) => {
            println!("status    : invalid");
            println!("runtime   : fail-closed");
            println!("error     : {e}");
        }
    }
    Ok(())
}

#[cfg(not(feature = "trust"))]
fn run_policy_org_status() -> anyhow::Result<()> {
    println!("organization_policy :");
    println!("status    : unavailable");
    println!("runtime   : user active profile");
    println!("reason    : binary was built without the `trust` feature");
    Ok(())
}

fn run_skill_list(query: Option<String>) -> anyhow::Result<()> {
    let paths = default_skill_discovery_paths();
    let enabled = skill::get_enabled_skills();
    let discovered =
        skill::filter_explicitly_enabled_external(skill::discover_with_source(&paths), &enabled);
    let skills: Vec<skill::Skill> = discovered.into_iter().map(|entry| entry.skill).collect();
    #[cfg(feature = "trust")]
    let skills = {
        let mut skills = skills;
        let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
        if let Some(registry) =
            ai_terminal::skill_registry::load_default_organization_skill_registry(now)
                .map_err(anyhow::Error::from)?
        {
            let discovered_count = skills.len();
            skills.retain(|skill| registry.verified.allows_skill(skill));
            ai_terminal::skill_registry::record_skill_registry_audit(
                "skill_registry_enforced",
                &registry,
                Some(discovered_count),
                Some(skills.len()),
            );
        }
        skills
    };
    let shown: Vec<&skill::Skill> = match &query {
        Some(q) => skill::match_skills(&skills, q, 5),
        None => skills.iter().collect(),
    };
    if shown.is_empty() {
        let path_list: Vec<PathBuf> = paths.iter().map(|(path, _)| path.clone()).collect();
        println!("(스킬 없음 — {:?})", path_list);
    }
    for s in shown {
        println!("- {} — {}", s.name, s.description);
    }
    Ok(())
}

fn default_skill_discovery_paths() -> Vec<(PathBuf, skill::SkillSource)> {
    let mut paths = vec![(
        PathBuf::from("./.ai-terminal/skills"),
        skill::SkillSource::Workspace,
    )];
    if let Ok(cd) = config::config_dir() {
        paths.push((cd.join("skills"), skill::SkillSource::External));
    }
    paths
}

fn run_skill_enabled() {
    let enabled = skill::get_enabled_skills();
    println!("external_skill_enabled :");
    if enabled.is_empty() {
        println!("(none)");
    } else {
        for name in enabled {
            println!("- {name}");
        }
    }
}

#[cfg(feature = "trust")]
fn validate_external_skill_enable_policy(
    entry: &skill::DiscoveredSkill,
) -> anyhow::Result<Option<&'static str>> {
    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    let external_source_policy = ai_terminal::policy_d::load_default_organization_policy(now)
        .map_err(anyhow::Error::from)?
        .map(|policy| policy.verified.external_skill_sources)
        .unwrap_or_default();
    let source_policy_label = external_source_policy.as_str();
    let registry = ai_terminal::skill_registry::load_default_organization_skill_registry(now)
        .map_err(anyhow::Error::from)?;
    match external_source_policy {
        ai_terminal::policy_d::ExternalSkillSourcePolicy::Disabled => {
            anyhow::bail!(
                "organization policy disables external skill sources: {}",
                entry.skill.name
            );
        }
        ai_terminal::policy_d::ExternalSkillSourcePolicy::RegistryOnly => {
            let Some(registry) = registry else {
                anyhow::bail!(
                    "organization policy requires a signed skill registry before enabling \
                     external skills: {}",
                    entry.skill.name
                );
            };
            if !registry.verified.allows_skill(&entry.skill) {
                anyhow::bail!(
                    "external skill is not active in the signed organization registry: {}",
                    entry.skill.name
                );
            }
        }
        ai_terminal::policy_d::ExternalSkillSourcePolicy::UserEnabled => {
            if let Some(registry) = registry {
                if !registry.verified.allows_skill(&entry.skill) {
                    anyhow::bail!(
                        "external skill is not active in the signed organization registry: {}",
                        entry.skill.name
                    );
                }
            }
        }
    }
    Ok(Some(source_policy_label))
}

#[cfg(not(feature = "trust"))]
fn validate_external_skill_enable_policy(
    _entry: &skill::DiscoveredSkill,
) -> anyhow::Result<Option<&'static str>> {
    Ok(None)
}

fn skill_enable_confirmation_matches(input: &str, name: &str) -> bool {
    input.trim() == name
}

fn confirm_external_skill_enable(
    entry: &skill::DiscoveredSkill,
    source_policy_label: Option<&str>,
    assume_yes: bool,
) -> anyhow::Result<bool> {
    if assume_yes {
        return Ok(true);
    }

    use std::io::{IsTerminal, Write};
    if !std::io::stdin().is_terminal() {
        anyhow::bail!(
            "external skill enable confirmation requires a TTY; rerun with --yes for automation"
        );
    }

    println!("external_skill : {}", entry.skill.name);
    println!("description    : {}", entry.skill.description);
    println!("source         : {}", entry.source.as_str());
    if let Some(source_policy_label) = source_policy_label {
        println!("source_policy  : {source_policy_label}");
    }
    print!(
        "Type `{}` to enable this external skill: ",
        entry.skill.name
    );
    std::io::stdout().flush()?;

    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(skill_enable_confirmation_matches(&line, &entry.skill.name))
}

fn run_skill_enable(name: String, yes: bool) -> anyhow::Result<()> {
    let paths = default_skill_discovery_paths();
    let discovered = skill::discover_with_source(&paths);
    let matches = skill::external_skills_named(&discovered, &name);
    if matches.is_empty() {
        anyhow::bail!("external skill not found: {name}");
    }
    if matches.len() > 1 {
        anyhow::bail!("multiple external skills named {name}; remove duplicate SKILL.md names");
    }

    let entry = matches[0];
    let source_policy_label = validate_external_skill_enable_policy(entry)?;
    if !confirm_external_skill_enable(entry, source_policy_label, yes)? {
        println!("external_skill : {name}");
        if let Some(source_policy_label) = source_policy_label {
            println!("source_policy  : {source_policy_label}");
        }
        println!("status         : declined");
        println!("reason         : confirmation_required");
        return Ok(());
    }

    let inserted = skill::enable_skill_name(&name)?;
    skill::record_skill_enable_audit("skill_enabled", &name, skill::SkillSource::External);
    println!("external_skill : {name}");
    if let Some(source_policy_label) = source_policy_label {
        println!("source_policy  : {source_policy_label}");
    }
    println!(
        "status         : {}",
        if inserted {
            "enabled"
        } else {
            "already_enabled"
        }
    );
    Ok(())
}

fn run_skill_disable(name: String) -> anyhow::Result<()> {
    let removed = skill::disable_skill_name(&name)?;
    skill::record_skill_enable_audit("skill_disabled", &name, skill::SkillSource::External);
    println!("external_skill : {name}");
    println!(
        "status         : {}",
        if removed {
            "disabled"
        } else {
            "already_disabled"
        }
    );
    Ok(())
}

#[cfg(feature = "trust")]
fn run_skill_registry_status() -> anyhow::Result<()> {
    let paths =
        ai_terminal::skill_registry::default_skill_registry_paths().map_err(anyhow::Error::from)?;
    println!("organization_skill_registry :");
    println!(
        "  registry : {} ({})",
        paths.registry_path.display(),
        path_state(&paths.registry_path, false)
    );
    println!(
        "  manifest : {} ({})",
        paths.manifest_path.display(),
        path_state(&paths.manifest_path, false)
    );
    println!(
        "  anchor   : {} ({})",
        paths.anchor_path.display(),
        path_state(&paths.anchor_path, false)
    );
    println!("  anchor_source: {}", paths.anchor_source.as_str());
    println!("  subject  : {}", paths.expected_subject);

    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    match ai_terminal::skill_registry::load_default_organization_skill_registry(now) {
        Ok(Some(loaded)) => {
            let revoked_names = loaded.verified.revoked_skill_names();
            println!("status    : active");
            println!("runtime   : ai skill filters to active name+hash matches");
            println!("entries   : {}", loaded.verified.entries.len());
            println!("active    : {}", loaded.verified.active_count());
            println!("revoked   : {}", loaded.verified.revoked_count());
            if !revoked_names.is_empty() {
                println!("revoked_names: {}", revoked_names.join(", "));
            }
            println!("key_id    : {}", loaded.verified.manifest.key_id);
            println!(
                "manifest  : {}",
                loaded.verified.manifest.manifest.manifest_id
            );
            println!("version   : {}", loaded.verified.manifest.manifest.version);
            println!(
                "issued_at : {}",
                loaded.verified.manifest.manifest.issued_at_unix
            );
            println!(
                "expires_at: {}",
                loaded.verified.manifest.manifest.expires_at_unix
            );
        }
        Ok(None) => {
            println!("status    : absent");
            println!("runtime   : local skill discovery");
            println!("entries   : 0");
        }
        Err(e) => {
            println!("status    : invalid");
            println!("runtime   : fail-closed");
            println!("error     : {e}");
        }
    }
    Ok(())
}

#[cfg(feature = "trust")]
fn run_skill_registry_update(
    registry: PathBuf,
    manifest: PathBuf,
    require_revoked: bool,
) -> anyhow::Result<()> {
    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    let candidate = ai_terminal::skill_registry::load_default_skill_registry_update_candidate(
        &registry, &manifest, now,
    )
    .map_err(anyhow::Error::from)?;
    if require_revoked && candidate.verified.revoked_count() == 0 {
        anyhow::bail!(
            "signed registry contains no revoked entries; use `ai skill registry update` \
             for non-revocation updates"
        );
    }
    let loaded = ai_terminal::skill_registry::install_default_skill_registry_update(candidate)
        .map_err(anyhow::Error::from)?;
    let event_type = if require_revoked {
        "skill_registry_revoked"
    } else {
        "skill_registry_updated"
    };
    ai_terminal::skill_registry::record_skill_registry_audit(event_type, &loaded, None, None);
    println!("organization_skill_registry :");
    println!("status    : installed");
    println!("event     : {event_type}");
    println!("entries   : {}", loaded.verified.entries.len());
    println!("active    : {}", loaded.verified.active_count());
    println!("revoked   : {}", loaded.verified.revoked_count());
    println!("key_id    : {}", loaded.verified.manifest.key_id);
    println!(
        "manifest  : {}",
        loaded.verified.manifest.manifest.manifest_id
    );
    println!("version   : {}", loaded.verified.manifest.manifest.version);
    println!("registry  : {}", loaded.registry_path.display());
    println!("manifest_path: {}", loaded.manifest_path.display());
    Ok(())
}

#[cfg(not(feature = "trust"))]
fn run_skill_registry_update(
    _registry: PathBuf,
    _manifest: PathBuf,
    _require_revoked: bool,
) -> anyhow::Result<()> {
    anyhow::bail!("skill registry update requires the `trust` feature")
}

#[cfg(not(feature = "trust"))]
fn run_skill_registry_status() -> anyhow::Result<()> {
    println!("organization_skill_registry :");
    println!("status    : unavailable");
    println!("runtime   : local skill discovery");
    println!("reason    : binary was built without the `trust` feature");
    Ok(())
}

#[cfg(feature = "trust")]
fn run_release_manifest_status() -> anyhow::Result<()> {
    let paths = ai_terminal::binary_manifest::default_binary_manifest_paths()
        .map_err(anyhow::Error::from)?;
    println!("organization_binary_manifest :");
    println!(
        "  payload  : {} ({})",
        paths.payload_path.display(),
        path_state(&paths.payload_path, false)
    );
    println!(
        "  manifest : {} ({})",
        paths.manifest_path.display(),
        path_state(&paths.manifest_path, false)
    );
    println!(
        "  anchor   : {} ({})",
        paths.anchor_path.display(),
        path_state(&paths.anchor_path, false)
    );
    println!("  anchor_source: {}", paths.anchor_source.as_str());
    println!("  subject  : {}", paths.expected_subject);

    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    match ai_terminal::binary_manifest::load_default_organization_binary_manifest(now) {
        Ok(Some(loaded)) => {
            println!("status    : active");
            println!("runtime   : install/update callers can require signed artifact matches");
            println!("artifacts : {}", loaded.verified.artifact_count());
            println!("key_id    : {}", loaded.verified.manifest.key_id);
            println!(
                "manifest  : {}",
                loaded.verified.manifest.manifest.manifest_id
            );
            println!("version   : {}", loaded.verified.manifest.manifest.version);
            println!(
                "issued_at : {}",
                loaded.verified.manifest.manifest.issued_at_unix
            );
            println!(
                "expires_at: {}",
                loaded.verified.manifest.manifest.expires_at_unix
            );
        }
        Ok(None) => {
            println!("status    : absent");
            println!(
                "runtime   : release downloads rely on checksums unless caller supplies a manifest"
            );
            println!("artifacts : 0");
        }
        Err(e) => {
            println!("status    : invalid");
            println!("runtime   : install/update enforcement must fail closed");
            println!("error     : {e}");
        }
    }
    Ok(())
}

#[cfg(feature = "trust")]
fn run_release_manifest_verify(
    payload: PathBuf,
    manifest: PathBuf,
    name: Option<String>,
    artifact: Option<PathBuf>,
) -> anyhow::Result<()> {
    if name.is_some() != artifact.is_some() {
        anyhow::bail!("--name and --artifact must be provided together");
    }

    let now = ai_terminal::policy_d::current_unix_time().map_err(anyhow::Error::from)?;
    let verified = ai_terminal::binary_manifest::load_default_binary_manifest_from_files(
        &payload, &manifest, now,
    )
    .map_err(anyhow::Error::from)?;

    println!("organization_binary_manifest :");
    println!("status    : verified");
    println!("artifacts : {}", verified.artifact_count());
    println!("key_id    : {}", verified.manifest.key_id);
    println!("manifest  : {}", verified.manifest.manifest.manifest_id);
    println!("version   : {}", verified.manifest.manifest.version);
    println!("payload   : {}", payload.display());
    println!("manifest_path: {}", manifest.display());

    if let (Some(name), Some(artifact)) = (name, artifact) {
        let sha256 = ai_terminal::binary_manifest::verify_artifact_against_manifest(
            &verified, &name, &artifact,
        )
        .map_err(anyhow::Error::from)?;
        println!("artifact  : {name}");
        println!("artifact_path: {}", artifact.display());
        println!("sha256    : {sha256}");
        println!("artifact_status: matched");
    }
    Ok(())
}

#[cfg(not(feature = "trust"))]
fn run_release_manifest_status() -> anyhow::Result<()> {
    println!("organization_binary_manifest :");
    println!("status    : unavailable");
    println!("runtime   : release downloads rely on checksums");
    println!("reason    : binary was built without the `trust` feature");
    Ok(())
}

#[cfg(not(feature = "trust"))]
fn run_release_manifest_verify(
    _payload: PathBuf,
    _manifest: PathBuf,
    _name: Option<String>,
    _artifact: Option<PathBuf>,
) -> anyhow::Result<()> {
    anyhow::bail!("binary release manifest verification requires the `trust` feature")
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
            let req = daemon::GateRequest {
                command: command.to_string(),
                context_origin: Some(ai_terminal::context::RemoteContextOrigin::gather()),
            };
            if let Ok(reply) = daemon::query_with_context(&sock, &req) {
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
fn run_gate_daemon(
    device_id: Option<String>,
    transport: String,
    relay_endpoint_url: Option<String>,
    relay_deployment_mode: String,
    private_network_name: Option<String>,
    relay_ttl_seconds: u64,
) -> anyhow::Result<()> {
    #[cfg(any(not(feature = "remote"), not(unix)))]
    let _ = (
        &device_id,
        &transport,
        &relay_endpoint_url,
        &relay_deployment_mode,
        &private_network_name,
        relay_ttl_seconds,
    );

    #[cfg(unix)]
    {
        use ai_terminal::daemon;
        let sock = daemon::socket_path()?;
        println!("원격 게이트 데몬 시작: {} (Ctrl-C 종료)", sock.display());
        let rt = tokio::runtime::Runtime::new()?;
        #[cfg(feature = "remote")]
        {
            let daemon_transport = resolve_daemon_transport_selection(
                &transport,
                relay_endpoint_url.as_deref(),
                &relay_deployment_mode,
                private_network_name.as_deref(),
                relay_ttl_seconds,
            )?;
            let registry = ai_terminal::device_registry::DeviceRegistry::load(
                &ai_terminal::device_registry::registry_path()?,
            )?;
            match device_id.as_deref() {
                Some(id) => {
                    registry.select_device(Some(id))?;
                    println!("원격 승인 디바이스 선택: {id}");
                }
                None if registry.devices.len() == 1 => {
                    println!("원격 승인 디바이스 선택: {}", registry.devices[0].id);
                }
                None => {
                    println!(
                        "원격 승인 디바이스 선택: <ambiguous> (registered={}, 필요 시 --device-id 사용)",
                        registry.devices.len()
                    );
                }
            }
            let transport_mode = ai_terminal::remote_transport::active_product_mode();
            let relay_keyring_path =
                ai_terminal::remote_transport::companion_relay_ticket_keyring_path()?;
            let relay_keyring =
                ai_terminal::remote_transport::load_or_create_companion_relay_ticket_keyring(
                    &relay_keyring_path,
                )?;
            let relay_issuer = relay_keyring.issuer()?;
            let relay_setup = if daemon_transport.mode
                == ai_terminal::remote_transport::CompanionTransportMode::Relay
            {
                let device = registry.select_device(device_id.as_deref())?;
                let daemon_key_path = ai_terminal::pairing::daemon_key_path()?;
                let daemon_key = ai_terminal::pairing::load_or_create_daemon_key(&daemon_key_path)?;
                let setup = ai_terminal::remote_transport::issue_self_hosted_relay_runtime_setup(
                    &relay_keyring,
                    ai_terminal::remote_transport::CompanionRelaySelfHostedSetupInput {
                        relay_endpoint_url: daemon_transport
                            .relay_endpoint_url
                            .clone()
                            .expect("relay endpoint URL must be validated for relay mode"),
                        deployment_mode: Some(daemon_transport.relay_deployment_mode.clone()),
                        private_network_name: daemon_transport.private_network_name.clone(),
                        daemon_pubkey: daemon_key.public.clone(),
                        companion_device_id: device.id.clone(),
                        companion_noise_pubkey: device.noise_pubkey.clone(),
                        companion_approval_pubkey: device.approval_pubkey,
                        issued_at_ms: ai_terminal::pairing::now_ms(),
                        ttl_ms: daemon_transport.relay_ttl_ms,
                        session_id: None,
                        session_token: None,
                    },
                )?;
                Some((daemon_key_path, setup))
            } else {
                None
            };
            println!("PWA transport mode : {}", transport_mode.id());
            println!("PWA requested transport: {}", daemon_transport.mode.id());
            println!("PWA relay keyring  : {}", relay_keyring_path.display());
            println!(
                "PWA relay ticket key: {}",
                relay_issuer.active_key_id().unwrap_or("<legacy>")
            );
            if let Some((daemon_key_path, setup)) = relay_setup {
                let mut relay_runtime = daemon::CompanionRelayDaemonRuntime::new(setup.clone())?;
                relay_runtime.register_session(std::time::Duration::from_secs(5))?;
                println!("PWA relay runtime  : enabled");
                println!("PWA relay daemon key: {}", daemon_key_path.display());
                println!("PWA relay deployment: {}", setup.deployment_mode);
                if let Some(private_network_name) = setup.private_network_name.as_deref() {
                    println!("PWA private network: {}", private_network_name);
                }
                println!("PWA relay endpoint : {}", setup.relay_endpoint_url);
                println!(
                    "PWA relay device   : {}",
                    setup.companion_identity.device_id
                );
                println!(
                    "PWA relay expires  : {}",
                    setup.signed_session_ticket.ticket.expires_at_ms
                );
                println!("PWA relay ticket registered: true");
                println!("PWA relay setup json: {}", serde_json::to_string(&setup)?);
                rt.block_on(daemon::serve_with_remote_relay(
                    &sock,
                    registry,
                    relay_runtime,
                    device_id,
                ))
            } else {
                println!("PWA relay runtime  : disabled");
                let live_endpoint =
                    daemon::spawn_companion_live_endpoint(registry.clone(), device_id.clone())?;
                println!("PWA live endpoint  : {}", live_endpoint.base_url);
                println!("PWA message endpoint: {}", live_endpoint.message_url);
                println!("PWA events endpoint : {}", live_endpoint.events_url);
                let listener = live_endpoint.listener;
                rt.block_on(daemon::serve_with_remote(
                    &sock, registry, listener, device_id,
                ))
            }
        }
        #[cfg(not(feature = "remote"))]
        {
            rt.block_on(daemon::serve(&sock))
        }
    }
    #[cfg(not(unix))]
    {
        println!("게이트 데몬은 Unix 전용입니다.");
        Ok(())
    }
}

#[cfg(feature = "remote")]
#[derive(Debug, Clone, PartialEq, Eq)]
struct DaemonTransportSelection {
    mode: ai_terminal::remote_transport::CompanionTransportMode,
    relay_endpoint_url: Option<String>,
    relay_deployment_mode: String,
    private_network_name: Option<String>,
    relay_ttl_ms: u64,
}

#[cfg(feature = "remote")]
fn resolve_daemon_transport_selection(
    transport: &str,
    relay_endpoint_url: Option<&str>,
    relay_deployment_mode: &str,
    private_network_name: Option<&str>,
    relay_ttl_seconds: u64,
) -> anyhow::Result<DaemonTransportSelection> {
    use std::str::FromStr;

    let mode = ai_terminal::remote_transport::CompanionTransportMode::from_str(transport)
        .map_err(anyhow::Error::msg)?;
    match mode {
        ai_terminal::remote_transport::CompanionTransportMode::LiveLoopback => {
            if relay_endpoint_url.is_some() {
                anyhow::bail!("--relay-endpoint-url은 --transport relay에서만 사용할 수 있습니다");
            }
            if relay_deployment_mode
                != ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED
            {
                anyhow::bail!(
                    "--relay-deployment-mode은 --transport relay에서만 self-hosted 외 값을 사용할 수 있습니다"
                );
            }
            if private_network_name
                .map(str::trim)
                .is_some_and(|name| !name.is_empty())
            {
                anyhow::bail!(
                    "--private-network-name은 --transport relay에서만 사용할 수 있습니다"
                );
            }
            Ok(DaemonTransportSelection {
                mode,
                relay_endpoint_url: None,
                relay_deployment_mode:
                    ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED
                        .into(),
                private_network_name: None,
                relay_ttl_ms: 0,
            })
        }
        ai_terminal::remote_transport::CompanionTransportMode::Relay => {
            if relay_ttl_seconds == 0 {
                anyhow::bail!("relay setup TTL은 1초 이상이어야 합니다");
            }
            let relay_ttl_ms = relay_ttl_seconds
                .checked_mul(1000)
                .ok_or_else(|| anyhow::anyhow!("relay setup TTL overflow"))?;
            let relay_endpoint_url = relay_endpoint_url
                .filter(|url| !url.trim().is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!("--transport relay에는 --relay-endpoint-url이 필요합니다")
                })?
                .to_string();
            if !ai_terminal::remote_transport::valid_relay_websocket_endpoint_url(
                &relay_endpoint_url,
            ) {
                anyhow::bail!("relay endpoint URL must be wss:// or localhost ws://");
            }
            let (relay_deployment_mode, private_network_name) =
                resolve_relay_deployment_selection(relay_deployment_mode, private_network_name)?;
            Ok(DaemonTransportSelection {
                mode,
                relay_endpoint_url: Some(relay_endpoint_url),
                relay_deployment_mode,
                private_network_name,
                relay_ttl_ms,
            })
        }
        ai_terminal::remote_transport::CompanionTransportMode::DeviceSocket
        | ai_terminal::remote_transport::CompanionTransportMode::Tailscale
        | ai_terminal::remote_transport::CompanionTransportMode::WebSocket => {
            anyhow::bail!("{mode} transport는 daemon runtime에서 아직 선택할 수 없습니다")
        }
    }
}

#[cfg(feature = "remote")]
fn resolve_relay_deployment_selection(
    relay_deployment_mode: &str,
    private_network_name: Option<&str>,
) -> anyhow::Result<(String, Option<String>)> {
    match relay_deployment_mode {
        ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED => {
            if private_network_name
                .map(str::trim)
                .is_some_and(|name| !name.is_empty())
            {
                anyhow::bail!(
                    "--private-network-name은 --relay-deployment-mode private-network에서만 사용할 수 있습니다"
                );
            }
            Ok((
                ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_SELF_HOSTED.into(),
                None,
            ))
        }
        ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK => {
            let name = private_network_name
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "--relay-deployment-mode private-network에는 --private-network-name이 필요합니다"
                    )
                })?;
            if !ai_terminal::remote_transport::valid_relay_private_network_name(name) {
                anyhow::bail!("private-network relay name format error");
            }
            Ok((
                ai_terminal::remote_transport::COMPANION_RELAY_DEPLOYMENT_MODE_PRIVATE_NETWORK
                    .into(),
                Some(name.to_string()),
            ))
        }
        _ => anyhow::bail!("relay deployment mode는 self-hosted 또는 private-network여야 합니다"),
    }
}

#[cfg(all(feature = "remote", unix))]
fn remote_pair_transport_addr() -> anyhow::Result<String> {
    Ok(format!(
        "unix://{}",
        ai_terminal::daemon::device_socket_path()?.display()
    ))
}

#[cfg(all(feature = "remote", not(unix)))]
fn remote_pair_transport_addr() -> anyhow::Result<String> {
    Ok("unsupported://local-daemon-unavailable".into())
}

#[cfg(feature = "remote")]
fn run_remote_devices() -> anyhow::Result<()> {
    let registry_path = ai_terminal::device_registry::registry_path()?;
    let registry = ai_terminal::device_registry::DeviceRegistry::load(&registry_path)?;
    println!("등록 원격 디바이스");
    println!("registry_file       : {}", registry_path.display());
    println!("count               : {}", registry.devices.len());
    if registry.devices.is_empty() {
        println!("(none)");
        return Ok(());
    }

    for device in &registry.devices {
        println!("- device_id         : {}", device.id);
        println!("  epoch             : {}", device.epoch);
        println!("  paired_at_ms      : {}", device.paired_at_ms);
        println!(
            "  noise_pubkey_hex  : {}",
            ai_terminal::pairing::hex_encode(&device.noise_pubkey)
        );
        println!(
            "  approval_pubkey_hex: {}",
            ai_terminal::pairing::hex_encode(&device.approval_pubkey)
        );
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_transport() -> anyhow::Result<()> {
    let active = ai_terminal::remote_transport::active_product_mode();
    println!("원격 companion transport");
    println!("active_product_mode : {}", active.id());
    println!("user_selectable     : false");
    println!("modes:");
    for mode in ai_terminal::remote_transport::all_modes() {
        let descriptor = mode.descriptor();
        let marker = if descriptor.mode.is_product_default() {
            " (active)"
        } else {
            ""
        };
        println!(
            "- {:<13} {:<8} {}{}",
            descriptor.id, descriptor.readiness, descriptor.role, marker
        );
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_relay_setup(
    relay_endpoint_url: String,
    relay_deployment_mode: String,
    private_network_name: Option<String>,
    device_id: Option<String>,
    ttl_seconds: u64,
) -> anyhow::Result<()> {
    if ttl_seconds == 0 {
        anyhow::bail!("relay setup TTL은 1초 이상이어야 합니다");
    }
    let ttl_ms = ttl_seconds
        .checked_mul(1000)
        .ok_or_else(|| anyhow::anyhow!("relay setup TTL overflow"))?;
    let daemon_key_path = ai_terminal::pairing::daemon_key_path()?;
    let daemon_key = ai_terminal::pairing::load_or_create_daemon_key(&daemon_key_path)?;
    let registry_path = ai_terminal::device_registry::registry_path()?;
    let registry = ai_terminal::device_registry::DeviceRegistry::load(&registry_path)?;
    let device = registry.select_device(device_id.as_deref())?;
    let keyring_path = ai_terminal::remote_transport::companion_relay_ticket_keyring_path()?;
    let keyring = ai_terminal::remote_transport::load_or_create_companion_relay_ticket_keyring(
        &keyring_path,
    )?;
    if !ai_terminal::remote_transport::valid_relay_websocket_endpoint_url(&relay_endpoint_url) {
        anyhow::bail!("relay endpoint URL must be wss:// or localhost ws://");
    }
    let (relay_deployment_mode, private_network_name) = resolve_relay_deployment_selection(
        &relay_deployment_mode,
        private_network_name.as_deref(),
    )?;
    let setup = ai_terminal::remote_transport::issue_self_hosted_relay_runtime_setup(
        &keyring,
        ai_terminal::remote_transport::CompanionRelaySelfHostedSetupInput {
            deployment_mode: Some(relay_deployment_mode),
            private_network_name,
            relay_endpoint_url,
            daemon_pubkey: daemon_key.public.clone(),
            companion_device_id: device.id.clone(),
            companion_noise_pubkey: device.noise_pubkey.clone(),
            companion_approval_pubkey: device.approval_pubkey,
            issued_at_ms: ai_terminal::pairing::now_ms(),
            ttl_ms,
            session_id: None,
            session_token: None,
        },
    )?;

    println!("원격 relay self-hosted setup");
    println!("registry_file       : {}", registry_path.display());
    println!("daemon_key_file     : {}", daemon_key_path.display());
    println!("relay_keyring_file  : {}", keyring_path.display());
    println!("transport_mode      : {}", setup.transport_mode);
    println!("deployment_mode     : {}", setup.deployment_mode);
    if let Some(private_network_name) = setup.private_network_name.as_deref() {
        println!("private_network_name: {}", private_network_name);
    }
    println!("relay_endpoint_url  : {}", setup.relay_endpoint_url);
    println!(
        "device_id           : {}",
        setup.companion_identity.device_id
    );
    println!(
        "relay_ticket_key_id : {}",
        setup
            .signed_session_ticket
            .key_id
            .as_deref()
            .unwrap_or("<legacy>")
    );
    println!(
        "expires_at_ms       : {}",
        setup.signed_session_ticket.ticket.expires_at_ms
    );
    println!("relay_setup_json    : {}", serde_json::to_string(&setup)?);
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_pair(
    device_id: Option<String>,
    code: Option<String>,
    noise_pubkey_hex: Option<String>,
    approval_pubkey_hex: Option<String>,
    ttl_seconds: u64,
    pwa_url: Option<String>,
) -> anyhow::Result<()> {
    let daemon_key_path = ai_terminal::pairing::daemon_key_path()?;
    let daemon_key = ai_terminal::pairing::load_or_create_daemon_key(&daemon_key_path)?;
    let pairing_path = ai_terminal::pairing::pairing_path()?;
    let registry_path = ai_terminal::device_registry::registry_path()?;

    match (device_id, code, noise_pubkey_hex, approval_pubkey_hex) {
        (None, None, None, None) => {
            let session =
                ai_terminal::pairing::start_pairing(&pairing_path, &daemon_key, ttl_seconds)?;
            let transport_addr = remote_pair_transport_addr()?;
            let payload = ai_terminal::pairing::pairing_payload(&session, &transport_addr);
            println!("원격 디바이스 페어링 시작");
            println!("code              : {}", session.code);
            println!(
                "daemon_pubkey_hex : {}",
                ai_terminal::pairing::hex_encode(&session.daemon_pubkey)
            );
            println!("transport_addr    : {}", transport_addr);
            println!("expires_at_ms     : {}", session.expires_at_ms);
            println!("pairing_file      : {}", pairing_path.display());
            println!(
                "pair_payload_json : {}",
                ai_terminal::pairing::pairing_payload_json(&payload)?
            );
            println!(
                "pair_url          : {}",
                ai_terminal::pairing::pairing_url(&payload)?
            );
            println!("pair_qr:");
            println!("{}", ai_terminal::pairing::pairing_qr_text(&payload)?);
            if let Some(pwa_url) = pwa_url {
                println!(
                    "pwa_pair_url      : {}",
                    ai_terminal::pairing::pairing_pwa_url(&payload, &pwa_url)?
                );
                println!("pwa_pair_qr:");
                println!(
                    "{}",
                    ai_terminal::pairing::pairing_pwa_qr_text(&payload, &pwa_url)?
                );
            }
        }
        (Some(device_id), Some(code), Some(noise_pubkey_hex), Some(approval_pubkey_hex)) => {
            let noise_pubkey = ai_terminal::pairing::hex_decode(&noise_pubkey_hex)?;
            let approval_pubkey = ai_terminal::pairing::hex_decode_32(&approval_pubkey_hex)?;
            let registered = ai_terminal::pairing::complete_pairing(
                &pairing_path,
                &registry_path,
                &device_id,
                &code,
                noise_pubkey,
                approval_pubkey,
            )?;
            println!("원격 디바이스 등록 완료");
            println!("device_id         : {}", registered.id);
            println!("epoch             : {}", registered.epoch);
            println!("registry_file     : {}", registry_path.display());
        }
        _ => {
            anyhow::bail!(
                "pair complete에는 --device-id, --code, --noise-pubkey-hex, --approval-pubkey-hex가 모두 필요합니다"
            );
        }
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_approval_url(request_json: String, pwa_url: Option<String>) -> anyhow::Result<()> {
    let request: ai_terminal::session::ApprovalRequestMsg = serde_json::from_str(&request_json)?;
    println!("원격 승인 요청 URL 생성");
    println!(
        "approval_request_json : {}",
        ai_terminal::session::approval_request_json(&request)?
    );
    println!(
        "approval_url          : {}",
        ai_terminal::session::approval_url(&request)?
    );
    println!("approval_qr:");
    println!("{}", ai_terminal::session::approval_qr_text(&request)?);
    if let Some(pwa_url) = pwa_url {
        println!(
            "pwa_approval_url      : {}",
            ai_terminal::session::approval_pwa_url(&request, &pwa_url)?
        );
        println!("pwa_approval_qr:");
        println!(
            "{}",
            ai_terminal::session::approval_pwa_qr_text(&request, &pwa_url)?
        );
    }
    Ok(())
}

#[cfg(feature = "remote")]
fn run_remote_approval_verify(
    request_json: String,
    response_json: String,
    device_id: Option<String>,
    approval_pubkey_hex: Option<String>,
    device_epoch: Option<u64>,
    now: Option<u64>,
    context_hash: Option<String>,
) -> anyhow::Result<()> {
    let request: ai_terminal::session::ApprovalRequestMsg = serde_json::from_str(&request_json)?;
    let response: ai_terminal::session::ApprovalResponseMsg = serde_json::from_str(&response_json)?;
    let pending = request.to_pending()?;
    let signed = response.to_signed()?;
    let (device_label, device) =
        remote_approval_verify_device(&request, device_id, approval_pubkey_hex, device_epoch)?;
    let now = now.unwrap_or_else(now_secs);
    let context_hash = context_hash.unwrap_or_else(|| request.context_hash.clone());
    let outcome = ai_terminal::approval::validate(&pending, &device, now, &context_hash, &signed);
    println!("원격 승인 응답 검증");
    println!("device_id        : {device_label}");
    println!("approval_outcome : {outcome:?}");
    Ok(())
}

#[cfg(feature = "remote")]
fn remote_approval_verify_device(
    request: &ai_terminal::session::ApprovalRequestMsg,
    device_id: Option<String>,
    approval_pubkey_hex: Option<String>,
    device_epoch: Option<u64>,
) -> anyhow::Result<(String, ai_terminal::approval::DeviceRecord)> {
    match (device_id, approval_pubkey_hex) {
        (Some(_), Some(_)) => {
            anyhow::bail!("--device-id와 --approval-pubkey-hex는 동시에 지정할 수 없습니다")
        }
        (Some(id), None) => {
            let registry = ai_terminal::device_registry::DeviceRegistry::load(
                &ai_terminal::device_registry::registry_path()?,
            )?;
            let device = registry
                .get(&id)
                .ok_or_else(|| anyhow::anyhow!("등록되지 않은 디바이스: {id}"))?;
            Ok((device.id.clone(), device.to_approval_record()))
        }
        (None, Some(hex)) => Ok((
            "(direct-pubkey)".into(),
            ai_terminal::approval::DeviceRecord {
                pubkey: ai_terminal::pairing::hex_decode_32(&hex)?,
                epoch: device_epoch.unwrap_or(request.device_epoch),
            },
        )),
        (None, None) => {
            let registry = ai_terminal::device_registry::DeviceRegistry::load(
                &ai_terminal::device_registry::registry_path()?,
            )?;
            let device = registry.single_device().ok_or_else(|| {
                anyhow::anyhow!("--device-id 또는 --approval-pubkey-hex가 필요합니다")
            })?;
            Ok((device.id.clone(), device.to_approval_record()))
        }
    }
}

#[cfg(not(feature = "remote"))]
fn run_remote_approval_verify(
    _request_json: String,
    _response_json: String,
    _device_id: Option<String>,
    _approval_pubkey_hex: Option<String>,
    _device_epoch: Option<u64>,
    _now: Option<u64>,
    _context_hash: Option<String>,
) -> anyhow::Result<()> {
    anyhow::bail!("`ai remote approval-verify`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_approval_url(_request_json: String, _pwa_url: Option<String>) -> anyhow::Result<()> {
    anyhow::bail!("`ai remote approval-url`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_pair(
    _device_id: Option<String>,
    _code: Option<String>,
    _noise_pubkey_hex: Option<String>,
    _approval_pubkey_hex: Option<String>,
    _ttl_seconds: u64,
    _pwa_url: Option<String>,
) -> anyhow::Result<()> {
    anyhow::bail!("`ai remote pair`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_devices() -> anyhow::Result<()> {
    anyhow::bail!("`ai remote devices`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_transport() -> anyhow::Result<()> {
    anyhow::bail!("`ai remote transport`는 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(not(feature = "remote"))]
fn run_remote_relay_setup(
    _relay_endpoint_url: String,
    _relay_deployment_mode: String,
    _private_network_name: Option<String>,
    _device_id: Option<String>,
    _ttl_seconds: u64,
) -> anyhow::Result<()> {
    anyhow::bail!("`ai remote relay-setup`은 remote feature 빌드에서만 사용할 수 있습니다")
}

#[cfg(feature = "remote")]
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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
            let p = resolve_requested_profile(profile)?;
            print!("{}", format_risk(&command, &p));
            Ok(())
        }
        Some(Command::Policy { action }) => match action {
            PolicyAction::Show { profile } => {
                print!(
                    "{}",
                    describe_effective_policy(&profile.unwrap_or_else(config::get_active_profile))?
                );
                Ok(())
            }
            PolicyAction::Set { profile } => {
                let p = resolve_profile(&profile)?;
                #[cfg(feature = "trust")]
                let effective_after_set =
                    ai_terminal::policy_d::resolve_effective_profile(p.clone())
                        .map_err(anyhow::Error::from)?;
                config::set_active_profile(p.name)?;
                println!("활성 정책 프로파일을 '{}'(으)로 설정했습니다.", p.name);
                #[cfg(feature = "trust")]
                {
                    if effective_after_set.profile.name != p.name {
                        println!(
                            "조직 정책이 우선 적용됩니다: effective='{}'",
                            effective_after_set.profile.name
                        );
                    }
                }
                Ok(())
            }
            PolicyAction::Org { action } => match action {
                PolicyOrgAction::Status => run_policy_org_status(),
            },
        },
        Some(Command::ShellHook { shell }) => {
            let sh = resolve_shell(Some(&shell))?;
            print!("{}", shell::hook_script(sh));
            Ok(())
        }
        Some(Command::Tui { profile }) => {
            let p = resolve_requested_profile(profile)?;
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
                RemoteAction::Daemon {
                    device_id,
                    transport,
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    relay_ttl_seconds,
                } => run_gate_daemon(
                    device_id,
                    transport,
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    relay_ttl_seconds,
                )?,
                RemoteAction::Devices {} => run_remote_devices()?,
                RemoteAction::Transport {} => run_remote_transport()?,
                RemoteAction::RelaySetup {
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    device_id,
                    ttl_seconds,
                } => run_remote_relay_setup(
                    relay_endpoint_url,
                    relay_deployment_mode,
                    private_network_name,
                    device_id,
                    ttl_seconds,
                )?,
                RemoteAction::Pair {
                    device_id,
                    code,
                    noise_pubkey_hex,
                    approval_pubkey_hex,
                    ttl_seconds,
                    pwa_url,
                } => run_remote_pair(
                    device_id,
                    code,
                    noise_pubkey_hex,
                    approval_pubkey_hex,
                    ttl_seconds,
                    pwa_url,
                )?,
                RemoteAction::ApprovalUrl {
                    request_json,
                    pwa_url,
                } => run_remote_approval_url(request_json, pwa_url)?,
                RemoteAction::ApprovalVerify {
                    request_json,
                    response_json,
                    device_id,
                    approval_pubkey_hex,
                    device_epoch,
                    now,
                    context_hash,
                } => run_remote_approval_verify(
                    request_json,
                    response_json,
                    device_id,
                    approval_pubkey_hex,
                    device_epoch,
                    now,
                    context_hash,
                )?,
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
            let profile = resolve_effective_profile(&config::get_active_profile())?;
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
            let profile = resolve_effective_profile(&config::get_active_profile())?;
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
        Some(Command::Skill { query, action }) => match action {
            Some(SkillAction::Enable { name, yes }) => run_skill_enable(name, yes),
            Some(SkillAction::Disable { name }) => run_skill_disable(name),
            Some(SkillAction::Enabled) => {
                run_skill_enabled();
                Ok(())
            }
            Some(SkillAction::Registry {
                action: SkillRegistryAction::Status,
            }) => run_skill_registry_status(),
            Some(SkillAction::Registry {
                action: SkillRegistryAction::Update { registry, manifest },
            }) => run_skill_registry_update(registry, manifest, false),
            Some(SkillAction::Registry {
                action: SkillRegistryAction::Revoke { registry, manifest },
            }) => run_skill_registry_update(registry, manifest, true),
            None => run_skill_list(query),
        },
        Some(Command::Release { action }) => match action {
            ReleaseAction::Manifest { action } => match action {
                ReleaseManifestAction::Status => run_release_manifest_status(),
                ReleaseManifestAction::Verify {
                    payload,
                    manifest,
                    name,
                    artifact,
                } => run_release_manifest_verify(payload, manifest, name, artifact),
            },
        },
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
                println!("이 파일은 AI Terminal GUI 앱이 아니라 명령줄(CLI) 도구 `ai.exe`입니다.");
                println!("Windows GUI 터미널은 릴리즈의 `ai-terminal-windows-*.zip` 또는 설치 파일을 받아 `ai-terminal.exe`를 실행하세요.");
                println!();
                println!("CLI로 사용할 때는 더블클릭이 아니라 터미널에서 실행하세요:");
                println!("  ai doctor                 # 환경 진단");
                println!("  ai risk \"rm -rf /tmp/x\"   # 위험도 평가");
                println!("  ai --help                 # 전체 명령");
                println!(
                    "CLI PATH 등록: scripts/install.ps1 · 문서: https://github.com/ai-cli-terminal/terminal"
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

    let prof = resolve_requested_profile(profile)?;
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

    let prof = resolve_requested_profile(profile)?;
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
    fn cli_parses_policy_org_status() {
        let cli = Cli::try_parse_from(["ai", "policy", "org", "status"]).unwrap();
        match cli.command {
            Some(Command::Policy {
                action:
                    PolicyAction::Org {
                        action: PolicyOrgAction::Status,
                    },
            }) => {}
            _ => panic!("expected policy org status"),
        }
    }

    #[test]
    fn cli_parses_skill_command() {
        let cli = Cli::try_parse_from(["ai", "skill", "--query", "deploy"]).unwrap();
        match cli.command {
            Some(Command::Skill { query, action }) => {
                assert_eq!(query.as_deref(), Some("deploy"));
                assert!(action.is_none());
            }
            _ => panic!("expected skill subcommand"),
        }
    }

    #[test]
    fn cli_parses_skill_registry_status() {
        let cli = Cli::try_parse_from(["ai", "skill", "registry", "status"]).unwrap();
        match cli.command {
            Some(Command::Skill {
                query: None,
                action:
                    Some(SkillAction::Registry {
                        action: SkillRegistryAction::Status,
                    }),
            }) => {}
            _ => panic!("expected skill registry status"),
        }
    }

    #[test]
    fn cli_parses_skill_enable_disable_and_enabled() {
        let guarded_enable = Cli::try_parse_from(["ai", "skill", "enable", "deploy"]).unwrap();
        match guarded_enable.command {
            Some(Command::Skill {
                query: None,
                action: Some(SkillAction::Enable { name, yes }),
            }) => {
                assert_eq!(name, "deploy");
                assert!(!yes);
            }
            _ => panic!("expected guarded skill enable"),
        }

        let enable = Cli::try_parse_from(["ai", "skill", "enable", "deploy", "--yes"]).unwrap();
        match enable.command {
            Some(Command::Skill {
                query: None,
                action: Some(SkillAction::Enable { name, yes }),
            }) => {
                assert_eq!(name, "deploy");
                assert!(yes);
            }
            _ => panic!("expected skill enable"),
        }

        let disable = Cli::try_parse_from(["ai", "skill", "disable", "deploy"]).unwrap();
        match disable.command {
            Some(Command::Skill {
                query: None,
                action: Some(SkillAction::Disable { name }),
            }) => assert_eq!(name, "deploy"),
            _ => panic!("expected skill disable"),
        }

        let enabled = Cli::try_parse_from(["ai", "skill", "enabled"]).unwrap();
        match enabled.command {
            Some(Command::Skill {
                query: None,
                action: Some(SkillAction::Enabled),
            }) => {}
            _ => panic!("expected skill enabled"),
        }
    }

    #[test]
    fn skill_enable_confirmation_requires_exact_skill_name() {
        assert!(skill_enable_confirmation_matches("deploy\n", "deploy"));
        assert!(!skill_enable_confirmation_matches("yes\n", "deploy"));
        assert!(!skill_enable_confirmation_matches("Deploy\n", "deploy"));
        assert!(!skill_enable_confirmation_matches("deploy now\n", "deploy"));
    }

    #[test]
    fn cli_parses_skill_registry_update_and_revoke() {
        let update = Cli::try_parse_from([
            "ai",
            "skill",
            "registry",
            "update",
            "--registry",
            "org-registry.json",
            "--manifest",
            "org-registry.manifest.json",
        ])
        .unwrap();
        match update.command {
            Some(Command::Skill {
                query: None,
                action:
                    Some(SkillAction::Registry {
                        action: SkillRegistryAction::Update { registry, manifest },
                    }),
            }) => {
                assert_eq!(registry, PathBuf::from("org-registry.json"));
                assert_eq!(manifest, PathBuf::from("org-registry.manifest.json"));
            }
            _ => panic!("expected skill registry update"),
        }

        let revoke = Cli::try_parse_from([
            "ai",
            "skill",
            "registry",
            "revoke",
            "--registry",
            "revoked-registry.json",
            "--manifest",
            "revoked-registry.manifest.json",
        ])
        .unwrap();
        match revoke.command {
            Some(Command::Skill {
                query: None,
                action:
                    Some(SkillAction::Registry {
                        action: SkillRegistryAction::Revoke { registry, manifest },
                    }),
            }) => {
                assert_eq!(registry, PathBuf::from("revoked-registry.json"));
                assert_eq!(manifest, PathBuf::from("revoked-registry.manifest.json"));
            }
            _ => panic!("expected skill registry revoke"),
        }
    }

    #[test]
    fn cli_parses_release_manifest_status_and_verify() {
        let status = Cli::try_parse_from(["ai", "release", "manifest", "status"]).unwrap();
        match status.command {
            Some(Command::Release {
                action:
                    ReleaseAction::Manifest {
                        action: ReleaseManifestAction::Status,
                    },
            }) => {}
            _ => panic!("expected release manifest status"),
        }

        let verify = Cli::try_parse_from([
            "ai",
            "release",
            "manifest",
            "verify",
            "--payload",
            "binary-manifest.json",
            "--manifest",
            "binary-manifest.manifest.json",
            "--name",
            "ai-linux-x86_64",
            "--artifact",
            "ai-linux-x86_64",
        ])
        .unwrap();
        match verify.command {
            Some(Command::Release {
                action:
                    ReleaseAction::Manifest {
                        action:
                            ReleaseManifestAction::Verify {
                                payload,
                                manifest,
                                name,
                                artifact,
                            },
                    },
            }) => {
                assert_eq!(payload, PathBuf::from("binary-manifest.json"));
                assert_eq!(manifest, PathBuf::from("binary-manifest.manifest.json"));
                assert_eq!(name.as_deref(), Some("ai-linux-x86_64"));
                assert_eq!(artifact, Some(PathBuf::from("ai-linux-x86_64")));
            }
            _ => panic!("expected release manifest verify"),
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
        match Cli::try_parse_from(["ai", "remote", "daemon"])
            .unwrap()
            .command
        {
            Some(Command::Remote {
                action:
                    RemoteAction::Daemon {
                        device_id,
                        transport,
                        relay_endpoint_url,
                        relay_deployment_mode,
                        private_network_name,
                        relay_ttl_seconds,
                    },
            }) => {
                assert_eq!(device_id, None);
                assert_eq!(transport, "live-loopback");
                assert_eq!(relay_endpoint_url, None);
                assert_eq!(relay_deployment_mode, "self-hosted");
                assert_eq!(private_network_name, None);
                assert_eq!(relay_ttl_seconds, 300);
            }
            _ => panic!("expected remote daemon"),
        }

        match Cli::try_parse_from(["ai", "remote", "daemon", "--device-id", "phone-2"])
            .unwrap()
            .command
        {
            Some(Command::Remote {
                action:
                    RemoteAction::Daemon {
                        device_id,
                        transport,
                        relay_endpoint_url,
                        relay_deployment_mode,
                        private_network_name,
                        relay_ttl_seconds,
                    },
            }) => {
                assert_eq!(device_id.as_deref(), Some("phone-2"));
                assert_eq!(transport, "live-loopback");
                assert_eq!(relay_endpoint_url, None);
                assert_eq!(relay_deployment_mode, "self-hosted");
                assert_eq!(private_network_name, None);
                assert_eq!(relay_ttl_seconds, 300);
            }
            _ => panic!("expected remote daemon with device id"),
        }
    }

    #[test]
    fn cli_parses_remote_daemon_relay_transport() {
        let parsed = Cli::try_parse_from([
            "ai",
            "remote",
            "daemon",
            "--device-id",
            "phone-1",
            "--transport",
            "relay",
            "--relay-endpoint-url",
            "wss://relay.example.test/session",
            "--relay-deployment-mode",
            "private-network",
            "--private-network-name",
            "tailnet-dev",
            "--relay-ttl-seconds",
            "180",
        ])
        .unwrap();
        match parsed.command {
            Some(Command::Remote {
                action:
                    RemoteAction::Daemon {
                        device_id,
                        transport,
                        relay_endpoint_url,
                        relay_deployment_mode,
                        private_network_name,
                        relay_ttl_seconds,
                    },
            }) => {
                assert_eq!(device_id.as_deref(), Some("phone-1"));
                assert_eq!(transport, "relay");
                assert_eq!(
                    relay_endpoint_url.as_deref(),
                    Some("wss://relay.example.test/session")
                );
                assert_eq!(relay_deployment_mode, "private-network");
                assert_eq!(private_network_name.as_deref(), Some("tailnet-dev"));
                assert_eq!(relay_ttl_seconds, 180);
            }
            _ => panic!("expected remote daemon relay transport"),
        }
    }

    #[cfg(feature = "remote")]
    #[test]
    fn daemon_transport_selection_validates_relay_inputs() {
        let live =
            resolve_daemon_transport_selection("live-loopback", None, "self-hosted", None, 300)
                .unwrap();
        assert_eq!(
            live.mode,
            ai_terminal::remote_transport::CompanionTransportMode::LiveLoopback
        );
        assert!(live.relay_endpoint_url.is_none());
        assert_eq!(live.relay_deployment_mode, "self-hosted");
        assert!(live.private_network_name.is_none());

        let relay = resolve_daemon_transport_selection(
            "relay",
            Some("wss://relay.example.test/session"),
            "self-hosted",
            None,
            120,
        )
        .unwrap();
        assert_eq!(
            relay.mode,
            ai_terminal::remote_transport::CompanionTransportMode::Relay
        );
        assert_eq!(
            relay.relay_endpoint_url.as_deref(),
            Some("wss://relay.example.test/session")
        );
        assert_eq!(relay.relay_deployment_mode, "self-hosted");
        assert!(relay.private_network_name.is_none());
        assert_eq!(relay.relay_ttl_ms, 120_000);

        let private_network = resolve_daemon_transport_selection(
            "relay",
            Some("wss://relay.tailnet.example/relay"),
            "private-network",
            Some("tailnet-dev"),
            120,
        )
        .unwrap();
        assert_eq!(private_network.relay_deployment_mode, "private-network");
        assert_eq!(
            private_network.private_network_name.as_deref(),
            Some("tailnet-dev")
        );

        assert!(
            resolve_daemon_transport_selection("relay", None, "self-hosted", None, 120).is_err()
        );
        assert!(resolve_daemon_transport_selection(
            "relay",
            Some("ws://relay.example.test/relay"),
            "private-network",
            Some("tailnet-dev"),
            120,
        )
        .is_err());
        assert!(resolve_daemon_transport_selection(
            "relay",
            Some("wss://relay.tailnet.example/relay"),
            "private-network",
            None,
            120,
        )
        .is_err());
        assert!(resolve_daemon_transport_selection(
            "live-loopback",
            Some("wss://relay.example.test/session"),
            "self-hosted",
            None,
            120,
        )
        .is_err());
        assert!(resolve_daemon_transport_selection(
            "websocket",
            Some("wss://relay.example.test/session"),
            "self-hosted",
            None,
            120,
        )
        .is_err());
    }

    #[test]
    fn cli_parses_remote_devices() {
        assert!(matches!(
            Cli::try_parse_from(["ai", "remote", "devices"])
                .unwrap()
                .command,
            Some(Command::Remote {
                action: RemoteAction::Devices {}
            })
        ));
    }

    #[test]
    fn cli_parses_remote_transport() {
        assert!(matches!(
            Cli::try_parse_from(["ai", "remote", "transport"])
                .unwrap()
                .command,
            Some(Command::Remote {
                action: RemoteAction::Transport {}
            })
        ));
    }

    #[test]
    fn cli_parses_remote_relay_setup() {
        let parsed = Cli::try_parse_from([
            "ai",
            "remote",
            "relay-setup",
            "--relay-endpoint-url",
            "wss://relay.example.test/session",
            "--relay-deployment-mode",
            "private-network",
            "--private-network-name",
            "tailnet-dev",
            "--device-id",
            "phone-1",
            "--ttl-seconds",
            "120",
        ])
        .unwrap();
        match parsed.command {
            Some(Command::Remote {
                action:
                    RemoteAction::RelaySetup {
                        relay_endpoint_url,
                        relay_deployment_mode,
                        private_network_name,
                        device_id,
                        ttl_seconds,
                    },
            }) => {
                assert_eq!(relay_endpoint_url, "wss://relay.example.test/session");
                assert_eq!(relay_deployment_mode, "private-network");
                assert_eq!(private_network_name.as_deref(), Some("tailnet-dev"));
                assert_eq!(device_id.as_deref(), Some("phone-1"));
                assert_eq!(ttl_seconds, 120);
            }
            _ => panic!("expected remote relay-setup"),
        }
    }

    #[test]
    fn cli_parses_remote_pair_start_and_complete() {
        let start = Cli::try_parse_from([
            "ai",
            "remote",
            "pair",
            "--ttl-seconds",
            "60",
            "--pwa-url",
            "http://127.0.0.1:8787/index.html",
        ])
        .unwrap();
        match start.command {
            Some(Command::Remote {
                action:
                    RemoteAction::Pair {
                        device_id,
                        code,
                        noise_pubkey_hex,
                        approval_pubkey_hex,
                        ttl_seconds,
                        pwa_url,
                    },
            }) => {
                assert_eq!(device_id, None);
                assert_eq!(code, None);
                assert_eq!(noise_pubkey_hex, None);
                assert_eq!(approval_pubkey_hex, None);
                assert_eq!(ttl_seconds, 60);
                assert_eq!(pwa_url.as_deref(), Some("http://127.0.0.1:8787/index.html"));
            }
            _ => panic!("expected remote pair start"),
        }

        let complete = Cli::try_parse_from([
            "ai",
            "remote",
            "pair",
            "--device-id",
            "phone-1",
            "--code",
            "123456",
            "--noise-pubkey-hex",
            "aaaaaaaa",
            "--approval-pubkey-hex",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ])
        .unwrap();
        match complete.command {
            Some(Command::Remote {
                action:
                    RemoteAction::Pair {
                        device_id,
                        code,
                        noise_pubkey_hex,
                        approval_pubkey_hex,
                        ..
                    },
            }) => {
                assert_eq!(device_id.as_deref(), Some("phone-1"));
                assert_eq!(code.as_deref(), Some("123456"));
                assert_eq!(noise_pubkey_hex.as_deref(), Some("aaaaaaaa"));
                assert_eq!(
                    approval_pubkey_hex.as_deref(),
                    Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
                );
            }
            _ => panic!("expected remote pair complete"),
        }
    }

    #[test]
    fn cli_parses_remote_approval_url() {
        let request_json = r#"{"approval_id":[1],"nonce":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"command_masked":"rm -rf build","context_hash":"ctx","expires_at":9999,"device_epoch":1}"#;
        let parsed = Cli::try_parse_from([
            "ai",
            "remote",
            "approval-url",
            "--request-json",
            request_json,
            "--pwa-url",
            "http://127.0.0.1:8787/index.html",
        ])
        .unwrap();
        match parsed.command {
            Some(Command::Remote {
                action:
                    RemoteAction::ApprovalUrl {
                        request_json: got,
                        pwa_url,
                    },
            }) => {
                assert_eq!(got, request_json);
                assert_eq!(pwa_url.as_deref(), Some("http://127.0.0.1:8787/index.html"));
            }
            _ => panic!("expected remote approval-url"),
        }
    }

    #[test]
    fn cli_parses_remote_approval_verify() {
        let request_json = r#"{"approval_id":[1],"nonce":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"command_masked":"rm -rf build","context_hash":"ctx","expires_at":9999,"device_epoch":1}"#;
        let response_json = r#"{"approval_id":[1],"nonce":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],"approve":true,"sig":[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]}"#;
        let parsed = Cli::try_parse_from([
            "ai",
            "remote",
            "approval-verify",
            "--request-json",
            request_json,
            "--response-json",
            response_json,
            "--approval-pubkey-hex",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "--device-epoch",
            "2",
            "--now",
            "500",
            "--context-hash",
            "ctx",
        ])
        .unwrap();
        match parsed.command {
            Some(Command::Remote {
                action:
                    RemoteAction::ApprovalVerify {
                        request_json: got_request,
                        response_json: got_response,
                        device_id,
                        approval_pubkey_hex,
                        device_epoch,
                        now,
                        context_hash,
                    },
            }) => {
                assert_eq!(got_request, request_json);
                assert_eq!(got_response, response_json);
                assert_eq!(
                    approval_pubkey_hex.as_deref(),
                    Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
                );
                assert_eq!(device_id, None);
                assert_eq!(device_epoch, Some(2));
                assert_eq!(now, Some(500));
                assert_eq!(context_hash.as_deref(), Some("ctx"));
            }
            _ => panic!("expected remote approval-verify"),
        }
    }

    #[test]
    fn cli_parses_remote_approval_verify_device_id() {
        let parsed = Cli::try_parse_from([
            "ai",
            "remote",
            "approval-verify",
            "--request-json",
            "{}",
            "--response-json",
            "{}",
            "--device-id",
            "phone-1",
        ])
        .unwrap();
        match parsed.command {
            Some(Command::Remote {
                action:
                    RemoteAction::ApprovalVerify {
                        device_id,
                        approval_pubkey_hex,
                        ..
                    },
            }) => {
                assert_eq!(device_id.as_deref(), Some("phone-1"));
                assert_eq!(approval_pubkey_hex, None);
            }
            _ => panic!("expected remote approval-verify device-id"),
        }
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
