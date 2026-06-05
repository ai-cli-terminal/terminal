# S1a — 비교·불리언 표현식 + `where` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** S0 셸 위에 비교(`== != < <= > >=`)·불리언(`and or not`) 연산자와 우선순위 파서를 얹고 `where` 행 조건(`ls | where size > 1000`, `ls | where type == "dir"`)을 구현한다. + S0 리뷰 cleanup.

**Architecture:** 표현식 진입점을 atom(명령 인자용, 연산자 없음)과 expr(우선순위 등반, 표현식 위치 전용)로 분리. `where`는 파서 특수형 `Stage::Where(Expr)`로, 엔진이 행마다 행-컨텍스트(맨이름=필드)로 평가. 연산자 의미는 순수 `ops.rs`. enum 변형 추가가 match를 깨지 않도록 ast 변형+파서+엔진을 한 task로 원자화.

**Tech Stack:** Rust(2021) · anyhow · std only. C-free, 새 의존성 0. 정본: `docs/superpowers/specs/2026-06-05-independent-shell-s1a-expressions-design.md`.

---

## 실행 환경 메모 (공통)
- 빌드/테스트는 WSL 단일라인: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`. 멀티라인 금지.
- 파일은 Read/Write/Edit(Windows 경로). 커밋은 브랜치 `feat/independent-shell-s1a`에 명시적 `git add`(절대 `git add -A` 금지), 메시지 끝에 `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. push 안 함.
- 우선순위(낮음→높음): `or` < `and` < `not`(prefix) < 비교. `not`은 비교보다 느슨 → `not a == b` = `not (a == b)`.

---

## Task 1: 렉서 — 비교 연산자 토큰 + and/or/not 키워드

**Files:** Modify `src/shellcore/lexer.rs`

- [ ] **Step 1: 실패 테스트 추가** — `src/shellcore/lexer.rs` 테스트 모듈에 추가:
```rust
    #[test]
    fn tokenizes_comparison_and_boolean() {
        use Token::*;
        assert_eq!(lex("size > 100").unwrap(), vec![Word("size".into()), Gt, Int(100)]);
        assert_eq!(
            lex("a == 1 and b != 2 or not c").unwrap(),
            vec![Word("a".into()), EqEq, Int(1), And, Word("b".into()), NotEq, Int(2), Or, Not, Word("c".into())]
        );
        assert_eq!(lex("x <= 1 >= y < z >").unwrap(), vec![
            Word("x".into()), Le, Int(1), Ge, Word("y".into()), Lt, Word("z".into()), Gt
        ]);
    }

    #[test]
    fn equals_vs_eqeq_and_paths_unaffected() {
        use Token::*;
        assert_eq!(lex("let x = 1").unwrap(), vec![Let, Word("x".into()), Equals, Int(1)]);
        // 경로/플래그/소수는 연산자 영향 없음(바레워드/숫자 유지).
        assert_eq!(lex("ls -rf ./src").unwrap(), vec![Word("ls".into()), Word("-rf".into()), Word("./src".into())]);
        assert_eq!(lex("3.5").unwrap(), vec![Float(3.5)]);
    }
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::lexer 2>&1 | tail -15'`
  Expected: 컴파일 에러(`Gt`/`EqEq`/`And`… 미정의).

- [ ] **Step 3: 구현** —
(a) `Token` enum에 변형 추가(기존 변형 뒤):
```rust
    EqEq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
```
(b) `SPECIAL` 상수에 `'!'`, `'<'`, `'>'` 추가(나머지 유지):
```rust
const SPECIAL: &[char] = &[
    '|', ';', '[', ']', '{', '}', '(', ')', ':', ',', '=', '#', '"', '\'', '$', '!', '<', '>',
];
```
(c) `lex`의 `'=' => { … }` 아크를 2글자 우선으로 교체하고, `!`/`<`/`>` 아크 추가(`'#'` 아크 등 기존 single-char 아크들 곁에):
```rust
            '=' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Token::EqEq);
                    i += 2;
                } else {
                    out.push(Token::Equals);
                    i += 1;
                }
            }
            '!' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Token::NotEq);
                    i += 2;
                } else {
                    bail!("예상치 못한 '!' (부정은 `not` 키워드)");
                }
            }
            '<' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Token::Le);
                    i += 2;
                } else {
                    out.push(Token::Lt);
                    i += 1;
                }
            }
            '>' => {
                if chars.get(i + 1) == Some(&'=') {
                    out.push(Token::Ge);
                    i += 2;
                } else {
                    out.push(Token::Gt);
                    i += 1;
                }
            }
```
(d) `classify_word`의 키워드 매치에 추가(`"let"`/`"true"`/… 곁에):
```rust
        "and" => return Token::And,
        "or" => return Token::Or,
        "not" => return Token::Not,
```

