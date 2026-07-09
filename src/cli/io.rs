pub(crate) struct StdoutSink;
impl ai_terminal::pipeline::OutputSink for StdoutSink {
    fn write(&mut self, chunk: &str) {
        print!("{chunk}");
    }
}

pub(crate) struct AutoYes;
impl ai_terminal::pipeline::Confirmer for AutoYes {
    fn confirm(&mut self, _: &ai_terminal::pipeline::ConfirmRequest) -> bool {
        true
    }
}

pub(crate) struct StdinConfirmer;
impl ai_terminal::pipeline::Confirmer for StdinConfirmer {
    fn confirm(&mut self, req: &ai_terminal::pipeline::ConfirmRequest) -> bool {
        use std::io::Write;
        eprintln!("위험 등급 {:?} 명령: {}", req.level, req.command);
        for f in &req.factors {
            eprintln!("  - {f}");
        }
        if !req.backup_files.is_empty() {
            eprintln!("  백업 대상: {}", req.backup_files.join(", "));
        }
        eprint!("실행할까요? [y/N] ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_err() {
            return false;
        }
        matches!(line.trim(), "y" | "Y" | "yes")
    }
}
