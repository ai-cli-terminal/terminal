//! 빌트인 레지스트리 + 구현. 시그니처: (args 평가값, 파이프라인 입력, engine) -> Value.

use anyhow::{anyhow, bail, Context, Result};

use crate::shellcore::engine::Engine;
use crate::shellcore::format::format_value;
use crate::shellcore::value::{OrderedMap, Value};

pub type Builtin = fn(&[Value], Value, &mut Engine) -> Result<Value>;

/// 이름으로 빌트인을 찾는다. (S0 T5: print/echo/cd/exit. T6에서 ls/get/first/length 추가.)
pub fn lookup(name: &str) -> Option<Builtin> {
    match name {
        "print" | "echo" => Some(b_print),
        "cd" => Some(b_cd),
        "exit" => Some(b_exit),
        "ls" => Some(b_ls),
        "get" => Some(b_get),
        "first" => Some(b_first),
        "length" => Some(b_length),
        "each" => Some(b_each),
        _ => None,
    }
}

fn b_print(args: &[Value], input: Value, _e: &mut Engine) -> Result<Value> {
    if args.is_empty() {
        println!("{}", format_value(&input));
    } else {
        let parts: Vec<String> = args.iter().map(format_value).collect();
        println!("{}", parts.join(" "));
    }
    Ok(Value::Nothing)
}

fn b_cd(args: &[Value], _input: Value, e: &mut Engine) -> Result<Value> {
    let target = match args.first() {
        Some(v) => e.cwd.join(v.coerce_string()),
        None => crate::shellcore::util::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")),
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
        rec.insert(
            "name",
            Value::String(entry.file_name().to_string_lossy().into_owned()),
        );
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

fn b_length(_args: &[Value], input: Value, _e: &mut Engine) -> Result<Value> {
    match input {
        Value::List(items) => Ok(Value::Int(items.len() as i64)),
        other => bail!("length: 리스트가 아님 ({})", other.type_name()),
    }
}

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

#[cfg(test)]
mod tests {
    use crate::shellcore::engine::{eval_line, Engine};
    use crate::shellcore::value::Value;

    #[test]
    fn get_first_length_over_table_literal() {
        let mut e = Engine::new();
        assert_eq!(
            eval_line("[{name: a} {name: b} {name: c}] | get name", &mut e).unwrap(),
            Value::List(vec![
                Value::String("a".into()),
                Value::String("b".into()),
                Value::String("c".into())
            ])
        );
        assert_eq!(
            eval_line("[1 2 3] | length", &mut e).unwrap(),
            Value::Int(3)
        );
        assert_eq!(
            eval_line("[1 2 3] | first 2", &mut e).unwrap(),
            Value::List(vec![Value::Int(1), Value::Int(2)])
        );
        assert_eq!(
            eval_line("[1 2 3] | first", &mut e).unwrap(),
            Value::List(vec![Value::Int(1)])
        );
    }

    #[test]
    fn get_field_from_record() {
        let mut e = Engine::new();
        assert_eq!(
            eval_line("{a: 1, b: 2} | get b", &mut e).unwrap(),
            Value::Int(2)
        );
        assert!(eval_line("{a: 1} | get zzz", &mut e).is_err());
    }

    #[test]
    fn length_on_non_list_errors() {
        let mut e = Engine::new();
        assert!(eval_line("5 | length", &mut e).is_err());
    }

    #[test]
    fn first_rejects_negative() {
        let mut e = Engine::new();
        assert!(eval_line("[1 2 3] | first -1", &mut e).is_err());
        assert_eq!(
            eval_line("[1 2 3] | first 0", &mut e).unwrap(),
            Value::List(vec![])
        );
    }

    #[test]
    fn print_joins_multiple_args() {
        let mut e = Engine::new();
        assert_eq!(eval_line("print 1 2 3", &mut e).unwrap(), Value::Nothing);
    }

    #[test]
    fn each_maps_closure_over_list() {
        let mut e = Engine::new();
        assert_eq!(
            eval_line("[{name: a} {name: b}] | each { $it.name }", &mut e).unwrap(),
            Value::List(vec![Value::String("a".into()), Value::String("b".into())])
        );
        assert_eq!(
            eval_line("[{a: {b: 9}}] | each { $it.a.b }", &mut e).unwrap(),
            Value::List(vec![Value::Int(9)])
        );
        assert_eq!(
            eval_line("[1 2 3] | each { $it }", &mut e).unwrap(),
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
        assert_eq!(
            eval_line("[{xs: [10 20 30]}] | each { $it.xs.1 }", &mut e).unwrap(),
            Value::List(vec![Value::Int(20)])
        );
    }

    #[test]
    fn each_errors_on_non_closure_or_non_list() {
        let mut e = Engine::new();
        assert!(eval_line("[1 2] | each 5", &mut e).is_err());
        assert!(eval_line("5 | each { $it }", &mut e).is_err());
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
        let Value::List(names) = out else {
            panic!("리스트 기대: {out:?}")
        };
        let names: Vec<String> = names.iter().map(|v| v.coerce_string()).collect();
        assert!(names.contains(&"one.txt".to_string()), "{names:?}");
        assert!(names.contains(&"sub".to_string()), "{names:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
