# S1a — 표현식(비교·불리언) + `where` 행 조건 설계

> **작성일**: 2026-06-05 · 브레인스토밍 산출.
> **상위**: 독립 AI-네이티브 셸(피벗). S0 코어(`2026-06-05-independent-shell-s0-core-design.md`) 위에 표현식(비교/불리언)과 첫 술어 명령 `where`를 얹는다.
> **상태**: 계획. 본 스펙은 **S1a**를 구현 가능 수준으로 명세한다. S1b/S1c는 개요만.

---

## 0. S1 분해

S1(구조화 데이터 명령)은 여러 서브시스템이라 슬라이스로 나눈다:

| 슬라이스 | 내용 |
|---------|------|
| **S1a**(본 스펙) | 비교(`== != < <= > >=`)·불리언(`and or not`) 연산자 + 우선순위 파서 + `where` 행 조건 + Float 비교 의미 확정 + S0 리뷰 cleanup |
| S1b | `each`(클로저/`$it`)·`sort-by`·`select`·`range` + 셀경로 `.field`/인덱스 + 스칼라 리스트 필터 + **산술 연산자**(`+ - * /`, 아래 결정 참조) |
| S1c | 외부 stdout 캡처 모드 + `lines`/`split` + `from json` (텍스트↔구조화 브리지) |

순서 근거: 비교·불리언·술어가 S1b 데이터 명령의 토대. 외부 캡처는 별도 관심사라 마지막.

**산술 연산자를 S1a에서 제외한 이유(중요)**: `+ - * /`는 셸 토큰과 정면 충돌한다 — `-`는 플래그(`-rf`)·음수, `/`는 경로(`./src`,`a/b`), `.`는 소수/경로, `*`는 글롭(`*.txt`). 이를 모호성 없이 처리하려면 인접성(공백) 기반 문맥 렉싱이 필요해 비용이 크다. 반면 **비교·불리언 연산자 문자(`= ! < >`)는 경로/플래그/숫자에 등장하지 않아 충돌이 없다.** `where` 술어 목표(`where size > 100`)는 비교+불리언만으로 100% 충족되므로, 산술은 충돌을 제대로 설계할 수 있는 S1b로 미룬다(YAGNI + 모호성 제거).

---

## 1. S1a — 비교·불리언 표현식 + `where`

### 1.1 문법 분리 (atom vs expr)
연산자가 명령 인자를 깨지 않도록 표현식 진입점을 둘로 나눈다:
- **`parse_atom`** (= S0 `parse_expr` 개명): 리터럴 / `$var` / 바레워드(`Word`) / 리스트 `[…]` / 레코드 `{…}` / `( … )`. **명령 인자는 이것만** 사용 → `ls -rf foo`의 `-rf`·경로가 그대로 `Word`로 남는다(연산자 미적용).
- **`parse_expr`** (신규, 우선순위 등반): `parse_atom`을 피연산자로 이항/단항 연산자 결합. **표현식 위치에서만** 사용 — `let name = <expr>`, 선두 표현식 스테이지, `where` 조건, `( <expr> )` 내부.

### 1.2 연산자 + 우선순위 (낮음→높음)
| 단계 | 연산자 | 비고 |
|------|--------|------|
| or | `or` | 단축평가 |
| and | `and` | 단축평가 |
| 비교 | `== != < <= > >=` | 비연쇄(좌결합) |
| 단항(prefix) | `not` | |
그룹화는 `( … )`. **S1a 비포함**: 산술 `+ - * /`(→S1b), `%`/비트, 연쇄 비교(`a < b < c`), `++`.

