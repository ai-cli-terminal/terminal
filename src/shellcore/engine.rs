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
                let args: Vec<Value> = c
                    .args
                    .iter()
                    .map(|a| eval_expr(a, engine))
                    .collect::<Result<_>>()?;
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
            let vals: Vec<Value> = items
                .iter()
                .map(|x| eval_expr(x, engine))
                .collect::<Result<_>>()?;
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
