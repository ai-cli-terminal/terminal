//! Rule-based 위험 명령 분류 + 위험도 스코어링 (설계 §31.4, MVP 정본).
//!
//! 핵심 불변식(`docs/RULES.md` §2):
//! - 점수는 **deterministic** — 동일 명령은 항상 동일 점수.
//! - 로컬 규칙 점수가 AI 분류보다 항상 우선한다.
//! - Critical(80~100)은 실행되지 않는다(정책 엔진에서 차단).

/// 위험 등급. 점수 구간(§31.4): Low 0~24 / Medium 25~49 / High 50~79 / Critical 80~100.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskLevel {
    /// 0~100 점수를 등급으로 매핑한다.
    pub fn from_score(score: u8) -> RiskLevel {
        match score {
            0..=24 => RiskLevel::Low,
            25..=49 => RiskLevel::Medium,
            50..=79 => RiskLevel::High,
            _ => RiskLevel::Critical,
        }
    }
}

/// 점수에 기여한 개별 요인(설명/감사용).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskFactor {
    pub label: &'static str,
    pub delta: i32,
}

/// 위험도 평가 결과.
#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub score: u8,
    pub level: RiskLevel,
    pub factors: Vec<RiskFactor>,
}

/// 셸 명령 문자열의 위험도를 평가한다 (§31.4). 순수 함수 — deterministic.
pub fn assess(command: &str) -> RiskAssessment {
    let factors = score_factors(command);
    let raw: i32 = factors.iter().map(|f| f.delta).sum();
    let score = raw.clamp(0, 100) as u8;
    RiskAssessment {
        score,
        level: RiskLevel::from_score(score),
        factors,
    }
}