### 1.3 토큰 (lexer 확장)
신규 토큰: `EqEq(==)` `NotEq(!=)` `Lt(<)` `Le(<=)` `Gt(>)` `Ge(>=)`. 키워드 `and`/`or`/`not`(`classify_word`에서 키워드화).
- **2글자 우선**: `==`/`!=`/`<=`/`>=`를 먼저 매칭, 아니면 1글자 `<`/`>`. 기존 `Equals(=)`(let 전용)는 유지하되, `=` 다음이 `=`면 `EqEq`.
- **충돌 없음**: 연산자 문자 `= ! < >`를 바레워드 문자집합에서 제외(SPECIAL에 `! < >` 추가; `=`는 이미 special). `+ - * / . / ~` 등 다른 바레워드 문자는 **S0 그대로** → 플래그(`-rf`)·경로(`./src`,`a/b`)·소수(`3.5`)는 변경 없이 `Word`/`Int`/`Float`로 렉싱된다. (재결합 같은 특수처리 불필요.)
- `!`는 `!=`에서만 의미를 가진다. 단독 `!`(부정)는 없음 — 부정은 키워드 `not`.

### 1.4 파서 (`parse_atom` / `parse_expr` / `where`)
- **명령 스테이지 인자**: `parse_atom` 반복(S0와 동일, 연산자 토큰을 만나면 인자 끝). 명령 인자엔 연산자가 없다.
- **`parse_expr`**(우선순위 등반/Pratt): 1.2 우선순위로 `parse_atom` 피연산자를 결합. 단항 `not` 접두. `( <expr> )` 그룹.
- **`where` 특수형**: 스테이지 선두 Word가 `where`면 나머지를 `parse_expr` **하나**로 읽어 `Stage::Where(Expr)`(신규 AST 노드)로 저장(즉시평가 인자 경로 미사용). (`where`를 파서에서 하드코딩 — MVP. 일반화는 후속 명령 시그니처로.)
- `let name = <parse_expr>` · 선두 비-Word 스테이지 = `Stage::Expr(parse_expr)`.

### 1.5 AST 확장 (`ast.rs`)
```
enum BinOp { Eq, Ne, Lt, Le, Gt, Ge, And, Or }
enum UnOp  { Not }
Expr::Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> }
Expr::Unary  { op: UnOp, expr: Box<Expr> }
Stage::Where(Expr)
```

### 1.6 연산자 의미 (`ops.rs` 신규 — 순수 `apply_binary`/`apply_unary`)
- **`==`/`!=`**: 임의 타입 구조 동등. 타입 다르면 `==`=false / `!=`=true(에러 아님). Float은 IEEE(`NaN != NaN`) → **S0 Float 비교 의미 확정**(파생 `PartialEq` 유지, NaN 의미는 IEEE로 문서화).
- **순서 `< <= > >=`**: 숫자(Int↔Float 승격)·String(사전순)만. 비교 불가 타입 조합→타입 에러. NaN 개입→false 반환.
- **불리언 `and or not`**: 피연산자 Bool 강제(아니면 타입 에러). `and`/`or` 단축평가. **암묵 truthiness 없음**(비-Bool은 에러).
- 결과는 항상 `Bool`(비교/불리언) — S1a 연산자는 모두 Bool 산출.

### 1.7 엔진 (`engine.rs`)
- `eval_expr(expr, engine, row: Option<&OrderedMap>)`로 시그니처 확장(행 컨텍스트 전달; 일반 호출은 `None`). 기존 호출부는 `None` 전달로 갱신.
  - `Expr::Word(w)`: `row`가 `Some(rec)`면 **그 행의 필드 `w`**(없으면 에러); `None`이면 S0대로 `String(w)`.
  - `Expr::Var(n)`: 항상 스코프 변수(`row` 무관). → `where size > $limit` 지원.
  - `Expr::Binary`: `and`/`or`는 단축평가(좌 평가 후 필요시 우), 그 외 양변 평가 → `ops::apply_binary`. 자식에 행 컨텍스트 전파.
  - `Expr::Unary(Not, e)`: 평가 → `ops::apply_unary`.
  - 기타(리터럴/리스트/레코드/Sub)는 S0 + 행 컨텍스트 전파.
- `eval_pipeline`: `Stage::Where(cond)` 처리 — 입력이 `List`가 아니면 에러; 각 원소가 `Record`가 아니면 에러; 각 행에 `eval_expr(cond, engine, Some(rec))` → 결과가 `Bool`이 아니면 에러, `true`인 행만 모아 `List` 반환.

