//! REPL: 프롬프트 → stdin 한 줄 → eval_line → 결과 출력. 오류는 출력 후 루프 지속.
//! 라인에디터/히스토리/보완은 S2.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::shellcore::engine::{eval_line, Engine};
use crate::shellcore::external::ExternalRunner;
use crate::shellcore::format::format_value;
use crate::shellcore::value::Value;

/// REPL 호스트 설정(데스크톱 config에서 주입). shellcore는 `crate::config`를 모른다.
#[derive(Debug, Clone, Default)]
pub struct ReplSettings {
    pub history_limit: usize,
    pub default_shell: Option<String>,
}

/// 주입된 설정을 엔진 상태에 매핑한다(S4 history 등이 소비).
pub(crate) fn apply_settings(engine: &mut Engine, settings: &ReplSettings) {
    engine.history_limit = settings.history_limit;
    engine.default_shell = settings.default_shell.clone();
}

/// 한 줄 읽기 결과.
#[derive(Debug)]
pub enum ReadOutcome {
    Line(String),
    Eof,
    Interrupted,
}

/// 프롬프트를 표시하고 한 줄을 읽는다. 구현은 호스트가 주입한다.
pub trait LineReader {
    fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome>;
}

/// 주입된 reader에서 한 줄을 읽어 결과로 분류한다(프롬프트 I/O 없는 순수 코어).
pub(crate) fn read_outcome_from(
    reader: &mut impl std::io::BufRead,
) -> std::io::Result<ReadOutcome> {
    let mut line = String::new();
    let n = reader.read_line(&mut line)?;
    if n == 0 {
        Ok(ReadOutcome::Eof)
    } else {
        Ok(ReadOutcome::Line(line.trim_end().to_string()))
    }
}

/// 기본 라인 reader(편집 없음). 임베드/비-TTY/테스트용. std만 사용.
pub struct StdinLineReader;

/// 입력이 AI 질의면 처리(응답 출력)하고 true, 셸이면 false를 반환한다.
pub trait AiRouter {
    fn try_handle(&mut self, input: &str) -> bool;
}

/// 기본 라우터: 항상 false(모든 입력을 셸로). 임베드/비-AI/테스트용. std만 사용.
pub struct NoAiRouter;
impl AiRouter for NoAiRouter {
    fn try_handle(&mut self, _input: &str) -> bool {
        false
    }
}
impl LineReader for StdinLineReader {
    fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome> {
        print!("{prompt}");
        io::stdout().flush().ok();
        let stdin = io::stdin();
        let mut lock = stdin.lock();
        read_outcome_from(&mut lock)
    }
}

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

/// REPL을 실행한다. 라인 reader가 EOF/Interrupt/Line을 돌려준다.
pub fn run(
    settings: ReplSettings,
    runner: Box<dyn ExternalRunner>,
    mut reader: Box<dyn LineReader>,
    mut router: Box<dyn AiRouter>,
) -> Result<()> {
    let mut engine = Engine::with_external_runner(runner);
    apply_settings(&mut engine, &settings);
    let home = crate::shellcore::util::home_dir();
    loop {
        let prompt = make_prompt(&engine.cwd, home.as_ref());
        match reader.read_line(&prompt)? {
            ReadOutcome::Eof => {
                println!();
                break;
            }
            ReadOutcome::Interrupted => continue,
            ReadOutcome::Line(line) => {
                if line.is_empty() {
                    continue;
                }
                if router.try_handle(&line) {
                    continue;
                }
                match eval_line(&line, &mut engine) {
                    Ok(Value::Nothing) => {}
                    Ok(v) => println!("{}", format_value(&v)),
                    Err(e) => eprintln!("error: {e}"),
                }
                if let Some(code) = engine.exit_code {
                    std::process::exit(code);
                }
            }
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

    #[test]
    fn repl_settings_default_is_neutral() {
        let s = ReplSettings::default();
        assert_eq!(s.history_limit, 0);
        assert_eq!(s.default_shell, None);
    }

    #[test]
    fn apply_settings_maps_onto_engine() {
        let mut engine = crate::shellcore::engine::Engine::new();
        let settings = ReplSettings {
            history_limit: 99,
            default_shell: Some("/bin/zsh".to_string()),
        };
        apply_settings(&mut engine, &settings);
        assert_eq!(engine.history_limit, 99);
        assert_eq!(engine.default_shell.as_deref(), Some("/bin/zsh"));
    }

    #[test]
    fn read_outcome_eof_on_empty_input() {
        let mut c = std::io::Cursor::new(&b""[..]);
        assert!(matches!(
            read_outcome_from(&mut c).unwrap(),
            ReadOutcome::Eof
        ));
    }

    #[test]
    fn read_outcome_line_trims_newline() {
        let mut c = std::io::Cursor::new(&b"echo hi\n"[..]);
        match read_outcome_from(&mut c).unwrap() {
            ReadOutcome::Line(l) => assert_eq!(l, "echo hi"),
            other => panic!("expected Line, got {other:?}"),
        }
    }

    #[test]
    fn no_ai_router_never_handles() {
        let mut r = NoAiRouter;
        assert!(!r.try_handle("ls -al"));
        assert!(!r.try_handle("how do I undo a commit?"));
    }
}