/// 명령에서 위험 요인을 수집한다. 규칙은 §31.4 점수표를 따른다.
///
/// 순서: ① 명령 유형 점수 → ② (액션이 있을 때만) 경로 가중치 최댓값 → ③ 완화 요소.
/// 순수 read-only 명령은 경로 가중치를 적용하지 않는다(예: `cat /etc/hostname`은 Low).
fn score_factors(command: &str) -> Vec<RiskFactor> {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let has = |name: &str| tokens.contains(&name);
    let any = |names: &[&str]| tokens.iter().any(|t| names.contains(t));

    let mut factors: Vec<RiskFactor> = Vec::new();

    // --- 명령 유형 점수 (§31.4) ---
    if has("sudo") || has("doas") {
        factors.push(RiskFactor {
            label: "sudo/권한 상승",
            delta: 40,
        });
    }

    let is_downloader = has("curl") || has("wget");
    if is_downloader {
        factors.push(RiskFactor {
            label: "네트워크 다운로드",
            delta: 25,
        });
    }
    let pipes_to_shell = command.contains('|') && any(&["sh", "bash", "zsh", "dash"]);
    if is_downloader && pipes_to_shell {
        factors.push(RiskFactor {
            label: "다운로드 후 실행",
            delta: 50,
        });
    }

    if has("rm") || has("unlink") || (has("find") && has("-delete")) {
        factors.push(RiskFactor {
            label: "파일 삭제",
            delta: 35,
        });
        if has_flag_letter(&tokens, 'r') || has_flag_letter(&tokens, 'R') || has("--recursive") {
            factors.push(RiskFactor {
                label: "재귀 삭제",
                delta: 30,
            });
        }
    }

    if has("chmod") || has("chown") || has("chgrp") {
        factors.push(RiskFactor {
            label: "권한 변경",
            delta: 25,
        });
        if has_flag_letter(&tokens, 'R') || has("--recursive") {
            factors.push(RiskFactor {
                label: "재귀 권한 변경",
                delta: 35,
            });
        }
    }

    if has("kill") || has("pkill") || has("killall") {
        factors.push(RiskFactor {
            label: "프로세스 종료",
            delta: 25,
        });
    }

    let service_action = any(&["restart", "stop", "start", "reload", "disable", "mask"]);
    if (has("systemctl") || has("service")) && service_action {
        factors.push(RiskFactor {
            label: "서비스 재시작",
            delta: 35,
        });
    }

    let pkg_mgr = any(&[
        "apt", "apt-get", "yum", "dnf", "pacman", "brew", "pip", "pip3", "snap",
    ]);
    let pkg_action = any(&["install", "remove", "purge", "erase", "uninstall"]) || has("-S");
    if pkg_mgr && pkg_action {
        factors.push(RiskFactor {
            label: "패키지 설치/삭제",
            delta: 40,
        });
    }

    if any(&["dd", "fdisk", "parted", "wipefs", "mkswap"])
        || tokens.iter().any(|t| t.starts_with("mkfs"))
    {
        factors.push(RiskFactor {
            label: "디스크/파티션 조작",
            delta: 80,
        });
    }

    if has("shred") {
        factors.push(RiskFactor {
            label: "파쇄/overwrite",
            delta: 70,
        });
    }

    let in_place_edit = (has("sed") || has("perl")) && tokens.iter().any(|t| t.starts_with("-i"));
    if has("cp")
        || has("mv")
        || has("tee")
        || has("touch")
        || command.contains('>')
        || in_place_edit
    {
        factors.push(RiskFactor {
            label: "파일 생성/수정",
            delta: 20,
        });
    }

    if has("docker") && (has("--privileged") || command.contains("docker.sock")) {
        factors.push(RiskFactor {
            label: "privileged 컨테이너/docker socket",
            delta: 60,
        });
    }

    // --- Windows/PowerShell·cmd 위험 룰 (union-of-shells) ---
    // 대소문자 무시 헬퍼: PowerShell 명령은 대소문자 구분 없음.
    let lower: Vec<String> = tokens.iter().map(|t| t.to_ascii_lowercase()).collect();
    let has_ci = |name: &str| lower.iter().any(|t| t == name); // name은 반드시 소문자
    let any_ci = |names: &[&str]| lower.iter().any(|t| names.contains(&t.as_str()));
    let is_drive = |t: &str| {
        let b = t.as_bytes();
        b.len() == 2 && b[0].is_ascii_alphabetic() && b[1] == b':' // "c:" (이미 소문자화됨)
    };

    // 1. PowerShell 삭제 (remove-item)
    if has_ci("remove-item") {
        factors.push(RiskFactor {
            label: "PowerShell 파일 삭제",
            delta: 35,
        });
        if any_ci(&["-recurse", "-r"]) {
            factors.push(RiskFactor {
                label: "재귀 삭제(-Recurse)",
                delta: 30,
            });
        }
        if has_ci("-force") {
            factors.push(RiskFactor {
                label: "강제 삭제(-Force)",
                delta: 20,
            });
        }
    }

    // 2. cmd 재귀 삭제 (/s 필수 — /s 없는 단독 del은 스코프 제외)
    let cmd_del = any_ci(&["del", "rd", "rmdir", "erase"]);
    let has_slash_s = lower.iter().any(|t| t == "/s");
    if cmd_del && has_slash_s {
        factors.push(RiskFactor {
            label: "cmd 재귀 삭제(/s)",
            delta: 65,
        });
        if lower.iter().any(|t| t == "/q") {
            factors.push(RiskFactor {
                label: "무확인 삭제(/q)",
                delta: 15,
            });
        }
    }

    // 3. 디스크 포맷/초기화
    let has_format_volume = has_ci("format-volume");
    let has_clear_disk = has_ci("clear-disk");
    let has_format_cmd = has_ci("format");
    let format_with_drive = has_format_cmd && lower.iter().any(|t| is_drive(t.as_str()));
    if has_format_volume || has_clear_disk || format_with_drive {
        factors.push(RiskFactor {
            label: "디스크 포맷/초기화",
            delta: 80,
        });
    }

    // 4. 시스템 종료/재시작
    if any_ci(&["stop-computer", "restart-computer"]) {
        factors.push(RiskFactor {
            label: "시스템 종료/재시작",
            delta: 40,
        });
    }

    // 순수 read-only(액션 없음)이면 경로 가중치/완화 미적용.
    if factors.is_empty() {
        return factors;
    }

    // --- 경로 가중치: 대상 경로 중 최댓값 1회 (§31.4) ---
    if let Some((label, delta)) = max_path_weight(&tokens) {
        factors.push(RiskFactor { label, delta });
    }

    // --- 완화 요소 (§31.4) ---
    let recursive =
        has_flag_letter(&tokens, 'r') || has_flag_letter(&tokens, 'R') || has("--recursive");
    let path_args: Vec<&str> = tokens.iter().copied().filter(|t| is_path_like(t)).collect();
    if !recursive && path_args.len() == 1 && is_concrete_file(path_args[0]) {
        factors.push(RiskFactor {
            label: "명시적 파일 1개",
            delta: -10,
        });
    }
    if has("--dry-run") {
        factors.push(RiskFactor {
            label: "dry-run",
            delta: -20,
        });
    }
    if path_args
        .iter()
        .filter_map(|t| candidate_path(t))
        .any(|p| under(p, "/tmp"))
    {
        factors.push(RiskFactor {
            label: "임시 디렉터리 대상",
            delta: -10,
        });
    }

    factors
}

