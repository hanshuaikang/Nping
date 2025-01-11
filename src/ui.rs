use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Block, Borders, Chart, Dataset, Paragraph, Wrap};
use ratatui::{symbols, Frame, Terminal};
use crate::ip_data::IpData;
use std::io::{self, Stdout};
use std::error::Error;

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
    ip_data: &[IpData],
    errs: &[String],
) -> Result<(), Box<dyn Error>> {
    terminal.draw(|f| {
        let size = f.area();
        let rows = (ip_data.len() as f64 / 5.0).ceil() as usize;
        let mut chunks = Vec::new();

        // compute the constraints
        for _ in 0..rows {
            chunks.push(Constraint::Percentage(100 / rows as u16));
        }

        chunks.push(Constraint::Min(7));


        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(chunks)
            .split(size);

        for (row, vertical_chunk) in vertical_chunks.iter().enumerate().take(rows) {
            let start =row * 5;
            let end = (start + 5).min(ip_data.len());
            let row_data = &ip_data[start..end];

            let horizontal_constraints: Vec<Constraint> = if row_data.len() == 5 {
                row_data.iter().map(|_| Constraint::Percentage(20)).collect()
            } else {
                // when the number of targets is less than 5, we need to adjust the size of each target
                let mut size = 100;
                if ip_data.len() > 5 {
                    size = row_data.len() * 20;
                }
                row_data.iter().map(|_| Constraint::Percentage(size as u16 / row_data.len() as u16)).collect()
            };

            let horizontal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(horizontal_constraints)
                .split(*vertical_chunk);

            for (i, data) in row_data.iter().enumerate() {
                // compute the loss package rate for each target
                let loss_pkg = if data.timeout > 0 {
                    (data.timeout as f64 / (data.received as f64 + data.timeout as f64)) * 100.0
                } else {
                    0.0
                };

                let loss_pkg_color = if loss_pkg > 50.0 {
                    Color::Red
                } else if loss_pkg > 0.0 {
                    Color::Yellow
                } else {
                    Color::Green
                };

                // render the content of each target
                let render_content = |f: &mut Frame, area: Rect| {
                    let inner_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .margin(1)
                        .constraints(
                            [
                                Constraint::Length(1),  // 目标标题
                                Constraint::Max(4),  // 段落占用较少行
                                Constraint::Max(20),     // chart 动态扩大
                                Constraint::Length(1),
                                Constraint::Length(6),
                            ]
                                .as_ref(),
                        )
                        .split(area);

                    // render the content of each target
                    let avg_rtt = if !data.rtts.is_empty() {
                        let valid_rtt: Vec<f64> = data.rtts.iter().cloned().filter(|&rtt| rtt >= 0.0).collect();
                        if !valid_rtt.is_empty() {
                            let sum: f64 = valid_rtt.iter().sum();
                            sum / valid_rtt.len() as f64
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };

                    // calculate the jitter
                    let jitter = if data.rtts.len() > 1 {
                        let diffs: Vec<f64> = data
                            .rtts
                            .iter()
                            .zip(data.rtts.iter().skip(1))
                            .map(|(y1, y2)| (y2 - y1).abs())
                            .collect();
                        let sum: f64 = diffs.iter().sum();
                        sum / diffs.len() as f64
                    } else {
                        0.0
                    };

                    // render the target text
                    let target_text = Line::from(vec![
                        Span::styled("target: ", Style::default()),
                        Span::styled(&data.addr, Style::default().fg(Color::Green)),
                    ]);

                    let base_metric_text = Line::from(vec![
                        Span::styled("last: ", Style::default()),
                        Span::styled(
                            if data.last_attr == 0.0 {
                                "< 0.01ms".to_string()
                            } else {
                                format!("{:?}ms", data.last_attr)
                            },
                            Style::default().fg(Color::Green)
                        ),
                        Span::raw("  "),
                        Span::styled("avg rtt : ", Style::default()),
                        Span::styled(format!("{:.2} ms", avg_rtt), Style::default().fg(Color::Green)),
                        Span::raw("  "),
                        Span::styled("jitter: ", Style::default()),
                        Span::styled(format!("{:.2} ms", jitter), Style::default().fg(Color::Green)),
                        Span::raw("  "),
                        Span::styled("max: ", Style::default()),
                        Span::styled(format!("{:.2} ms", data.max_rtt), Style::default().fg(Color::Green)),
                        Span::raw("  "),
                        Span::styled("min: ", Style::default()),
                        Span::styled(format!("{:.2} ms", data.min_rtt), Style::default().fg(Color::Green)),
                        Span::raw("  "),
                        Span::styled("loss: ", Style::default()),
                        Span::styled(format!("{:.2}%", loss_pkg), Style::default().fg(loss_pkg_color)),
                    ]);


                    let target_paragraph = Paragraph::new(target_text).block(Block::default());
                    f.render_widget(target_paragraph, inner_chunks[0]);

                    let base_metric_paragraph = Paragraph::new(base_metric_text).block(Block::default()).wrap(Wrap { trim: false });
                    f.render_widget(base_metric_paragraph, inner_chunks[1]);


                    let data_points = data
                        .rtts
                        .iter()
                        .enumerate()
                        .map(|(i, &y)| (data.pop_count as f64 + i as f64 + 1.0, y))
                        .collect::<Vec<(f64, f64)>>();

                    let datasets = vec![Dataset::default()
                        .marker(symbols::Marker::HalfBlock)
                        .style(Style::default().fg(Color::Red))
                        .graph_type(ratatui::widgets::GraphType::Line)
                        .data(&data_points)];

                    let y_bounds = [0.0, data.max_rtt * 1.2];

                    let x_range = data
                        .rtts
                        .iter()
                        .enumerate()
                        .map(|(i, _)| Span::styled(format!("{}", i + 1 + data.pop_count), Style::default()))
                        .collect::<Vec<Span>>();

                    let chart = Chart::new(datasets)
                        .x_axis(
                            Axis::default()
                                .title("count")
                                .style(Style::default())
                                .bounds([1.0 + data.pop_count as f64, 1.0 + data.pop_count as f64 + data.rtts.len() as f64 - 1.0])
                                .labels(x_range),
                        )
                        .y_axis(
                            Axis::default()
                                .title("rtt")
                                .style(Style::default())
                                .bounds(y_bounds)
                                .labels(
                                    (0..=5)
                                        .map(|i| Span::raw(format!("{:.2}ms", i as f64 * (y_bounds[1] / 5.0) as f64)))
                                        .collect::<Vec<Span>>(),
                                ),
                        )
                        .style(Style::default());

                    f.render_widget(chart, inner_chunks[2]);

                    let recent_records: Vec<Line> = data
                        .rtts
                        .iter()
                        .rev()
                        .take(5)
                        .map(|&rtt| {
                            let display_text = if rtt == -1.0 {
                                "timeout".to_string()
                            } else {
                                format!("{}ms", rtt)
                            };
                            let display_color = if rtt == -1.0  {
                                Color::Red
                            } else {
                                Color::Green
                            };
                            Line::from(vec![
                                Span::styled(&data.ip, Style::default()),
                                Span::raw(" "),
                                Span::styled(display_text, Style::default().fg(display_color)),
                            ])
                        })
                        .collect();

                    let blank_line = Line::from(vec![]);
                    let blank_paragraph = Paragraph::new(blank_line).block(Block::default());
                    f.render_widget(blank_paragraph, inner_chunks[3]);

                    let recent_paragraph = Paragraph::new(recent_records).block(Block::default().title("Recent Records:"));
                    f.render_widget(recent_paragraph, inner_chunks[4]);
                };

                render_content(f, horizontal_chunks[i]);
            }
        }


        let recent_errors: Vec<Line> = errs
            .iter()
            .rev()
            .take(5)
            .map(|err| {
                Line::from(vec![
                    Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
                    Span::styled(err, Style::default().fg(Color::Red))
                ])
            })
            .collect();

        let errors_chunk = vertical_chunks.last().unwrap();
        if errs.is_empty() {
            let blank_line = Line::from(vec![]);
            let blank_paragraph = Paragraph::new(blank_line).block(Block::default());
            f.render_widget(blank_paragraph, *errors_chunk);
        }else {
            let errors_paragraph = Paragraph::new(recent_errors)
                .block(Block::default()
                    .title("🚨Recent Errors:")
                    .borders(Borders::ALL))
                .wrap(Wrap { trim: true });
            f.render_widget(errors_paragraph, *errors_chunk);
        }


    })?;
    Ok(())
}