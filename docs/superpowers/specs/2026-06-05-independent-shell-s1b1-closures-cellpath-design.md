# S1b-1 — 클로저·셀경로·each·scalar where 설계

> **작성일**: 2026-06-05 · 브레인스토밍 산출.
> **상위**: 독립 AI-네이티브 셸. S1a(비교/불리언 표현식 + where 행조건) 위에 클로저/셀경로/`each`/스칼라 필터를 얹는다.
> **상태**: 계획. 본 스펙은 **S1b-1**을 구현 가능 수준으로 명세한다. S1b-2/S1b-3은 개요만.

---

## 0. S1b 분해

| 슬라이스 | 내용 |
|---------|------|
| **S1b-1**(본 스펙) | 블록/클로저 `{...}`(`$it`) + 셀경로 `$it.field`/`$it.0` + `each` + 스칼라 리스트 `where` + eval `it` 일반화 + S1a forward 노트 |
| S1b-2 | `sort-by` · `select` · `range` · 리스트 슬라이스 |
| S1b-3 | 산술 연산자 `+ - * /` (S1a 이연 — 플래그/경로/글롭/소수 충돌의 본격 설계) |

순서 근거: 클로저·셀경로가 `each`/스칼라필터의 토대; 데이터 명령은 그 위; 산술은 직교·충돌위험이라 마지막.

---

## 1. S1b-1

### 1.1 블록 vs 레코드 (파서 휴리스틱)
`parse_atom`이 `{`를 만나면 lookahead:
- 다음이 `Word`/`Str` **그리고** 그 다음이 `:` → **레코드 리터럴**(기존 `parse_record`).
- 빈 `{}` → 빈 레코드.
- 그 외 → **블록**: 단일 표현식(`parse_expr`)을 `}`까지 읽어 `Expr::Block(Box<Expr>)`.

예: `{a: 1}`·`{name: $x}`=레코드, `{ $it.size }`·`{ $it > 1 }`=블록.

### 1.2 블록 → Closure 값
`eval_expr(Expr::Block(e))` → `Value::Closure(Box<Expr>)`(블록 내부 식 캡처). 환경 캡처 없음 — 적용 시점의 엔진 스코프를 쓴다(클로저는 정의 직후 같은 스코프에서 적용되므로 충분). `value.rs`:
- `Value` 에 `Closure(Box<Expr>)` 추가(파생 `Debug/Clone/PartialEq` 유지 — `Box<Expr>`는 모두 파생됨).
- `type_name` → `"closure"`. `coerce_string`·`format_value` → `"<closure>"`.
- `value.rs`가 `ast::Expr`를 import(값→AST 단방향, 순환 없음).

### 1.3 셀경로 `$it.field` / `$it.0` (충돌 없는 렉싱)
`.`을 일반 토큰화하면 소수(`3.5`)·경로(`./src`)와 충돌하므로, **`$` 변수 렉서가 `$ident(.seg)*`를 한 토큰으로** 읽는다(seg=ident 또는 정수). 예 `$it.size.0` → `Token::Var("it.size.0")`. `.`은 일반 토큰이 아니다(충돌 0).
파서: `Token::Var(s)`를 `.`로 분해 — 경로 없으면 `Expr::Var(base)`, 있으면 `Expr::CellPath { base: String, segs: Vec<CellSeg> }`.
```
enum CellSeg { Field(String), Index(usize) }   // seg가 전부 숫자면 Index, 아니면 Field
```

### 1.4 eval `it` 일반화 (engine)
S1a의 `eval_expr(e, engine, row: Option<&OrderedMap>)`를 **`eval_expr(e, engine, it: Option<&Value>)`**로 바꾼다:
- `Expr::Var("it")`: `it`가 `Some(v)`면 `v.clone()`, 아니면 스코프 변수(없으면 에러).
- `Expr::Var(other)`: 스코프 변수.
- `Expr::Word(w)`: `it`가 `Some(Value::Record(r))`면 그 필드(없으면 에러) — `where size > 100` 유지; 그 외엔 `String(w)`(S0).
- `Expr::CellPath { base, segs }`: base(`Var` 규칙)로 시작값 평가 후 segs 순회 — `Field`는 Record 필드(없으면 에러), `Index`는 List 인덱스(범위 밖 에러), 타입 불일치 에러.
- `Expr::Block(e)`: `Value::Closure(e.clone())`.
- 기존 호출부(Stage::Expr/Command 인자/리스트/레코드/Binary/Unary 자식)는 `it` 전파(최상위는 `None`).
- `pub fn eval_closure(block: &Expr, it: &Value, engine: &mut Engine) -> Result<Value>` = `eval_expr(block, engine, Some(it))` (빌트인에서 호출).