- [ ] **Step 4: 통과 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::lexer 2>&1 | tail -8'`
  Expected: 모든 lexer 테스트 통과(기존 + 신규 2).

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/lexer.rs && git commit -m "feat(shell): 비교 연산자 토큰 + and/or/not 키워드 (S1a T1)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: ops.rs(연산자 의미) + util.rs(home_dir) + BinOp/UnOp enum

**Files:** Create `src/shellcore/ops.rs`, `src/shellcore/util.rs`; Modify `src/shellcore/ast.rs`(enum 추가), `src/shellcore/mod.rs`

- [ ] **Step 1: 실패 테스트** — `src/shellcore/ops.rs` 하단:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::ast::BinOp;
    use crate::shellcore::value::Value;

    #[test]
    fn equality_across_types_and_floats() {
        assert_eq!(apply_compare(BinOp::Eq, &Value::Int(1), &Value::Int(1)).unwrap(), Value::Bool(true));
        assert_eq!(apply_compare(BinOp::Ne, &Value::Int(1), &Value::Int(2)).unwrap(), Value::Bool(true));
        // 타입 불일치 = not equal (에러 아님)
        assert_eq!(apply_compare(BinOp::Eq, &Value::Int(1), &Value::String("1".into())).unwrap(), Value::Bool(false));
        // Int/Float 혼합 동등
        assert_eq!(apply_compare(BinOp::Eq, &Value::Int(2), &Value::Float(2.0)).unwrap(), Value::Bool(true));
        // NaN != NaN
        assert_eq!(apply_compare(BinOp::Eq, &Value::Float(f64::NAN), &Value::Float(f64::NAN)).unwrap(), Value::Bool(false));
    }

    #[test]
    fn ordering_numbers_and_strings_and_errors() {
        assert_eq!(apply_compare(BinOp::Gt, &Value::Int(200), &Value::Int(100)).unwrap(), Value::Bool(true));
        assert_eq!(apply_compare(BinOp::Le, &Value::Float(1.5), &Value::Int(2)).unwrap(), Value::Bool(true));
        assert_eq!(apply_compare(BinOp::Lt, &Value::String("a".into()), &Value::String("b".into())).unwrap(), Value::Bool(true));
        // 비교 불가 타입 → 에러
        assert!(apply_compare(BinOp::Lt, &Value::Bool(true), &Value::Int(1)).is_err());
        // NaN 개입 → false (에러 아님)
        assert_eq!(apply_compare(BinOp::Gt, &Value::Float(f64::NAN), &Value::Int(1)).unwrap(), Value::Bool(false));
    }

    #[test]
    fn as_bool_strict() {
        assert!(as_bool(&Value::Bool(true)).unwrap());
        assert!(as_bool(&Value::Int(1)).is_err());
    }
}
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::ops 2>&1 | tail -12'`
  Expected: 컴파일 에러(`BinOp`/`apply_compare`/`as_bool` 미정의).

- [ ] **Step 3: 구현** —
(a) `src/shellcore/ast.rs`에 enum 2개 추가(파일 상단 또는 적당한 위치):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
}
```
(b) `src/shellcore/ops.rs`(테스트 위):
```rust
//! 연산자 의미: 비교(==,!=,<,<=,>,>=) 적용 + 불리언 강제. 순수 함수.
//! and/or 단축평가·not 은 엔진에서 처리(여기선 as_bool 제공).

use std::cmp::Ordering;

use anyhow::{bail, Result};

use crate::shellcore::ast::BinOp;
use crate::shellcore::value::Value;

