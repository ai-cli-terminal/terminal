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

/// 안전(읽기 전용) 미리보기 한도.
const MAX_DIFF_BYTES: u64 = 64 * 1024; // cp/mv diff: LCS DP 메모리 보호
const MAX_RISK_BYTES: u64 = 1024 * 1024; // content-at-risk 읽기 상한
const HEAD_LINES: usize = 10;

/// 안전 미리보기 렌더 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewRender {
    /// cp/mv 덮어쓰기 unified diff.
    Diff(String),
    /// 삭제/truncate로 사라질 내용 요약.
    ContentAtRisk {
        path: String,
        lines: usize,
        bytes: u64,
        head: String,
    },
    /// 분류 전략 안내(dry-run/external/chmod/sed-i 보류/not-needed).
    Info(String),
}

/// 명령 실행 없이(읽기 전용) 안전 미리보기를 생성한다. 대상 파일은 수정하지 않는다.
#[must_use]
pub fn render_preview(command: &str) -> Vec<PreviewRender> {
    match classify_preview(command) {
        PreviewPlan::NotNeeded => vec![PreviewRender::Info("변경 없음 (읽기 전용)".into())],
        PreviewPlan::DryRun(c) => vec![PreviewRender::Info(format!("dry-run 제안: {c}"))],
        PreviewPlan::NotAvailable(r) => vec![PreviewRender::Info(format!("미리보기 불가 — {r}"))],
        PreviewPlan::ListTargets(targets) => render_targets(command, &targets),
        PreviewPlan::TempCopyDiff => render_temp_copy(command),
    }
}

/// rm/shred/unlink → content-at-risk; chmod/chown/chgrp → 목록 안내.
fn render_targets(command: &str, targets: &[String]) -> Vec<PreviewRender> {
    let prog = program_token(command);
    if matches!(
        prog.as_deref(),
        Some("chmod") | Some("chown") | Some("chgrp")
    ) {
        let list = targets.join(", ");
        return vec![PreviewRender::Info(format!(
            "권한 변경(내용 손실 없음) 대상: {list}"
        ))];
    }
    let mut out = Vec::new();
    for t in targets {
        match content_at_risk(t) {
            Some(r) => out.push(r),
            None => out.push(PreviewRender::Info(format!(
                "{t}: 미리볼 내용 없음(미존재/디렉터리)"
            ))),
        }
    }
    if out.is_empty() {
        out.push(PreviewRender::Info("대상 없음".into()));
    }
    out
}

/// cp/mv 덮어쓰기 diff / 리다이렉트 truncate content-at-risk / 그 외(sed -i 등) 보류 안내.
fn render_temp_copy(command: &str) -> Vec<PreviewRender> {
    let prog = program_token(command);
    if matches!(prog.as_deref(), Some("cp") | Some("mv")) {
        let paths = path_args(command);
        if paths.len() == 2 {
            let (src, dst) = (&paths[0], &paths[1]);
            let dst_is_file = std::fs::metadata(dst).map(|m| m.is_file()).unwrap_or(false);
            if dst_is_file {
                return vec![cp_mv_diff(src, dst)];
            }
            return vec![PreviewRender::Info(format!(
                "새 파일 생성 또는 디렉터리 대상 — diff 없음 ({dst})"
            ))];
        }
    }
    if is_in_place_edit(command) {
        return vec![PreviewRender::Info(
            "실제 diff는 명령 실행이 필요 — 샌드박스(Phase 2+) 후속으로 보류".into(),
        )];
    }
    if let Some(t) = overwrite_redirect_target(command) {
        if let Some(r) = content_at_risk(&t) {
            return vec![r];
        }
    }
    vec![PreviewRender::Info(
        "미리보기 미지원 명령 — 실행 후 확인 필요(실제 diff는 샌드박스 후속)".into(),
    )]
}

/// 기존 파일의 content-at-risk 요약(읽기 전용). 미존재/디렉터리면 None, 과대 파일은 Info.
fn content_at_risk(path: &str) -> Option<PreviewRender> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    if meta.len() > MAX_RISK_BYTES {
        return Some(PreviewRender::Info(format!(
            "{path}: 파일이 커 미리보기 생략 ({} bytes)",
            meta.len()
        )));
    }
    let bytes = meta.len();
    let raw = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&raw);
    let lines = text.lines().count();
    let head = text.lines().take(HEAD_LINES).collect::<Vec<_>>().join("\n");
    Some(PreviewRender::ContentAtRisk {
        path: path.to_string(),
        lines,
        bytes,
        head,
    })
}