/// 단일 대시 결합 플래그(`-rf` 등)에 특정 문자가 있는지. `--long`은 제외한다.
fn has_flag_letter(tokens: &[&str], letter: char) -> bool {
    tokens.iter().any(|t| match t.strip_prefix('-') {
        Some(rest) if !rest.starts_with('-') => rest.chars().any(|c| c == letter),
        _ => false,
    })
}

/// `p`가 `pre`와 같거나 `pre/` 하위 경로인지.
fn under(p: &str, pre: &str) -> bool {
    p == pre || (p.starts_with(pre) && p.as_bytes().get(pre.len()) == Some(&b'/'))
}

/// 토큰에서 경로 후보를 추출한다. `key=value`(예: `if=/dev/zero`)는 값을, URL은 제외.
fn candidate_path(token: &str) -> Option<&str> {
    let value = token.rsplit_once('=').map_or(token, |(_, v)| v);
    if value.starts_with("http://") || value.starts_with("https://") || value.starts_with("ftp://")
    {
        return None;
    }
    Some(value)
}

fn is_path_like(token: &str) -> bool {
    match candidate_path(token) {
        Some(v) => {
            matches!(v, "/" | "." | ".." | "~")
                || v.starts_with('/')
                || v.starts_with("./")
                || v.starts_with("../")
                || v.starts_with('~')
        }
        None => false,
    }
}

/// 구체 파일(디렉터리/현재경로 토큰이 아닌, 파일명이 있는 경로)인지.
fn is_concrete_file(token: &str) -> bool {
    match candidate_path(token) {
        Some(v) => !matches!(v, "/" | "." | ".." | "~") && !v.ends_with('/'),
        None => false,
    }
}