/// 비교 연산(Eq/Ne/Lt/Le/Gt/Ge)을 적용해 Bool 을 반환한다.
pub fn apply_compare(op: BinOp, lhs: &Value, rhs: &Value) -> Result<Value> {
    let b = match op {
        BinOp::Eq => values_equal(lhs, rhs),
        BinOp::Ne => !values_equal(lhs, rhs),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => match compare_order(lhs, rhs)? {
            None => false, // NaN 개입
            Some(o) => match op {
                BinOp::Lt => o == Ordering::Less,
                BinOp::Le => o != Ordering::Greater,
                BinOp::Gt => o == Ordering::Greater,
                BinOp::Ge => o != Ordering::Less,
                _ => unreachable!(),
            },
        },
        BinOp::And | BinOp::Or => unreachable!("불리언은 엔진에서 단축평가"),
    };
    Ok(Value::Bool(b))
}

// Float 동등은 IEEE 의미(NaN != NaN)가 의도된 동작 — clippy float_cmp 허용.
#[allow(clippy::float_cmp)]
fn values_equal(a: &Value, b: &Value) -> bool {
    use Value::*;
    match (a, b) {
        (Int(x), Int(y)) => x == y,
        (Float(x), Float(y)) => x == y, // IEEE: NaN != NaN
        (Int(x), Float(y)) | (Float(y), Int(x)) => (*x as f64) == *y,
        (Bool(x), Bool(y)) => x == y,
        (Str(x), Str(y)) => x == y,
        (Nothing, Nothing) => true,
        (List(x), List(y)) => x == y,
        (Record(x), Record(y)) => x == y,
        _ => false, // 타입 불일치 = not equal
    }
}

fn compare_order(a: &Value, b: &Value) -> Result<Option<Ordering>> {
    use Value::*;
    let r = match (a, b) {
        (Int(x), Int(y)) => x.partial_cmp(y),
        (Float(x), Float(y)) => x.partial_cmp(y),
        (Int(x), Float(y)) => (*x as f64).partial_cmp(y),
        (Float(x), Int(y)) => x.partial_cmp(&(*y as f64)),
        (Str(x), Str(y)) => x.partial_cmp(y),
        _ => bail!("비교할 수 없는 타입: {} 와 {}", a.type_name(), b.type_name()),
    };
    Ok(r)
}

/// 불리언 컨텍스트에서 Bool 을 강제한다(암묵 truthiness 없음).
pub fn as_bool(v: &Value) -> Result<bool> {
    match v {
        Value::Bool(b) => Ok(*b),
        other => bail!("bool 이 필요합니다 ({})", other.type_name()),
    }
}
```
(c) `src/shellcore/util.rs`:
```rust
//! 공용 유틸.

use std::path::PathBuf;

/// 홈 디렉터리(HOME → USERPROFILE). 둘 다 없으면 None.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
```
(d) `src/shellcore/mod.rs`에 추가: `pub mod ops;` 와 `pub mod util;`.

- [ ] **Step 4: 통과 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::ops 2>&1 | tail -8'`
  Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/ops.rs src/shellcore/util.rs src/shellcore/ast.rs src/shellcore/mod.rs && git commit -m "feat(shell): 연산자 의미(ops) + util.home_dir + BinOp/UnOp (S1a T2)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: AST 변형 + 파서(우선순위) + 엔진(행조건) — 키스톤

**Files:** Modify `src/shellcore/ast.rs`, `src/shellcore/parser.rs`, `src/shellcore/engine.rs`

> enum 변형 추가가 match exhaustiveness 를 깨므로 ast+parser+engine 을 한 task로 원자 적용한다. `Expr::Sub`(미사용 sub-pipeline)를 제거하고 `(…)`를 표현식 그룹으로 바꾼다.

