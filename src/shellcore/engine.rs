//! 엔진: 스코프(cwd/vars) + 표현식/파이프라인 평가 + eval_line(테스트 진입점).

use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, bail, Result};

use crate::shellcore::ast::{Expr, Pipeline, Stage, Stmt};
use crate::shellcore::value::{OrderedMap, Value};
use crate::shellcore::{builtins, external, lexer, ops, parser};

/// 셸 실행 상태.
pub struct Engine {
    pub cwd: PathBuf,
    pub vars: OrderedMap,
    pub exit_code: Option<i32>,
    pub workspace_root: Option<PathBuf>,
    pub history_limit: usize,
    pub default_shell: Option<String>,
    external_runner: Box<dyn external::ExternalRunner>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::with_external_runner(Box::new(external::DesktopRunner))
    }

    /// Spawn 없는 순수 `shellcore` 모드. 모바일 FFI·WASM·단위 테스트에서 사용한다.
    pub fn pure() -> Self {
        Self::with_external_runner(Box::new(external::DisabledRunner))
    }

    pub fn with_external_runner(runner: Box<dyn external::ExternalRunner>) -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            vars: OrderedMap::new(),
            exit_code: None,
            workspace_root: None,
            history_limit: 0,
            default_shell: None,
            external_runner: runner,
        }
    }

    pub fn execution_capabilities(&self) -> external::ExecutionCapabilities {
        self.external_runner.capabilities()
    }

    pub fn set_workspace_root(&mut self, root: PathBuf) {
        let root = normalize_existing_or_raw(root);
        self.workspace_root = Some(root.clone());
        if !path_is_within(&self.cwd, &root) {
            self.cwd = root;
        }
    }

    pub fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    pub fn resolve_workspace_path(&self, raw: &str) -> Result<PathBuf> {
        let requested = PathBuf::from(raw);
        let candidate = if requested.is_absolute() {
            requested
        } else {
            self.cwd.join(requested)
        };
        let resolved = normalize_existing_or_raw(candidate);
        if let Some(root) = &self.workspace_root {
            if !path_is_within(&resolved, root) {
                bail!(
                    "workspace boundary: path is outside workspace root: {}",
                    resolved.display()
                );
            }
        }
        Ok(resolved)
    }

    pub fn default_directory(&self) -> PathBuf {
        self.workspace_root
            .clone()
            .or_else(crate::shellcore::util::home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

fn normalize_existing_or_raw(path: PathBuf) -> PathBuf {
    path.canonicalize()
        .unwrap_or_else(|_| normalize_lexically(&path))
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
            Component::RootDir | Component::Prefix(_) => out.push(component.as_os_str()),
        }
    }
    if out.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        out
    }
}

fn path_is_within(path: &Path, root: &Path) -> bool {
    let path = normalize_existing_or_raw(path.to_path_buf());
    let root = normalize_existing_or_raw(root.to_path_buf());
    path.starts_with(root)
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
                    engine.external_runner.run(external::ExternalCommand {
                        name: &c.name,
                        args: &args,
                        cwd: &engine.cwd,
                    })?
                }
            }
        };
    }
    Ok(input)
}

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

    #[test]
    fn pure_engine_disables_external_spawn_but_keeps_core_eval() {
        let mut e = Engine::pure();
        assert_eq!(
            e.execution_capabilities(),
            external::ExecutionCapabilities::pure_core()
        );
        assert_eq!(
            eval_line(
                "[{size: 50} {size: 200}] | where size > 100 | length",
                &mut e
            )
            .unwrap(),
            Value::Int(1)
        );
        let err = eval_line("definitely-not-a-builtin", &mut e)
            .unwrap_err()
            .to_string();
        assert!(err.contains("external execution disabled"), "{err}");
    }

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
        let out = eval_line(
            "[{type: \"dir\"} {type: \"file\"}] | where type == \"dir\" | length",
            &mut e,
        )
        .unwrap();
        assert_eq!(out, Value::Int(1));
        eval_line("let limit = 100", &mut e).unwrap();
        let out = eval_line("[{size: 200}] | where size > $limit | length", &mut e).unwrap();
        assert_eq!(out, Value::Int(1));
        let out = eval_line(
            "[{a: 1} {a: 2} {a: 3}] | where a == 1 or a == 3 | length",
            &mut e,
        )
        .unwrap();
        assert_eq!(out, Value::Int(2));
    }

    #[test]
    fn where_errors_are_clean() {
        let mut e = Engine::new();
        assert!(eval_line("[{size: 1}] | where size", &mut e).is_err());
        assert!(eval_line("[{size: 1}] | where nope > 0", &mut e).is_err());
        assert!(eval_line("5 | where x > 1", &mut e).is_err());
    }

    #[test]
    fn comparison_expression_value() {
        let mut e = Engine::new();
        assert_eq!(eval_line("3 > 2", &mut e).unwrap(), Value::Bool(true));
        assert_eq!(
            eval_line("\"a\" < \"b\"", &mut e).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn engine_settings_fields_default_neutral() {
        let e = Engine::new();
        assert_eq!(e.history_limit, 0);
        assert_eq!(e.default_shell, None);
    }
}