/// cp/mv: dst(기존 파일) vs src의 unified diff. 과대/비파일은 Info.
fn cp_mv_diff(src: &str, dst: &str) -> PreviewRender {
    let too_big = |p: &str| {
        std::fs::metadata(p)
            .map(|m| m.len() > MAX_DIFF_BYTES)
            .unwrap_or(true)
    };
    if too_big(src) || too_big(dst) {
        return PreviewRender::Info(format!(
            "{dst}: 파일이 너무 크거나 읽기 불가 — diff 생략 (한도 {MAX_DIFF_BYTES} bytes)"
        ));
    }
    let before = std::fs::read(dst).map(|b| String::from_utf8_lossy(&b).into_owned());
    let after = std::fs::read(src).map(|b| String::from_utf8_lossy(&b).into_owned());
    match (before, after) {
        (Ok(b), Ok(a)) => {
            let d = crate::diff::unified_diff(&b, &a, dst, src);
            if d.is_empty() {
                PreviewRender::Info(format!("{dst}: 변경 없음(내용 동일)"))
            } else {
                PreviewRender::Diff(d)
            }
        }
        _ => PreviewRender::Info(format!("{dst}: 읽기 실패")),
    }
}

/// 선행 sudo/env/`VAR=` 를 건너뛴 프로그램 토큰.
fn program_token(command: &str) -> Option<String> {
    for t in command.split_whitespace() {
        if matches!(t, "sudo" | "doas" | "env" | "nohup" | "nice") {
            continue;
        }
        if t.contains('=') && !t.starts_with('/') && !t.starts_with('.') {
            continue;
        }
        return Some(t.to_string());
    }
    None
}

/// 프로그램 토큰 이후의 경로 인자(플래그/리다이렉트/연산자 제외).
fn path_args(command: &str) -> Vec<String> {
    let mut it = command.split_whitespace();
    for t in it.by_ref() {
        if matches!(t, "sudo" | "doas" | "env" | "nohup" | "nice") {
            continue;
        }
        if t.contains('=') && !t.starts_with('/') && !t.starts_with('.') {
            continue;
        }
        break; // 프로그램 토큰 소비
    }
    it.filter(|t| {
        !t.starts_with('-')
            && !t.starts_with('>')
            && !t.starts_with("2>")
            && !t.starts_with("&>")
            && !t.chars().all(|c| c.is_ascii_digit())
            && !t.contains('=')
            && !matches!(*t, "|" | "&&" | ";" | ">" | ">>")
    })
    .map(String::from)
    .collect()
}

/// 덮어쓰기 리다이렉트(`>`) 대상 파일명. append `>>`·stderr `2>`·`&>`는 대상에서 제외한다.
fn overwrite_redirect_target(command: &str) -> Option<String> {
    let toks: Vec<&str> = command.split_whitespace().collect();
    for (i, t) in toks.iter().enumerate() {
        if *t == ">" {
            return toks.get(i + 1).map(|s| s.to_string());
        }
        if t.starts_with('>') && !t.starts_with(">>") && t.len() > 1 {
            return Some(t[1..].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering};
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ai_prev_{}_{}_{}", std::process::id(), tag, n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn cp_over_existing_file_renders_diff() {
        let d = tmpdir("cp");
        let src = d.join("src.txt");
        let dst = d.join("dst.txt");
        std::fs::write(&src, "a\nB\nc\n").unwrap();
        std::fs::write(&dst, "a\nb\nc\n").unwrap();
        let cmd = format!("cp {} {}", src.display(), dst.display());
        let r = render_preview(&cmd);
        assert!(
            r.iter().any(
                |x| matches!(x, PreviewRender::Diff(d) if d.contains("-b") && d.contains("+B"))
            ),
            "{r:?}"
        );
    }

    #[test]
    fn cp_to_missing_dst_is_info() {
        let d = tmpdir("cpnew");
        let src = d.join("src.txt");
        std::fs::write(&src, "x\n").unwrap();
        let dst = d.join("new.txt");
        let cmd = format!("cp {} {}", src.display(), dst.display());
        let r = render_preview(&cmd);
        assert!(
            r.iter().all(|x| matches!(x, PreviewRender::Info(_))),
            "{r:?}"
        );
    }

    #[test]
    fn rm_existing_file_is_content_at_risk() {
        let d = tmpdir("rm");
        let f = d.join("data.txt");
        std::fs::write(&f, "l1\nl2\nl3\n").unwrap();
        let cmd = format!("rm {}", f.display());
        let r = render_preview(&cmd);
        assert!(
            r.iter()
                .any(|x| matches!(x, PreviewRender::ContentAtRisk { lines, .. } if *lines == 3)),
            "{r:?}"
        );
    }

    #[test]
    fn redirect_overwrite_existing_is_content_at_risk() {
        let d = tmpdir("redir");
        let f = d.join("out.txt");
        std::fs::write(&f, "old1\nold2\n").unwrap();
        let cmd = format!("echo hi > {}", f.display());
        let r = render_preview(&cmd);
        assert!(
            r.iter()
                .any(|x| matches!(x, PreviewRender::ContentAtRisk { .. })),
            "{r:?}"
        );
    }

    #[test]
    fn sed_in_place_is_deferred_info() {
        let d = tmpdir("sed");
        let f = d.join("f.txt");
        std::fs::write(&f, "a\n").unwrap();
        let cmd = format!("sed -i s/a/b/ {}", f.display());
        let r = render_preview(&cmd);
        assert!(
            r.iter()
                .any(|x| matches!(x, PreviewRender::Info(m) if m.contains("실행"))),
            "{r:?}"
        );
    }

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