- [ ] **Step 1: 실패 테스트** —
`src/shellcore/parser.rs` 테스트 모듈에 추가:
```rust
    #[test]
    fn parses_where_and_precedence() {
        // where 는 Stage::Where
        let stmts = p("ls | where size > 100");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        assert_eq!(pl.stages.len(), 2);
        let Stage::Where(cond) = &pl.stages[1] else { panic!("where 기대") };
        assert_eq!(
            *cond,
            Expr::Binary {
                op: BinOp::Gt,
                lhs: Box::new(Expr::Word("size".into())),
                rhs: Box::new(Expr::Int(100)),
            }
        );
        // 우선순위: a == 1 and b == 2 → And(Eq, Eq)
        let stmts = p("where a == 1 and b == 2");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        let Stage::Where(c) = &pl.stages[0] else { panic!() };
        assert!(matches!(c, Expr::Binary { op: BinOp::And, .. }));
        // not 은 비교보다 느슨: not a == b → Not(Eq)
        let stmts = p("where not a == b");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        let Stage::Where(c) = &pl.stages[0] else { panic!() };
        assert!(matches!(c, Expr::Unary { op: UnOp::Not, expr } if matches!(**expr, Expr::Binary { op: BinOp::Eq, .. })));
    }

    #[test]
    fn command_args_have_no_operators() {
        // 명령 인자는 atom — `-rf` 는 Word, 연산자 비적용
        let stmts = p("ls -rf");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        let Stage::Command(c) = &pl.stages[0] else { panic!() };
        assert_eq!(c.name, "ls");
        assert_eq!(c.args, vec![Expr::Word("-rf".into())]);
    }
```
`src/shellcore/engine.rs` 테스트 모듈에 추가:
```rust
    #[test]
    fn where_filters_table_rows() {
        let mut e = Engine::new();
        assert_eq!(
            eval_line("[{size: 50} {size: 200}] | where size > 100", &mut e).unwrap(),
            Value::List(vec![{
                let mut r = crate::shellcore::value::OrderedMap::new();
                r.insert("size", Value::Int(200));
                Value::Record(r)
            }])
        );
        // type == "dir" 필터 → 1행
        let out = eval_line("[{type: \"dir\"} {type: \"file\"}] | where type == \"dir\" | length", &mut e).unwrap();
        assert_eq!(out, Value::Int(1));
        // 변수 비교
        eval_line("let limit = 100", &mut e).unwrap();
        let out = eval_line("[{size: 200}] | where size > $limit | length", &mut e).unwrap();
        assert_eq!(out, Value::Int(1));
        // or
        let out = eval_line("[{a: 1} {a: 2} {a: 3}] | where a == 1 or a == 3 | length", &mut e).unwrap();
        assert_eq!(out, Value::Int(2));
    }

    #[test]
    fn where_errors_are_clean() {
        let mut e = Engine::new();
        assert!(eval_line("[{size: 1}] | where size", &mut e).is_err()); // 비-Bool 조건
        assert!(eval_line("[{size: 1}] | where nope > 0", &mut e).is_err()); // 없는 필드
        assert!(eval_line("5 | where x > 1", &mut e).is_err()); // 비-List 입력
    }

    #[test]
    fn comparison_expression_value() {
        let mut e = Engine::new();
        assert_eq!(eval_line("3 > 2", &mut e).unwrap(), Value::Bool(true));
        assert_eq!(eval_line("\"a\" < \"b\"", &mut e).unwrap(), Value::Bool(true));
    }
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore 2>&1 | tail -15'`
  Expected: 컴파일 에러(`Stage::Where`/`Expr::Binary`/`Expr::Unary` 미정의).

- [ ] **Step 3a: AST 변형** — `src/shellcore/ast.rs`:
- `Expr` enum에서 `Sub(Box<Pipeline>)` 변형을 **삭제**하고, 다음 두 변형을 추가:
```rust
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Unary {
        op: UnOp,
        expr: Box<Expr>,
    },
```
- `Stage` enum에 추가:
```rust
    Where(Expr),
```

- [ ] **Step 3b: 파서** — `src/shellcore/parser.rs`:
- 기존 메서드 `fn parse_expr` 를 **`fn parse_atom` 으로 개명**한다. 그 안의 `LParen` 아크를 그룹으로 교체(Sub 제거):
```rust
            Some(Token::LParen) => {
                let e = self.parse_expr()?;
                match self.next() {
                    Some(Token::RParen) => Ok(e),
                    other => bail!("')' 기대, got {other:?}"),
                }
            }
```
  그리고 `parse_atom` 내부에서 list/record 요소를 읽는 호출은 `self.parse_atom()` 으로 둔다(개명 반영; 아래 parse_list/parse_record 참고).
