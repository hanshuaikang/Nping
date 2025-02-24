use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::{Terminal};
use crate::ip_data::IpData;
use std::io::{self, Stdout};
use std::error::Error;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crate::ui::{draw_graph_view, draw_table_view};

/// init terminal
pub fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // enter alternate screen
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    Ok(terminal)
}

// restore terminal and show cursor
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    terminal.show_cursor()?;
    // leave alternate screen
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}


/// draw ui interface
pub fn draw_interface<B: Backend>(
    terminal: &mut Terminal<B>,
    view_type: &str,
    ip_data: &[IpData],
    errs: &[String],
) -> Result<(), Box<dyn Error>> {
    terminal.draw(|f| {
        match view_type {
            "graph" => {
                draw_graph_view::<B>(f, ip_data, errs);
            }
            "table" => {
                let size = f.area();
                draw_table_view::<B>(f, ip_data, errs, size);
            }

            _ => {
                draw_graph_view::<B>(f, ip_data, errs);
            }
        }
    })?;
    Ok(())
}