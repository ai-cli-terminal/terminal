# W9 안전 미리보기(실행 없는 diff/content-at-risk) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `ai preview`가 명령 실행 없이(읽기 전용) cp/mv 덮어쓰기의 진짜 unified diff와 rm/truncate의 content-at-risk 요약을 보여준다.

**Architecture:** 순수 LCS unified diff 모듈(`diff.rs`)을 추가하고, `preview.rs`에 `render_preview`(안전 파일 읽기 + classify 재사용 + diff/요약 생성)를 추가한다. `main.rs` `format_preview`가 이를 호출해 실제 내용을 출력한다. 대상 파일은 절대 수정하지 않는다.

**Tech Stack:** Rust(표준 라이브러리만 — 새 의존성 없음). 기존 `preview.rs` 분류 재사용.

설계 정본: `docs/superpowers/specs/2026-06-03-safe-preview-render-design.md`

빌드/검증(WSL): `wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`
**주의: 이 하니스에서 `$?`는 항상 0(측정 불가). cargo `test result:` 텍스트 또는 `cmd && echo OK || echo FAIL`로 판정.**

---

### Task 1: 순수 LCS unified diff (`src/diff.rs` 신규)

**Files:** Create `src/diff.rs`; Modify `src/lib.rs`(모듈 등록)

- [ ] **Step 1: `src/diff.rs` 작성(구현 + 테스트 동시 — 순수 함수)**

```rust
//! 라인 단위 unified diff (LCS 기반, 순수·결정적). W9 안전 미리보기용.
//!
//! 외부 의존성 없이 표준 LCS DP로 공통 부분수열을 구해 삭제/추가/유지 라인을 출력한다.
//! 대용량은 호출측(`preview.rs`)에서 바이트 상한으로 보호한다.

/// 두 텍스트의 라인 단위 unified diff 문자열. 라인 집합이 동일하면 빈 문자열.
/// `--- <before_label>` / `+++ <after_label>` 헤더 + `-`(삭제)/`+`(추가)/` `(유지) 라인.
pub fn unified_diff(before: &str, after: &str, before_label: &str, after_label: &str) -> String {
    let a: Vec<&str> = before.lines().collect();
    let b: Vec<&str> = after.lines().collect();
    if a == b {
        return String::new();
    }
    let (n, m) = (a.len(), b.len());
    // dp[i][j] = LCS length of a[i..], b[j..]
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut body = String::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            body.push(' ');
            body.push_str(a[i]);
            body.push('\n');
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            body.push('-');
            body.push_str(a[i]);
            body.push('\n');
            i += 1;
        } else {
            body.push('+');
            body.push_str(b[j]);
            body.push('\n');
            j += 1;
        }
    }
    while i < n {
        body.push('-');
        body.push_str(a[i]);
        body.push('\n');
        i += 1;
    }
    while j < m {
        body.push('+');
        body.push_str(b[j]);
        body.push('\n');
        j += 1;
    }

    format!("--- {before_label}\n+++ {after_label}\n{body}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_is_empty() {
        assert_eq!(unified_diff("a\nb\n", "a\nb\n", "x", "y"), "");
    }

    #[test]
    fn single_line_change_shows_minus_plus() {
        let d = unified_diff("a\nb\nc\n", "a\nB\nc\n", "old", "new");
        assert!(d.contains("--- old"), "{d}");
        assert!(d.contains("+++ new"), "{d}");
        assert!(d.contains("-b"), "{d}");
        assert!(d.contains("+B"), "{d}");
        assert!(d.contains(" a"), "{d}");
        assert!(d.contains(" c"), "{d}");
    }

    #[test]
    fn pure_addition_and_deletion() {
        let add = unified_diff("a\n", "a\nb\n", "o", "n");
        assert!(add.contains("+b"), "{add}");
        let del = unified_diff("a\nb\n", "a\n", "o", "n");
        assert!(del.contains("-b"), "{del}");
    }
}
```

- [ ] **Step 2: `src/lib.rs`에 모듈 등록**

`pub mod context;`(약 14행)와 `pub mod dispatch;` 사이에 추가:
```rust
pub mod diff;
```

- [ ] **Step 3: 테스트 실행**

Run: `wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib diff 2>&1 | grep -E "test result|error\["'`
Expected: `test result: ok` (3개 통과).

- [ ] **Step 4: fmt + 커밋**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all'
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add src/diff.rs src/lib.rs && git commit -m "feat(diff): pure LCS unified diff for safe preview

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

### Task 2: `render_preview` + `PreviewRender` (`src/preview.rs`)

**Files:** Modify `src/preview.rs`(enum + render_preview + 헬퍼 + 테스트)