- `parse_list`/`parse_record` 내부의 `self.parse_expr()` 호출을 `self.parse_atom()` 으로 변경(요소는 atom).
- 새 우선순위 메서드 추가(impl 블록 안):
```rust
    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or()
    }
    fn parse_or(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_and()?;
        while matches!(self.peek(), Some(Token::Or)) {
            self.next();
            let rhs = self.parse_and()?;
            lhs = Expr::Binary { op: BinOp::Or, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }
    fn parse_and(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_not()?;
        while matches!(self.peek(), Some(Token::And)) {
            self.next();
            let rhs = self.parse_not()?;
            lhs = Expr::Binary { op: BinOp::And, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }
    fn parse_not(&mut self) -> Result<Expr> {
        if matches!(self.peek(), Some(Token::Not)) {
            self.next();
            let expr = self.parse_not()?;
            Ok(Expr::Unary { op: UnOp::Not, expr: Box::new(expr) })
        } else {
            self.parse_cmp()
        }
    }
    fn parse_cmp(&mut self) -> Result<Expr> {
        let mut lhs = self.parse_atom()?;
        while let Some(op) = self.peek().and_then(cmp_op) {
            self.next();
            let rhs = self.parse_atom()?;
            lhs = Expr::Binary { op, lhs: Box::new(lhs), rhs: Box::new(rhs) };
        }
        Ok(lhs)
    }
```
- `impl Parser` 블록 밖(파일 끝, 테스트 위)에 자유 함수 추가:
```rust
fn cmp_op(t: &Token) -> Option<BinOp> {
    Some(match t {
        Token::EqEq => BinOp::Eq,
        Token::NotEq => BinOp::Ne,
        Token::Lt => BinOp::Lt,
        Token::Le => BinOp::Le,
        Token::Gt => BinOp::Gt,
        Token::Ge => BinOp::Ge,
        _ => return None,
    })
}
```
- `parse_stage` 를 교체(where 특수형 + 명령 인자는 parse_atom):
```rust
    fn parse_stage(&mut self) -> Result<Stage> {
        let is_where = matches!(self.peek(), Some(Token::Word(w)) if w == "where");
        if is_where {
            self.next();
            return Ok(Stage::Where(self.parse_expr()?));
        }
        if let Some(Token::Word(_)) = self.peek() {
            let name = match self.next() {
                Some(Token::Word(w)) => w,
                _ => unreachable!(),
            };
            let mut args = Vec::new();
            while !self.at_stage_end() {
                args.push(self.parse_atom()?);
            }
            Ok(Stage::Command(Command { name, args }))
        } else {
            Ok(Stage::Expr(self.parse_expr()?))
        }
    }
```
- import 확인: `use crate::shellcore::ast::*;` 가 이미 있으므로 `BinOp`/`UnOp`/`Expr`/`Stage` 모두 가시(ast 의 `*`). `use crate::shellcore::lexer::Token;` 도 기존대로.

