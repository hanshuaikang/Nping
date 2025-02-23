use ratatui::backend::Backend;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::prelude::{Color, Line, Span, Style};
use ratatui::widgets::{ Paragraph, Wrap};
use crate::ip_data::IpData;
use crate::ui::utils::{calculate_avg_rtt, calculate_jitter, calculate_loss_pkg, draw_errors_section};

pub fn draw_point_view<B: Backend>(
    f: &mut Frame,
    ip_data: &[IpData],
    errs: &[String],
) {
    let size = f.area();
    // 每行一个目标，则可以直接按行布局
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            ip_data
                .iter()
                .map(|_| Constraint::Length(4)) // 两行内容 + 一些留白
                .chain(std::iter::once(Constraint::Min(7)))
                .collect::<Vec<_>>(),
        )
        .split(size);

    for (index, area) in chunks.iter().take(ip_data.len()).enumerate() {
        let data = &ip_data[index];
        let avg_rtt = calculate_avg_rtt(&data.rtts);
        let jitter = calculate_jitter(&data.rtts);
        let loss_pkg = calculate_loss_pkg(data.timeout, data.received);

        // 两行布局
        let sub_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(2),
            ].as_ref())
            .split(*area);

        // 第1行：基础指标
        let metrics_line = Line::from(vec![
            Span::raw("Target: "), Span::styled(&data.addr, Style::default().fg(Color::Cyan)),
            Span::raw(" IP: "), Span::styled(&data.ip, Style::default().fg(Color::Cyan)),
            Span::raw(" Last: "), Span::styled(format!("{:.2}ms", data.last_attr), Style::default().fg(Color::Green)),
            Span::raw(" Max: "), Span::styled(format!("{:.2}ms", data.max_rtt), Style::default().fg(Color::Green)),
            Span::raw(" Min: "), Span::styled(format!("{:.2}ms", data.min_rtt), Style::default().fg(Color::Green)),
            Span::raw(" Avg: "), Span::styled(format!("{:.2}ms", avg_rtt), Style::default().fg(Color::Green)),
            Span::raw(" Jitter: "), Span::styled(format!("{:.2}ms", jitter), Style::default().fg(Color::Green)),
            Span::raw(" Loss: "),
            Span::styled(
                format!("{:.2}%", loss_pkg),
                Style::default().fg(if loss_pkg > 50.0 { Color::Red } else if loss_pkg > 0.0 { Color::Yellow } else { Color::Green })
            ),
        ]);
        let metrics_paragraph = Paragraph::new(metrics_line);
        f.render_widget(metrics_paragraph, sub_chunks[0]);

        // 第2行：用小方块模拟状态（如只展示最近10次 RTT）

        // 在第2行中，替换原有 canvas 实现为 Paragraph 实现
        let status_area = sub_chunks[1];

        // 计算每行状态块个数，假设每个状态块占 2 个字符宽度
        let available_width = status_area.width as usize;
        let block_width = 2;
        let blocks_per_line = if available_width > block_width { available_width / block_width } else { 1 };

        let mut spans_vec = Vec::new();

        // 使用所有 RTT 数据（或者可以只取最近10个，根据需求）
        let recent_rtts = data.rtts.iter().rev();
        for (i, rtt) in recent_rtts.enumerate() {
            let color = if *rtt == -1.0 { Color::Red } else { Color::Green };
            spans_vec.push(Span::styled("▇  ", Style::default().fg(color)));
            // 当达到每行上限时，换行
            if (i + 1) % blocks_per_line == 0 {
                spans_vec.push(Span::raw("\n"));
            }
        }

        let status_line = Line::from(spans_vec);
        let paragraph = Paragraph::new(status_line)
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, status_area);

        // 第3行：展示每个状态块对应的 index（序号）
        let index_area = sub_chunks[2];
        let mut index_spans = Vec::new();
        // 遍历同样的数据，显示每个状态块的 index（从 pop_count+1 开始）
        for (i, _rtt) in data.rtts.iter().rev().enumerate() {
            let ping_index = data.pop_count + i + 1; // 当前 ping 的整体计数

            if ping_index < 10 && ping_index > 1 {
                index_spans.push(Span::raw(" "));
                index_spans.push(Span::raw(format!("{} ", ping_index)));
            }else if ping_index == 1{
                index_spans.push(Span::raw(format!("{} ", ping_index)));
            }
            else if ping_index >= 10{
                index_spans.push(Span::raw(format!("{} ", ping_index)));
            }
            if (i + 1) % blocks_per_line == 0 {
                index_spans.push(Span::raw("\n"));
            }
        }
        let index_line = Line::from(index_spans);
        let index_paragraph = Paragraph::new(index_line)
            .wrap(Wrap { trim: false });
        f.render_widget(index_paragraph, index_area);

    }

    // 最后一行区域展示最近错误
    let errors_chunk = chunks.last().unwrap();
    draw_errors_section::<B>(f, errs, *errors_chunk);
}