- [ ] **Step 1: 실패 테스트 작성** — `src/preview.rs` `mod tests`에 추가:

```rust
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
            r.iter().any(|x| matches!(x, PreviewRender::Diff(d) if d.contains("-b") && d.contains("+B"))),
            "{r:?}"
        );
    }

    #[test]
    fn cp_to_missing_dst_is_info() {
        let d = tmpdir("cpnew");
        let src = d.join("src.txt");
        std::fs::write(&src, "x\n").unwrap();
        let dst = d.join("new.txt"); // 미존재
        let cmd = format!("cp {} {}", src.display(), dst.display());
        let r = render_preview(&cmd);
        assert!(r.iter().all(|x| matches!(x, PreviewRender::Info(_))), "{r:?}");
    }

    #[test]
    fn rm_existing_file_is_content_at_risk() {
        let d = tmpdir("rm");
        let f = d.join("data.txt");
        std::fs::write(&f, "l1\nl2\nl3\n").unwrap();
        let cmd = format!("rm {}", f.display());
        let r = render_preview(&cmd);
        assert!(
            r.iter().any(|x| matches!(x, PreviewRender::ContentAtRisk { lines, .. } if *lines == 3)),
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
            r.iter().any(|x| matches!(x, PreviewRender::ContentAtRisk { .. })),
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
            r.iter().any(|x| matches!(x, PreviewRender::Info(m) if m.contains("실행"))),
            "{r:?}"
        );
    }
```

- [ ] **Step 2: 테스트 실패 확인**

Run: `wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib preview 2>&1 | tail -15'`
Expected: 컴파일 에러 — `render_preview`/`PreviewRender` 미정의.

- [ ] **Step 3: 구현** — `src/preview.rs`에 추가(파일 끝 `#[cfg(test)]` 앞):

```rust
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
    if matches!(prog.as_deref(), Some("chmod") | Some("chown") | Some("chgrp")) {
        let list = targets.join(", ");
        return vec![PreviewRender::Info(format!("권한 변경(내용 손실 없음) 대상: {list}"))];
    }
    let mut out = Vec::new();
    for t in targets {
        match content_at_risk(t) {
            Some(r) => out.push(r),
            None => out.push(PreviewRender::Info(format!("{t}: 미리볼 내용 없음(미존재/디렉터리)"))),
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
    vec![PreviewRender::Info("임시 복사본 diff 대상 — 실제 생성은 후속(보류)".into())]
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
    let head = text
        .lines()
        .take(HEAD_LINES)
        .collect::<Vec<_>>()
        .join("\n");
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
        return PreviewRender::Info(format!("{dst}: 파일이 커 diff 생략 (>{MAX_DIFF_BYTES} bytes)"));
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
            && !t.contains('=')
            && !matches!(*t, "|" | "&&" | ";" | ">" | ">>")
    })
    .map(String::from)
    .collect()
}

/// 덮어쓰기 리다이렉트(`>`, append `>>` 제외) 대상 파일명.
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
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test --lib preview 2>&1 | grep -E "test result|error\["'`
Expected: `test result: ok` (신규 5개 + 기존 classify 테스트 통과).

- [ ] **Step 5: fmt + clippy + 커밋**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --lib --all-targets -- -D warnings 2>&1 | tail -3'
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add src/preview.rs && git commit -m "feat(preview): safe render_preview (diff/content-at-risk) without execution

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

### Task 3: `ai preview` 강화 (`src/main.rs`)

**Files:** Modify `src/main.rs`(`format_preview` 재작성)

- [ ] **Step 1: `format_preview` 교체** — `src/main.rs`(약 501행) 전체 함수를 교체:

```rust
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
    s
}
```

- [ ] **Step 2: 기존 `format_preview` 테스트 확인/조정**

`src/main.rs` `mod tests`에서 `format_preview`를 참조하는 테스트가 있으면(예: dry-run 문자열 비교) 새 출력 형식에 맞게 보정한다. 다음 명령으로 해당 테스트를 찾아 실패 여부 확인:
`wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; grep -n "format_preview" src/main.rs'`
그리고 빌드/테스트로 깨진 단정을 확인해 최소 보정(예: `assert!(out.contains("dry-run"))` 유지되도록). 새 `Info` 출력은 `"preview  : dry-run 제안: <cmd>"` 형태라 `contains("dry-run")`은 유지된다.