- [ ] **Step 3c: 엔진** — `src/shellcore/engine.rs`:
- import 추가: 상단 `use anyhow::{bail, Result};` 를 `use anyhow::{anyhow, bail, Result};` 로, 그리고 `use crate::shellcore::{builtins, external, lexer, parser};` 에 `ops` 추가 → `use crate::shellcore::{builtins, external, lexer, ops, parser};`.
- `eval_pipeline` 의 `for stage` match 에 `Stage::Where` 아크 추가하고 `Stage::Expr`/`Stage::Command` 의 `eval_expr` 호출에 `None` 인자 추가:
```rust
        input = match stage {
            Stage::Expr(e) => eval_expr(e, engine, None)?,
            Stage::Where(cond) => {
                let items = match input {
                    Value::List(items) => items,
                    other => bail!("where: 리스트(테이블)가 아닙니다 ({})", other.type_name()),
                };
                let mut kept = Vec::new();
                for it in items {
                    let keep = {
                        let rec = match &it {
                            Value::Record(r) => r,
                            other => bail!("where: 테이블 행이 아닙니다 ({})", other.type_name()),
                        };
                        ops::as_bool(&eval_expr(cond, engine, Some(rec))?)?
                    };
                    if keep {
                        kept.push(it);
                    }
                }
                Value::List(kept)
            }
            Stage::Command(c) => {
                let args: Vec<Value> = c
                    .args
                    .iter()
                    .map(|a| eval_expr(a, engine, None))
                    .collect::<Result<_>>()?;
                if let Some(b) = builtins::lookup(&c.name) {
                    b(&args, input, engine)?
                } else {
                    external::run(&c.name, &args, engine)?
                }
            }
        };
```
- `eval_expr` 를 행 컨텍스트 + Binary/Unary 지원 + Sub 제거로 **전체 교체**:
```rust
fn eval_expr(e: &Expr, engine: &mut Engine, row: Option<&OrderedMap>) -> Result<Value> {
    use crate::shellcore::ast::{BinOp, UnOp};
    Ok(match e {
        Expr::Int(n) => Value::Int(*n),
        Expr::Float(f) => Value::Float(*f),
        Expr::Str(s) => Value::String(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Null => Value::Nothing,
        Expr::Word(w) => match row {
            Some(rec) => rec
                .get(w)
                .cloned()
                .ok_or_else(|| anyhow!("필드를 찾을 수 없습니다: {w}"))?,
            None => Value::String(w.clone()),
        },
        Expr::Var(name) => match engine.vars.get(name) {
            Some(v) => v.clone(),
            None => bail!("변수를 찾을 수 없습니다: ${name}"),
        },
        Expr::List(items) => {
            let vals: Vec<Value> = items
                .iter()
                .map(|x| eval_expr(x, engine, row))
                .collect::<Result<_>>()?;
            Value::List(vals)
        }
        Expr::Record(pairs) => {
            let mut rec = OrderedMap::new();
            for (k, x) in pairs {
                rec.insert(k.clone(), eval_expr(x, engine, row)?);
            }
            Value::Record(rec)
        }
        Expr::Binary { op, lhs, rhs } => match op {
            BinOp::And => {
                if !ops::as_bool(&eval_expr(lhs, engine, row)?)? {
                    Value::Bool(false)
                } else {
                    Value::Bool(ops::as_bool(&eval_expr(rhs, engine, row)?)?)
                }
            }
            BinOp::Or => {
                if ops::as_bool(&eval_expr(lhs, engine, row)?)? {
                    Value::Bool(true)
                } else {
                    Value::Bool(ops::as_bool(&eval_expr(rhs, engine, row)?)?)
                }
            }
            _ => {
                let l = eval_expr(lhs, engine, row)?;
                let r = eval_expr(rhs, engine, row)?;
                ops::apply_compare(*op, &l, &r)?
            }
        },
        Expr::Unary { op, expr } => match op {
            UnOp::Not => Value::Bool(!ops::as_bool(&eval_expr(expr, engine, row)?)?),
        },
    })
}
```

- [ ] **Step 4: 통과 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore 2>&1 | grep -E "test result|FAILED" | tail; cargo clippy --all-targets -- -D warnings 2>&1 | tail -2'`
  Expected: 모든 shellcore 테스트 통과(parser 신규 2 + engine 신규 3 포함), clippy clean.

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/ast.rs src/shellcore/parser.rs src/shellcore/engine.rs && git commit -m "feat(shell): 표현식 연산자 + where 행조건 (S1a T3)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: S0 리뷰 cleanup (first 가드 · print 다중인자 · home_dir 단일화)

**Files:** Modify `src/shellcore/builtins.rs`, `src/shellcore/repl.rs`

- [ ] **Step 1: 실패 테스트** — `src/shellcore/builtins.rs` 테스트 모듈에 추가:
```rust
    #[test]
    fn first_rejects_negative() {
        let mut e = Engine::new();
        assert!(eval_line("[1 2 3] | first -1", &mut e).is_err());
        // 0 은 허용(빈 리스트)
        assert_eq!(eval_line("[1 2 3] | first 0", &mut e).unwrap(), Value::List(vec![]));
    }

    #[test]
    fn print_joins_multiple_args() {
        // 다중 인자가 패닉/에러 없이 동작(공백 join). 반환은 Nothing.
        let mut e = Engine::new();
        assert_eq!(eval_line("print 1 2 3", &mut e).unwrap(), Value::Nothing);
    }
```
> 참고: `first -1` 은 렉서에서 `Minus` 가 아니라… S1a 렉서는 `-` 를 바레워드 문자로 유지하므로 `-1` 은 `Word("-1")`? 아니다 — `-1` 은 `classify_word` 에서 `i64` 파싱 성공 → `Int(-1)`. 따라서 `first -1` → `first` 명령 + `Int(-1)` 인자. (명령 인자는 atom, 음수 정수 리터럴 OK.)

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::builtins 2>&1 | tail -12'`
  Expected: `first_rejects_negative` 실패(현재 `-1` 이 usize 래핑되어 전체 리스트 반환 → `is_err()` 거짓).

