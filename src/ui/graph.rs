use ratatui::backend::Backend;
use ratatui::{symbols, Frame};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Color, Line, Span, Style};
use ratatui::widgets::{Axis, Block, Chart, Dataset, Paragraph, Wrap};
use crate::ip_data::IpData;
use crate::ui::utils::{calculate_avg_rtt, calculate_jitter, draw_errors_section};

pub fn draw_graph_view<B: Backend>(
    f: &mut Frame,
    ip_data: &[IpData],
    errs: &[String]) {
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
        let start = row * 5;
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
                let avg_rtt = calculate_avg_rtt(&data.rtts);


                // calculate the jitter
                let jitter = calculate_jitter(&data.rtts);


                // render the target text
                let target_text = Line::from(vec![
                    Span::styled("Target: ", Style::default()),
                    Span::styled(&data.addr, Style::default().fg(Color::Green)),
                ]);

                let base_metric_text = Line::from(vec![
                    Span::styled("Last: ", Style::default()),
                    Span::styled(
                        if data.last_attr == 0.0 {
                            "< 0.01ms".to_string()
                        } else if data.last_attr == -1.0 {
                            "0.0ms".to_string()
                        } else {
                            format!("{:?}ms", data.last_attr)
                        },
                        Style::default().fg(Color::Green)
                    ),
                    Span::raw("  "),
                    Span::styled("Avg Rtt : ", Style::default()),
                    Span::styled(format!("{:.2} ms", avg_rtt), Style::default().fg(Color::Green)),
                    Span::raw("  "),
                    Span::styled("Jitter: ", Style::default()),
                    Span::styled(format!("{:.2} ms", jitter), Style::default().fg(Color::Green)),
                    Span::raw("  "),
                    Span::styled("Max: ", Style::default()),
                    Span::styled(format!("{:.2} ms", data.max_rtt), Style::default().fg(Color::Green)),
                    Span::raw("  "),
                    Span::styled("Min: ", Style::default()),
                    Span::styled(format!("{:.2} ms", data.min_rtt), Style::default().fg(Color::Green)),
                    Span::raw("  "),
                    Span::styled("Loss: ", Style::default()),
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
                        let display_color = if rtt == -1.0 {
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

    let errors_chunk = vertical_chunks.last().unwrap();
    draw_errors_section::<B>(f, errs, *errors_chunk);
}