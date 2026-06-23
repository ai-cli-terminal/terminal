# 리다이렉트 인식 백업 대상 추출 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**목표:** `backup_targets`가 셸 리다이렉트(`>f`/`>>f`/`N>f`/`&>f`/`> f`) 대상을 인식해 덮어쓰기 전 기존 파일을 백업하도록 한다.

**Architecture:** 새 순수 함수 `strip_redirect_op`/`redirect_targets`로 리다이렉트 대상을 추출하고, `backup_targets`는 (삭제/덮어쓰기 프로그램 인자 경로 ∪ 리다이렉트 대상)을 dedup 후 기존 일반 파일만 백업. `command.contains('>')` 거친 트리거 제거. 새 의존성 0(C-free 유지).

**기술 스택:** Rust, 기존 `src/pipeline.rs`. 빌드·검증은 WSL.

설계 정본: `docs/superpowers/specs/2026-06-03-redirect-backup-parsing-design.md`.

---

## File Structure

- **Modify** `src/pipeline.rs` — `strip_redirect_op`/`redirect_targets` 추가, `backup_targets`/`candidate_paths` 수정, 테스트 추가.
- **Modify** `docs/HISTORY.md` — 백로그 보완 항목 기록.

## 검증 환경 (메모리 `terminal-build-env`)

모든 cargo는 WSL에서 단일 라인으로(멀티라인 금지):
```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test pipeline::'
```
이하 `실행:`은 `cargo ...` 부분만 적는다. git은 Windows에서 직접 실행.

---

## 작업 1: 리다이렉트 파싱 순수 함수

**Files:**
- Modify: `src/pipeline.rs` (헬퍼 함수 영역 + tests)

- [ ] **단계 1: 실패 테스트를 추가한다**

`src/pipeline.rs`의 `mod tests`에 추가:

```rust
    #[test]
    fn strip_redirect_op_recognizes_forms() {
        assert_eq!(strip_redirect_op(">out"), Some("out"));
        assert_eq!(strip_redirect_op(">>log"), Some("log"));
        assert_eq!(strip_redirect_op("2>err"), Some("err"));
        assert_eq!(strip_redirect_op("&>all"), Some("all"));
        assert_eq!(strip_redirect_op("2>>log"), Some("log"));
        assert_eq!(strip_redirect_op(">"), Some(""));
        assert_eq!(strip_redirect_op(">>"), Some(""));
        assert_eq!(strip_redirect_op("2>"), Some(""));
        assert_eq!(strip_redirect_op("123"), None);
        assert_eq!(strip_redirect_op("-i"), None);
        assert_eq!(strip_redirect_op("a=b"), None);
        assert_eq!(strip_redirect_op("file"), None);
    }

    #[test]
    fn redirect_targets_extracts_attached_and_detached() {
        assert_eq!(
            redirect_targets(&["echo", "hi", ">out.txt"]),
            vec!["out.txt".to_string()]
        );
        assert_eq!(
            redirect_targets(&["cmd", ">", "out.txt"]),
            vec!["out.txt".to_string()]
        );
        assert_eq!(
            redirect_targets(&["cmd", "2>err", ">>log"]),
            vec!["err".to_string(), "log".to_string()]
        );
        assert!(redirect_targets(&["cmd", ">"]).is_empty());
        assert!(redirect_targets(&["ls", "-al"]).is_empty());
    }
```

- [ ] **단계 2: 실패를 확인한다**

실행: `cargo test pipeline::tests::strip_redirect_op_recognizes_forms`
기대: FAIL — `strip_redirect_op`/`redirect_targets` 미정의(컴파일 에러).

- [ ] **단계 3: 두 함수를 구현한다**

`src/pipeline.rs`의 헬퍼 영역(`candidate_paths` 아래)에 추가:

```rust
/// 토큰이 리다이렉트 연산자로 시작하면 연산자 뒤 나머지(대상; 분리형이면 "")를 반환한다.
/// 인식: 선택적 fd 접두(`[0-9]*` 또는 단일 `&`) + `>` + 선택적 `>`(append).
fn strip_redirect_op(tok: &str) -> Option<&str> {
    let bytes = tok.as_bytes();
    let mut j = 0;
    // 선택적 fd 접두: 단일 '&' 또는 숫자들
    if j < bytes.len() && bytes[j] == b'&' {
        j += 1;
    } else {
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
    }
    // 반드시 '>' 가 와야 한다
    if j >= bytes.len() || bytes[j] != b'>' {
        return None;
    }
    j += 1;
    // append '>>'
    if j < bytes.len() && bytes[j] == b'>' {
        j += 1;
    }
    Some(&tok[j..])
}

/// 리다이렉트 대상 파일명들을 추출한다. 붙은 형태는 토큰에서, 분리형(`> f`)은 다음 토큰에서.
fn redirect_targets(toks: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if let Some(rest) = strip_redirect_op(toks[i]) {
            if !rest.is_empty() {
                out.push(rest.to_string());
            } else if i + 1 < toks.len() {
                out.push(toks[i + 1].to_string());
                i += 1;
            }
        }
        i += 1;
    }
    out
}
```