### 1.5 `each` (builtins)
`fn b_each(args, input, engine)`: `args[0]`이 `Value::Closure(block)`이 아니면 에러; 입력이 `List`가 아니면 에러; 각 원소 `elem`에 `engine::eval_closure(block, &elem, engine)?`를 적용해 결과 `List` 반환. `lookup`에 `"each"` 추가. 예 `[{name: a}{name: b}] | each { $it.name }` → `["a","b"]`.

### 1.6 `where` 일반화 (engine eval_pipeline)
`Stage::Where(cond)`: 입력 List의 각 원소 `elem`에 `eval_expr(cond, engine, Some(&elem))` → Bool 아니면 에러, true면 보존. **비-Record 행 에러 제거**(스칼라 허용). → `[1 2 3] | where $it > 1` → `[2,3]`, `ls | where size > 100` 유지(레코드 맨이름=필드). 비-List 입력은 여전히 에러.

### 1.7 S1a forward 노트 흡수
- 연쇄 비교 파서: `parse_cmp`의 `while` → `if`(비교는 1회만; `1 < 2 < 3`은 파스 단계에서 거부해 조기 에러). 스펙 §1.2의 "연쇄 허용"을 "비연쇄"로 정정.
- 테스트 보강: 이중부정 `not not x`, 셀경로 없는 필드 에러.
- (S1a의 비-Record 행 테스트는 §1.6에서 스칼라 허용으로 동작이 바뀌므로 해당 음성 케이스 제거.)

### 1.8 파일 (순수 Rust · C-free · 새 의존성 0)
```
수정: src/shellcore/value.rs    — Value::Closure + type_name/coerce/format 처리 + use ast::Expr
      src/shellcore/ast.rs      — Expr::Block, Expr::CellPath, enum CellSeg
      src/shellcore/lexer.rs    — $ 변수에 .세그먼트 흡수
      src/shellcore/parser.rs   — { 레코드/블록 휴리스틱, Var→CellPath 분해, parse_cmp while→if
      src/shellcore/engine.rs   — eval_expr row→it, Block/CellPath, eval_closure, where it-바인딩
      src/shellcore/builtins.rs — each + lookup
      src/shellcore/format.rs   — Value::Closure arm
```
> enum 변형 추가가 match를 깨므로 ast/value/format/engine 의 관련 변경은 한 task로 원자 적용(아래 plan에서 슬라이싱).

### 1.9 테스트 (TDD)
- **lexer**: `$it.size.0`→`Var("it.size.0")`, `$x`→`Var("x")`, `3.5`/`./src`/`a/b` 영향 없음.
- **parser**: `{a: 1}`→Record · `{ $it }`→Block · `$it.size`→CellPath(Field) · `$x.0`→CellPath(Index) · `1 < 2 < 3` 파스 에러 · `not not true` 파스.
- **value/format**: Closure `type_name`="closure", `format_value`→"<closure>".
- **engine(eval_line)**:
  - `[{name: a} {name: b}] | each { $it.name }` → `["a","b"]`
  - `[{a: {b: 9}}] | each { $it.a.b }` → `[9]` (중첩 셀경로)
  - `[1 2 3] | where $it > 1` → `[2,3]` (스칼라 필터)
  - `[{size: 50} {size: 200}] | where size > 100` → 1행 (레코드 유지)
  - `[{v: 1}] | each { $it.bad }` → 에러 (없는 필드)
  - `[10 20 30] | each { $it } | ...` / 인덱스 경로 동작
  - `each` non-closure 인자/비-List 입력 → 에러
- 각 task 검증에 **`cargo fmt --check` 포함**(S1a 정렬 누락 갭 교정).
- 검증: default `cargo test shellcore`, fmt/clippy(`--all-targets -D warnings`) clean, 기존 전체 무회귀.

### 1.10 비목표 (S1b-1)
`sort-by`/`select`/`range`/슬라이스(S1b-2) · 산술 `+ - * /`(S1b-3) · 다중 파라미터 클로저(`{|a b| }`) · 클로저 변수 저장·재사용(즉시 적용만) · `(expr).field` 일반 후위 셀경로(현재 `$var` 접두만) · 환경 캡처 클로저.

---

## 2. 시퀀싱
S1b-1(클로저/셀경로/each/scalar where) → S1b-2(sort-by/select/range/슬라이스) → S1b-3(산술). 각 슬라이스 독립 동작·테스트. S1b-1 완료 시 `ls | each { $it.name }`·`ls | where size > 1000`·`[1 2 3] | where $it > 1`가 실동작.
