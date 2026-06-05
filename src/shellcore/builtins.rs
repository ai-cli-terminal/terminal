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
