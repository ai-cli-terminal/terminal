# S1b-1 — 클로저·셀경로·each·scalar where Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 독립 셸에 블록/클로저 `{...}`($it), 셀경로 `$it.field`/`$it.0`, `each`, 스칼라 리스트 `where`를 추가해 `ls | each { $it.name }`·`[1 2 3] | where $it > 1`을 동작하게 한다.

**Architecture:** `$` 변수 렉서가 셀경로를 한 토큰으로 흡수(점 충돌 회피). `{...}`는 `{Word:` 휴리스틱으로 레코드/블록 구분; 블록은 `Value::Closure(Box<Expr>)`. `eval_expr`의 행 컨텍스트를 `row: Option<&OrderedMap>` → `it: Option<&Value>`로 일반화해 스칼라·레코드 모두 지원. `each`는 `engine::eval_closure`로 원소별 적용.

**Tech Stack:** Rust(2021) · anyhow · std only. C-free, 새 의존성 0. 정본: `docs/superpowers/specs/2026-06-05-independent-shell-s1b1-closures-cellpath-design.md`.

---

## 실행 환경 메모 (공통)
- 빌드/테스트 WSL 단일라인: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; <cmd>'`. 멀티라인 금지.
- 파일은 Read/Write/Edit. 커밋은 브랜치 `feat/independent-shell-s1b1`, 명시적 `git add`, 메시지 끝에 `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. push 안 함.
- **각 task 검증 단계에 `cargo fmt --all -- --check` 포함**(S1a에서 누락됐던 갭 교정). FMT_FAIL이면 `cargo fmt --all` 적용 후 그 파일들도 같은 커밋에 포함.

---

## Task 1: 렉서 — `$` 변수에 셀경로 흡수

**Files:** Modify `src/shellcore/lexer.rs`

- [ ] **Step 1: 실패 테스트** — lexer 테스트 모듈에 추가:
```rust
    #[test]
    fn var_absorbs_cell_path() {
        use Token::*;
        assert_eq!(lex("$it").unwrap(), vec![Var("it".into())]);
        assert_eq!(lex("$it.size.0").unwrap(), vec![Var("it.size.0".into())]);
        assert_eq!(lex("$x.name").unwrap(), vec![Var("x.name".into())]);
        // 셀경로는 $var 한정 — 소수/경로 바레워드는 영향 없음
        assert_eq!(lex("3.5").unwrap(), vec![Float(3.5)]);
        assert_eq!(lex("./src").unwrap(), vec![Word("./src".into())]);
        // 점 뒤가 비-식별자면 셀경로 종료
        assert_eq!(lex("$x .y").unwrap(), vec![Var("x".into()), Word(".y".into())]);
    }
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::lexer 2>&1 | tail -12'` → `$it.size.0` 가 `Var("it")` + 이후 분리 토큰으로 나와 실패.

- [ ] **Step 3: 구현** — `lex`의 `'$' => { … }` 아크를 셀경로 흡수로 교체:
```rust
            '$' => {
                i += 1;
                let start = i;
                while i < chars.len() && is_word_char(chars[i]) {
                    i += 1;
                }
                if start == i {
                    bail!("$ 뒤에 변수 이름이 필요합니다");
                }
                // 셀경로: `.세그먼트`(식별자/정수)를 변수 토큰에 흡수한다($var 한정 — `.` 충돌 회피).
                while i < chars.len()
                    && chars[i] == '.'
                    && chars.get(i + 1).is_some_and(|c| is_word_char(*c))
                {
                    i += 1; // '.'
                    while i < chars.len() && is_word_char(chars[i]) {
                        i += 1;
                    }
                }
                let name: String = chars[start..i].iter().collect();
                out.push(Token::Var(name));
            }
```

- [ ] **Step 4: 통과 + fmt 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::lexer 2>&1 | tail -6; cargo fmt --all -- --check >/dev/null 2>&1 && echo FMT_OK || echo FMT_FAIL'` → lexer 통과 + FMT_OK.

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/lexer.rs && git commit -m "feat(shell): $ 변수에 셀경로 흡수 (S1b-1 T1)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: AST/Value/format/parser/engine — 블록·셀경로·it 일반화·scalar where (키스톤)