- [ ] **단계 4: 통과를 확인한다**

실행: `cargo test pipeline::tests::strip_redirect_op_recognizes_forms pipeline::tests::redirect_targets_extracts_attached_and_detached`
기대: PASS (둘 다). (이 단계에서 두 함수는 아직 `backup_targets`에서 호출되지 않아 dead_code 경고가 날 수 있으나 작업 2에서 사용된다 — clippy는 작업 2 후 실행.)

- [ ] **단계 5: fmt 후 커밋**

실행: `cargo fmt --all`
```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): add redirect operator parsing helpers"
```

---

## 작업 2: backup_targets 재배선 + candidate_paths 조정

**Files:**
- Modify: `src/pipeline.rs` (`backup_targets`, `candidate_paths`, tests)

- [ ] **단계 1: 실패 통합 테스트를 추가한다**

`mod tests`에 추가:

```rust
    #[test]
    fn backup_targets_picks_up_redirect_overwrite() {
        let work = tmp("rt");
        std::fs::create_dir_all(&work).unwrap();
        let f = work.join("out.txt");
        std::fs::write(&f, "x").unwrap();
        let cmd = format!("echo hi >{}", f.display());
        let t = backup_targets(&cmd);
        assert!(t.contains(&f), "attached redirect target missing: {t:?}");
        let cmd2 = format!("echo hi > {}", f.display());
        let t2 = backup_targets(&cmd2);
        assert!(t2.contains(&f), "detached redirect target missing: {t2:?}");
    }

    #[test]
    fn backup_targets_skips_new_redirect_file_and_chmod() {
        let work = tmp("rt2");
        std::fs::create_dir_all(&work).unwrap();
        let missing = work.join("new.txt");
        let cmd = format!("echo hi >{}", missing.display());
        assert!(
            backup_targets(&cmd).is_empty(),
            "new file should not be backed up"
        );
        let existing = work.join("e.txt");
        std::fs::write(&existing, "x").unwrap();
        let cmd2 = format!("chmod 755 {}", existing.display());
        assert!(
            backup_targets(&cmd2).is_empty(),
            "chmod must be excluded: {:?}",
            backup_targets(&cmd2)
        );
    }
```

- [ ] **단계 2: 실패를 확인한다**

실행: `cargo test pipeline::tests::backup_targets_picks_up_redirect_overwrite`
기대: FAIL — 현재 `backup_targets`는 붙은 `>out.txt` 토큰을 `is_file(">out.txt")`로 걸러 대상이 비어 `t.contains(&f)`가 거짓.

- [ ] **단계 3: `backup_targets`를 재작성한다**

`src/pipeline.rs`의 기존 `backup_targets` 함수 전체를 다음으로 교체:

```rust
/// 백업 대상 파일을 산출한다. 삭제/덮어쓰기/in-place 편집 명령의 인자 경로와
/// 리다이렉트 대상 중 **기존 일반 파일**만. 권한 변경(chmod/chown/chgrp)은
/// 내용 백업이 무의미하므로 제외한다.
fn backup_targets(command: &str) -> Vec<PathBuf> {
    let toks: Vec<&str> = command.split_whitespace().collect();
    let prog = program_token(&toks);
    let in_place =
        matches!(prog, Some("sed") | Some("perl")) && toks.iter().any(|t| t.starts_with("-i"));
    let prog_backupable = matches!(
        prog,
        Some("rm")
            | Some("unlink")
            | Some("shred")
            | Some("cp")
            | Some("mv")
            | Some("tee")
            | Some("touch")
    );

    let mut cands: Vec<String> = Vec::new();
    if prog_backupable || in_place {
        cands.extend(candidate_paths(&toks));
    }
    cands.extend(redirect_targets(&toks));

    let mut seen = std::collections::HashSet::new();
    cands
        .into_iter()
        .filter(|c| seen.insert(c.clone()))
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .collect()
}
```

