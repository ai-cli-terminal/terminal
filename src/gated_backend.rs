//! 백엔드 지정 안전 실행 경로 (G3-C 코어).
//!
//! `Backend` enum으로 pwsh/wsl/cmd를 지정하고, raw 명령을 기존 게이트
//! (risk→policy→mask→confirm→audit)에 태운 뒤 비대화형 호스트로 실행한다.
//!
//! 핵심 정확성 요건 — **assess-inner-then-wrap**:
//! - `pipeline::execute`에 **raw_command**를 넘겨 게이트가 원문을 평가한다.
//! - 실행자(`BackendExecutor`)는 pipeline이 넘기는 명령 문자열을 **무시**하고,
//!   생성 시 캡처한 `host_wrap` 결과(program, args)를 그대로 spawn한다.
//! - 감싼 문자열("pwsh -c ...") 을 게이트가 보면 F6a 위험 토큰 매칭이 깨진다.

use std::cell::Cell;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::pipeline::{
    self, ConfirmRequest, Confirmer, ExecConfig, ExecOutcome, Executor, OutputSink,
};
use crate::shellcore::external::spawn_inherit;
use crate::shellcore::value::Value;

// ── Backend enum ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Pwsh,
    Wsl,
    Cmd,
}

impl Backend {
    /// audit source 문자열 ("gated-pwsh" / "gated-wsl" / "gated-cmd").
    pub fn audit_source(self) -> &'static str {
        match self {
            Backend::Pwsh => "gated-pwsh",
            Backend::Wsl => "gated-wsl",
            Backend::Cmd => "gated-cmd",
        }
    }

    /// 문자열 → Backend (대소문자 무시). "pwsh"/"wsl"/"cmd".
    pub fn parse(s: &str) -> Option<Backend> {
        match s.to_ascii_lowercase().as_str() {
            "pwsh" => Some(Backend::Pwsh),
            "wsl" => Some(Backend::Wsl),
            "cmd" => Some(Backend::Cmd),
            _ => None,
        }
    }
}

// ── host_wrap — 순수함수 ──────────────────────────────────────────────────────

/// raw 명령을 백엔드 호스트 프로세스 호출로 감싼다.
/// raw는 argv 원소 1개로 보존(공백·따옴표 재파싱 없음).
pub fn host_wrap(backend: Backend, raw: &str) -> (String, Vec<String>) {
    match backend {
        Backend::Pwsh => (
            "pwsh.exe".to_string(),
            vec![
                "-NoProfile".to_string(),
                "-Command".to_string(),
                raw.to_string(),
            ],
        ),
        Backend::Wsl => (
            "wsl.exe".to_string(),
            vec![
                "--".to_string(),
                "bash".to_string(),
                "-lc".to_string(),
                raw.to_string(),
            ],
        ),
        Backend::Cmd => (
            "cmd.exe".to_string(),
            vec!["/d".to_string(), "/c".to_string(), raw.to_string()],
        ),
    }
}

// ── BackendExecutor ──────────────────────────────────────────────────────────

/// pipeline Executor 구현: host_wrap 결과를 생성 시 캡처하고,
/// `run`에서 pipeline이 넘긴 command 문자열을 무시하고 spawn_inherit 호출.
/// (ArgvExecutor 패턴 미러 — gated_runner.rs L69 참조.)
struct BackendExecutor {
    program: String,
    args: Vec<String>,
    cwd: PathBuf,
    /// 테스트용: run이 실제로 호출됐는지 기록.
    called: Cell<bool>,
}

impl BackendExecutor {
    fn new(backend: Backend, raw: &str, cwd: &Path) -> Self {
        let (program, args) = host_wrap(backend, raw);
        Self {
            program,
            args,
            cwd: cwd.to_path_buf(),
            called: Cell::new(false),
        }
    }
}

