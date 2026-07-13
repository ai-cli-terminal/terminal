use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// AI CLI 통합 리눅스 터미널.
#[derive(Parser, Debug)]
#[command(
    name = "ai",
    version,
    about = "일반 셸 호환 + 안전한 AI 보조 터미널 (설계 v3.3)"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
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
        /// 백엔드 지정 시 해당 호스트로 게이트 통과 실행: pwsh|wsl|cmd.
        #[arg(long)]
        backend: Option<String>,
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
pub(crate) enum PolicyAction {
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
pub(crate) enum PolicyOrgAction {
    /// signed policy.d 파일 세트와 검증 상태를 표시한다.
    Status,
}

#[derive(Subcommand, Debug)]
pub(crate) enum SkillAction {
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
pub(crate) enum SkillRegistryAction {
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
pub(crate) enum ReleaseAction {
    /// signed binary release manifest 진단.
    Manifest {
        #[command(subcommand)]
        action: ReleaseManifestAction,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ReleaseManifestAction {
    /// active signed binary manifest 파일 세트와 검증 상태를 표시한다.
    Status,
    /// artifact 파일들에서 binary-manifest JSON payload를 생성한다.
    Create {
        /// 생성할 binary-manifest JSON 파일.
        #[arg(long)]
        output: PathBuf,
        /// manifest에 포함할 release artifact 파일. 여러 번 지정할 수 있다.
        #[arg(long, required = true)]
        artifact: Vec<PathBuf>,
        /// 선택: 각 artifact entry에 기록할 릴리스 버전.
        #[arg(long)]
        release_version: Option<String>,
    },
    /// binary-manifest JSON payload를 trust manifest로 서명한다.
    Sign {
        /// 서명할 binary-manifest JSON 파일.
        #[arg(long)]
        payload: PathBuf,
        /// 생성할 signed manifest JSON 파일.
        #[arg(long)]
        output: PathBuf,
        /// 서명 key id.
        #[arg(long)]
        key_id: String,
        /// 32-byte Ed25519 signing key hex를 담은 환경변수 이름.
        #[arg(long, default_value = "AI_TERMINAL_RELEASE_SIGNING_KEY_HEX")]
        private_key_env: String,
        /// trust manifest version. 조직 anchor의 min_version rollback guard와 비교된다.
        #[arg(long)]
        manifest_version: u64,
        /// 선택: trust manifest id. 미지정 시 manifest version에서 파생한다.
        #[arg(long)]
        manifest_id: Option<String>,
        /// 선택: trust manifest subject.
        #[arg(long, default_value = "release/binary-manifest.json")]
        subject: String,
        /// 선택: issued_at Unix timestamp. 미지정 시 현재 시각.
        #[arg(long)]
        issued_at_unix: Option<i64>,
        /// 유효 기간(일). expires_at은 issued_at + valid_days로 계산된다.
        #[arg(long, default_value_t = 180)]
        valid_days: i64,
    },
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
pub(crate) enum RemoteAction {
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
pub(crate) enum InitTarget {
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
pub(crate) enum InitMode {
    Install,
    DryRun,
    Diff,
    Uninstall,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
