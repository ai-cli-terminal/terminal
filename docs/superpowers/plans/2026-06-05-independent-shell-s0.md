# S0 — 독립 셸 코어 MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 자체 명령 언어를 가진 독립 구조화 셸의 최소 코어(값 모델 → 렉서 → 파서 → 평가기 → 빌트인 7종 → 외부 실행 → 최소 REPL)를 만들어 `ls | get name | first 3` 같은 구조화 파이프라인과 외부 명령이 도는 `ash` 바이너리를 낸다.

**Architecture:** 새 모듈 트리 `src/shellcore/*`(lib `ai_terminal` 안)에 언어 코어를 두고, 새 바이너리 `src/bin/ash.rs`가 REPL을 띄운다. 명령은 구조화 `Value`(레코드/테이블 포함)를 파이프라인으로 흘리고, 빌트인 아닌 이름은 외부 바이너리로 stdio 상속 spawn한다. 순수 Rust, C-free, 새 의존성 0(anyhow만 사용 — 이미 있음).

**Tech Stack:** Rust(edition 2021) · anyhow · std only. 정본 스펙: `docs/superpowers/specs/2026-06-05-independent-shell-s0-core-design.md` §1.

---

## 실행 환경 메모 (모든 task 공통)

- **빌드/테스트는 WSL**(Rust 툴체인이 WSL에만). 단일 라인 래퍼:
  ```
  wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore 2>&1 | tail -20'
  ```
  멀티라인 `bash -lc` 금지(CRLF). 파일 생성/수정은 Read/Write/Edit(Windows 경로).
- 기본 빌드는 C-free — 본 코어는 default feature(추가 feature 불필요).
- 커밋은 브랜치 `feat/independent-shell-s0`에 명시적 `git add`(절대 `git add -A` 금지). 커밋 메시지 끝에 `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. push 안 함.
- 모듈은 `ai_terminal` lib 하위 `shellcore`. 각 파일 상단에 `//!` 한 줄 설명(기존 코드 관례).

---

## Task 1: 값 모델 + OrderedMap + 모듈 등록

**Files:**
- Create: `src/shellcore/mod.rs`
- Create: `src/shellcore/value.rs`
- Modify: `src/lib.rs` (모듈 선언 추가)

- [ ] **Step 1: 실패 테스트 작성** — `src/shellcore/value.rs` 하단에:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_map_preserves_order_and_overwrites() {
        let mut m = OrderedMap::new();
        m.insert("b", Value::Int(1));
        m.insert("a", Value::Int(2));
        m.insert("b", Value::Int(9)); // 덮어쓰기(순서 유지)
        let keys: Vec<&str> = m.keys().collect();
        assert_eq!(keys, vec!["b", "a"]);
        assert_eq!(m.get("b"), Some(&Value::Int(9)));
        assert_eq!(m.get("missing"), None);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn value_type_name_and_coerce_string() {
        assert_eq!(Value::Nothing.type_name(), "nothing");
        assert_eq!(Value::List(vec![]).type_name(), "list");
        assert_eq!(Value::Int(42).coerce_string(), "42");
        assert_eq!(Value::Bool(true).coerce_string(), "true");
        assert_eq!(Value::String("x".into()).coerce_string(), "x");
        assert_eq!(Value::Nothing.coerce_string(), "");
    }
}
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::value 2>&1 | tail -15'`
  Expected: 컴파일 에러(`OrderedMap`/`Value` 미정의, 모듈 미등록).

- [ ] **Step 3: 구현** — `src/shellcore/value.rs` 상단(테스트 위)에:

```rust
//! 셸 값 모델: 구조화 파이프라인이 흘리는 데이터 타입 + 순서 보존 레코드 맵.

/// 순서 보존 맵(레코드/변수 스코프용). 새 의존성(indexmap) 회피용 경량 구현.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrderedMap {
    entries: Vec<(String, Value)>,
}

impl OrderedMap {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }
    /// 키가 있으면 값 갱신(순서 유지), 없으면 끝에 추가.
    pub fn insert(&mut self, key: impl Into<String>, val: Value) {
        let key = key.into();
        if let Some(e) = self.entries.iter_mut().find(|(k, _)| *k == key) {
            e.1 = val;
        } else {
            self.entries.push((key, val));
        }
    }
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|(k, _)| k.as_str())
    }
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 셸 값. 테이블 = `List`(원소가 모두 `Record`). 별도 타입 아님.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nothing,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Record(OrderedMap),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nothing => "nothing",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Record(_) => "record",
        }
    }
    /// 외부 명령 인자·필드명 등에서 쓰는 문자열 강제.
    pub fn coerce_string(&self) -> String {
        match self {
            Value::Nothing => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::String(s) => s.clone(),
            other => format!("{other:?}"),
        }
    }
}
```

`src/shellcore/mod.rs`:
```rust
//! 독립 셸 코어(S0): 값 모델·렉서·파서·평가기·빌트인·외부 실행·포매터·REPL.

pub mod value;
```

`src/lib.rs`에 모듈 선언 추가(다른 `pub mod` 선언들 곁에):
```rust
pub mod shellcore;
```

- [ ] **Step 4: 통과 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::value 2>&1 | tail -8'`
  Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/mod.rs src/shellcore/value.rs src/lib.rs && git commit -m "feat(shell): 값 모델 + OrderedMap (S0 T1)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: 값 포매터

**Files:**
- Create: `src/shellcore/format.rs`
- Modify: `src/shellcore/mod.rs` (`pub mod format;`)

