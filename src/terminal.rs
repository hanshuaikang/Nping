use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::terminal::disable_raw_mode;
use ratatui::Terminal;
use crate::draw;

pub(crate) struct TerminalGuard {
    pub(crate) terminal: Option<Terminal<CrosstermBackend<std::io::Stdout>>>,
}

impl TerminalGuard {
    pub(crate) fn new(terminal: Terminal<CrosstermBackend<std::io::Stdout>>) -> Self {
        Self {
            terminal: Some(terminal),
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // be sure to disable raw mode before exiting
        let _ = disable_raw_mode();
        // Restore terminal state
        if let Some(mut terminal) = self.terminal.take() {
            if let Err(err) = draw::restore_terminal(&mut terminal) {
                eprintln!("Failed to restore terminal: {}", err);
            }
        }
    }
}