impl Executor for BackendExecutor {
    /// pipeline이 넘기는 command 문자열은 무시하고, 캡처된 host argv로 spawn.
    fn run(&self, _command: &str, _sink: &mut dyn OutputSink) -> Result<i32> {
        self.called.set(true);
        let code = spawn_inherit(&self.program, &self.args, &self.cwd)?;
        Ok(code.unwrap_or(-1))
    }
}

// ── NullSink (로컬 — stdio 상속 spawn이므로 sink 불필요) ────────────────────

struct NullSink;
impl OutputSink for NullSink {
    fn write(&mut self, _chunk: &str) {}
}

// ── gated_backend_run_with (테스트용 DI 진입점) ──────────────────────────────

/// 테스트용 내부 함수: executor/confirmer/sink를 주입받아 pipeline을 호출한다.
/// `pipeline::execute`에 **raw_command**를 넘기는 것이 assess-inner 요건이다.
pub(crate) fn gated_backend_run_with(
    _backend: Backend,
    raw_command: &str,
    cfg: &ExecConfig,
    executor: &dyn Executor,
    confirmer: &mut dyn Confirmer,
    sink: &mut dyn OutputSink,
) -> Result<ExecOutcome> {
    pipeline::execute(raw_command, cfg, executor, confirmer, sink)
}

// ── AlwaysYesConfirmer (--yes 자동 승인용) ───────────────────────────────────

/// `--yes` 플래그 시 confirm 프롬프트를 자동 승인한다.
/// Block(Critical)은 pipeline이 confirm 전에 처리하므로 우회 불가.
struct AlwaysYesConfirmer;

impl Confirmer for AlwaysYesConfirmer {
    fn confirm(&mut self, _req: &ConfirmRequest) -> bool {
        true
    }
}

// ── gated_backend_run (공개 진입점) ──────────────────────────────────────────

/// 실제 환경(profile/undo/tty)으로 wiring한 공개 함수.
/// `GatedRunner::from_environment`와 동일한 환경 구성을 공유한다(DRY).
///
/// - `profile_override`: 지정 시 해당 프로파일 이름으로 override(없는 이름이면 에러).
/// - `auto_yes`: true 시 confirm 프롬프트 자동 승인(`AlwaysYesConfirmer`), false 시 `BackendConfirmer`.
pub fn gated_backend_run(
    backend: Backend,
    raw_command: &str,
    cwd: &Path,
    profile_override: Option<&str>,
    auto_yes: bool,
) -> Result<Value> {
    let (default_profile, policy_error, undo_dir, limits, is_tty) =
        crate::gated_runner::build_exec_environment();

    if let Some(error) = policy_error {
        anyhow::bail!("ash: organization policy unavailable — {error}");
    }

    // profile_override 있으면 by_name으로 대체, 없는 이름이면 에러.
    let profile = if let Some(name) = profile_override {
        crate::policy::PolicyProfile::by_name(name)
            .ok_or_else(|| anyhow::anyhow!("unknown profile: {name} (balanced|paranoid)"))?
    } else {
        default_profile
    };

    let cfg = ExecConfig {
        profile: &profile,
        undo_dir: &undo_dir,
        limits,
    };
    let executor = BackendExecutor::new(backend, raw_command, cwd);
    let mut confirmer_tty = BackendConfirmer { is_tty };
    let mut confirmer_yes = AlwaysYesConfirmer;
    let confirmer: &mut dyn Confirmer = if auto_yes {
        &mut confirmer_yes
    } else {
        &mut confirmer_tty
    };
    let mut sink = NullSink;

    let outcome = gated_backend_run_with(
        backend,
        raw_command,
        &cfg,
        &executor,
        confirmer,
        &mut sink,
    )?;

    // 감사 기록 — gated_runner.rs L179~188 미러, source = backend.audit_source().
    match &outcome {
        ExecOutcome::Ran { exit_code, .. } => {
            crate::shell_audit::record_ran_command(raw_command, *exit_code, backend.audit_source());
        }
        other => {
            if let Some(rec) =
                crate::shell_audit::shell_outcome_audit(raw_command, backend.audit_source(), other)
            {
                crate::shell_audit::record_outcome_audit(&rec);
            }
        }
    }

    let (msg, value) = crate::gated_runner::outcome_message(&outcome, backend.audit_source());
    if let Some(m) = msg {
        eprintln!("{m}");
    }
    Ok(value)
}

