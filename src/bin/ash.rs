//! `ash` — AI SHell(가칭). 독립 구조화 셸 REPL 진입점.

fn main() {
    let loaded = ai_terminal::config::load();
    if let Some(warning) = &loaded.warning {
        eprintln!("ash: {warning}");
    }
    let settings = ai_terminal::shellcore::repl::ReplSettings {
        history_limit: loaded.config.general.history_limit,
        default_shell: loaded.config.general.default_shell.clone(),
    };
    if let Err(e) = ai_terminal::shellcore::repl::run(settings) {
        eprintln!("ash: {e}");
        std::process::exit(1);
    }
}