- [ ] **Step 3: 빌드·clippy·fmt·테스트(기본 + storage)**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo fmt --all && cargo clippy --all-targets -- -D warnings 2>&1 | tail -3 && cargo test 2>&1 | grep -E "test result|error\["'
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo clippy --all-targets --features storage -- -D warnings 2>&1 | tail -3 && cargo test --features storage 2>&1 | grep -E "test result|error\["'
```
Expected: clippy/fmt clean, 모든 `test result: ok`.

- [ ] **Step 4: 커밋**

```
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add src/main.rs && git commit -m "feat(cli): ai preview shows real diff and content-at-risk

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

### Task 4: WSL e2e + 문서 갱신

**Files:** Modify `docs/TASK.md`, `docs/HISTORY.md`

- [ ] **Step 1: e2e — cp diff / rm content-at-risk / sed 보류**

```
wsl.exe -- bash -c 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build 2>&1 | tail -1; BIN=$HOME/targets/ai-terminal/debug/ai; D=$(mktemp -d); printf "a\\nb\\nc\\n" > $D/dst; printf "a\\nB\\nc\\n" > $D/src; echo "=== cp diff ==="; $BIN preview "cp $D/src $D/dst"; echo "=== rm risk ==="; $BIN preview "rm $D/dst"; echo "=== sed deferred ==="; $BIN preview "sed -i s/a/b/ $D/dst"; echo "=== verify dst untouched ==="; cat $D/dst'
```
Expected: cp → `-b`/`+B` 포함 diff; rm → "손실 예정 ... (3줄, ... bytes)" + 내용; sed → "실행이 필요 ... 보류"; **dst는 `a/b/c` 그대로**(미수정 확인 — 안전성 핵심).

- [ ] **Step 2: `docs/TASK.md` 갱신**

`### W9 Preview / Diff 엔진` 아래의 다음 줄:
```
- [ ] 실제 temp-copy 실행→diff(sed류) 생성 — WSL 연동 후속(현재 전략 표시까지)
```
을 갱신:
```
- [x] 안전(실행 없는) 실제 미리보기 (2026-06-03): cp/mv 덮어쓰기 → 진짜 unified diff(읽기 전용), rm/truncate → content-at-risk 요약. `src/diff.rs`(LCS) + `preview::render_preview`. sed -i/perl -i 등 **실행 필요** diff는 샌드박스(§31.11, Phase 2+) 후속. 설계/계획: `docs/superpowers/{specs,plans}/2026-06-03-safe-preview-render*`
```

- [ ] **Step 3: `docs/HISTORY.md` 엔트리 추가**

`docs/HISTORY.md` 최신 엔트리 형식 확인 후 최상단에 추가(스타일 보정):
```markdown
## 2026-06-03 — W9 안전 미리보기 (실행 없는 diff/content-at-risk)

- **diff**(`diff.rs` 신규): 순수 LCS 라인 unified diff(`unified_diff`, 외부 의존성 없음).
- **preview**(`preview.rs`): `render_preview`/`PreviewRender` 추가 — cp/mv 덮어쓰기(dst 기존)는 read-only로 진짜 unified diff, rm/shred/unlink·`> file` truncate는 content-at-risk(행·바이트·head). sed -i/perl -i/formatter 등 실행 필요 diff는 보류(샌드박스 후속). 크기 상한(diff 64KiB/risk 1MiB)·비UTF8 lossy·미존재/디렉터리 안전 처리. 대상 파일 절대 미수정.
- **cli**(`main.rs`): `ai preview`가 `format_preview`로 실제 diff/요약 출력(기존 분류 메시지는 Info로 유지).
- 검증: diff 단위(추가/삭제/변경), preview 단위(temp 파일: cp diff·rm risk·redirect·sed 보류·미존재), WSL e2e(dst 미수정 확인). clippy/fmt clean, default+storage 전체 통과.
- 설계/계획: `docs/superpowers/specs/2026-06-03-safe-preview-render-design.md`, `docs/superpowers/plans/2026-06-03-safe-preview-render.md`.
```

- [ ] **Step 4: 커밋**

```
wsl.exe -- bash -c 'cd /mnt/d/workspace/terminal-project/terminal; git add docs/TASK.md docs/HISTORY.md && git commit -m "docs: record W9 safe preview render

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"'
```

---

## 완료 기준 (DoD)

- `ai preview "cp src dst"`(dst 기존) → 진짜 unified diff; `rm f` → content-at-risk; `sed -i` → 보류 안내.
- 모든 경로가 **대상 파일을 수정하지 않음**(e2e로 dst 미수정 확인).
- 단위 테스트: diff(3) + preview(5). clippy/fmt clean, 기본 + storage 전체 PASS.
- 문서(TASK/HISTORY) 갱신. 비목표(실행 필요 diff = 샌드박스 후속)는 미포함.
