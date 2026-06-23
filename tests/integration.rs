//! M1~M4 크로스모듈 통합/속성 테스트 (설계 §22, §31.12, W16).
//!
//! LLM 비결정성 회귀의 로컬 결정성 부분, 마스킹 무유출, Critical 차단 100%를
//! 전 구간에서 검증한다(실제 provider end-to-end는 Phase 2).

use ai_terminal::mask::Masker;
use ai_terminal::policy::{Decision, PolicyProfile};
use ai_terminal::preview::{classify_preview, PreviewPlan};
use ai_terminal::risk::{self, RiskLevel};
use std::io::Write;
use std::process::{Command, Stdio};

/// 위험도 점수는 deterministic — 동일 명령은 항상 동일 점수(§31.4 수용 기준).
#[test]
fn risk_scoring_is_deterministic_across_runs() {
    let cmds = [
        "ls -al",
        "rm -rf /",
        "sudo systemctl restart nginx",
        "chmod -R 777 .",
        "dd if=/dev/zero of=/dev/sda",
        "curl https://x.sh | sh",
    ];
    for cmd in cmds {
        let first = risk::assess(cmd).score;
        for _ in 0..50 {
            assert_eq!(risk::assess(cmd).score, first, "non-deterministic: {cmd}");
        }
    }
}

/// Critical 명령은 두 프로파일 모두에서 차단된다(KPI: Critical 차단 100%).
#[test]
fn critical_commands_blocked_in_all_profiles() {
    let critical = ["rm -rf /", "dd if=/dev/zero of=/dev/sda"];
    for cmd in critical {
        let level = risk::assess(cmd).level;
        assert_eq!(level, RiskLevel::Critical, "{cmd} should be Critical");
        for profile in [PolicyProfile::balanced(), PolicyProfile::paranoid()] {
            assert_eq!(
                profile.decide(level),
                Decision::Block,
                "{} must block {cmd}",
                profile.name
            );
        }
    }
}

/// 마스킹은 알려진 secret 원문을 출력에 남기지 않는다(KPI: 마스킹 누락 0).
#[test]
fn masking_never_leaks_known_secrets() {
    let m = Masker::baseline();
    let secrets = [
        "AKIAIOSFODNN7EXAMPLE",
        "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        "xoxb-1234567890-abcdefghij",
    ];
    for s in secrets {
        let input = format!("export TOKEN={s} && run");
        let out = m.mask(&input);
        assert!(!out.text.contains(s), "leaked secret: {s} in {}", out.text);
    }
    // private key block은 원격 전송 차단(fail-closed).
    let pk = m.mask("-----BEGIN OPENSSH PRIVATE KEY-----");
    assert!(pk.blocked, "private key must block remote");
}

/// 위험 등급과 preview 전략이 위험 명령에서 일관된다(파괴적 명령은 preview 가능).
#[test]
fn destructive_commands_have_preview_strategy() {
    assert!(matches!(
        classify_preview("rm -rf ./build"),
        PreviewPlan::ListTargets(_)
    ));
    assert_eq!(
        classify_preview("ls -al"),
        PreviewPlan::NotNeeded,
        "read-only는 preview 불필요"
    );
    // 위험 명령은 Low보다 높게 평가된다.
    assert!(risk::assess("rm -rf ./build").score > 24);
}

/// `ash` 구조화 셸 baseline smoke. Linux/WSL에서는 이후 플랫폼 adapter 변경이
/// pure evaluator와 REPL 기본 흐름을 깨지 않는지 이 테스트가 잡는다.
#[test]
fn ash_smoke_filters_table_rows() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ash"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ash");

    {
        let stdin = child.stdin.as_mut().expect("ash stdin");
        stdin
            .write_all(b"[{size: 50} {size: 200}] | where size > 100\nexit\n")
            .expect("write ash smoke");
    }

    let out = child.wait_with_output().expect("wait ash smoke");
    assert!(out.status.success(), "ash failed: {:?}", out.status);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stdout.contains("200"),
        "stdout={stdout:?} stderr={stderr:?}"
    );
    assert!(
        !stdout.contains("50"),
        "stdout={stdout:?} stderr={stderr:?}"
    );
}

/// 누적 지출이 세션 예산($2)을 넘으면 게이트웨이가 원격 AI 호출을 차단한다(§31.7).
/// store(지출) → total_cost → 예산 스냅샷 주입 → `ask` 차단의 end-to-end 결선.
#[cfg(feature = "storage")]
#[tokio::test]
async fn over_budget_blocks_remote_ask_end_to_end() {
    use ai_terminal::gateway::{EchoBackend, Gateway, GatewayOutcome};
    use ai_terminal::store::Store;
    use ai_terminal::usage::BudgetConfig;

    let store = Store::open_in_memory().unwrap();
    // 세션 한도 $2를 넘는 원격 지출을 기록.
    store
        .record_usage("openai", "gpt-x", 1000, 1000, 0, 2.5, None)
        .unwrap();
    let spent = store.total_cost(None).unwrap();
    assert!(spent >= 2.0, "spent should exceed budget: {spent}");

    let cap = ai_terminal::provider::Provider::mock().models[0].clone();
    let gw = Gateway::new(Box::new(EchoBackend), cap).with_budget(spent, BudgetConfig::defaults());
    let out = gw.ask("새로운 원격 질문", "").await.unwrap();
    assert!(
        matches!(out, GatewayOutcome::Blocked(_)),
        "over-budget remote ask must be blocked: {out:?}"
    );
}
