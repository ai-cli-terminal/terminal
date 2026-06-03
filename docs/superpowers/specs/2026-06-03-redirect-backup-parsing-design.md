# 리다이렉트 인식 백업 대상 추출 설계

> 작성일: 2026-06-03 · 중앙 실행 파이프라인 백로그(리뷰 LOW 보완)
> 정본: `2026-06-03-central-execution-pipeline-design.md` §4-4(백업 트리거), §31.6.

## 1. 배경 / 문제

`src/pipeline.rs`의 `backup_targets`가 셸 리다이렉트 대상을 제대로 잡지 못한다.

- 트리거가 `command.contains('>')` — 인용/산술 `>`에도 오탐.
- `candidate_paths`가 공백 분리 후 **정확히** `>`/`>>` 토큰만 제외 → 붙은 형태 `>out.txt`는 토큰으로 남고 `is_file(">out.txt")`=false → **실제 대상 `out.txt`가 백업되지 않음**(덮어쓰기 무백업, 조용한 갭).

## 2. 범위

타겟팅 휴리스틱(문자열 수준, **새 의존성 0**, C-free 유지). 인용/이스케이프 완전 구분은 의도적으로 이연(shell-words 토크나이저 영역).

## 3. 설계

### 새 순수 함수 2개

```rust
/// 토큰이 리다이렉트 연산자로 시작하면 연산자 뒤 나머지(대상; 분리형이면 "")를 반환.
/// 인식: 선택적 fd 접두(`[0-9]*` 또는 단일 `&`) + `>` + 선택적 `>`(append).
fn strip_redirect_op(tok: &str) -> Option<&str>;
//  ">out"→Some("out")  "2>err"→Some("err")  "&>all"→Some("all")  "2>>log"→Some("log")
//  ">"/">>"/"2>"→Some("")(분리형)  "123"→None  "-i"→None  "a=b"→None  "file"→None

/// 리다이렉트 대상 파일명들을 추출. 붙은 형태는 토큰에서, 분리형(`> f`)은 다음 토큰에서.
fn redirect_targets(toks: &[&str]) -> Vec<String>;
```

`strip_redirect_op` 알고리즘(바이트 스캔):
1. `j=0`. `bytes[0]=='&'`면 `j=1`; 아니면 선행 ASCII 숫자들을 건너뛴다.
2. `bytes[j] != '>'`면 `None`.
3. `'>'` 소비(`j+=1`), 다음이 또 `'>'`면 append로 한 번 더 소비.
4. `Some(&tok[j..])` 반환(나머지가 비면 분리형 신호).

`redirect_targets`: 토큰을 순회하며 `strip_redirect_op` 적용. 나머지가 비어 있지 않으면 그 값을 대상으로, 비어 있으면(분리형) 다음 토큰을 대상으로 삼고 인덱스를 하나 더 전진.

### `backup_targets` 재작성

`command.contains('>')` 트리거 제거. **삭제/덮어쓰기 프로그램 인자 경로 ∪ 리다이렉트 대상**을 dedup 후 기존 일반 파일만:

```rust
fn backup_targets(command: &str) -> Vec<PathBuf> {
    let toks: Vec<&str> = command.split_whitespace().collect();
    let prog = program_token(&toks);
    let in_place = matches!(prog, Some("sed") | Some("perl"))
        && toks.iter().any(|t| t.starts_with("-i"));
    let prog_backupable = matches!(
        prog,
        Some("rm") | Some("unlink") | Some("shred")
            | Some("cp") | Some("mv") | Some("tee") | Some("touch")
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

### `candidate_paths` 필터 조정

하드코딩된 `> >>` 제외를 `strip_redirect_op(t).is_none()`로 교체(리다이렉트 토큰은 `redirect_targets`가 전담). `| && ;` 제외는 유지:

```rust
it.filter(|t| {
    !t.starts_with('-')
        && !t.chars().all(|c| c.is_ascii_digit())
        && !t.contains('=')
        && !matches!(*t, "|" | "&&" | ";")
        && strip_redirect_op(t).is_none()
})
```

## 4. 동작 결과

- `echo hi >out.txt`(붙음) / `cmd > out.txt`(분리) / `cmd 2>err.txt` / `cmd >>log.txt` → 기존 대상 파일 백업 후 실행.
- 미존재 파일로 리다이렉트 → `is_file` 필터로 백업 없음, 명령 실행(복구할 원본 없음).
- `>`를 연산자 경계에서만 인식 → `echo "a>b"`(공백 없는 인용 내 `>`)는 트리거 안 함.

## 5. 한계(문서화)

공백으로 분리된 인용 내 `>`(예: `echo "a > b"`)는 여전히 리다이렉트로 오인 가능. 단 `is_file` 필터로 무해(존재하는 동명 파일이 있을 때만 불필요 백업, 최악의 경우 한도 초과 시 불필요한 BackupRefused). 완전 정확성은 shell-words 토크나이저 영역 — 이연.

## 6. 테스트 (TDD)

순수 함수 단위 + 통합(전부 mock, PTY 불필요):
- `strip_redirect_op`: 붙음/분리/fd/append/`&>`/비리다이렉트(숫자·플래그·일반·`=`) 케이스.
- `redirect_targets`: 붙은 대상, 분리형 다음 토큰, 다중 리다이렉트, 대상 없는 분리형(`cmd >`)은 빈 결과.
- `backup_targets` 통합: 붙은/분리 리다이렉트 대상 백업 생성·복구 라운드트립, 미존재 파일 무백업, 비리다이렉트 명령 빈 결과, chmod 여전히 제외.
- 기존 7개 파이프라인 테스트 회귀 없음. clippy(default+storage)·fmt clean.

## 7. 영향 파일

`src/pipeline.rs`만: `backup_targets`/`candidate_paths` 수정 + `strip_redirect_op`/`redirect_targets` 추가 + 테스트.
