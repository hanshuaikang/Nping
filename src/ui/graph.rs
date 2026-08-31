use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Line, Span, Style};
use ratatui::style::Modifier;
use ratatui::widgets::{Axis, Block, BorderType, Borders, Chart, Dataset, Paragraph, Wrap};
use ratatui::{symbols, Frame};

use crate::ip_data::IpData;
use crate::ui::theme::Theme;
use crate::ui::utils::{calculate_avg_rtt, calculate_jitter, calculate_p95, draw_errors_section};

const GRAPH_WINDOW: usize = 60;
const MIN_CHART_WIDTH: u16 = 48;

pub fn draw_graph_view(
    f: &mut Frame,
    ip_data: &[IpData],
    errs: &[String],
    area: Rect,
    theme: &Theme,
) {
    if ip_data.is_empty() {
        // No targets yet (e.g. DNS resolution still running). Still surface
        // whatever errors are pending — that's exactly when the user needs them.
        draw_errors_section(f, errs, area, theme);
        return;
    }

    // Responsive grid: cols fit by terminal width with a minimum width per chart.
    let cols = ((area.width / MIN_CHART_WIDTH) as usize)
        .max(1)
        .min(ip_data.len());
    let rows = ip_data.len().div_ceil(cols);

    let mut vert_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Percentage(100 / rows as u16))
        .collect();
    vert_constraints.push(Constraint::Min(5));

    let vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(vert_constraints)
        .split(area);

    for row in 0..rows {
        let start = row * cols;
        let end = (start + cols).min(ip_data.len());
        let row_data = &ip_data[start..end];

        // Keep grid column width consistent across rows — last row's empty
        // trailing slots stay blank rather than stretching the tail card.
        let horizontal_constraints: Vec<Constraint> =
            (0..cols).map(|_| Constraint::Ratio(1, cols as u32)).collect();

        let horizontal_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(horizontal_constraints)
            .split(vertical_chunks[row]);

        for (i, data) in row_data.iter().enumerate() {
            render_target(f, horizontal_chunks[i], data, theme);
        }
    }

    let errors_chunk = vertical_chunks.last().unwrap();
    draw_errors_section(f, errs, *errors_chunk, theme);
}

fn render_target(f: &mut Frame, area: Rect, data: &IpData, theme: &Theme) {
    let loss_pkg = if data.timeout > 0 {
        (data.timeout as f64 / (data.received as f64 + data.timeout as f64)) * 100.0
    } else {
        0.0
    };
    let loss_color = theme.loss_color(loss_pkg);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(
                format!("◆ {} ", data.addr),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("({}) ", data.ip),
                Style::default().fg(theme.dim),
            ),
        ]));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(2), // metrics
            Constraint::Min(3),    // chart
            Constraint::Length(5), // recent
        ])
        .split(inner);

    let avg_rtt = calculate_avg_rtt(&data.rtts);
    let jitter = calculate_jitter(&data.rtts);
    let p95 = calculate_p95(&data.rtts);

    let metric_pair = |k: &'static str, v: String, color: ratatui::style::Color| -> Vec<Span<'static>> {
        vec![
            Span::styled(k, Style::default().fg(theme.dim)),
            Span::styled(v, Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
        ]
    };

    let last_str = if data.last_attr == 0.0 {
        "< 0.01ms".to_string()
    } else if data.last_attr == -1.0 {
        "timeout".to_string()
    } else {
        format!("{:.2}ms", data.last_attr)
    };
    let last_color = if data.last_attr == -1.0 {
        theme.danger
    } else {
        theme.rtt_color(data.last_attr, data.max_rtt.max(1.0))
    };

    let mut spans: Vec<Span<'static>> = Vec::new();
    spans.extend(metric_pair("last ", last_str, last_color));
    spans.extend(metric_pair("avg ", format!("{:.2}ms", avg_rtt), theme.secondary));
    spans.extend(metric_pair("jit ", format!("{:.2}ms", jitter), theme.secondary));
    spans.extend(metric_pair("max ", format!("{:.2}ms", data.max_rtt), theme.warning));
    spans.extend(metric_pair("p95 ", format!("{:.2}ms", p95), theme.warning));
    spans.extend(metric_pair("min ", format!("{:.2}ms", data.min_rtt), theme.success));
    spans.extend(metric_pair("loss ", format!("{:.2}%", loss_pkg), loss_color));

    let metric_para = Paragraph::new(Line::from(spans)).wrap(Wrap { trim: false });
    f.render_widget(metric_para, inner_chunks[0]);

    let total_len = data.rtts.len();
    let skip = total_len.saturating_sub(GRAPH_WINDOW);
    let window_offset = data.pop_count as f64 + skip as f64;
    let max_in_window = data
        .rtts
        .iter()
        .skip(skip)
        .filter(|&&r| r >= 0.0)
        .cloned()
        .fold(0.0_f64, f64::max);
    let plot_max = if max_in_window > 0.0 {
        max_in_window
    } else {
        data.max_rtt.max(1.0)
    };

    // Timeouts must read as packet loss, not as a normal-looking line (#119):
    // the RTT line breaks at each timeout, and the timeout itself is drawn as
    // a red marker pinned above the highest RTT in the window.
    let timeout_y = plot_max * 1.05;
    let (line_segments, timeout_points) =
        split_rtt_series(&data.rtts, skip, data.pop_count as f64, timeout_y);

    let line_color = theme.rtt_color(avg_rtt, plot_max);
    let mut datasets: Vec<Dataset> = line_segments
        .iter()
        .map(|segment| {
            Dataset::default()
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(line_color).bg(theme.bg))
                .graph_type(ratatui::widgets::GraphType::Line)
                .data(segment)
        })
        .collect();
    datasets.push(
        Dataset::default()
            .marker(symbols::Marker::HalfBlock)
            .style(Style::default().fg(theme.danger).bg(theme.bg))
            .graph_type(ratatui::widgets::GraphType::Scatter)
            .data(&timeout_points),
    );

    let window_len = total_len - skip;
    let y_bounds = [0.0, plot_max * 1.2];
    let x_start = window_offset + 1.0;
    let x_end = x_start + (window_len.max(1) as f64) - 1.0;

    // Pin every chart sub-style's bg to the theme bg, otherwise ratatui's
    // Style::default() resets bg to terminal default and the plot area
    // shows as white on night mode.
    let chart = Chart::new(datasets)
        .style(Style::default().bg(theme.bg))
        .block(Block::default().borders(Borders::NONE))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(theme.dim).bg(theme.bg))
                .bounds([x_start, x_end.max(x_start + 1.0)]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(theme.dim).bg(theme.bg))
                .bounds(y_bounds)
                .labels(
                    (0..=3)
                        .map(|i| {
                            Span::styled(
                                format!("{:.0}", i as f64 * (y_bounds[1] / 3.0)),
                                Style::default().fg(theme.dim).bg(theme.bg),
                            )
                        })
                        .collect::<Vec<Span>>(),
                ),
        );

    f.render_widget(chart, inner_chunks[1]);

    let recent_records: Vec<Line> = data
        .rtts
        .iter()
        .rev()
        .take(5)
        .map(|&rtt| {
            let (text, color) = if rtt == -1.0 {
                ("✗ timeout".to_string(), theme.danger)
            } else {
                (
                    format!("● {:.2}ms", rtt),
                    theme.rtt_color(rtt, plot_max),
                )
            };
            Line::from(Span::styled(text, Style::default().fg(color)))
        })
        .collect();

    let recent = Paragraph::new(recent_records).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(theme.dim))
            .title(Span::styled(
                " recent ",
                Style::default().fg(theme.dim),
            )),
    );
    f.render_widget(recent, inner_chunks[2]);
}

