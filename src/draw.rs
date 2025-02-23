use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::Terminal;
use crate::ip_data::IpData;
use std::io::{self, Stdout};
use std::error::Error;
use crate::ui::{draw_graph_view, draw_table_view};

/// init terminal
pub fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn Error>> {
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    Ok(terminal)
}

/// restore terminal
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Box<dyn Error>> {
    terminal.show_cursor()?;
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