- [ ] **Step 1: 실패 테스트** — `src/shellcore/format.rs` 하단:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::value::{OrderedMap, Value};

    #[test]
    fn scalars_render_inline() {
        assert_eq!(format_value(&Value::Int(5)), "5");
        assert_eq!(format_value(&Value::String("hi".into())), "hi");
        assert_eq!(format_value(&Value::Nothing), "");
    }

    #[test]
    fn table_has_header_and_rows() {
        let mut r = OrderedMap::new();
        r.insert("name", Value::String("README.md".into()));
        r.insert("size", Value::Int(12));
        let table = Value::List(vec![Value::Record(r)]);
        let out = format_value(&table);
        assert!(out.contains("name"), "헤더 포함: {out}");
        assert!(out.contains("README.md"), "값 포함: {out}");
        assert!(out.contains("size"), "헤더 포함: {out}");
    }

    #[test]
    fn scalar_list_is_indexed() {
        let out = format_value(&Value::List(vec![Value::Int(1), Value::Int(2)]));
        assert!(out.contains('1') && out.contains('2'), "{out}");
    }
}
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::format 2>&1 | tail -12'`
  Expected: 컴파일 에러(`format_value` 미정의).

- [ ] **Step 3: 구현** — `src/shellcore/format.rs` 상단:
```rust
//! 값 렌더링: 스칼라는 한 줄, 리스트/레코드/테이블은 정렬 표(MVP — ASCII).

use crate::shellcore::value::{OrderedMap, Value};

/// 값을 사람이 읽는 문자열로 렌더한다.
pub fn format_value(v: &Value) -> String {
    match v {
        Value::Nothing => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => s.clone(),
        Value::Record(r) => format_record(r),
        Value::List(items) => format_list(items),
    }
}

fn format_list(items: &[Value]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    if items.iter().all(|v| matches!(v, Value::Record(_))) {
        return format_table(items);
    }
    let mut rows = vec![vec!["#".to_string(), "value".to_string()]];
    for (i, v) in items.iter().enumerate() {
        rows.push(vec![i.to_string(), format_value(v)]);
    }
    render_grid(&rows)
}

fn format_table(items: &[Value]) -> String {
    let mut cols: Vec<String> = Vec::new();
    for v in items {
        if let Value::Record(r) = v {
            for k in r.keys() {
                if !cols.iter().any(|c| c == k) {
                    cols.push(k.to_string());
                }
            }
        }
    }
    let mut header = vec!["#".to_string()];
    header.extend(cols.iter().cloned());
    let mut rows = vec![header];
    for (i, v) in items.iter().enumerate() {
        if let Value::Record(r) = v {
            let mut row = vec![i.to_string()];
            for c in &cols {
                row.push(r.get(c).map(format_value).unwrap_or_default());
            }
            rows.push(row);
        }
    }
    render_grid(&rows)
}

fn format_record(r: &OrderedMap) -> String {
    let mut rows = vec![vec!["key".to_string(), "value".to_string()]];
    for (k, v) in r.iter() {
        rows.push(vec![k.to_string(), format_value(v)]);
    }
    render_grid(&rows)
}

fn render_grid(rows: &[Vec<String>]) -> String {
    let ncol = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; ncol];
    for r in rows {
        for (i, c) in r.iter().enumerate() {
            widths[i] = widths[i].max(c.chars().count());
        }
    }
    let mut out = String::new();
    for r in rows {
        for (i, c) in r.iter().enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            out.push_str(c);
            for _ in c.chars().count()..widths[i] {
                out.push(' ');
            }
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}
```
`src/shellcore/mod.rs`에 `pub mod format;` 추가.

- [ ] **Step 4: 통과 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::format 2>&1 | tail -8'`
  Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/format.rs src/shellcore/mod.rs && git commit -m "feat(shell): 값 포매터(표/리스트/스칼라) (S0 T2)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: 렉서

**Files:**
- Create: `src/shellcore/lexer.rs`
- Modify: `src/shellcore/mod.rs` (`pub mod lexer;`)

- [ ] **Step 1: 실패 테스트** — `src/shellcore/lexer.rs` 하단:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_pipeline_and_args() {
        let t = lex("ls | get name | first 3").unwrap();
        assert_eq!(
            t,
            vec![
                Token::Word("ls".into()),
                Token::Pipe,
                Token::Word("get".into()),
                Token::Word("name".into()),
                Token::Pipe,
                Token::Word("first".into()),
                Token::Int(3),
            ]
        );
    }

    #[test]
    fn tokenizes_let_var_and_literals() {
        let t = lex("let x = 3.5").unwrap();
        assert_eq!(
            t,
            vec![Token::Let, Token::Word("x".into()), Token::Equals, Token::Float(3.5)]
        );
        assert_eq!(lex("$y").unwrap(), vec![Token::Var("y".into())]);
        assert_eq!(lex("true false null").unwrap(), vec![Token::True, Token::False, Token::Null]);
        assert_eq!(lex("\"hi there\"").unwrap(), vec![Token::Str("hi there".into())]);
    }

    #[test]
    fn list_record_and_comment() {
        assert_eq!(
            lex("[1 2]").unwrap(),
            vec![Token::LBracket, Token::Int(1), Token::Int(2), Token::RBracket]
        );
        assert_eq!(
            lex("{a: 1}").unwrap(),
            vec![Token::LBrace, Token::Word("a".into()), Token::Colon, Token::Int(1), Token::RBrace]
        );
        assert_eq!(lex("ls # comment").unwrap(), vec![Token::Word("ls".into())]);
    }

    #[test]
    fn path_like_word_and_newline() {
        assert_eq!(lex("cd ./src").unwrap(), vec![Token::Word("cd".into()), Token::Word("./src".into())]);
        assert_eq!(lex("a\nb").unwrap(), vec![Token::Word("a".into()), Token::Newline, Token::Word("b".into())]);
    }
}
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::lexer 2>&1 | tail -12'`
  Expected: 컴파일 에러(`Token`/`lex` 미정의).

