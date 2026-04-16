use crate::app::{App, OfficialTunnelMode, TunnelEditorMode, TunnelListItem, TunnelPageMode};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn draw_tunnels(f: &mut Frame, app: &App, area: Rect) {
    if app.is_editing_tunnel() {
        draw_tunnel_editor(f, app, area);
        return;
    }

    match app.tunnel_page_mode {
        TunnelPageMode::List => draw_tunnel_list(f, app, area),
        TunnelPageMode::OfficialNodeSelect => draw_official_node_select(f, app, area),
        TunnelPageMode::OfficialForm => draw_official_form(f, app, area),
        TunnelPageMode::ApiDeleteConfirm => draw_api_delete_confirm(f, app, area),
    }
}

fn draw_tunnel_list(f: &mut Frame, app: &App, area: Rect) {
    if app.tunnel_items.is_empty() {
        let msg = Paragraph::new(" 暂无隧道，按 c 创建官方隧道，按 n 新建本地隧道或按 r 刷新")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().title(" 隧道列表 ").borders(Borders::ALL));
        f.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec![
        Cell::from("来源"),
        Cell::from("ID"),
        Cell::from("名称"),
        Cell::from("类型"),
        Cell::from("节点/服务器"),
        Cell::from("本地地址"),
        Cell::from("远程地址"),
        Cell::from("状态"),
    ])
    .style(Style::default().fg(Color::Cyan).bold())
    .height(1);

    let rows: Vec<Row> = app
        .tunnel_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let tunnel_id = item.id();
            let is_running = app.running_tunnels.contains(&tunnel_id);
            let status = if is_running {
                Span::styled("● 运行中", Style::default().fg(Color::Green))
            } else {
                Span::styled("○ 已停止", Style::default().fg(Color::DarkGray))
            };

            let style = if i == app.selected_tunnel {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                Style::default()
            };

            let auto_start_mark = if app.settings.auto_start_tunnel_ids.contains(&tunnel_id) {
                "[A] "
            } else {
                ""
            };

            let (source, raw_id, tunnel_type, node_or_server, local_addr, remote_addr) = match item {
                TunnelListItem::Api(tunnel) => {
                    let remote = if tunnel.tunnel_type == "http" || tunnel.tunnel_type == "https" {
                        tunnel.dorp.clone()
                    } else {
                        format!("{}:{}", tunnel.node_ip, tunnel.dorp)
                    };
                    (
                        "API",
                        tunnel.id.to_string(),
                        tunnel.tunnel_type.clone(),
                        tunnel.node.clone(),
                        format!("{}:{}", tunnel.localip, tunnel.nport),
                        remote,
                    )
                }
                TunnelListItem::Local(tunnel) => {
                    let remote = if let Some(domains) = tunnel.custom_domains.as_ref() {
                        domains.clone()
                    } else if let Some(subdomain) = tunnel.subdomain.as_ref() {
                        subdomain.clone()
                    } else if let (Some(server_addr), Some(remote_port)) =
                        (tunnel.server_addr.as_ref(), tunnel.remote_port)
                    {
                        format!("{}:{}", server_addr, remote_port)
                    } else {
                        "-".to_string()
                    };
                    (
                        "LOCAL",
                        tunnel.id.clone(),
                        tunnel.tunnel_type.clone().unwrap_or_else(|| "-".to_string()),
                        tunnel.server_addr.clone().unwrap_or_else(|| "-".to_string()),
                        match (&tunnel.local_ip, tunnel.local_port) {
                            (Some(ip), Some(port)) => format!("{}:{}", ip, port),
                            _ => "-".to_string(),
                        },
                        remote,
                    )
                }
            };

            Row::new(vec![
                Cell::from(source),
                Cell::from(raw_id),
                Cell::from(format!("{}{}", auto_start_mark, item.name())),
                Cell::from(tunnel_type),
                Cell::from(node_or_server),
                Cell::from(local_addr),
                Cell::from(remote_addr),
                Cell::from(status),
            ])
            .style(style)
            .height(1)
        })
        .collect();

    let widths = [
        Constraint::Length(8),
        Constraint::Length(12),
        Constraint::Min(16),
        Constraint::Length(8),
        Constraint::Length(18),
        Constraint::Length(18),
        Constraint::Min(22),
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

fn draw_official_node_select(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .official_nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let selected = index == app.selected_official_node;
            let style = if selected {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", node.name), style.add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{} | {} | web:{} udp:{}", node.area, node.nodegroup, node.web, node.udp),
                    style,
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" 选择节点 ")
            .title_bottom(" ↑↓ 选择 Enter 继续 Esc 返回 ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(list, area);
}

fn draw_official_form(f: &mut Frame, app: &App, area: Rect) {
    let node_name = app
        .selected_official_node()
        .map(|node| node.name.as_str())
        .unwrap_or("-");
    let form = &app.official_tunnel_form;
    let is_http = form.protocol.is_http();
    let rport = app.current_official_rport().unwrap_or("-");
    let (title, title_bottom) = match app.official_tunnel_mode {
        OfficialTunnelMode::Create => (
            format!(" 创建官方隧道 | 节点: {} ", node_name),
            " ↑↓ 切换字段 Tab/空格切换 Enter 提交 Esc 返回节点 ",
        ),
        OfficialTunnelMode::Edit { .. } => (
            format!(" 编辑官方隧道 | 节点: {} ", node_name),
            " ↑↓ 切换字段 Tab/空格切换 Enter 保存 Esc 返回节点 ",
        ),
    };

    let fields = [
        (
            "隧道名称",
            form.tunnel_name.as_str().to_string(),
            form.selected_field == 0,
        ),
        (
            "本地 IP",
            form.local_ip.as_str().to_string(),
            form.selected_field == 1,
        ),
        (
            "本地端口",
            form.local_port.as_str().to_string(),
            form.selected_field == 2,
        ),
        (
            if is_http { "域名" } else { "远程端口" },
            if is_http {
                form.domain.as_str().to_string()
            } else {
                format!("{} (允许: {})", form.remote_port, rport)
            },
            form.selected_field == 3,
        ),
        (
            "协议",
            form.protocol.label().to_string(),
            form.selected_field == 4,
        ),
        (
            "加密",
            if form.encryption { "开启" } else { "关闭" }.to_string(),
            form.selected_field == 5,
        ),
        (
            "压缩",
            if form.compression { "开启" } else { "关闭" }.to_string(),
            form.selected_field == 6,
        ),
        (
            "可用端口",
            rport.to_string(),
            false,
        ),
    ];

    let rows: Vec<Row> = fields
        .into_iter()
        .map(|(label, value, selected)| {
            let style = if selected {
                Style::default().fg(Color::White).bg(Color::DarkGray)
            } else {
                Style::default()
            };
            Row::new(vec![Cell::from(label), Cell::from(value)]).style(style)
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(12), Constraint::Min(24)])
        .block(
            Block::default()
                .title(title)
                .title_bottom(title_bottom)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if app.is_submitting_official_tunnel {
                    Color::Yellow
                } else {
                    Color::Green
                })),
        )
        .column_spacing(2);

    f.render_widget(table, area);
}

