use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn draw_logs(f: &mut Frame, app: &App, area: Rect) {
    if app.logs.is_empty() {
        let msg = Paragraph::new(" 暂无日志，启动隧道后将显示日志")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().title(" 日志 ").borders(Borders::ALL));
        f.render_widget(msg, area);
        return;
    }

    let log_lines: Vec<Line> = app
        .logs
        .iter()
        .map(|log| {
            let color = if log.message.contains("[ERR]") {
                Color::Red
            } else if log.message.contains("[W]") || log.message.contains("warn") {
                Color::Yellow
            } else if log.message.contains("ChmlFrpLauncher") {
                Color::Cyan
            } else {
                Color::White
            };

            Line::from(vec![
                Span::styled(
                    format!("[{}] ", log.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("T{} ", log.tunnel_id),
                    Style::default().fg(Color::Blue),
                ),
                Span::styled(&log.message, Style::default().fg(color)),
            ])
        })
        .collect();

    let total_lines = log_lines.len() as u16;
    let visible_height = area.height.saturating_sub(2); // 减去边框
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = if app.log_scroll > max_scroll {
        max_scroll
    } else {
        app.log_scroll
    };

    // 自动滚动到底部（如果用户没手动滚动）
    let scroll = if app.log_scroll == 0 && total_lines > visible_height {
        max_scroll
    } else {
        scroll
    };

    let paragraph = Paragraph::new(log_lines)
        .block(
            Block::default()
                .title(format!(" 日志 ({}) ", app.logs.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .scroll((scroll, 0));

    f.render_widget(paragraph, area);
}