- [ ] **Step 3: 구현** — `src/shellcore/lexer.rs` 상단:
```rust
//! 렉서: 소스를 토큰으로. 바레워드는 숫자/키워드 판별; =·:·,·파이프·괄호류는 전용 토큰.
//! (S0 한계: 외부 인자에 = 포함 시 따옴표 필요 — `"--k=v"`. URL 등 : 포함도 따옴표.)

use anyhow::{bail, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),
    Int(i64),
    Float(f64),
    Str(String),
    Var(String),
    Pipe,
    Semicolon,
    Newline,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Colon,
    Comma,
    Equals,
    Let,
    True,
    False,
    Null,
}

const SPECIAL: &[char] = &['|', ';', '[', ']', '{', '}', '(', ')', ':', ',', '=', '#', '"', '\'', '$'];

pub fn lex(src: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' | '\t' | '\r' => {
                i += 1;
            }
            '\n' => {
                out.push(Token::Newline);
                i += 1;
            }
            '#' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '|' => {
                out.push(Token::Pipe);
                i += 1;
            }
            ';' => {
                out.push(Token::Semicolon);
                i += 1;
            }
            '[' => {
                out.push(Token::LBracket);
                i += 1;
            }
            ']' => {
                out.push(Token::RBracket);
                i += 1;
            }
            '{' => {
                out.push(Token::LBrace);
                i += 1;
            }
            '}' => {
                out.push(Token::RBrace);
                i += 1;
            }
            '(' => {
                out.push(Token::LParen);
                i += 1;
            }
            ')' => {
                out.push(Token::RParen);
                i += 1;
            }
            ':' => {
                out.push(Token::Colon);
                i += 1;
            }
            ',' => {
                out.push(Token::Comma);
                i += 1;
            }
            '=' => {
                out.push(Token::Equals);
                i += 1;
            }
            '"' | '\'' => {
                let quote = c;
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                if i >= chars.len() {
                    bail!("닫히지 않은 문자열");
                }
                let s: String = chars[start..i].iter().collect();
                out.push(Token::Str(s));
                i += 1; // 닫는 따옴표
            }
            '$' => {
                i += 1;
                let start = i;
                while i < chars.len() && is_word_char(chars[i]) {
                    i += 1;
                }
                if start == i {
                    bail!("$ 뒤에 변수 이름이 필요합니다");
                }
                let name: String = chars[start..i].iter().collect();
                out.push(Token::Var(name));
            }
            _ => {
                // 바레워드: 공백/개행/특수문자 전까지.
                let start = i;
                while i < chars.len() && !chars[i].is_whitespace() && !SPECIAL.contains(&chars[i]) {
                    i += 1;
                }
                let w: String = chars[start..i].iter().collect();
                out.push(classify_word(w));
            }
        }
    }
    Ok(out)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn classify_word(w: String) -> Token {
    match w.as_str() {
        "let" => return Token::Let,
        "true" => return Token::True,
        "false" => return Token::False,
        "null" => return Token::Null,
        _ => {}
    }
    if let Ok(n) = w.parse::<i64>() {
        return Token::Int(n);
    }
    // 정확히 점 하나 + 양쪽 숫자일 때만 Float(경로 192.168.0.1 은 Word).
    if w.matches('.').count() == 1 {
        if let Ok(f) = w.parse::<f64>() {
            return Token::Float(f);
        }
    }
    Token::Word(w)
}
```
`src/shellcore/mod.rs`에 `pub mod lexer;` 추가.

- [ ] **Step 4: 통과 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::lexer 2>&1 | tail -8'`
  Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/lexer.rs src/shellcore/mod.rs && git commit -m "feat(shell): 렉서 (S0 T3)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: AST + 파서

**Files:**
- Create: `src/shellcore/ast.rs`
- Create: `src/shellcore/parser.rs`
- Modify: `src/shellcore/mod.rs` (`pub mod ast; pub mod parser;`)