### 1.8 S0 리뷰 cleanup (요청 포함)
- `b_first`: 인자가 음수면 에러(`first: 음수 불가`). (`*n as usize` 래핑 제거.)
- `b_print`: 인자 여러 개면 공백으로 join 출력(Nushell 호환). 인자 없으면 `$in`.
- `home_dir`: `util.rs`에 `pub fn home_dir() -> Option<PathBuf>` 단일 정의, `builtins`(cd)·`repl` 공용. builtins의 `PathBuf` 폴백판 제거.
- Float 비교: §1.6 비교 연산자 의미로 해소(추가 작업 없음).

### 1.9 파일 (순수 Rust · C-free · 새 의존성 0)
```
신규: src/shellcore/ops.rs   — BinOp/UnOp apply (순수)
      src/shellcore/util.rs  — home_dir
수정: src/shellcore/lexer.rs   — 비교 토큰(== != <= >= < >) + and/or/not 키워드 + SPECIAL에 ! < > 추가
      src/shellcore/ast.rs     — BinOp/UnOp, Expr::Binary/Unary, Stage::Where
      src/shellcore/parser.rs  — parse_expr→parse_atom 개명 + parse_expr(우선순위) + where 특수형
      src/shellcore/engine.rs  — eval_expr 행컨텍스트 + Binary/Unary + Stage::Where
      src/shellcore/builtins.rs— first 가드 · print 다중인자 · home_dir 위임
      src/shellcore/repl.rs    — util::home_dir 사용
```

### 1.10 테스트 (TDD)
- **lexer**: `==`/`!=`/`<=`/`>=` 2글자 토큰, `<`/`>` 1글자, `and/or/not` 키워드, `-rf`/`./src`/`a/b`/`3.5` 바레워드·숫자 유지(연산자 영향 없음), `let x = 1`의 `=`는 `Equals`.
- **parser**: `size > 100` → `Binary(Gt, Word(size), Int(100))`, 우선순위(`a == 1 and b == 2` → `And(Eq.., Eq..)`), `not a == b` 단항, `( )` 그룹, `where type == "dir"` → `Stage::Where(...)`, 명령 인자 `ls -rf`는 `Word("-rf")`(연산자 미적용).
- **ops**: `==`/`!=`(숫자/문자열/타입불일치 false·NaN), 순서(숫자·문자열·비교불가 에러·NaN false), 불리언(Bool 강제·`and`/`or` 단축평가·`not`).
- **engine(eval_line)**:
  - `[{size: 50} {size: 200}] | where size > 100` → `[{size: 200}]`
  - `[{type: "dir"} {type: "file"}] | where type == "dir"` → 1행(dir)
  - `let limit = 100` 후 `[{size: 200}] | where size > $limit` → 1행
  - `[{a: 1} {a: 2}] | where a == 1 or a == 2` → 2행
  - 비-Bool 조건(`where size`) → 에러
  - 없는 필드(`where nope > 1`) → 에러
  - 비-List 입력(`5 | where x > 1`) → 에러
- **cleanup**: `[1 2 3] | first -1` 에러 · `print 1 2 3` 공백 join · home_dir 단일화 동작.
- 검증: default `cargo test`, fmt/clippy(`--all-targets -D warnings`) clean, 기존 287 테스트 무회귀.

### 1.11 비목표 (S1a)
산술 `+ - * /`(→S1b) · `$it`·셀경로 `.field`/인덱스(→S1b) · `each`/`sort-by`/`select`/`range`(→S1b) · 외부 stdout 캡처·`from json`/`lines`(→S1c) · 문자열 concat · `%`/비트/연쇄비교 · 스칼라 리스트 `$it` 필터 · 암묵 truthiness.

---

## 2. 시퀀싱
S1a(비교/불리언+where) → S1b(each/sort-by/select/range + 셀경로 + 산술) → S1c(외부 캡처 + 텍스트→구조화). 각 슬라이스는 그 자체로 동작·테스트 가능. S1a 완료 시 `ls | where size > 1000`·`ls | where type == "dir"`가 실동작한다.