**Files:** Modify `src/shellcore/ast.rs`, `src/shellcore/value.rs`, `src/shellcore/format.rs`, `src/shellcore/parser.rs`, `src/shellcore/engine.rs`

> enum 변형(`Expr::Block`/`Expr::CellPath`/`Value::Closure`) 추가가 exhaustive match 를 깨므로 한 task로 원자 적용.

- [ ] **Step 1: 실패 테스트** —
`src/shellcore/parser.rs` 테스트 모듈에 추가:
```rust
    #[test]
    fn brace_record_vs_block_and_cellpath() {
        // {Word:} → 레코드
        let stmts = p("{a: 1}");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        assert!(matches!(pl.stages[0], Stage::Expr(Expr::Record(_))));
        // { expr } → 블록
        let stmts = p("{ $it }");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        assert!(matches!(pl.stages[0], Stage::Expr(Expr::Block(_))));
        // $it.size → CellPath(Field)
        let stmts = p("$it.size");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        assert!(matches!(
            &pl.stages[0],
            Stage::Expr(Expr::CellPath { base, segs }) if base == "it" && segs.len() == 1
        ));
        // $x.0 → CellPath(Index)
        let stmts = p("$x.0");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        let Stage::Expr(Expr::CellPath { segs, .. }) = &pl.stages[0] else { panic!() };
        assert_eq!(segs[0], CellSeg::Index(0));
    }

    #[test]
    fn chained_comparison_rejected_and_double_not_ok() {
        assert!(parse(lex("1 < 2 < 3").unwrap()).is_err());
        assert!(parse(lex("1 < 2").unwrap()).is_ok());
        // 이중 부정은 허용(우결합)
        assert!(parse(lex("not not true").unwrap()).is_ok());
    }
```
`src/shellcore/engine.rs` 테스트 모듈에 추가:
```rust
    #[test]
    fn block_evaluates_to_closure() {
        let mut e = Engine::new();
        assert!(matches!(eval_line("{ $it }", &mut e).unwrap(), Value::Closure(_)));
    }

    #[test]
    fn where_scalar_and_record_and_cellpath() {
        let mut e = Engine::new();
        // 스칼라 리스트 필터
        assert_eq!(
            eval_line("[1 2 3] | where $it > 1", &mut e).unwrap(),
            Value::List(vec![Value::Int(2), Value::Int(3)])
        );
        // 레코드 맨이름=필드 (유지)
        let out = eval_line("[{size: 50} {size: 200}] | where size > 100 | length", &mut e).unwrap();
        assert_eq!(out, Value::Int(1));
        // 셀경로 in where
        let out = eval_line("[{a: {b: 9}} {a: {b: 1}}] | where $it.a.b > 5 | length", &mut e).unwrap();
        assert_eq!(out, Value::Int(1));
    }

    #[test]
    fn cellpath_errors() {
        let mut e = Engine::new();
        assert!(eval_line("[{a: 1}] | where $it.nope > 0", &mut e).is_err()); // 없는 필드
    }
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore 2>&1 | tail -15'` → 컴파일 에러(`Expr::Block`/`Expr::CellPath`/`CellSeg`/`Value::Closure` 미정의).

- [ ] **Step 3a: ast.rs** — `Expr` enum에 추가, 그리고 `CellSeg` enum 추가:
```rust
    Block(Box<Expr>),
    CellPath {
        base: String,
        segs: Vec<CellSeg>,
    },
```
(파일 내 `BinOp`/`UnOp` 곁에)
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellSeg {
    Field(String),
    Index(usize),
}
```

- [ ] **Step 3b: value.rs** — `use crate::shellcore::ast::Expr;` 추가(상단). `Value` enum에 `Closure(Box<Expr>)` 추가. `type_name`에 `Value::Closure(_) => "closure",` 추가. `coerce_string`에 `other` 캐치올 앞에 `Value::Closure(_) => "<closure>".to_string(),` 추가.

- [ ] **Step 3c: format.rs** — `format_value`의 match에 `Value::Closure(_) => "<closure>".to_string(),` 추가.

- [ ] **Step 3d: parser.rs** —
- `peek2` 헬퍼 추가(impl Parser):
```rust
    fn peek2(&self) -> Option<&Token> {
        self.toks.get(self.pos + 1)
    }
