//! Preview / Diff 전략 분류 (설계 §31.5, M3/W9).
//!
//! 파일 변경 가능 명령에 대해 어떤 방식으로 미리보기할지 결정한다(§31.5):
//! native dry-run 우선 → 임시 복사본 diff → 삭제 대상 목록 → 불가 사유 표시.
//!
//! 불변식(`docs/RULES.md` §1): 파일 변경은 preview/dry-run/diff를 우선 제공한다.

/// preview 전략.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewPlan {
    /// 파일 변경 없음(읽기 전용 등) — preview 불필요.
    NotNeeded,
    /// native dry-run 명령으로 미리보기(제안 명령 포함).
    DryRun(String),
    /// 임시 복사본에서 실행 후 diff(in-place 편집 등).
    TempCopyDiff,
    /// 삭제/권한 변경 — 대상 목록을 표시.
    ListTargets(Vec<String>),
    /// preview 불가 — 사유.
    NotAvailable(String),
}

/// 명령의 preview 전략을 판정한다.
pub fn classify_preview(command: &str) -> PreviewPlan {
    // 1) native dry-run 우선
    if let Some(dr) = dry_run_command(command) {
        return PreviewPlan::DryRun(dr);
    }
    // 2) in-place 편집 → 임시 복사본 diff
    if is_in_place_edit(command) {
        return PreviewPlan::TempCopyDiff;
    }
    // 3) 외부 시스템 상태 변경 → 불가
    if let Some(reason) = external_reason(command) {
        return PreviewPlan::NotAvailable(reason);
    }
    // 4) 삭제 / 권한 변경 → 대상 목록
    let toks: Vec<&str> = command.split_whitespace().collect();
    let has = |s: &str| toks.contains(&s);
    if has("rm") || has("unlink") || has("shred") || has("chmod") || has("chown") || has("chgrp") {
        return PreviewPlan::ListTargets(extract_targets(command));
    }
    // 5) 그 외 파일 생성/수정 → 임시 복사본 diff
    if has("cp") || has("mv") || has("tee") || has("touch") || command.contains('>') {
        return PreviewPlan::TempCopyDiff;
    }
    PreviewPlan::NotNeeded
}

/// native dry-run 제안 명령(있으면).
fn dry_run_command(command: &str) -> Option<String> {
    let t: Vec<&str> = command.split_whitespace().collect();
    let has = |s: &str| t.contains(&s);
    let already =
        command.contains("--dry-run") || command.contains("-n ") || command.ends_with("-n");
    if has("rsync") && !already {
        return Some(format!("{command} --dry-run"));
    }
    if has("git") && has("clean") && !already {
        return Some(format!("{command} -n"));
    }
    if has("terraform") && has("apply") {
        return Some(command.replacen("apply", "plan", 1));
    }
    if has("kubectl") && (has("apply") || has("delete") || has("create")) && !already {
        return Some(format!("{command} --dry-run=client"));
    }
    if has("helm") && (has("install") || has("upgrade")) && !already {
        return Some(format!("{command} --dry-run"));
    }
    None
}

/// in-place 파일 편집 명령인지(sed -i, formatter 등).
fn is_in_place_edit(command: &str) -> bool {
    let t: Vec<&str> = command.split_whitespace().collect();
    let has = |s: &str| t.contains(&s);
    let in_place_flag = t.iter().any(|x| x.starts_with("-i"));
    ((has("sed") || has("perl")) && in_place_flag)
        || (has("prettier") && has("--write"))
        || has("black")
        || (has("gofmt") && has("-w"))
        || (has("ruff") && has("--fix"))
}

/// 외부 시스템 상태를 바꿔 preview 불가한 사유(있으면).
fn external_reason(command: &str) -> Option<String> {
    let t: Vec<&str> = command.split_whitespace().collect();
    let has = |s: &str| t.contains(&s);
    let any = |xs: &[&str]| t.iter().any(|x| xs.contains(x));
    if (has("systemctl") || has("service")) && any(&["restart", "stop", "start", "reload"]) {
        return Some("서비스 상태 변경은 미리볼 수 없습니다".into());
    }
    if any(&[
        "apt", "apt-get", "yum", "dnf", "pacman", "brew", "pip", "pip3",
    ]) && any(&["install", "remove", "purge", "uninstall"])
    {
        return Some("패키지 설치/삭제는 미리볼 수 없습니다".into());
    }
    if has("docker") && any(&["run", "rm", "rmi"]) {
        return Some("컨테이너 변경은 미리볼 수 없습니다".into());
    }
    if any(&["scp", "sftp"]) {
        return Some("네트워크 전송은 미리볼 수 없습니다".into());
    }
    if any(&["migrate", "alembic", "flyway"]) {
        return Some("DB 마이그레이션은 미리볼 수 없습니다".into());
    }
    None
}

/// 명령의 대상 경로(플래그/숫자/옵션 제외)를 추출한다.
fn extract_targets(command: &str) -> Vec<String> {
    let mut toks = command.split_whitespace();
    // 선행 sudo/env 와 VAR=value 건너뛰고 프로그램 토큰 소비
    for tok in toks.by_ref() {
        if matches!(tok, "sudo" | "doas" | "env" | "nohup" | "nice") {
            continue;
        }
        if tok.contains('=') && !tok.starts_with('/') && !tok.starts_with('.') {
            continue;
        }
        break; // 프로그램 토큰
    }
    toks.filter(|t| {
        !t.starts_with('-')                       // 플래그 제외
            && !t.chars().all(|c| c.is_ascii_digit()) // 숫자(예: chmod mode) 제외
            && !t.contains('=')
    })
    .map(String::from)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_tools_are_suggested() {
        assert_eq!(
            classify_preview("rsync -a a/ b/"),
            PreviewPlan::DryRun("rsync -a a/ b/ --dry-run".into())
        );
        assert_eq!(
            classify_preview("git clean -fd"),
            PreviewPlan::DryRun("git clean -fd -n".into())
        );
        assert_eq!(
            classify_preview("kubectl apply -f x.yaml"),
            PreviewPlan::DryRun("kubectl apply -f x.yaml --dry-run=client".into())
        );
    }

    #[test]
    fn in_place_edit_uses_temp_copy_diff() {
        assert_eq!(
            classify_preview("sed -i s/a/b/ f.txt"),
            PreviewPlan::TempCopyDiff
        );
        assert_eq!(
            classify_preview("prettier --write src/"),
            PreviewPlan::TempCopyDiff
        );
    }

    #[test]
    fn deletion_lists_targets() {
        assert_eq!(
            classify_preview("rm -rf ./build /tmp/x"),
            PreviewPlan::ListTargets(vec!["./build".into(), "/tmp/x".into()])
        );
        assert_eq!(
            classify_preview("chmod -R 777 ."),
            PreviewPlan::ListTargets(vec![".".into()])
        );
    }

    #[test]
    fn external_state_is_not_available() {
        assert!(matches!(
            classify_preview("sudo systemctl restart nginx"),
            PreviewPlan::NotAvailable(_)
        ));
        assert!(matches!(
            classify_preview("apt install ripgrep"),
            PreviewPlan::NotAvailable(_)
        ));
    }

    #[test]
    fn read_only_not_needed() {
        assert_eq!(classify_preview("ls -al"), PreviewPlan::NotNeeded);
        assert_eq!(classify_preview("git status"), PreviewPlan::NotNeeded);
    }
}
