use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::{Terminal};
use crate::ip_data::IpData;
use std::io::{self, Stdout};
use std::error::Error;
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crate::ui::{draw_graph_view, draw_point_view, draw_table_view, draw_sparkline_view};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use ratatui::crossterm::event;
use ratatui::crossterm::event::{Event, KeyCode, KeyModifiers};

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
            "point" => {
                let size = f.area();
                draw_point_view::<B>(f, ip_data, errs, size);
            }
            "sparkline" => {
                let size = f.area();
                draw_sparkline_view::<B>(f, ip_data, errs, size);
            }
            _ => {
                draw_graph_view::<B>(f, ip_data, errs);
            }
        }
    })?;
    Ok(())
}

/// draw ui interface with event loop
pub fn draw_interface_with_updates<B: Backend>(
    terminal: &mut Terminal<B>,
    view_type: &Arc<String>,
    ip_data: &Arc<Mutex<Vec<IpData>>>,
    ping_update_rx: mpsc::Receiver<IpData>,
    running: Arc<Mutex<bool>>,
    errs: Arc<Mutex<Vec<String>>>,
    output_file: Option<String>,
) -> Result<(), Box<dyn Error>> {
    let mut output_file_handle = if let Some(ref output_path) = output_file {
        match std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(output_path)
        {
            Ok(file) => Some(file),
            Err(e) => {
                let mut errs = errs.lock().unwrap();
                errs.push(format!("Failed to create output file: {}", e));
                None
            }
        }
    } else {
        None
    };

    loop {
        if !*running.lock().unwrap() {
            break Ok(());
        }

        // Check for keyboard events
        if let Ok(true) = event::poll(Duration::from_millis(50)) {
            if let Ok(Event::Key(key)) = event::read() {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        *running.lock().unwrap() = false;
                        break Ok(());
                    },
                    KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                        *running.lock().unwrap() = false;
                        break Ok(());
                    },
                    _ => {}
                }
            }
        }

        if let Ok(updated_data) = ping_update_rx.recv_timeout(Duration::from_millis(50)) {
            let mut ip_data = ip_data.lock().unwrap();

            let last_attr = updated_data.last_attr.clone();
            let addr = updated_data.addr.clone();
            let ip = updated_data.ip.clone();

            if let Some(pos) = ip_data.iter().position(|d| d.addr == updated_data.addr && d.ip == updated_data.ip) {
                ip_data[pos] = updated_data;
            }

            if let Some(ref mut file) = output_file_handle {
                use std::io::Write;

                let latency_str = if last_attr == -1.0 {
                    "timeout".to_string()
                } else {
                    format!("{:.2}ms", last_attr)
                };

                if let Err(e) = writeln!(file, "{} {} {}",
                                         addr,
                                         ip,
                                         latency_str
                ) {
                    let mut errs = errs.lock().unwrap();
                    errs.push(format!("Failed to write to output file: {}", e));
                }
            }

            draw_interface(
                terminal,
                view_type,
                &ip_data,
                &mut errs.lock().unwrap(),
            ).ok();
        }
    }
}