fn draw_api_delete_confirm(f: &mut Frame, app: &App, area: Rect) {
    let tunnel = app.api_delete_target.as_ref();
    let name = tunnel.map(|t| t.name.as_str()).unwrap_or("-");
    let tunnel_type = tunnel.map(|t| t.tunnel_type.as_str()).unwrap_or("-");
    let node = tunnel.map(|t| t.node.as_str()).unwrap_or("-");
    let remote = tunnel
        .map(|t| {
            if t.tunnel_type == "http" || t.tunnel_type == "https" {
                t.dorp.clone()
            } else {
                format!("{}:{}", t.node_ip, t.dorp)
            }
        })
        .unwrap_or_else(|| "-".to_string());

    let lines = vec![
        Line::from(vec![
            Span::styled("确认删除官方隧道", Style::default().fg(Color::Red).bold()),
        ]),
        Line::default(),
        Line::from(format!("名称: {}", name)),
        Line::from(format!("类型: {}", tunnel_type)),
        Line::from(format!("节点: {}", node)),
        Line::from(format!("远程地址: {}", remote)),
        Line::default(),
        Line::from("按 Enter / y 确认删除，按 Esc / n 取消"),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" 删除官方隧道确认 ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
}

fn draw_tunnel_editor(f: &mut Frame, app: &App, area: Rect) {
    let title = match app.tunnel_editor {
        Some(TunnelEditorMode::New) => " 新建本地隧道 ",
        Some(TunnelEditorMode::Edit { .. }) => " 编辑本地隧道 ",
        None => " 编辑器 ",
    };

    let editor = Paragraph::new(app.tunnel_editor_content.as_str())
        .block(
            Block::default()
                .title(title)
                .title_bottom(" 按 s 保存，Esc 取消 ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(editor, area);
}