/// Splits the windowed RTT series into contiguous line segments of valid
/// samples plus the timeout positions (the `-1.0` sentinel), so the chart
/// shows a gap with an explicit marker instead of bridging across timeouts.
#[allow(clippy::type_complexity)]
fn split_rtt_series(
    rtts: &std::collections::VecDeque<f64>,
    skip: usize,
    x_offset: f64,
    timeout_y: f64,
) -> (Vec<Vec<(f64, f64)>>, Vec<(f64, f64)>) {
    let mut segments: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut timeouts: Vec<(f64, f64)> = Vec::new();
    let mut current: Vec<(f64, f64)> = Vec::new();

    for (i, &y) in rtts.iter().enumerate().skip(skip) {
        let x = x_offset + i as f64 + 1.0;
        if y < 0.0 {
            timeouts.push((x, timeout_y));
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push((x, y));
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }

    (segments, timeouts)
}

#[cfg(test)]
mod tests {
    use super::split_rtt_series;
    use std::collections::VecDeque;

    #[test]
    fn test_split_no_timeouts_single_segment() {
        let rtts: VecDeque<f64> = vec![10.0, 12.0, 11.0].into();
        let (segments, timeouts) = split_rtt_series(&rtts, 0, 0.0, 100.0);
        assert_eq!(segments, vec![vec![(1.0, 10.0), (2.0, 12.0), (3.0, 11.0)]]);
        assert!(timeouts.is_empty());
    }

    #[test]
    fn test_split_timeout_breaks_line() {
        let rtts: VecDeque<f64> = vec![10.0, -1.0, 12.0].into();
        let (segments, timeouts) = split_rtt_series(&rtts, 0, 0.0, 100.0);
        assert_eq!(segments, vec![vec![(1.0, 10.0)], vec![(3.0, 12.0)]]);
        assert_eq!(timeouts, vec![(2.0, 100.0)]);
    }

    #[test]
    fn test_split_all_timeouts_no_line() {
        let rtts: VecDeque<f64> = vec![-1.0, -1.0, -1.0].into();
        let (segments, timeouts) = split_rtt_series(&rtts, 0, 0.0, 1.05);
        assert!(segments.is_empty());
        assert_eq!(timeouts, vec![(1.0, 1.05), (2.0, 1.05), (3.0, 1.05)]);
    }

    #[test]
    fn test_split_respects_skip_and_offset() {
        let rtts: VecDeque<f64> = vec![5.0, -1.0, 7.0, 8.0].into();
        let (segments, timeouts) = split_rtt_series(&rtts, 2, 10.0, 100.0);
        // Only indices 2 and 3 are in the window; x = offset + index + 1.
        assert_eq!(segments, vec![vec![(13.0, 7.0), (14.0, 8.0)]]);
        assert!(timeouts.is_empty());
    }
}