```
- `parse_atom`의 `Some(Token::LBrace) => self.parse_record(),` 아크를 휴리스틱으로 교체:
```rust
            Some(Token::LBrace) => {
                let is_record = matches!(self.peek(), Some(Token::RBrace))
                    || matches!(
                        (self.peek(), self.peek2()),
                        (Some(Token::Word(_)), Some(Token::Colon))
                            | (Some(Token::Str(_)), Some(Token::Colon))
                    );
                if is_record {
                    self.parse_record()
                } else {
                    self.parse_block()
                }
            }
```
- `parse_block` 메서드 추가(impl Parser):
```rust
    fn parse_block(&mut self) -> Result<Expr> {
        let e = self.parse_expr()?;
        match self.next() {
            Some(Token::RBrace) => Ok(Expr::Block(Box::new(e))),
            other => bail!("'}}' 기대(블록), got {other:?}"),
        }
    }
```
- `parse_atom`의 `Some(Token::Var(v)) => Ok(Expr::Var(v)),` 아크를 셀경로 분해로 교체:
```rust
            Some(Token::Var(v)) => Ok(var_to_expr(v)),
```
- 자유 함수 추가(impl 밖, `cmp_op` 곁):
```rust
fn var_to_expr(s: String) -> Expr {
    let mut parts = s.split('.');
    let base = parts.next().unwrap_or_default().to_string();
    let segs: Vec<CellSeg> = parts
        .map(|p| match p.parse::<usize>() {
            Ok(i) => CellSeg::Index(i),
            Err(_) => CellSeg::Field(p.to_string()),
        })
        .collect();
    if segs.is_empty() {
        Expr::Var(base)
    } else {
        Expr::CellPath { base, segs }
    }
}
```
- `parse_cmp`를 비연쇄(`while`→`if` + 연쇄 거부)로 교체:
```rust
    fn parse_cmp(&mut self) -> Result<Expr> {
        let lhs = self.parse_atom()?;
        if let Some(op) = self.peek().and_then(cmp_op) {
            self.next();
            let rhs = self.parse_atom()?;
            if self.peek().and_then(cmp_op).is_some() {
                bail!("연쇄 비교는 지원하지 않습니다(괄호로 그룹화하세요)");
            }
            Ok(Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            })
        } else {
            Ok(lhs)
        }
    }
```

- [ ] **Step 3e: engine.rs** —
- `eval_pipeline`의 `Stage::Where` 아크를 스칼라 허용으로 교체:
```rust
            Stage::Where(cond) => {
                let items = match input {
                    Value::List(items) => items,
                    other => bail!("where: 리스트가 아닙니다 ({})", other.type_name()),
                };
                let mut kept = Vec::new();
                for it in items {
                    if ops::as_bool(&eval_expr(cond, engine, Some(&it))?)? {
                        kept.push(it);
                    }
                }
                Value::List(kept)
            }
