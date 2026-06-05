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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn prompt_abbreviates_home() {
        let home = PathBuf::from("/home/u");
        assert_eq!(
            make_prompt(&PathBuf::from("/home/u/projects"), Some(&home)),
            "~/projects〉 "
        );
        assert_eq!(make_prompt(&PathBuf::from("/etc"), Some(&home)), "/etc〉 ");
        assert_eq!(make_prompt(&PathBuf::from("/home/u"), Some(&home)), "~〉 ");
    }
}
