//! 외부 명령 실행: 빌트인 아닌 이름을 PATH 바이너리로 stdio 상속 spawn(대화형 정상).

use anyhow::{bail, Result};

use crate::shellcore::engine::Engine;
use crate::shellcore::value::Value;

/// 외부 명령을 셸 cwd·현재 env로 실행한다. stdout/stderr는 터미널로 통과.
/// 반환은 Nothing. 비0 종료는 안내만 하고 에러로 만들지 않는다(REPL 지속).
pub fn run(name: &str, args: &[Value], engine: &mut Engine) -> Result<Value> {
    use std::process::Command;
    let arg_strs: Vec<String> = args.iter().map(|v| v.coerce_string()).collect();
    match Command::new(name)
        .args(&arg_strs)
        .current_dir(&engine.cwd)
        .status()
    {
        Ok(st) => {
            if !st.success() {
                let code = st
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".into());
                eprintln!("[{name}: exit {code}]");
            }
            Ok(Value::Nothing)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => bail!("command not found: {name}"),
        Err(e) => bail!("failed to run {name}: {e}"),
    }
}