// ── BackendConfirmer (stdin 기반, StdinConfirmer 미러) ───────────────────────

struct BackendConfirmer {
    is_tty: bool,
}

impl Confirmer for BackendConfirmer {
    fn confirm(&mut self, req: &ConfirmRequest) -> bool {
        use std::io::Write as _;
        if !self.is_tty {
            eprintln!("ash: 비대화형 입력 — 확인 불가로 거부: {}", req.command);
            return false;
        }
        eprintln!("⚠ 확인 필요: {}", req.command);
        eprintln!(
            "  위험도: {:?}  요인: {}",
            req.level,
            req.factors.join(", ")
        );
        if !req.backup_files.is_empty() {
            eprintln!("  백업 대상: {}", req.backup_files.join(", "));
        }
        eprint!("  실행할까요? [y/N] ");
        let _ = std::io::stderr().flush();
        let mut answer = String::new();
        if std::io::stdin().read_line(&mut answer).is_err() {
            return false;
        }
        crate::gated_runner::decide_confirm(&answer)
    }
}

// ── 단위 테스트 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{ExecConfig, ExecOutcome};
    use crate::policy::PolicyProfile;
    use crate::risk::RiskLevel;
    use crate::undo::UndoLimits;
    use std::cell::RefCell;

    // ── 테스트 헬퍼 ──────────────────────────────────────────────────────────

    fn tmp_undo(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("ai_gb_test_{}_{}_{n}", std::process::id(), tag))
    }

    fn test_cfg<'a>(profile: &'a PolicyProfile, undo: &'a Path) -> ExecConfig<'a> {
        ExecConfig {
            profile,
            undo_dir: undo,
            limits: UndoLimits::defaults(),
        }
    }

    /// pipeline Executor — 호출 여부를 RefCell로 기록, exit 0 반환.
    struct RecordingExecutor {
        was_called: RefCell<bool>,
    }
    impl RecordingExecutor {
        fn new() -> Self {
            Self {
                was_called: RefCell::new(false),
            }
        }
    }
    impl Executor for RecordingExecutor {
        fn run(&self, _command: &str, _sink: &mut dyn OutputSink) -> Result<i32> {
            *self.was_called.borrow_mut() = true;
            Ok(0)
        }
    }

    /// Confirmer — 항상 true(자동 승인).
    struct AutoYes;
    impl Confirmer for AutoYes {
        fn confirm(&mut self, _req: &ConfirmRequest) -> bool {
            true
        }
    }

    struct CollectSink(String);
    impl OutputSink for CollectSink {
        fn write(&mut self, chunk: &str) {
            self.0.push_str(chunk);
        }
    }

    // ── 1. host_wrap 정확성 ───────────────────────────────────────────────────

    #[test]
    fn host_wrap_pwsh() {
        let raw = r#"a b "c""#;
        let (prog, args) = host_wrap(Backend::Pwsh, raw);
        assert_eq!(prog, "pwsh.exe");
        assert_eq!(args, vec!["-NoProfile", "-Command", raw]);
        // raw가 마지막 원소 1개로 보존(재파싱 없음)
        assert_eq!(args.last().unwrap(), raw);
    }

    #[test]
    fn host_wrap_wsl() {
        let raw = r#"a b "c""#;
        let (prog, args) = host_wrap(Backend::Wsl, raw);
        assert_eq!(prog, "wsl.exe");
        assert_eq!(args, vec!["--", "bash", "-lc", raw]);
        assert_eq!(args.last().unwrap(), raw);
    }

    #[test]
    fn host_wrap_cmd() {
        let raw = r#"a b "c""#;
        let (prog, args) = host_wrap(Backend::Cmd, raw);
        assert_eq!(prog, "cmd.exe");
        assert_eq!(args, vec!["/d", "/c", raw]);
        assert_eq!(args.last().unwrap(), raw);
    }

    // ── 2. Backend::parse / audit_source ────────────────────────────────────

    #[test]
    fn backend_parse_case_insensitive() {
        assert_eq!(Backend::parse("pwsh"), Some(Backend::Pwsh));
        assert_eq!(Backend::parse("PWSH"), Some(Backend::Pwsh));
        assert_eq!(Backend::parse("Pwsh"), Some(Backend::Pwsh));
        assert_eq!(Backend::parse("wsl"), Some(Backend::Wsl));
        assert_eq!(Backend::parse("WSL"), Some(Backend::Wsl));
        assert_eq!(Backend::parse("cmd"), Some(Backend::Cmd));
        assert_eq!(Backend::parse("CMD"), Some(Backend::Cmd));
        assert_eq!(Backend::parse("bash"), None);
        assert_eq!(Backend::parse(""), None);
    }

    #[test]
    fn backend_audit_source() {
        assert_eq!(Backend::Pwsh.audit_source(), "gated-pwsh");
        assert_eq!(Backend::Wsl.audit_source(), "gated-wsl");
        assert_eq!(Backend::Cmd.audit_source(), "gated-cmd");
    }

    // ── 3. assess-inner 차단 (핵심 가드) ────────────────────────────────────
    // raw 위험 명령을 gate가 평가해 Critical로 차단, executor 미호출 증명.

    #[test]
    fn assess_inner_blocks_critical_before_executor() {
        let profile = PolicyProfile::balanced();
        let undo = tmp_undo("block");
        let cfg = test_cfg(&profile, &undo);

        let recording = RecordingExecutor::new();
        let mut confirmer = AutoYes;
        let mut sink = CollectSink(String::new());

        // Windows-style critical command (F6a가 잡아야 함)
        let outcome = gated_backend_run_with(
            Backend::Pwsh,
            "Remove-Item -Recurse -Force C:\\",
            &cfg,
            &recording,
            &mut confirmer,
            &mut sink,
        )
        .unwrap();

        // 게이트가 Critical로 차단했어야 함
        assert!(
            matches!(
                outcome,
                ExecOutcome::Blocked {
                    level: RiskLevel::Critical,
                    ..
                }
            ),
            "expected Blocked(Critical), got {outcome:?}"
        );
        // executor.run이 호출되지 않았어야 함 — raw가 평가됐고 F6a가 잡았다는 증명
        assert!(
            !*recording.was_called.borrow(),
            "executor must NOT be called when gate blocks"
        );
    }

    // ── 4. 안전 명령 통과 ───────────────────────────────────────────────────
    // 안전 명령 → gate 승인 → executor 호출됨·Ran.

    #[test]
    fn safe_command_passes_gate_and_calls_executor() {
        let profile = PolicyProfile::balanced();
        let undo = tmp_undo("safe");
        let cfg = test_cfg(&profile, &undo);

        let recording = RecordingExecutor::new();
        let mut confirmer = AutoYes;
        let mut sink = CollectSink(String::new());

        let outcome = gated_backend_run_with(
            Backend::Pwsh,
            "echo hi",
            &cfg,
            &recording,
            &mut confirmer,
            &mut sink,
        )
        .unwrap();

        // executor 호출됨
        assert!(
            *recording.was_called.borrow(),
            "executor must be called for safe command"
        );
        // Ran 결과
        assert!(
            matches!(outcome, ExecOutcome::Ran { exit_code: 0, .. }),
            "expected Ran(0), got {outcome:?}"
        );
    }
}