- [ ] **Step 1: 실패 테스트** — `src/shellcore/parser.rs` 하단:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::ast::*;
    use crate::shellcore::lexer::lex;

    fn p(src: &str) -> Vec<Stmt> {
        parse(lex(src).unwrap()).unwrap()
    }

    #[test]
    fn parses_pipeline_of_commands() {
        let stmts = p("ls | get name | first 3");
        assert_eq!(stmts.len(), 1);
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!("pipeline 기대") };
        assert_eq!(pl.stages.len(), 3);
        let Stage::Command(c0) = &pl.stages[0] else { panic!() };
        assert_eq!(c0.name, "ls");
        assert!(c0.args.is_empty());
        let Stage::Command(c1) = &pl.stages[1] else { panic!() };
        assert_eq!(c1.name, "get");
        assert_eq!(c1.args, vec![Expr::Word("name".into())]);
        let Stage::Command(c2) = &pl.stages[2] else { panic!() };
        assert_eq!(c2.args, vec![Expr::Int(3)]);
    }

    #[test]
    fn parses_let_and_leading_expr() {
        let stmts = p("let x = 5");
        assert_eq!(stmts[0], Stmt::Let { name: "x".into(), value: Pipeline { stages: vec![Stage::Expr(Expr::Int(5))] } });
        let stmts = p("$x");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        assert_eq!(pl.stages[0], Stage::Expr(Expr::Var("x".into())));
    }

    #[test]
    fn parses_list_and_record_literals() {
        let stmts = p("[1 2]");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        assert_eq!(pl.stages[0], Stage::Expr(Expr::List(vec![Expr::Int(1), Expr::Int(2)])));
        let stmts = p("{a: 1, b: hi}");
        let Stmt::Pipeline(pl) = &stmts[0] else { panic!() };
        assert_eq!(
            pl.stages[0],
            Stage::Expr(Expr::Record(vec![("a".into(), Expr::Int(1)), ("b".into(), Expr::Word("hi".into()))]))
        );
    }

    #[test]
    fn multiple_statements_split_by_newline_and_semicolon() {
        assert_eq!(p("print 1; print 2").len(), 2);
        assert_eq!(p("print 1\nprint 2").len(), 2);
    }
}
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::parser 2>&1 | tail -12'`
  Expected: 컴파일 에러(`ast`/`parse` 미정의).

- [ ] **Step 3: 구현** — `src/shellcore/ast.rs`:
```rust
//! AST: 문장(let/pipeline) · 파이프라인 · 스테이지(expr/command) · 표현식.

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Null,
    Var(String),
    Word(String),
    List(Vec<Expr>),
    Record(Vec<(String, Expr)>),
    Sub(Box<Pipeline>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stage {
    Expr(Expr),
    Command(Command),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let { name: String, value: Pipeline },
    Pipeline(Pipeline),
}
```

`src/shellcore/parser.rs` 상단:
```rust
//! 파서: 토큰 → AST. 스테이지 선두가 Word면 Command(이름+인자), 아니면 Expr.

use anyhow::{bail, Result};

use crate::shellcore::ast::*;
use crate::shellcore::lexer::Token;

pub fn parse(tokens: Vec<Token>) -> Result<Vec<Stmt>> {
    let mut p = Parser { toks: tokens, pos: 0 };
    let mut stmts = Vec::new();
    p.skip_separators();
    while p.peek().is_some() {
        stmts.push(p.parse_stmt()?);
        p.skip_separators();
    }
    Ok(stmts)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn skip_separators(&mut self) {
        while matches!(self.peek(), Some(Token::Newline) | Some(Token::Semicolon)) {
            self.pos += 1;
        }
    }
    fn at_stage_end(&self) -> bool {
        matches!(
            self.peek(),
            None | Some(Token::Pipe)
                | Some(Token::Newline)
                | Some(Token::Semicolon)
                | Some(Token::RBracket)
                | Some(Token::RBrace)
                | Some(Token::RParen)
        )
    }

    fn parse_stmt(&mut self) -> Result<Stmt> {
        if matches!(self.peek(), Some(Token::Let)) {
            self.next();
            let name = match self.next() {
                Some(Token::Word(w)) => w,
                other => bail!("let: 변수 이름 기대, got {other:?}"),
            };
            match self.next() {
                Some(Token::Equals) => {}
                other => bail!("let: '=' 기대, got {other:?}"),
            }
            let value = self.parse_pipeline()?;
            return Ok(Stmt::Let { name, value });
        }
        Ok(Stmt::Pipeline(self.parse_pipeline()?))
    }

    fn parse_pipeline(&mut self) -> Result<Pipeline> {
        let mut stages = vec![self.parse_stage()?];
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.next();
            stages.push(self.parse_stage()?);
        }
        Ok(Pipeline { stages })
    }

    fn parse_stage(&mut self) -> Result<Stage> {
        if let Some(Token::Word(_)) = self.peek() {
            let name = match self.next() {
                Some(Token::Word(w)) => w,
                _ => unreachable!(),
            };
            let mut args = Vec::new();
            while !self.at_stage_end() {
                args.push(self.parse_expr()?);
            }
            Ok(Stage::Command(Command { name, args }))
        } else {
            Ok(Stage::Expr(self.parse_expr()?))
        }
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        match self.next() {
            Some(Token::Int(n)) => Ok(Expr::Int(n)),
            Some(Token::Float(f)) => Ok(Expr::Float(f)),
            Some(Token::Str(s)) => Ok(Expr::Str(s)),
            Some(Token::True) => Ok(Expr::Bool(true)),
            Some(Token::False) => Ok(Expr::Bool(false)),
            Some(Token::Null) => Ok(Expr::Null),
            Some(Token::Var(v)) => Ok(Expr::Var(v)),
            Some(Token::Word(w)) => Ok(Expr::Word(w)),
            Some(Token::LBracket) => self.parse_list(),
            Some(Token::LBrace) => self.parse_record(),
            Some(Token::LParen) => {
                let pl = self.parse_pipeline()?;
                match self.next() {
                    Some(Token::RParen) => Ok(Expr::Sub(Box::new(pl))),
                    other => bail!("')' 기대, got {other:?}"),
                }
            }
            other => bail!("표현식 기대, got {other:?}"),
        }
    }

    fn parse_list(&mut self) -> Result<Expr> {
        let mut items = Vec::new();
        while !matches!(self.peek(), Some(Token::RBracket) | None) {
            items.push(self.parse_expr()?);
            if matches!(self.peek(), Some(Token::Comma)) {
                self.next();
            }
        }
        match self.next() {
            Some(Token::RBracket) => Ok(Expr::List(items)),
            other => bail!("']' 기대, got {other:?}"),
        }
    }

    fn parse_record(&mut self) -> Result<Expr> {
        let mut pairs = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            let key = match self.next() {
                Some(Token::Word(w)) => w,
                Some(Token::Str(s)) => s,
                other => bail!("레코드 키(이름) 기대, got {other:?}"),
            };
            match self.next() {
                Some(Token::Colon) => {}
                other => bail!("레코드 ':' 기대, got {other:?}"),
            }
            let val = self.parse_expr()?;
            pairs.push((key, val));
            if matches!(self.peek(), Some(Token::Comma)) {
                self.next();
            }
        }
        match self.next() {
            Some(Token::RBrace) => Ok(Expr::Record(pairs)),
            other => bail!("'}}' 기대, got {other:?}"),
        }
    }
}
```
`src/shellcore/mod.rs`에 `pub mod ast;` 와 `pub mod parser;` 추가.

