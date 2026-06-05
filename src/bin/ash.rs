//! `ash` — AI SHell(가칭). 독립 구조화 셸 REPL 진입점.

fn main() {
    if let Err(e) = ai_terminal::shellcore::repl::run() {
        eprintln!("ash: {e}");
        std::process::exit(1);
    }
}
