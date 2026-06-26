//! reedline 기반 라인 에디터(데스크톱 호스트 계층). shellcore는 이 모듈을 모른다(경계 유지).

use std::borrow::Cow;

use reedline::{Prompt, PromptEditMode, PromptHistorySearch, Reedline, Signal};

use crate::shellcore::repl::{LineReader, ReadOutcome};

/// reedline Signal → ReadOutcome. CtrlC=취소(Interrupted), CtrlD=EOF.
pub(crate) fn map_signal(sig: Signal) -> ReadOutcome {
    match sig {
        Signal::Success(line) => ReadOutcome::Line(line),
        Signal::CtrlD => ReadOutcome::Eof,
        Signal::CtrlC => ReadOutcome::Interrupted,
        _ => ReadOutcome::Interrupted,
    }
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
    /// 실패 시 호출측이 StdinLineReader로 폴백할 수 있게 Result 반환.
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            editor: Reedline::create(),
        })
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
}