- [ ] **Step 4: 통과 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::parser 2>&1 | tail -8'`
  Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/ast.rs src/shellcore/parser.rs src/shellcore/mod.rs && git commit -m "feat(shell): AST + 파서 (S0 T4)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: 엔진 + 기본 빌트인(print/cd/exit) + 외부 실행 + eval_line

**Files:**
- Create: `src/shellcore/engine.rs`
- Create: `src/shellcore/builtins.rs`
- Create: `src/shellcore/external.rs`
- Modify: `src/shellcore/mod.rs` (`pub mod engine; pub mod builtins; pub mod external;`)

- [ ] **Step 1: 실패 테스트** — `src/shellcore/engine.rs` 하단:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::value::Value;

    #[test]
    fn evaluates_literals_and_let_var() {
        let mut e = Engine::new();
        assert_eq!(eval_line("5", &mut e).unwrap(), Value::Int(5));
        assert_eq!(eval_line("let x = 7", &mut e).unwrap(), Value::Nothing);
        assert_eq!(eval_line("$x", &mut e).unwrap(), Value::Int(7));
    }

    #[test]
    fn print_returns_nothing() {
        let mut e = Engine::new();
        assert_eq!(eval_line("print 3", &mut e).unwrap(), Value::Nothing);
    }

    #[test]
    fn list_literal_and_pipeline_passthrough() {
        let mut e = Engine::new();
        assert_eq!(
            eval_line("[1 2 3]", &mut e).unwrap(),
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn unknown_var_errors() {
        let mut e = Engine::new();
        assert!(eval_line("$nope", &mut e).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn external_command_runs_and_returns_nothing() {
        let mut e = Engine::new();
        // 절대경로 외부 명령(빌트인/키워드 아님) — spawn, 종료 0 → Nothing.
        // (주의: 베어 `true`는 키워드라 Bool 리터럴이 됨. 외부 실행 검증엔 경로 사용.)
        assert_eq!(eval_line("/bin/true", &mut e).unwrap(), Value::Nothing);
    }
}
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::engine 2>&1 | tail -12'`
  Expected: 컴파일 에러(`Engine`/`eval_line` 미정의).

- [ ] **Step 3: 구현** —

`src/shellcore/external.rs`:
```rust
//! 외부 명령 실행: 빌트인 아닌 이름을 PATH 바이너리로 stdio 상속 spawn(대화형 정상).

use anyhow::{bail, Result};

use crate::shellcore::engine::Engine;
use crate::shellcore::value::Value;

/// 외부 명령을 셸 cwd·현재 env로 실행한다. stdout/stderr는 터미널로 통과.
/// 반환은 Nothing. 비0 종료는 안내만 하고 에러로 만들지 않는다(REPL 지속).
pub fn run(name: &str, args: &[Value], engine: &mut Engine) -> Result<Value> {
    use std::process::Command;
    let arg_strs: Vec<String> = args.iter().map(|v| v.coerce_string()).collect();
    match Command::new(name).args(&arg_strs).current_dir(&engine.cwd).status() {
        Ok(st) => {
            if !st.success() {
                let code = st.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into());
                eprintln!("[{name}: exit {code}]");
            }
            Ok(Value::Nothing)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!("command not found: {name}"),
        Err(e) => bail!("failed to run {name}: {e}"),
    }
}
```

`src/shellcore/builtins.rs`:
```rust
//! 빌트인 레지스트리 + 구현. 시그니처: (args 평가값, 파이프라인 입력, engine) -> Value.

use anyhow::{bail, Result};

use crate::shellcore::engine::Engine;
use crate::shellcore::format::format_value;
use crate::shellcore::value::Value;

pub type Builtin = fn(&[Value], Value, &mut Engine) -> Result<Value>;

/// 이름으로 빌트인을 찾는다. (S0 T5: print/echo/cd/exit. T6에서 ls/get/first/length 추가.)
pub fn lookup(name: &str) -> Option<Builtin> {
    match name {
        "print" | "echo" => Some(b_print),
        "cd" => Some(b_cd),
        "exit" => Some(b_exit),
        _ => None,
    }
}

fn b_print(args: &[Value], input: Value, _e: &mut Engine) -> Result<Value> {
    let v = args.first().unwrap_or(&input);
    println!("{}", format_value(v));
    Ok(Value::Nothing)
}

fn b_cd(args: &[Value], _input: Value, e: &mut Engine) -> Result<Value> {
    let target = match args.first() {
        Some(v) => e.cwd.join(v.coerce_string()),
        None => home_dir(),
    };
    if !target.is_dir() {
        bail!("cd: 디렉터리가 없습니다: {}", target.display());
    }
    e.cwd = target.canonicalize().unwrap_or(target);
    Ok(Value::Nothing)
}

fn b_exit(args: &[Value], _input: Value, e: &mut Engine) -> Result<Value> {
    let code = match args.first() {
        Some(Value::Int(n)) => *n as i32,
        Some(other) => bail!("exit: 정수 코드 필요 ({})", other.type_name()),
        None => 0,
    };
    e.exit_code = Some(code);
    Ok(Value::Nothing)
}

fn home_dir() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

// T6에서 b_ls/b_get/b_first/b_length + 데이터 빌트인 추가.
```
> 참고: `b_print`은 인자 있으면 인자, 없으면 `$in`을 출력한다. T5 import는 `use anyhow::{bail, Result};`만(이 단계 빌트인은 `bail!`만 사용). `anyhow!`/`Context`는 T6에서 import에 추가한다.

`src/shellcore/engine.rs` 상단:
```rust
//! 엔진: 스코프(cwd/vars) + 표현식/파이프라인 평가 + eval_line(테스트 진입점).

use std::path::PathBuf;

use anyhow::{bail, Result};

use crate::shellcore::ast::{Expr, Pipeline, Stage, Stmt};
use crate::shellcore::value::{OrderedMap, Value};
use crate::shellcore::{builtins, external, lexer, parser};

/// 셸 실행 상태.
pub struct Engine {
    pub cwd: PathBuf,
    pub vars: OrderedMap,
    pub exit_code: Option<i32>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            vars: OrderedMap::new(),
            exit_code: None,
        }
    }
}

/// 한 줄(여러 문장 가능)을 평가하고 마지막 값(없으면 Nothing)을 반환한다. 테스트 진입점.
pub fn eval_line(src: &str, engine: &mut Engine) -> Result<Value> {
    let tokens = lexer::lex(src)?;
    let stmts = parser::parse(tokens)?;
    let mut last = Value::Nothing;
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, value } => {
                let v = eval_pipeline(&value, engine)?;
                engine.vars.insert(name, v);
                last = Value::Nothing;
            }
            Stmt::Pipeline(pl) => {
                last = eval_pipeline(&pl, engine)?;
            }
        }
    }
    Ok(last)
}

fn eval_pipeline(pl: &Pipeline, engine: &mut Engine) -> Result<Value> {
    let mut input = Value::Nothing;
    for stage in &pl.stages {
        input = match stage {
            Stage::Expr(e) => eval_expr(e, engine)?,
            Stage::Command(c) => {
                let args: Vec<Value> = c.args.iter().map(|a| eval_expr(a, engine)).collect::<Result<_>>()?;
                if let Some(b) = builtins::lookup(&c.name) {
                    b(&args, input, engine)?
                } else {
                    external::run(&c.name, &args, engine)?
                }
            }
        };
    }
    Ok(input)
}

fn eval_expr(e: &Expr, engine: &mut Engine) -> Result<Value> {
    Ok(match e {
        Expr::Int(n) => Value::Int(*n),
        Expr::Float(f) => Value::Float(*f),
        Expr::Str(s) => Value::String(s.clone()),
        Expr::Bool(b) => Value::Bool(*b),
        Expr::Null => Value::Nothing,
        Expr::Word(w) => Value::String(w.clone()),
        Expr::Var(name) => match engine.vars.get(name) {
            Some(v) => v.clone(),
            None => bail!("변수를 찾을 수 없습니다: ${name}"),
        },
        Expr::List(items) => {
            let vals: Vec<Value> = items.iter().map(|x| eval_expr(x, engine)).collect::<Result<_>>()?;
            Value::List(vals)
        }
        Expr::Record(pairs) => {
            let mut rec = OrderedMap::new();
            for (k, x) in pairs {
                rec.insert(k.clone(), eval_expr(x, engine)?);
            }
            Value::Record(rec)
        }
        Expr::Sub(pl) => eval_pipeline(pl, engine)?,
    })
}
```
`src/shellcore/mod.rs`에 `pub mod engine; pub mod builtins; pub mod external;` 추가.

- [ ] **Step 4: 통과 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::engine 2>&1 | tail -10'`
  Expected: `test result: ok. 5 passed`(unix; external 테스트 포함).

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/engine.rs src/shellcore/builtins.rs src/shellcore/external.rs src/shellcore/mod.rs && git commit -m "feat(shell): 엔진 + print/cd/exit + 외부 실행 + eval_line (S0 T5)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: 데이터 빌트인 (ls/get/first/length)

**Files:**
- Modify: `src/shellcore/builtins.rs` (lookup 확장 + 4종 구현)

- [ ] **Step 1: 실패 테스트** — `src/shellcore/builtins.rs` 하단에 추가:
```rust
#[cfg(test)]
mod tests {
    use crate::shellcore::engine::{eval_line, Engine};
    use crate::shellcore::value::Value;

    #[test]
    fn get_first_length_over_table_literal() {
        let mut e = Engine::new();
        // 테이블 리터럴 → 컬럼 추출 → 길이/앞n.
        assert_eq!(
            eval_line("[{name: a} {name: b} {name: c}] | get name", &mut e).unwrap(),
            Value::List(vec![Value::String("a".into()), Value::String("b".into()), Value::String("c".into())])
        );
        assert_eq!(eval_line("[1 2 3] | length", &mut e).unwrap(), Value::Int(3));
        assert_eq!(
            eval_line("[1 2 3] | first 2", &mut e).unwrap(),
            Value::List(vec![Value::Int(1), Value::Int(2)])
        );
        assert_eq!(eval_line("[1 2 3] | first", &mut e).unwrap(), Value::List(vec![Value::Int(1)]));
    }

    #[test]
    fn get_field_from_record() {
        let mut e = Engine::new();
        assert_eq!(eval_line("{a: 1, b: 2} | get b", &mut e).unwrap(), Value::Int(2));
        assert!(eval_line("{a: 1} | get zzz", &mut e).is_err());
    }

    #[test]
    fn length_on_non_list_errors() {
        let mut e = Engine::new();
        assert!(eval_line("5 | length", &mut e).is_err());
    }

    #[test]
    fn ls_produces_table_of_temp_dir() {
        let dir = std::env::temp_dir().join(format!("ash_ls_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("one.txt"), b"hello").unwrap();
        std::fs::create_dir_all(dir.join("sub")).unwrap();

        let mut e = Engine::new();
        e.cwd = dir.clone();
        let out = eval_line("ls | get name", &mut e).unwrap();
        let Value::List(names) = out else { panic!("리스트 기대: {out:?}") };
        let names: Vec<String> = names.iter().map(|v| v.coerce_string()).collect();
        assert!(names.contains(&"one.txt".to_string()), "{names:?}");
        assert!(names.contains(&"sub".to_string()), "{names:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
```
> (`coerce_string`은 `Value`의 메서드 — `use` 불필요.)

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::builtins 2>&1 | tail -12'`
  Expected: 실패 — `ls`/`get`/`first`/`length`가 외부 명령으로 빠져 에러(또는 컴파일은 되나 테스트 실패).

- [ ] **Step 3: 구현** — `src/shellcore/builtins.rs`의 `lookup` match에 4개 케이스 추가:
```rust
        "ls" => Some(b_ls),
        "get" => Some(b_get),
        "first" => Some(b_first),
        "length" => Some(b_length),
```
그리고 `home_dir` 함수 아래(테스트 모듈 위)에 4종 구현 추가:
```rust
fn b_ls(args: &[Value], _input: Value, e: &mut Engine) -> Result<Value> {
    let dir = match args.first() {
        Some(v) => e.cwd.join(v.coerce_string()),
        None => e.cwd.clone(),
    };
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| format!("ls: 디렉터리를 읽을 수 없습니다: {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    let mut rows = Vec::new();
    for entry in entries {
        let md = entry.metadata()?;
        let ty = if md.is_dir() {
            "dir"
        } else if md.file_type().is_symlink() {
            "symlink"
        } else {
            "file"
        };
        let mut rec = OrderedMap::new();
        rec.insert("name", Value::String(entry.file_name().to_string_lossy().into_owned()));
        rec.insert("type", Value::String(ty.to_string()));
        rec.insert("size", Value::Int(md.len() as i64));
        rows.push(Value::Record(rec));
    }
    Ok(Value::List(rows))
}

fn b_get(args: &[Value], input: Value, _e: &mut Engine) -> Result<Value> {
    let field = args
        .first()
        .ok_or_else(|| anyhow!("get: 필드 이름이 필요합니다"))?
        .coerce_string();
    match input {
        Value::Record(r) => r
            .get(&field)
            .cloned()
            .ok_or_else(|| anyhow!("get: 필드 없음: {field}")),
        Value::List(items) => {
            let mut out = Vec::new();
            for it in items {
                match it {
                    Value::Record(r) => out.push(
                        r.get(&field)
                            .cloned()
                            .ok_or_else(|| anyhow!("get: 필드 없음: {field}"))?,
                    ),
                    other => bail!("get: 테이블이 아님 ({})", other.type_name()),
                }
            }
            Ok(Value::List(out))
        }
        other => bail!("get: 레코드/테이블이 아님 ({})", other.type_name()),
    }
}

fn b_first(args: &[Value], input: Value, _e: &mut Engine) -> Result<Value> {
    let n = match args.first() {
        Some(Value::Int(n)) => *n as usize,
        Some(other) => bail!("first: 정수 필요 ({})", other.type_name()),
        None => 1,
    };
    match input {
        Value::List(items) => Ok(Value::List(items.into_iter().take(n).collect())),
        other => bail!("first: 리스트가 아님 ({})", other.type_name()),
    }
}

fn b_length(_args: &[Value], input: Value, _e: &mut Engine) -> Result<Value> {
    match input {
        Value::List(items) => Ok(Value::Int(items.len() as i64)),
        other => bail!("length: 리스트가 아님 ({})", other.type_name()),
    }
}
```
파일 상단 import에 `OrderedMap` 추가 필요: `use crate::shellcore::value::{OrderedMap, Value};` 로 변경. 그리고 `use anyhow::{anyhow, bail, Context, Result};` 로 `Context` 포함.

- [ ] **Step 4: 통과 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::builtins 2>&1 | tail -10'`
  Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/builtins.rs && git commit -m "feat(shell): 데이터 빌트인 ls/get/first/length (S0 T6)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: REPL + ash 바이너리

**Files:**
- Create: `src/shellcore/repl.rs`
- Create: `src/bin/ash.rs`
- Modify: `src/shellcore/mod.rs` (`pub mod repl;`)
- Modify: `Cargo.toml` (`[[bin]] ash`)

- [ ] **Step 1: 실패 테스트** — `src/shellcore/repl.rs` 하단(루프는 I/O라 비대상; 프롬프트 헬퍼만 테스트):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn prompt_abbreviates_home() {
        let home = PathBuf::from("/home/u");
        assert_eq!(make_prompt(&PathBuf::from("/home/u/projects"), Some(&home)), "~/projects〉 ");
        assert_eq!(make_prompt(&PathBuf::from("/etc"), Some(&home)), "/etc〉 ");
        assert_eq!(make_prompt(&PathBuf::from("/home/u"), Some(&home)), "~〉 ");
    }
}
```

- [ ] **Step 2: 실패 확인** — Run: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::repl 2>&1 | tail -10'`
  Expected: 컴파일 에러(`make_prompt` 미정의).

- [ ] **Step 3: 구현** — `src/shellcore/repl.rs`:
```rust
//! REPL: 프롬프트 → stdin 한 줄 → eval_line → 결과 출력. 오류는 출력 후 루프 지속.
//! 라인에디터/히스토리/보완은 S2.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::shellcore::engine::{eval_line, Engine};
use crate::shellcore::format::format_value;
use crate::shellcore::value::Value;

/// cwd 기반 프롬프트 문자열. 홈 하위는 `~`로 축약.
fn make_prompt(cwd: &Path, home: Option<&PathBuf>) -> String {
    let shown = match home {
        Some(h) if cwd == h.as_path() => "~".to_string(),
        Some(h) if cwd.starts_with(h) => {
            format!("~/{}", cwd.strip_prefix(h).unwrap().display())
        }
        _ => cwd.display().to_string(),
    };
    format!("{shown}〉 ")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// REPL을 실행한다. EOF(Ctrl-D) 또는 `exit`로 종료.
pub fn run() -> Result<()> {
    let mut engine = Engine::new();
    let home = home_dir();
    let stdin = io::stdin();
    loop {
        print!("{}", make_prompt(&engine.cwd, home.as_ref()));
        io::stdout().flush().ok();

        let mut line = String::new();
        let n = stdin.read_line(&mut line)?;
        if n == 0 {
            println!();
            break; // EOF
        }
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        match eval_line(line, &mut engine) {
            Ok(Value::Nothing) => {}
            Ok(v) => println!("{}", format_value(&v)),
            Err(e) => eprintln!("error: {e}"),
        }
        if let Some(code) = engine.exit_code {
            std::process::exit(code);
        }
    }
    Ok(())
}
```
`src/bin/ash.rs`:
```rust
//! `ash` — AI SHell(가칭). 독립 구조화 셸 REPL 진입점.

fn main() {
    if let Err(e) = ai_terminal::shellcore::repl::run() {
        eprintln!("ash: {e}");
        std::process::exit(1);
    }
}
```
`src/shellcore/mod.rs`에 `pub mod repl;` 추가.
`Cargo.toml`의 기존 `[[bin]] name = "ai"` 블록 아래에 추가:
```toml
[[bin]]
name = "ash"
path = "src/bin/ash.rs"
```

- [ ] **Step 4: 통과 확인 + 빌드 + 스모크** —
  단위테스트: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore::repl 2>&1 | tail -6'` → `test result: ok. 1 passed`.
  REPL 스모크(stdin 파이프): `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; printf "ls | get name | first 3\nexit\n" | cargo run --bin ash 2>&1 | tail -15'`
  Expected: 표 형태로 파일명 3개 출력 후 종료(에러 없음).

- [ ] **Step 5: 커밋**
```
cd C:/workspace/act-project/terminal && git add src/shellcore/repl.rs src/bin/ash.rs src/shellcore/mod.rs Cargo.toml && git commit -m "feat(shell): REPL + ash 바이너리 (S0 T7)" -m "Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## 최종 검증 (전체 task 완료 후)

- [ ] **shellcore 전체 테스트 green**: `wsl.exe -- bash -lc 'source ~/.cargo/env; cd /mnt/c/workspace/act-project/terminal; export CARGO_TARGET_DIR=$HOME/targets/ai-terminal; cargo test shellcore 2>&1 | grep -E "test result|FAILED"'` → 모든 `ok`.
- [ ] **기존 테스트 무회귀**: `cargo test --features "storage tls remote" 2>&1 | grep -E "test result|FAILED" | tail` → 263 + 신규 shellcore, 0 failed.
- [ ] **fmt/clippy clean**: `cargo fmt --all -- --check` + `cargo clippy --all-targets -- -D warnings`(default) + `cargo clippy --bin ash -- -D warnings`.
- [ ] **ash 스모크**: `printf "[1 2 3] | length\nls | first 2\nexit\n" | cargo run --bin ash` → 정상 출력.
- [ ] **DoD 대조(스펙 §1)**: 값모델(T1)·포매터(T2)·렉서(T3)·파서(T4)·엔진/eval_line/외부(T5)·빌트인 7종(T5+T6)·REPL/ash(T7) 전부 task로 커버.

> 이후: S1(연산자 + where/each/sort + 텍스트→구조화 + 외부 stdout 캡처). 별도 spec→plan.
