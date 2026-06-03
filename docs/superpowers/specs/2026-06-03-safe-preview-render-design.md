# 설계: W9 안전 미리보기 — 실행 없는 diff / content-at-risk

> 날짜: 2026-06-03 · 핸드오프 백로그 ③ 중 W9(안전 서브셋) · 관련: W9 Preview/Diff 엔진(§31.5)

## 문제

`src/preview.rs`의 `classify_preview`는 preview 전략(`DryRun`/`TempCopyDiff`/`ListTargets`/
`NotAvailable`/`NotNeeded`)을 **분류만** 하고 실제 내용(diff·사라질 내용)을 보여주지 않는다.
실제 diff 생성은 명령 실행이 필요해 샌드박스(§31.11, Phase 2+)에 의존(보류). 그러나 **파일을
읽기만 해도 안전하게** 미리보기 가능한 고가치 케이스가 있다.

## 범위 (명령 실행 0, 읽기 전용 — 샌드박스 불필요)

1. **`cp src dst` / `mv src dst` (dst가 기존 일반 파일)** → 진짜 **unified diff**(before=dst,
   after=src). 두 파일을 읽기만 한다.
2. **`rm`/`shred`/`unlink` <files>** → 각 기존 대상의 **content-at-risk 요약**(행 수·바이트·head N줄).
3. **리다이렉트 덮어쓰기 `> file` (기존 파일, append `>>` 제외)** → content-at-risk(현재 내용 교체).
4. **그 외** → 기존 분류 메시지를 `Info`로 전달:
   - `DryRun(cmd)` → 제안 명령 안내(기존 동작).
   - `NotAvailable(reason)` → 사유.
   - `NotNeeded` → 변경 없음.
   - `chmod`/`chown`/`chgrp` → 권한 변경(내용 손실 없음) → 대상 목록 안내.
   - **sed -i / perl -i / formatter(prettier/black/gofmt/ruff)** → "실제 diff는 명령 실행 필요 →
     샌드박스 후속(보류)" 명시.

## 비목표

- 명령 실행이 필요한 실제 diff(sed -i/perl -i/formatter, tee, 임의 명령) — 샌드박스 후속.
- 외부 상태(서비스/패키지/docker/네트워크/DB) — 기존대로 불가.
- diff 알고리즘 최적화(Myers 등 대용량) — LCS + 크기 상한으로 충분.

## 구조 (관심사 분리)

### `src/diff.rs` (신규) — 순수 unified diff

```rust
/// 두 텍스트의 라인 단위 unified diff 문자열을 만든다(LCS 기반, 결정적).
/// 동일하면 빈 문자열. `---`/`+++` 헤더 + `-`/`+`/공백(context) 라인.
pub fn unified_diff(before: &str, after: &str, before_label: &str, after_label: &str) -> String
```

- **in-house LCS 라인 diff** — 새 의존성 없음(프로젝트 미니멀 dep 기조 유지). DP-LCS로 공통
  부분수열을 구해 삭제(`-`)/추가(`+`)/유지(context) 라인을 출력한다. 대용량은 호출측 상한으로 보호.
- 순수 함수 → 단위 테스트 용이.

### `src/preview.rs` (확장) — 실제 렌더링

```rust
/// 안전 미리보기 렌더 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewRender {
    /// cp/mv 덮어쓰기 unified diff.
    Diff(String),
    /// 삭제/truncate로 사라질 내용 요약.
    ContentAtRisk { path: String, lines: usize, bytes: u64, head: String },
    /// 분류 전략 안내(dry-run/external/chmod/sed-i 보류/not-needed).
    Info(String),
}

/// 안전(실행 없는) 미리보기를 생성한다. rm 다중 대상 등으로 여러 항목일 수 있다.
pub fn render_preview(command: &str) -> Vec<PreviewRender>
```

- `classify_preview`(순수, **불변 유지**)를 재사용해 분기:
  - `TempCopyDiff` 경로에서 cp/mv를 재파싱 → dst가 기존 일반 파일이면 `unified_diff(read(dst),
    read(src), dst, src)` → `Diff`. dst가 없으면 `Info`("새 파일 생성, diff 없음"). sed -i류·tee면
    `Info`(보류 사유).
  - 리다이렉트 `> file`(append 아님, 기존 파일) → `ContentAtRisk`.
  - `ListTargets`에서 rm/shred/unlink 대상 → 각 기존 파일 `ContentAtRisk`; chmod류 → `Info`(목록).
  - `DryRun`/`NotAvailable`/`NotNeeded` → `Info`.
- **안전장치**: 파일 크기 상한(상수, 예 1 MiB) 초과 시 `Info`("파일이 커 미리보기 생략"); 비-UTF8은
  `from_utf8_lossy`; 경로가 없거나 디렉터리면 `Info`/건너뜀. 어떤 경우에도 **대상 파일을 수정하지 않는다**.

### `src/main.rs` — `ai preview` 강화

`ai preview "<cmd>"`가 현재 `classify_preview` 전략만 출력 → `render_preview` 결과를 포맷 출력
(Diff는 그대로, ContentAtRisk는 "삭제 예정: path (N줄, M bytes)\n<head>", Info는 메시지).
기존 분류 정보는 Info로 유지돼 회귀 없음.

## 테스트

- **diff.rs**(순수): 동일 입력 → 빈 문자열; 한 줄 변경 → `-old`/`+new` 포함; 추가/삭제만; 라벨이
  헤더에 반영.
- **preview.rs**(temp 파일 — `undo.rs`/`pipeline.rs` 패턴): 
  - `cp a b`(b 기존, 내용 다름) → `Diff` 1개, 변경 라인 포함.
  - `cp a b`(b 미존재) → `Info`.
  - `rm f`(기존) → `ContentAtRisk{lines, bytes, head}`; 다중 대상 → 여러 항목.
  - `echo x > f`(f 기존) → `ContentAtRisk`.
  - `sed -i s/a/b/ f` → `Info`(보류 사유 포함).
  - 크기 상한 초과/디렉터리/미존재 → `Info`.
- **main**(스모크): `ai preview` 파싱·출력 유지(기존 테스트 불변).

## 데이터 흐름

`ai preview "<cmd>"` → `render_preview(cmd)` → (`classify_preview` 분기 + 안전 파일 읽기 +
`diff::unified_diff`) → `Vec<PreviewRender>` → main이 포맷 출력. 대상 파일은 절대 변경하지 않음.