- [ ] **Step 3: 구현** — `src/shellcore/builtins.rs`:
(a) `b_first` 의 `n` 산출을 음수 가드로 교체:
```rust
fn b_first(args: &[Value], input: Value, _e: &mut Engine) -> Result<Value> {
    let n = match args.first() {
        Some(Value::Int(n)) => {
            if *n < 0 {
                bail!("first: 음수 불가: {n}");
            }
            *n as usize
        }
        Some(other) => bail!("first: 정수 필요 ({})", other.type_name()),
        None => 1,
    };
    match input {
        Value::List(items) => Ok(Value::List(items.into_iter().take(n).collect())),
        other => bail!("first: 리스트가 아님 ({})", other.type_name()),
    }
}
```
(b) `b_print` 를 다중 인자 공백 join 으로 교체:
```rust
fn b_print(args: &[Value], input: Value, _e: &mut Engine) -> Result<Value> {
    if args.is_empty() {
        println!("{}", format_value(&input));
    } else {
        let parts: Vec<String> = args.iter().map(format_value).collect();
        println!("{}", parts.join(" "));
    }
    Ok(Value::Nothing)
}
```
(c) `home_dir` 단일화: `builtins.rs` 의 로컬 `fn home_dir() -> std::path::PathBuf { … }` 정의를 **삭제**하고, `b_cd` 의 `None => home_dir()` 호출을 `None => crate::shellcore::util::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."))` 로 변경.

- [ ] **Step 4: repl home_dir 단일화** — `src/shellcore/repl.rs`:
- 로컬 `fn home_dir() -> Option<PathBuf> { … }` 정의를 **삭제**하고, `run()` 안의 `let home = home_dir();` 를 `let home = crate::shellcore::util::home_dir();` 로 변경. (`use std::path::{Path, PathBuf};` 는 `make_prompt` 시그니처가 계속 쓰므로 유지.)

- [ ] **Step 5: 통과 + 회귀 확인**
Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore 2>&1 | grep -E "test result|FAILED" | tail; cargo clippy --all-targets -- -D warnings 2>&1 | tail -2; cargo fmt --all -- --check >/dev/null 2>&1 && echo FMT_OK || echo FMT_FAIL'`
Expected: 모든 shellcore 테스트 통과, clippy clean, FMT_OK. (FMT_FAIL 이면 `cargo fmt --all` 적용 후 변경 파일을 이 커밋에 포함.)

- [ ] **Step 6: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/builtins.rs src/shellcore/repl.rs && git commit -m "fix(shell): first 음수 가드 · print 다중인자 · home_dir 단일화 (S1a T4)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## 최종 검증 (전체 task 완료 후)
- [ ] shellcore 전체: `cargo test shellcore 2>&1 | grep "test result"` → 모두 ok.
- [ ] 무회귀: `cargo test --features "storage tls remote" 2>&1 | grep -E "test result|FAILED" | tail` → lib 287 + S1a 신규, 0 failed.
- [ ] fmt/clippy: `cargo fmt --all -- --check` + `cargo clippy --all-targets -- -D warnings`.
- [ ] ash 스모크: `printf '[{size: 50} {size: 200}] | where size > 100\nexit\n' | cargo run --bin ash` → size 200 행만 표로 출력.
- [ ] DoD 대조(스펙 §1): 연산자 토큰(T1)·연산자 의미/util(T2)·표현식·where 행조건(T3)·cleanup(T4) 전부 커버.

> 이후: S1b(each/sort-by/select/range + 셀경로 `.field` + 산술 연산자). 별도 spec→plan.
