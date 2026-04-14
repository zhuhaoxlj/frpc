use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn draw_tunnels(f: &mut Frame, app: &App, area: Rect) {
    if app.tunnels.is_empty() {
        let msg = Paragraph::new(" 暂无隧道，按 r 刷新")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .title(" 隧道列表 ")
                    .borders(Borders::ALL),
            );
        f.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from(" ID"),
        Cell::from("名称"),
        Cell::from("类型"),
        Cell::from("节点"),
        Cell::from("本地地址"),
        Cell::from("远程地址"),
        Cell::from("状态"),
    ])
    .style(Style::default().fg(Color::Cyan).bold())
    .height(1);

    let rows: Vec<Row> = app
        .tunnels
        .iter()
        .enumerate()
        .map(|(i, tunnel)| {
            let is_running = app.running_tunnels.contains(&tunnel.id);
            let status = if is_running {
                Span::styled("● 运行中", Style::default().fg(Color::Green))
            } else {
                Span::styled("○ 已停止", Style::default().fg(Color::DarkGray))
            };

            let remote = if tunnel.tunnel_type == "http" || tunnel.tunnel_type == "https" {
                tunnel.dorp.clone()
            } else {
                format!("{}:{}", tunnel.node_ip, tunnel.dorp)
            };

            let style = if i == app.selected_tunnel {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(format!(" {}", tunnel.id)),
                Cell::from(tunnel.name.clone()),
                Cell::from(tunnel.tunnel_type.clone()),
                Cell::from(tunnel.node.clone()),
                Cell::from(format!("{}:{}", tunnel.localip, tunnel.nport)),
                Cell::from(remote),
                Cell::from(status),
            ])
            .style(style)
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Min(12),
        Constraint::Length(6),
        Constraint::Length(12),
        Constraint::Length(18),
        Constraint::Min(24),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(" 隧道列表 ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::White)),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    f.render_widget(table, area);
}