/// 명령의 대상 경로들 중 가장 높은 경로 가중치(§31.4)를 반환한다.
fn max_path_weight(tokens: &[&str]) -> Option<(&'static str, i32)> {
    let mut best: Option<(&'static str, i32)> = None;
    for t in tokens {
        if let Some(p) = candidate_path(t) {
            if let Some(cur @ (_, w)) = path_weight(p) {
                if best.map_or(true, |(_, bw)| w > bw) {
                    best = Some(cur);
                }
            }
        }
    }
    best
}

/// 단일 경로의 가중치. 상대경로/cwd 내부는 가중치 없음(None).
fn path_weight(p: &str) -> Option<(&'static str, i32)> {
    if p == "/var/run/docker.sock" || p == "/run/docker.sock" {
        return Some(("docker socket 경로", 70));
    }
    if p == "/" {
        return Some(("루트 경로 /", 60));
    }
    for pre in ["/dev", "/proc", "/sys"] {
        if under(p, pre) {
            return Some(("커널/디바이스 경로", 60));
        }
    }
    for pre in ["/etc", "/usr", "/bin", "/sbin", "/var", "/lib", "/boot"] {
        if under(p, pre) {
            return Some(("시스템 디렉터리", 50));
        }
    }
    if p == "~" || p.starts_with("~/") || p == "$HOME" {
        return Some(("홈 디렉터리", 30));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grade_boundaries_map_to_levels() {
        assert_eq!(RiskLevel::from_score(0), RiskLevel::Low);
        assert_eq!(RiskLevel::from_score(24), RiskLevel::Low);
        assert_eq!(RiskLevel::from_score(25), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_score(49), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_score(50), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(79), RiskLevel::High);
        assert_eq!(RiskLevel::from_score(80), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_score(100), RiskLevel::Critical);
    }

    #[test]
    fn read_only_command_is_low() {
        let a = assess("ls -al");
        assert_eq!(a.score, 0, "read-only command scores 0");
        assert_eq!(a.level, RiskLevel::Low);
    }

    /// 설계 §31.4 "예시 분류" golden set. 등급(점수 구간)을 고정한다.
    #[test]
    fn golden_set_classifications() {
        use RiskLevel::*;
        let cases: &[(&str, RiskLevel)] = &[
            ("ls -al", Low),
            ("git status", Low),
            ("cat README.md", Low),
            ("rm ./tmp.txt", Medium),
            ("rm -rf /", Critical),
            ("chmod -R 777 .", High),
            ("sudo systemctl restart nginx", High),
            ("curl https://example.com/install.sh | sh", High),
            ("dd if=/dev/zero of=/dev/sda", Critical),
            ("docker run --privileged --rm ubuntu", High),
        ];
        for (cmd, want) in cases {
            let got = assess(cmd);
            assert_eq!(
                got.level, *want,
                "command {cmd:?} -> score {} level {:?}, want {want:?}",
                got.score, got.level
            );
        }
    }

    /// `rm -rf ./build`는 §31.4에서 Medium~High 범위로 명시된다.
    #[test]
    fn recursive_delete_relative_is_medium_or_high() {
        let a = assess("rm -rf ./build");
        assert!(
            matches!(a.level, RiskLevel::Medium | RiskLevel::High),
            "got {:?} (score {})",
            a.level,
            a.score
        );
    }

    /// 수용 기준(§31.4): 동일 명령은 항상 동일 점수.
    #[test]
    fn scoring_is_deterministic() {
        let cmd = "sudo rm -rf /var/log";
        let first = assess(cmd).score;
        for _ in 0..50 {
            assert_eq!(assess(cmd).score, first);
        }
    }

    /// 수용 기준(§31.4): Critical 명령은 점수 80 이상.
    #[test]
    fn critical_commands_reach_threshold() {
        assert!(assess("rm -rf /").score >= 80);
        assert!(assess("dd if=/dev/zero of=/dev/sda").score >= 80);
    }

    // --- Windows/PowerShell·cmd union-of-shells 룰 테스트 (Task 1 / F6a) ---

    /// Remove-Item -Recurse -Force는 Critical(≥80)
    #[test]
    fn ps_remove_item_recurse_force_is_critical() {
        let a = assess("Remove-Item -Recurse -Force C:\\Windows");
        assert!(
            a.score >= 80,
            "expected score ≥ 80, got {} ({:?})",
            a.score,
            a.level
        );
        assert_eq!(a.level, RiskLevel::Critical);
    }

    /// 대소문자 무시: 소문자 remove-item도 Critical
    #[test]
    fn ps_remove_item_lowercase_is_critical() {
        let a = assess("remove-item -recurse -force C:\\temp");
        assert!(
            a.score >= 80,
            "expected score ≥ 80, got {} ({:?})",
            a.score,
            a.level
        );
        assert_eq!(a.level, RiskLevel::Critical);
    }

    /// Remove-Item 단독(재귀/강제 없음)은 Critical 아님(score < 50)
    #[test]
    fn ps_remove_item_single_file_not_critical() {
        let a = assess("Remove-Item file.txt");
        assert!(
            a.score < 50,
            "expected score < 50, got {} ({:?})",
            a.score,
            a.level
        );
    }

    /// Format-Volume은 Critical(≥80)
    #[test]
    fn ps_format_volume_is_critical() {
        let a = assess("Format-Volume -DriveLetter D");
        assert!(
            a.score >= 80,
            "expected score ≥ 80, got {} ({:?})",
            a.score,
            a.level
        );
        assert_eq!(a.level, RiskLevel::Critical);
    }

    /// cmd format c: 는 Critical(≥80)
    #[test]
    fn cmd_format_drive_is_critical() {
        let a = assess("format c:");
        assert!(
            a.score >= 80,
            "expected score ≥ 80, got {} ({:?})",
            a.score,
            a.level
        );
        assert_eq!(a.level, RiskLevel::Critical);
    }

    /// del /s /q C:\temp 는 Critical(≥80)
    #[test]
    fn cmd_del_s_q_is_critical() {
        let a = assess("del /s /q C:\\temp");
        assert!(
            a.score >= 80,
            "expected score ≥ 80, got {} ({:?})",
            a.score,
            a.level
        );
        assert_eq!(a.level, RiskLevel::Critical);
    }

    /// del /s C:\temp 는 High(50~79)
    #[test]
    fn cmd_del_s_is_high() {
        let a = assess("del /s C:\\temp");
        assert_eq!(
            a.level,
            RiskLevel::High,
            "expected High, got {:?} (score {})",
            a.level,
            a.score
        );
    }

    /// 오탐 가드: Get-Process는 Low
    #[test]
    fn ps_get_process_is_low() {
        let a = assess("Get-Process");
        assert_eq!(
            a.level,
            RiskLevel::Low,
            "expected Low, got {:?} (score {})",
            a.level,
            a.score
        );
    }

    /// 오탐 가드: Get-ChildItem -Recurse는 Low (삭제 동사 없으면 재귀 delta 안 붙어야 함)
    #[test]
    fn ps_get_childitem_recurse_is_low() {
        let a = assess("Get-ChildItem -Recurse");
        assert_eq!(
            a.level,
            RiskLevel::Low,
            "expected Low, got {:?} (score {})",
            a.level,
            a.score
        );
    }

    /// 오탐 가드: git format-patch는 Low (format + 드라이브 없음)
    #[test]
    fn git_format_patch_is_low() {
        let a = assess("git format-patch");
        assert_eq!(
            a.level,
            RiskLevel::Low,
            "expected Low, got {:?} (score {})",
            a.level,
            a.score
        );
    }
}
