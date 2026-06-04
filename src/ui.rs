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

    /// 명령 실행 출력을 히스토리에 줄 단위로 추가한다(PTY 출력 표시용).
    pub fn append_output(&mut self, output: &str) {
        for line in output.lines() {
            self.history.push(line.to_string());
        }
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

/// TUI 출력 싱크: pipeline 출력을 문자열로 모은다.
struct StringSink(String);
impl crate::pipeline::OutputSink for StringSink {
    fn write(&mut self, c: &str) {
        self.0.push_str(c);
    }
}

/// TUI 확인기: 이번 증분은 확인이 필요한(위험) 명령을 거부하고 안내한다.
/// Allow 등급 명령은 pipeline이 확인을 호출하지 않으므로 그대로 실행된다.
struct TuiDeny;
impl crate::pipeline::Confirmer for TuiDeny {
    fn confirm(&mut self, _: &crate::pipeline::ConfirmRequest) -> bool {
        false
    }
}

/// 통합 실행 결과 + 누적 출력을 TUI 표시 문자열로 만든다(순수). `output`은 sink에
/// 누적된 셸/AI 출력.
fn render_output(handled: &crate::dispatch::Handled, output: String) -> String {
    use crate::dispatch::{AiOutcome, Handled};
    use crate::pipeline::ExecOutcome;
    match handled {
        Handled::Empty => String::new(),
        Handled::Shell(ExecOutcome::Ran { exit_code, .. }) => {
            if *exit_code != 0 {
                format!("{output}[exit {exit_code}]\n")
            } else {
                output
            }
        }
        Handled::Shell(ExecOutcome::Blocked { level, .. }) => {
            format!("[차단됨: 위험 등급 {level:?} — 정책상 실행 불가]\n")
        }
        Handled::Shell(ExecOutcome::Declined) => {
            "[위험 명령 — 터미널에서 `ai exec --yes`로 실행하세요]\n".to_string()
        }
        Handled::Shell(ExecOutcome::BackupRefused(r)) => {
            format!("[백업 거부로 실행 중단: {r}]\n")
        }
        Handled::Ai(AiOutcome::Answered { .. }) => output,
        Handled::Ai(AiOutcome::Blocked(r)) => format!("{output}[차단: {r}]\n"),
        Handled::Ai(AiOutcome::Unavailable(e)) => format!("{output}[AI 사용 불가: {e}]\n"),
    }
}

/// 스트리밍 셸 실행의 **상태 꼬리**만 만든다(출력 본문은 이미 라이브로 표시됨).
/// `render_output`과 달리 출력을 다시 붙이지 않아 이중 표시를 막는다.
fn render_shell_tail(outcome: &crate::pipeline::ExecOutcome) -> String {
    use crate::pipeline::ExecOutcome;
    match outcome {
        ExecOutcome::Ran { exit_code: 0, .. } => String::new(),
        ExecOutcome::Ran { exit_code: 130, .. } => "[중단됨 (exit 130)]\n".to_string(),
        ExecOutcome::Ran { exit_code, .. } => format!("[exit {exit_code}]\n"),
        ExecOutcome::Blocked { level, .. } => {
            format!("[차단됨: 위험 등급 {level:?} — 정책상 실행 불가]\n")
        }
        ExecOutcome::Declined => {
            "[위험 명령 — 터미널에서 `ai exec --yes`로 실행하세요]\n".to_string()
        }
        ExecOutcome::BackupRefused(r) => format!("[백업 거부로 실행 중단: {r}]\n"),
    }
}

/// TUI 출력 싱크(채널형): 워커 스레드가 청크를 메인 루프로 보낸다(라이브 표시).
struct ChannelSink(std::sync::mpsc::Sender<String>);
impl crate::pipeline::OutputSink for ChannelSink {
    fn write(&mut self, c: &str) {
        // 메인 루프가 종료돼 수신단이 닫혔으면 무시(best-effort).
        let _ = self.0.send(c.to_string());
    }
}

/// 취소 가능 PTY 실행기(TUI 전용). `cancel` 플래그로 mid-exec 중단을 지원한다.
struct CancellableExecutor {
    shell: String,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl crate::pipeline::Executor for CancellableExecutor {
    fn run(
        &self,
        command: &str,
        sink: &mut dyn crate::pipeline::OutputSink,
    ) -> anyhow::Result<i32> {
        crate::pty::run_in_pty_streaming_cancellable(
            &self.shell,
            command,
            self.cancel.clone(),
            |c| sink.write(c),
        )
    }
}

/// 셸 명령을 워커 스레드에서 취소 가능하게 실행하며, 메인 루프는 출력 청크를 라이브로
/// 표시하고 Esc/Ctrl+C로 중단을 요청한다(§31.5/§16.2). 게이트(위험도·정책·백업)는
/// `pipeline::execute`가 워커에서 함께 수행한다. AI 경로는 호출측(메인 스레드)이 처리한다.
fn run_shell_streaming<B: ratatui::backend::Backend>(
    term: &mut ratatui::Terminal<B>,
    state: &mut UiState,
    command: &str,
    profile: &crate::policy::PolicyProfile,
    shell: &str,
) -> anyhow::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

    let undo_dir = match crate::undo::default_undo_dir() {
        Ok(d) => d,
        Err(e) => {
            state.append_output(&format!("error: undo 디렉터리: {e}\n"));
            return Ok(());
        }
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    let outcome = std::thread::scope(|s| -> anyhow::Result<crate::pipeline::ExecOutcome> {
        let worker = {
            let cancel = cancel.clone();
            s.spawn(move || {
                let executor = CancellableExecutor {
                    shell: shell.to_string(),
                    cancel,
                };
                let mut confirm = TuiDeny;
                let mut sink = ChannelSink(tx); // tx를 워커로 이동 → 완료 시 rx 연결 해제
                let cfg = crate::pipeline::ExecConfig {
                    profile,
                    undo_dir: &undo_dir,
                    limits: crate::undo::UndoLimits::defaults(),
                };
                crate::pipeline::execute(command, &cfg, &executor, &mut confirm, &mut sink)
            })
        };

        loop {
            let mut got = false;
            while let Ok(chunk) = rx.try_recv() {
                state.append_output(&chunk);
                got = true;
            }
            if got {
                let _ = term.draw(|f| render(f, state));
            }
            if worker.is_finished() {
                break;
            }
            if event::poll(Duration::from_millis(20)).unwrap_or(false) {
                if let Ok(Event::Key(k)) = event::read() {
                    if k.kind == KeyEventKind::Press
                        && ((k.modifiers.contains(KeyModifiers::CONTROL)
                            && k.code == KeyCode::Char('c'))
                            || k.code == KeyCode::Esc)
                    {
                        cancel.store(true, Ordering::SeqCst);
                        state.append_output("[중단 요청…]\n");
                        let _ = term.draw(|f| render(f, state));
                    }
                }
            }
        }
        // 남은 청크 drain.
        while let Ok(chunk) = rx.try_recv() {
            state.append_output(&chunk);
        }
        worker
            .join()
            .map_err(|_| anyhow::anyhow!("실행 워커 패닉"))?
    });

    match outcome {
        Ok(o) => state.append_output(&render_shell_tail(&o)),
        Err(e) => state.append_output(&format!("error: {e}\n")),
    }
    Ok(())
}

/// 인터랙티브 TUI 이벤트 루프(TTY 필요). 단위 테스트 대상 아님.
///
/// 제출된 명령은 중앙 실행 파이프라인(위험도·정책·백업·실행)을 거친다.
pub fn run(profile: &str) -> anyhow::Result<()> {
    use std::io::stdout;

    use crossterm::event::{self, Event, KeyEventKind, KeyModifiers};
    use crossterm::execute;
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    use ratatui::backend::CrosstermBackend;
    use ratatui::Terminal;

    use crate::dispatch::AiResponder;

    // AI 응답기는 tokio 런타임을 보유하므로 루프 밖에서 1회만 만든다.
    let mut ai = crate::responder::GatewayResponder::mock()?;

    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let mut term = Terminal::new(CrosstermBackend::new(out))?;

    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let mut state = UiState::new(profile, &cwd);

    let result = loop {
        if let Err(e) = term.draw(|f| render(f, &state)) {
            break Err(anyhow::Error::from(e));
        }
        match event::read() {
            Ok(Event::Key(k)) if k.kind == KeyEventKind::Press => {
                if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c') {
                    break Ok(());
                }
                match handle_key(&mut state, k.code) {
                    Action::Quit => break Ok(()),
                    Action::Submit(cmd) if !cmd.trim().is_empty() => {
                        // 메인 스레드에서 분류: 셸은 워커에서 라이브 스트리밍·중단 가능,
                        // AI는 메인에서 동기 처리(타임아웃 상한; GatewayResponder Send 비보장).
                        let prof = crate::policy::PolicyProfile::by_name(profile)
                            .unwrap_or_else(crate::policy::PolicyProfile::balanced);
                        match crate::dispatch::dispatch(&cmd, &prof) {
                            crate::dispatch::Route::Empty => {}
                            crate::dispatch::Route::Shell { command, .. } => {
                                if let Err(e) = run_shell_streaming(
                                    &mut term, &mut state, &command, &prof, &shell,
                                ) {
                                    state.append_output(&format!("error: {e}\n"));
                                }
                            }
                            crate::dispatch::Route::Ai { prompt } => {
                                let mut buf = StringSink(String::new());
                                let msg = match ai.respond(&prompt, &mut buf) {
                                    Ok(out) => {
                                        render_output(&crate::dispatch::Handled::Ai(out), buf.0)
                                    }
                                    Err(e) => format!("error: {e}\n"),
                                };
                                state.append_output(&msg);
                            }
                        }
                    }
                    _ => {}
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
    fn append_output_adds_lines_to_history() {
        let mut s = UiState::new("balanced", "/home/u");
        s.append_output("line1\nline2\n");
        assert_eq!(s.history, vec!["line1".to_string(), "line2".to_string()]);
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

    #[test]
    fn render_output_shell_ran_zero_passthrough() {
        use crate::dispatch::Handled;
        use crate::pipeline::ExecOutcome;
        let h = Handled::Shell(ExecOutcome::Ran {
            exit_code: 0,
            undo_id: None,
        });
        assert_eq!(render_output(&h, "hi\n".into()), "hi\n");
    }

    #[test]
    fn render_output_shell_ran_nonzero_appends_exit() {
        use crate::dispatch::Handled;
        use crate::pipeline::ExecOutcome;
        let h = Handled::Shell(ExecOutcome::Ran {
            exit_code: 3,
            undo_id: None,
        });
        assert_eq!(render_output(&h, "x".into()), "x[exit 3]\n");
    }

    #[test]
    fn render_output_ai_answered_passthrough() {
        use crate::dispatch::{AiOutcome, Handled};
        let h = Handled::Ai(AiOutcome::Answered {
            text: "ans".into(),
            input_tokens: 1,
            output_tokens: 1,
            source: crate::cache::CacheSource::Backend,
        });
        assert_eq!(render_output(&h, "ans".into()), "ans");
    }

    #[test]
    fn render_output_ai_unavailable_appends_note() {
        use crate::dispatch::{AiOutcome, Handled};
        let h = Handled::Ai(AiOutcome::Unavailable("timeout".into()));
        assert_eq!(
            render_output(&h, String::new()),
            "[AI 사용 불가: timeout]\n"
        );
    }

    #[test]
    fn render_output_empty_is_blank() {
        use crate::dispatch::Handled;
        assert_eq!(render_output(&Handled::Empty, String::new()), "");
    }

    #[test]
    fn render_output_shell_blocked_shows_block_note() {
        use crate::dispatch::Handled;
        use crate::pipeline::ExecOutcome;
        use crate::risk::RiskLevel;
        let h = Handled::Shell(ExecOutcome::Blocked {
            level: RiskLevel::Critical,
            factors: vec![],
        });
        assert_eq!(
            render_output(&h, String::new()),
            "[차단됨: 위험 등급 Critical — 정책상 실행 불가]\n"
        );
    }

    #[test]
    fn render_output_shell_declined_shows_hint() {
        use crate::dispatch::Handled;
        use crate::pipeline::ExecOutcome;
        let h = Handled::Shell(ExecOutcome::Declined);
        assert_eq!(
            render_output(&h, String::new()),
            "[위험 명령 — 터미널에서 `ai exec --yes`로 실행하세요]\n"
        );
    }

    #[test]
    fn render_shell_tail_ran_zero_is_empty() {
        use crate::pipeline::ExecOutcome;
        assert_eq!(
            render_shell_tail(&ExecOutcome::Ran {
                exit_code: 0,
                undo_id: None
            }),
            ""
        );
    }

    #[test]
    fn render_shell_tail_cancel_shows_interrupted() {
        use crate::pipeline::ExecOutcome;
        // 취소는 executor가 exit 130을 돌려준다 → 중단 안내.
        let t = render_shell_tail(&ExecOutcome::Ran {
            exit_code: 130,
            undo_id: None,
        });
        assert!(t.contains("중단"), "{t:?}");
        assert!(t.contains("130"), "{t:?}");
    }

    #[test]
    fn render_shell_tail_nonzero_shows_exit() {
        use crate::pipeline::ExecOutcome;
        assert_eq!(
            render_shell_tail(&ExecOutcome::Ran {
                exit_code: 2,
                undo_id: None
            }),
            "[exit 2]\n"
        );
    }

    #[test]
    fn render_shell_tail_blocked_and_declined() {
        use crate::pipeline::ExecOutcome;
        use crate::risk::RiskLevel;
        assert!(render_shell_tail(&ExecOutcome::Blocked {
            level: RiskLevel::Critical,
            factors: vec![]
        })
        .contains("차단"));
        assert!(render_shell_tail(&ExecOutcome::Declined).contains("ai exec"));
        assert!(
            render_shell_tail(&ExecOutcome::BackupRefused("too big".into())).contains("too big")
        );
    }

    #[test]
    fn render_output_shell_backup_refused_shows_reason() {
        use crate::dispatch::Handled;
        use crate::pipeline::ExecOutcome;
        let h = Handled::Shell(ExecOutcome::BackupRefused("too big".into()));
        assert_eq!(
            render_output(&h, String::new()),
            "[백업 거부로 실행 중단: too big]\n"
        );
    }
}