```
(`Stage::Expr(e) => eval_expr(e, engine, None)?` 및 `Command` 인자의 `eval_expr(a, engine, None)`은 시그니처 변경에 맞춰 그대로 `None` 유지.)
- `eval_expr`를 `it` 일반화 + Block/CellPath 로 **전체 교체**:
```rust
fn eval_expr(e: &Expr, engine: &mut Engine, it: Option<&Value>) -> Result<Value> {
    use crate::shellcore::ast::{BinOp, CellSeg, UnOp};
    Ok(match e {
        Expr::Int(n) => Value::Int(*n),
        Expr::Float(f) => Value::Float(*f),
        Expr::Str(s) => Value::String(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Null => Value::Nothing,
        Expr::Word(w) => match it {
            Some(Value::Record(rec)) => rec
                .get(w)
                .cloned()
                .ok_or_else(|| anyhow!("필드를 찾을 수 없습니다: {w}"))?,
            _ => Value::String(w.clone()),
        },
        Expr::Var(name) => lookup_var(name, engine, it)?,
        Expr::CellPath { base, segs } => {
            let mut cur = lookup_var(base, engine, it)?;
            for seg in segs {
                cur = match seg {
                    CellSeg::Field(f) => match cur {
                        Value::Record(r) => r
                            .get(f)
                            .cloned()
                            .ok_or_else(|| anyhow!("필드를 찾을 수 없습니다: {f}"))?,
                        other => bail!("셀경로 .{f}: 레코드가 아닙니다 ({})", other.type_name()),
                    },
                    CellSeg::Index(idx) => match cur {
                        Value::List(items) => items
                            .get(*idx)
                            .cloned()
                            .ok_or_else(|| anyhow!("인덱스 범위를 벗어났습니다: {idx}"))?,
                        other => bail!("셀경로 .{idx}: 리스트가 아닙니다 ({})", other.type_name()),
                    },
                };
            }
            cur
        }
        Expr::Block(b) => Value::Closure(b.clone()),
        Expr::List(items) => {
            let vals: Vec<Value> = items
                .iter()
                .map(|x| eval_expr(x, engine, it))
                .collect::<Result<_>>()?;
            Value::List(vals)
        }
        Expr::Record(pairs) => {
            let mut rec = OrderedMap::new();
            for (k, x) in pairs {
                rec.insert(k.clone(), eval_expr(x, engine, it)?);
            }
            Value::Record(rec)
        }
        Expr::Binary { op, lhs, rhs } => match op {
            BinOp::And => {
                if !ops::as_bool(&eval_expr(lhs, engine, it)?)? {
                    Value::Bool(false)
                } else {
                    Value::Bool(ops::as_bool(&eval_expr(rhs, engine, it)?)?)
                }
            }
            BinOp::Or => {
                if ops::as_bool(&eval_expr(lhs, engine, it)?)? {
                    Value::Bool(true)
                } else {
                    Value::Bool(ops::as_bool(&eval_expr(rhs, engine, it)?)?)
                }
            }
            _ => {
                let l = eval_expr(lhs, engine, it)?;
                let r = eval_expr(rhs, engine, it)?;
                ops::apply_compare(*op, &l, &r)?
            }
        },
        Expr::Unary { op, expr } => match op {
            UnOp::Not => Value::Bool(!ops::as_bool(&eval_expr(expr, engine, it)?)?),
        },
    })
}

/// 변수 이름을 해석한다. `$it`은 활성 원소(it=Some)면 그것, 아니면 스코프.
fn lookup_var(name: &str, engine: &Engine, it: Option<&Value>) -> Result<Value> {
    if name == "it" {
        if let Some(v) = it {
            return Ok(v.clone());
        }
    }
    match engine.vars.get(name) {
        Some(v) => Ok(v.clone()),
        None => bail!("변수를 찾을 수 없습니다: ${name}"),
    }
}

/// 클로저(블록 식)를 원소 `it` 바인딩으로 적용한다(빌트인 `each` 등에서 호출).
pub fn eval_closure(block: &Expr, it: &Value, engine: &mut Engine) -> Result<Value> {
    eval_expr(block, engine, Some(it))
}
```

- [ ] **Step 4: 통과 + clippy + fmt** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore 2>&1 | grep -E "test result|FAILED|error\[" | tail; cargo clippy --all-targets -- -D warnings 2>&1 | tail -2; cargo fmt --all -- --check >/dev/null 2>&1 && echo FMT_OK || echo FMT_FAIL'`
  Expected: 모든 shellcore 테스트 통과(parser +2, engine +3), clippy clean, FMT_OK. (FMT_FAIL이면 `cargo fmt --all` 후 해당 파일 포함.)

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/ast.rs src/shellcore/value.rs src/shellcore/format.rs src/shellcore/parser.rs src/shellcore/engine.rs && git commit -m "feat(shell): 블록/Closure + 셀경로 + eval it 일반화 + scalar where (S1b-1 T2)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: `each` 빌트인

**Files:** Modify `src/shellcore/builtins.rs`

- [ ] **Step 1: 실패 테스트** — builtins 테스트 모듈에 추가:
```rust
    #[test]
    fn each_maps_closure_over_list() {
        let mut e = Engine::new();
        assert_eq!(
            eval_line("[{name: a} {name: b}] | each { $it.name }", &mut e).unwrap(),
            Value::List(vec![Value::String("a".into()), Value::String("b".into())])
        );
        // 중첩 셀경로
        assert_eq!(
            eval_line("[{a: {b: 9}}] | each { $it.a.b }", &mut e).unwrap(),
            Value::List(vec![Value::Int(9)])
        );
        // 스칼라 원소 $it
        assert_eq!(
            eval_line("[1 2 3] | each { $it }", &mut e).unwrap(),
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
        // 인덱스 셀경로
        assert_eq!(
            eval_line("[{xs: [10 20 30]}] | each { $it.xs.1 }", &mut e).unwrap(),
            Value::List(vec![Value::Int(20)])
        );
    }

    #[test]
    fn each_errors_on_non_closure_or_non_list() {
        let mut e = Engine::new();
        assert!(eval_line("[1 2] | each 5", &mut e).is_err()); // 비-클로저 인자
        assert!(eval_line("5 | each { $it }", &mut e).is_err()); // 비-List 입력
    }
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::builtins 2>&1 | tail -12'` → `each`가 외부 명령으로 빠져 실패.

- [ ] **Step 3: 구현** — `src/shellcore/builtins.rs`:
- `lookup` match에 추가(다른 빌트인 곁):
```rust
        "each" => Some(b_each),
```
- 구현 추가(다른 빌트인 함수 곁):
```rust
fn b_each(args: &[Value], input: Value, e: &mut Engine) -> Result<Value> {
    let block = match args.first() {
        Some(Value::Closure(b)) => b,
        Some(other) => bail!("each: 클로저 {{...}} 가 필요합니다 ({})", other.type_name()),
        None => bail!("each: 클로저 인자가 필요합니다"),
    };
    match input {
        Value::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                out.push(crate::shellcore::engine::eval_closure(block, &it, e)?);
            }
            Ok(Value::List(out))
        }
        other => bail!("each: 리스트가 아닙니다 ({})", other.type_name()),
    }
}
```
(`block`은 `&Box<Expr>` → `eval_closure(block, …)`에 deref 강제로 `&Expr` 전달. import 추가 불필요 — 완전경로 사용.)

- [ ] **Step 4: 통과 + clippy + fmt** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore 2>&1 | grep -E "test result|FAILED" | tail; cargo clippy --all-targets -- -D warnings 2>&1 | tail -2; cargo fmt --all -- --check >/dev/null 2>&1 && echo FMT_OK || echo FMT_FAIL'`
  Expected: 모든 shellcore 통과(each 2 포함), clippy clean, FMT_OK.

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/builtins.rs && git commit -m "feat(shell): each 빌트인 (S1b-1 T3)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## 최종 검증 (전체 task 완료 후)
- [ ] shellcore 전체: `cargo test shellcore 2>&1 | grep "test result" | head -1` → ok.
- [ ] 무회귀: `cargo test --features "storage tls remote" 2>&1 | grep -E "test result|FAILED" | tail -3` → 0 failed.
- [ ] fmt/clippy: `cargo fmt --all -- --check` (FMT_OK) + `cargo clippy --all-targets -- -D warnings` (clean).
- [ ] ash 스모크: `printf '[{name: a, size: 50} {name: b, size: 200}] | where size > 100 | each { $it.name }\nexit\n' | cargo run --bin ash` → `[b]` 형태(이름 b만).
- [ ] DoD 대조(스펙 §1): 셀경로 렉싱(T1) · 블록/Closure/셀경로/eval-it/scalar-where(T2) · each(T3) 전부 커버. forward 노트(연쇄비교 거부·이중부정·비-Record행 제거) 반영.

> 이후: S1b-2(sort-by/select/range/슬라이스). 별도 spec→plan.
