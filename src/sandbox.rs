//! 실행형 preview 샌드박스 (tmpdir 백엔드, 설계 §31.5/§31.11, §15 MVP).
//!
//! `sed -i`·포매터 등 **실행이 필요한 in-place 편집**을 임시 복사본에서 실행해 원본과
//! diff한다. **원본 파일은 절대 수정하지 않는다**(대상을 temp로 복사 → 명령의 경로
//! 토큰을 temp로 치환 → temp cwd에서 실행 → 원본 vs temp diff). bubblewrap/gVisor 격리는
//! 후속(§31.11) — MVP는 tmpdir + 알려진 in-place 편집 집합 한정 실행.

use std::path::{Path, PathBuf};

use crate::diff;
use crate::preview::PreviewRender;

/// 대상 파일 읽기 상한(거대 파일 회피). preview의 diff 상한과 같은 수준.
const MAX_TARGET_BYTES: u64 = 64 * 1024;

/// 명령에서 in-place 편집 **대상 파일**(기존 일반 파일)을 추린다. 플래그/sed 스크립트/
/// `=` 토큰은 제외한다([`crate::cmdparse::args_after_program`]로 선행 래퍼·프로그램 토큰 제외).
pub fn in_place_targets(command: &str) -> Vec<String> {
    crate::cmdparse::args_after_program(command)
        .filter(|t| !t.starts_with('-') && !t.contains('='))
        .filter(|t| Path::new(t).is_file())
        .map(String::from)
        .collect()
}

/// 명령 문자열에서 경로 토큰 `from`을 `to`로 치환한다(공백 토큰 단위, 순수).
pub fn rewrite_path(command: &str, from: &str, to: &str) -> String {
    command
        .split_whitespace()
        .map(|t| if t == from { to } else { t })
        .collect::<Vec<_>>()
        .join(" ")
}

/// 임시 디렉터리를 만들고 Drop 시 정리한다(best-effort).
struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> std::io::Result<TempDir> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("ai_sandbox_{}_{}_{}", std::process::id(), tag, n));
        std::fs::create_dir_all(&dir)?;
        Ok(TempDir(dir))
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// in-place 편집 명령을 샌드박스(temp 복사본)에서 실행하고 원본 대비 unified diff를 만든다.
/// 원본은 수정하지 않는다. 대상이 없거나 실행이 불가하면 `Err`(호출측이 보류 안내로 강등).
pub fn preview_in_place(command: &str) -> anyhow::Result<Vec<PreviewRender>> {
    // 샌드박스 실행은 POSIX 셸·경로에 의존한다(§15 tmpdir=MVP). 비-Unix(Windows 네이티브)
    // 에서는 실행하지 않고 보류로 강등한다 — 셸/경로 차이로 인한 원본 오염을 원천 차단.
    if !cfg!(unix) {
        anyhow::bail!("샌드박스 미리보기는 Unix(WSL/Linux)에서만 지원");
    }
    let targets = in_place_targets(command);
    if targets.is_empty() {
        anyhow::bail!("in-place 편집 대상 파일을 특정할 수 없음");
    }
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let tmp = TempDir::new("inplace")?;
    let mut out = Vec::new();

    for target in &targets {
        let orig = Path::new(target);
        let too_big = std::fs::metadata(orig)
            .map(|m| m.len() > MAX_TARGET_BYTES)
            .unwrap_or(true);
        if too_big {
            out.push(PreviewRender::Info(format!(
                "{target}: 대상이 너무 커 샌드박스 미리보기 생략(> {MAX_TARGET_BYTES} bytes)"
            )));
            continue;
        }
        let before = std::fs::read_to_string(orig)?;
        let file_name = orig
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("대상 파일명 없음: {target}"))?;
        let temp_file = tmp.0.join(file_name);
        std::fs::copy(orig, &temp_file)?;
        let temp_str = temp_file.to_string_lossy().into_owned();

        // 명령의 대상 경로를 temp 경로로 치환해 temp cwd에서 실행(원본 미접촉).
        let rewritten = rewrite_path(command, target, &temp_str);
        let status = std::process::Command::new(&shell)
            .arg("-c")
            .arg(&rewritten)
            .current_dir(&tmp.0)
            .output();
        if let Err(e) = status {
            out.push(PreviewRender::Info(format!(
                "{target}: 샌드박스 실행 불가({e}) — 보류"
            )));
            continue;
        }

        let after = std::fs::read_to_string(&temp_file).unwrap_or_default();
        let d = diff::unified_diff(&before, &after, target, &format!("{target} (편집 후)"));
        if d.trim().is_empty() {
            out.push(PreviewRender::Info(format!("{target}: 변경 없음")));
        } else {
            out.push(PreviewRender::Diff(d));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file(tag: &str, content: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ai_sbx_t_{}_{}_{}", std::process::id(), tag, n));
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn rewrite_path_replaces_only_matching_token() {
        assert_eq!(
            rewrite_path("sed -i s/a/b/ file.txt", "file.txt", "/tmp/x/file.txt"),
            "sed -i s/a/b/ /tmp/x/file.txt"
        );
        // 다른 토큰은 불변.
        assert_eq!(
            rewrite_path("black app.py", "other.py", "/t/o"),
            "black app.py"
        );
    }

    #[test]
    fn in_place_targets_picks_existing_files_only() {
        let f = tmp_file("t", "hello\n");
        let fs = f.to_string_lossy();
        let cmd = format!("sed -i s/hello/bye/ {fs}");
        let targets = in_place_targets(&cmd);
        assert_eq!(targets, vec![fs.to_string()], "기존 파일만 대상");
        // 플래그·스크립트·미존재 경로는 제외.
        let cmd2 = format!("sed -i s/x/y/ {fs} /nonexistent/zzz");
        assert_eq!(in_place_targets(&cmd2), vec![fs.to_string()]);
        std::fs::remove_file(&f).unwrap();
    }

    #[test]
    fn in_place_targets_empty_when_no_file() {
        assert!(in_place_targets("sed -i s/a/b/ /no/such/file").is_empty());
    }

    /// 실제 sed 실행은 셸이 필요 — WSL/unix에서 원본 미수정 + diff 생성을 검증한다.
    #[cfg(unix)]
    #[test]
    fn preview_in_place_diffs_without_touching_original() {
        let f = tmp_file("orig", "foo\nbar\n");
        let fs = f.to_string_lossy().into_owned();
        let cmd = format!("sed -i s/foo/FOO/ {fs}");
        let renders = preview_in_place(&cmd).unwrap();
        // 원본은 그대로.
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            "foo\nbar\n",
            "원본 미수정"
        );
        // diff가 foo→FOO 변화를 담는다.
        let has_diff = renders.iter().any(|r| match r {
            PreviewRender::Diff(d) => d.contains("foo") && d.contains("FOO"),
            _ => false,
        });
        assert!(has_diff, "sed 편집 diff가 있어야 함: {renders:?}");
        std::fs::remove_file(&f).unwrap();
    }
}
