//! TUI 렌더링 (설계 §5 Terminal UI Layer, M1/W2). ratatui + crossterm.
//!
//! 상태/키 처리/렌더를 분리해 `TestBackend`로 헤드리스 검증한다.
//! 실제 이벤트 루프([`run`])는 TTY가 필요해 단위 테스트하지 않는다.

use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::risk::{self, RiskLevel};

/// 키 처리 결과.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Continue,
    Quit,
    Submit(String),
}

/// TUI 상태.
#[derive(Debug, Clone)]
pub struct UiState {
    pub profile: String,
    pub cwd: String,
    pub input: String,
    pub history: Vec<String>,
}

impl UiState {
    pub fn new(profile: &str, cwd: &str) -> UiState {
        UiState {
            profile: profile.to_string(),
            cwd: cwd.to_string(),
            input: String::new(),
            history: Vec::new(),
        }
    }

    /// 입력 줄에 문자를 추가한다.
    pub fn on_char(&mut self, c: char) {
        self.input.push(c);
    }

    /// 입력 줄의 마지막 문자를 지운다.
    pub fn on_backspace(&mut self) {
        self.input.pop();
    }

    /// 현재 입력을 히스토리에 넣고 비운 뒤 그 문자열을 반환한다.
    pub fn submit(&mut self) -> String {
        let cmd = std::mem::take(&mut self.input);
        if !cmd.is_empty() {
            self.history.push(cmd.clone());
        }
        cmd
    }

    /// 현재 입력 줄의 위험 등급(로컬 평가).
    pub fn current_risk(&self) -> RiskLevel {
        risk::assess(&self.input).level
    }
}

/// 키 입력을 상태에 반영하고 다음 동작을 반환한다.
pub fn handle_key(state: &mut UiState, key: KeyCode) -> Action {
    match key {
        KeyCode::Char(c) => {
            state.on_char(c);
            Action::Continue
        }
        KeyCode::Backspace => {
            state.on_backspace();
            Action::Continue
        }
        KeyCode::Enter => Action::Submit(state.submit()),
        KeyCode::Esc => Action::Quit,
        _ => Action::Continue,
    }
}

/// 한 프레임을 렌더한다: 상태바 · 히스토리 · 입력(+위험도).
pub fn render(frame: &mut Frame, state: &UiState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(frame.area());

    let status = format!(" {} · {} ", state.profile, state.cwd);
    frame.render_widget(
        Paragraph::new(status).style(Style::default().bg(Color::Blue).fg(Color::White)),
        chunks[0],
    );

    let lines: Vec<Line> = state
        .history
        .iter()
        .map(|h| Line::from(h.clone()))
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("history")),
        chunks[1],
    );

    let input_line = format!("> {}   [risk: {:?}]", state.input, state.current_risk());
    frame.render_widget(Paragraph::new(input_line), chunks[2]);
}

/// 인터랙티브 TUI 이벤트 루프(TTY 필요). 단위 테스트 대상 아님.
///
/// 현재는 입력·위험도·히스토리만 표시한다(MVP 골격). 명령 실행/정책 게이트는 후속.
pub fn run(profile: &str) -> anyhow::Result<()> {
    use std::io::stdout;

    use crossterm::event::{self, Event, KeyEventKind, KeyModifiers};
    use crossterm::execute;
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(out))?;

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let mut state = UiState::new(profile, &cwd);

    let result = loop {
        if let Err(e) = term.draw(|f| render(f, &state)) {
            break Err(anyhow::Error::from(e));
        }
        match event::read() {
            Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => {
                let ctrl_c =
                    k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c');
                if ctrl_c || handle_key(&mut state, k.code) == Action::Quit {
                    break Ok(());
                }
            }
            Ok(_) => {}
            Err(e) => break Err(anyhow::Error::from(e)),
        }
    };

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_and_backspace_edit_input() {
        let mut s = UiState::new("balanced", "/home/u");
        for c in "rmm".chars() {
            s.on_char(c);
        }
        s.on_backspace();
        s.on_char(' ');
        s.on_char('x');
        assert_eq!(s.input, "rm x");
    }

    #[test]
    fn submit_moves_input_to_history() {
        let mut s = UiState::new("balanced", "/home/u");
        for c in "ls -al".chars() {
            s.on_char(c);
        }
        let submitted = s.submit();
        assert_eq!(submitted, "ls -al");
        assert_eq!(s.input, "");
        assert_eq!(s.history, vec!["ls -al".to_string()]);
    }

    #[test]
    fn current_risk_tracks_input() {
        let mut s = UiState::new("balanced", "/home/u");
        for c in "rm -rf /".chars() {
            s.on_char(c);
        }
        assert_eq!(s.current_risk(), RiskLevel::Critical);
    }

    #[test]
    fn render_shows_profile_and_risk() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut term = Terminal::new(TestBackend::new(80, 12)).unwrap();
        let mut s = UiState::new("paranoid", "/srv/app");
        for c in "rm -rf /".chars() {
            s.on_char(c);
        }
        term.draw(|f| render(f, &s)).unwrap();

        let text: String = term
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(
            text.contains("paranoid"),
            "status bar must show profile: {text:?}"
        );
        assert!(text.contains("Critical"), "input must show risk: {text:?}");
    }

    #[test]
    fn handle_key_maps_events() {
        let mut s = UiState::new("balanced", "/home/u");
        assert_eq!(handle_key(&mut s, KeyCode::Char('a')), Action::Continue);
        assert_eq!(s.input, "a");
        assert_eq!(handle_key(&mut s, KeyCode::Backspace), Action::Continue);
        assert_eq!(s.input, "");
        assert_eq!(handle_key(&mut s, KeyCode::Esc), Action::Quit);
        for c in "git status".chars() {
            handle_key(&mut s, KeyCode::Char(c));
        }
        assert_eq!(
            handle_key(&mut s, KeyCode::Enter),
            Action::Submit("git status".to_string())
        );
    }
}
