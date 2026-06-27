//! reedline 기반 라인 에디터(데스크톱 호스트 계층). shellcore는 이 모듈을 모른다(경계 유지).

use std::borrow::Cow;
use std::path::PathBuf;

use reedline::{
    FileBackedHistory, History, HistoryItem, HistoryItemId, HistorySessionId, Prompt,
    PromptEditMode, PromptHistorySearch, Reedline, SearchQuery, Signal,
};

use crate::shellcore::repl::{LineReader, ReadOutcome};

/// reedline History 래퍼: 민감명령은 저장에서 제외하고 나머지는 inner에 위임한다.
pub(crate) struct FilteringHistory {
    pub(crate) inner: FileBackedHistory,
}

impl History for FilteringHistory {
    fn save(&mut self, h: HistoryItem) -> reedline::Result<HistoryItem> {
        if is_sensitive_command(&h.command_line) {
            Ok(h) // 영속하지 않음(추가 안 함)
        } else {
            self.inner.save(h)
        }
    }
    fn load(&self, id: HistoryItemId) -> reedline::Result<HistoryItem> {
        self.inner.load(id)
    }
    fn count(&self, query: SearchQuery) -> reedline::Result<i64> {
        self.inner.count(query)
    }
    fn count_all(&self) -> reedline::Result<i64> {
        self.inner.count_all()
    }
    fn search(&self, query: SearchQuery) -> reedline::Result<Vec<HistoryItem>> {
        self.inner.search(query)
    }
    fn update(
        &mut self,
        id: HistoryItemId,
        updater: &dyn Fn(HistoryItem) -> HistoryItem,
    ) -> reedline::Result<()> {
        self.inner.update(id, updater)
    }
    fn clear(&mut self) -> reedline::Result<()> {
        self.inner.clear()
    }
    fn delete(&mut self, h: HistoryItemId) -> reedline::Result<()> {
        self.inner.delete(h)
    }
    fn sync(&mut self) -> std::io::Result<()> {
        self.inner.sync()
    }
    fn session(&self) -> Option<HistorySessionId> {
        self.inner.session()
    }
}

/// reedline Signal → ReadOutcome. CtrlC=취소(Interrupted), CtrlD=EOF.
pub(crate) fn map_signal(sig: Signal) -> ReadOutcome {
    match sig {
        Signal::Success(line) => ReadOutcome::Line(line),
        Signal::CtrlD => ReadOutcome::Eof,
        Signal::CtrlC => ReadOutcome::Interrupted,
        _ => ReadOutcome::Interrupted,
    }
}

/// 명령 텍스트에 secret/PII가 탐지되면 true → history 저장에서 제외한다.
pub(crate) fn is_sensitive_command(cmd: &str) -> bool {
    !crate::mask::Masker::baseline()
        .mask(cmd)
        .redactions
        .is_empty()
}

/// repl이 만든 프롬프트 문자열을 그대로 렌더하는 reedline Prompt.
struct AshPrompt {
    text: String,
}
impl AshPrompt {
    fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
        }
    }
}
impl Prompt for AshPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Owned(self.text.clone())
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("::: ")
    }
    fn render_prompt_history_search_indicator(&self, _hs: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
}

/// reedline 기반 라인 에디터(편집·in-session history·Ctrl-C/D).
pub struct ReedlineReader {
    editor: Reedline,
}
impl ReedlineReader {
    /// 파일 영속 history로 에디터를 만든다. capacity=0이면 영속 없이 메모리 history만.
    /// history 파일 로드 실패는 fail-soft(메모리 history + 경고).
    pub fn with_history(path: PathBuf, capacity: usize) -> anyhow::Result<Self> {
        let editor = if capacity == 0 {
            Reedline::create()
        } else {
            match FileBackedHistory::with_file(capacity, path) {
                Ok(fbh) => {
                    Reedline::create().with_history(Box::new(FilteringHistory { inner: fbh }))
                }
                Err(e) => {
                    eprintln!("ash: history 파일 로드 실패({e}) — 메모리 history 사용");
                    let mem = FileBackedHistory::new(capacity)?;
                    Reedline::create().with_history(Box::new(FilteringHistory { inner: mem }))
                }
            }
        };
        Ok(Self { editor })
    }
}
impl LineReader for ReedlineReader {
    fn read_line(&mut self, prompt: &str) -> std::io::Result<ReadOutcome> {
        let p = AshPrompt::new(prompt);
        self.editor
            .read_line(&p)
            .map(map_signal)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shellcore::repl::ReadOutcome;
    use reedline::Signal;

    #[test]
    fn map_signal_success_to_line() {
        match map_signal(Signal::Success("x".to_string())) {
            ReadOutcome::Line(l) => assert_eq!(l, "x"),
            o => panic!("expected Line, got {o:?}"),
        }
    }

    #[test]
    fn map_signal_ctrld_is_eof_and_ctrlc_is_interrupt() {
        assert!(matches!(map_signal(Signal::CtrlD), ReadOutcome::Eof));
        assert!(matches!(
            map_signal(Signal::CtrlC),
            ReadOutcome::Interrupted
        ));
    }

    #[test]
    fn ash_prompt_left_returns_injected_text() {
        let p = AshPrompt::new("~/x〉 ");
        assert_eq!(p.render_prompt_left(), "~/x〉 ");
    }

    #[test]
    fn sensitive_command_detects_secrets() {
        assert!(is_sensitive_command(
            "git push ghp_aaaaaaaaaaaaaaaaaaaaaaaa"
        ));
        assert!(is_sensitive_command("export PASSWORD=hunter2"));
    }

    #[test]
    fn sensitive_command_allows_plain_commands() {
        assert!(!is_sensitive_command("ls -al"));
        assert!(!is_sensitive_command("echo hi"));
    }

    #[test]
    fn filtering_history_persists_only_nonsensitive() {
        use reedline::{FileBackedHistory, History, HistoryItem, SearchDirection, SearchQuery};
        let path = std::env::temp_dir().join(format!("ash_hist_test_{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let mut fh = FilteringHistory {
                inner: FileBackedHistory::with_file(100, path.clone()).unwrap(),
            };
            fh.save(HistoryItem::from_command_line("ls -al")).unwrap();
            fh.save(HistoryItem::from_command_line(
                "git push ghp_aaaaaaaaaaaaaaaaaaaaaaaa",
            ))
            .unwrap();
            fh.sync().unwrap();
        }
        let reloaded = FileBackedHistory::with_file(100, path.clone()).unwrap();
        let all = reloaded
            .search(SearchQuery::everything(SearchDirection::Backward, None))
            .unwrap();
        let cmds: Vec<String> = all.into_iter().map(|i| i.command_line).collect();
        assert!(cmds.iter().any(|c| c == "ls -al"), "{cmds:?}");
        assert!(!cmds.iter().any(|c| c.contains("ghp_")), "{cmds:?}");
        let _ = std::fs::remove_file(&path);
    }
}