- [ ] **단계 4: `candidate_paths`의 리다이렉트 제외를 조정한다**

`candidate_paths`의 `.filter(|t| { ... })` 클로저를 다음으로 교체(리다이렉트 토큰은 `redirect_targets`가 전담하므로 여기서 제외):

```rust
    it.filter(|t| {
        !t.starts_with('-')
            && !t.chars().all(|c| c.is_ascii_digit())
            && !t.contains('=')
            && !matches!(*t, "|" | "&&" | ";")
            && strip_redirect_op(t).is_none()
    })
    .map(String::from)
    .collect()
```

- [ ] **단계 5: 전체 통과를 확인한다**

실행: `cargo test pipeline::`
기대: PASS — 기존 7개 + 신규 4개 = 11개 모두 통과.

- [ ] **단계 6: clippy + fmt clean**

실행: `cargo clippy --all-targets -- -D warnings`
기대: 경고 0.
실행: `cargo clippy --all-targets --features storage -- -D warnings`
기대: 경고 0.
실행: `cargo fmt --all -- --check`
기대: 차이 없음.

- [ ] **단계 7: 커밋**

```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): back up redirect targets, drop coarse '>' trigger"
```

---

## 작업 3: WSL e2e + HISTORY 기록

**Files:**
- Modify: `docs/HISTORY.md`

- [ ] **단계 1: WSL e2e — 리다이렉트 덮어쓰기 백업 확인**

다음을 ONE single-line WSL 명령으로 실행:
```bash
wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/d/workspace/terminal-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo build --features storage && BIN=$CARGO_TARGET_DIR/debug/ai; D=$(mktemp -d); echo original > $D/f.txt; SHELL=/bin/bash $BIN exec "echo overwritten > $D/f.txt" --yes; echo "after=$(cat $D/f.txt)"; $BIN undo last; echo "restored=$(cat $D/f.txt)"'
```
기대: stderr에 `(백업 생성: undo_...)`, `after=overwritten`, undo 후 `restored=original`. (붙은 형태도 동일하게 동작.) 실제 출력을 캡처. 기대와 다르면 STOP·BLOCKED 보고(코드를 억지로 고치지 말 것).

- [ ] **단계 2: `docs/HISTORY.md`에 항목 추가**

최상단(`---` 다음, 가장 최근 항목 위)에 추가:

```markdown
## 2026-06-03 — 그룹 C 백로그: 리다이렉트 인식 백업 대상 (W10 보완)

- **pipeline**(`pipeline.rs`): `strip_redirect_op`/`redirect_targets` 추가 — 셸 리다이렉트(`>f`/`>>f`/`N>f`/`&>f`/`> f`) 대상을 추출. `backup_targets`가 (삭제/덮어쓰기 프로그램 인자 ∪ 리다이렉트 대상)을 dedup 후 기존 일반 파일만 백업. `command.contains('>')` 거친 트리거 제거 → 붙은 `>out.txt`도 정확히 백업.
- **이유**: 기존엔 `echo x >out.txt`의 대상이 `is_file(">out.txt")`로 걸러져 덮어쓰기 전 백업이 안 됨(조용한 갭). 리뷰 LOW 보완.
- **한계**: 공백 분리된 인용 내 `>`(`echo "a > b"`)는 여전히 오인 가능하나 `is_file` 필터로 무해. 완전 정확성은 shell-words 토크나이저 영역 — 이연.
- 검증: TDD(strip_redirect_op/redirect_targets 단위 + backup_targets 통합 4), WSL e2e(`echo > f` 덮어쓰기→백업→`undo last` 복구). pipeline 11 + 전체 통과, clippy(default+storage)·fmt clean.
```

- [ ] **단계 3: 커밋**

```bash
git add docs/HISTORY.md
git commit -m "docs: record redirect-aware backup target extraction"
```

---

## 자기검토 메모

- **스펙 커버리지**: §3 두 함수=작업 1, §3 backup_targets 재작성=작업 2 단계 3, §3 candidate_paths 조정=작업 2 단계 4, §6 테스트=작업 1·2, §5 한계=HISTORY/작업 3.
- **타입 일관성**: `strip_redirect_op(&str)->Option<&str>`, `redirect_targets(&[&str])->Vec<String>`, `backup_targets(&str)->Vec<PathBuf>` — 작업 1 정의와 작업 2 호출 일치. `tmp()` 헬퍼는 기존 tests 모듈에 존재(재사용).
- **플레이스홀더**: 없음(모든 코드 블록 실제 내용).
