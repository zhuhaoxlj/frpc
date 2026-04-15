use crate::app::App;
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn draw_settings(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" 设置 ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let mut items = Vec::new();

    // 项 1: 守护进程/系统服务
    let systemd_status = if app.is_systemd_installed {
        Span::styled("已安装", Style::default().fg(Color::Green))
    } else {
        Span::styled("未安装", Style::default().fg(Color::DarkGray))
    };

    let mut line1 = vec![
        Span::raw(" ["),
        if app.selected_setting == 0 {
            Span::styled("*", Style::default().fg(Color::Yellow))
        } else {
            Span::raw(" ")
        },
        Span::raw("] 系统服务 (开机自启/进程守护): "),
        systemd_status,
    ];
    if app.selected_setting == 0 {
        line1.push(Span::raw("  "));
        if app.is_systemd_installed {
            line1.push(Span::styled("<按 Enter 查看提示>", Style::default().fg(Color::DarkGray)));
        } else {
            line1.push(Span::styled("<依赖 deb 包安装>", Style::default().fg(Color::DarkGray)));
        }
    }
    items.push(ListItem::new(Line::from(line1)));

    // 项 2: 全局自动启动隧道
    let auto_start_status = if app.settings.auto_start_tunnels_enabled {
        Span::styled("已开启", Style::default().fg(Color::Green))
    } else {
        Span::styled("已关闭", Style::default().fg(Color::DarkGray))
    };

    let mut line2 = vec![
        Span::raw(" ["),
        if app.selected_setting == 1 {
            Span::styled("*", Style::default().fg(Color::Yellow))
        } else {
            Span::raw(" ")
        },
        Span::raw("] 软件打开/后台启动时自动连接标记的隧道: "),
        auto_start_status,
    ];
    if app.selected_setting == 1 {
        line2.push(Span::raw("  "));
        line2.push(Span::styled("<按 Enter 切换>", Style::default().fg(Color::DarkGray)));
    }
    items.push(ListItem::new(Line::from(line2)));
    
    // 空行分割
    items.push(ListItem::new(Line::from("")));

    // 项 3: 当前标记的隧道
    items.push(ListItem::new(Line::from(Span::styled(
        " 已标记为自动启动的隧道 (在隧道列表按 'a' 标记):",
        Style::default().fg(Color::Cyan).bold()
    ))));
    
    if app.settings.auto_start_tunnel_ids.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            "   (暂无)",
            Style::default().fg(Color::DarkGray)
        ))));
    } else {
        for id in &app.settings.auto_start_tunnel_ids {
            // Try to find tunnel name
            let name = app.tunnels.iter().find(|t| t.id == *id).map(|t| t.name.clone()).unwrap_or_else(|| "未知隧道".to_string());
            items.push(ListItem::new(Line::from(format!("   - {} (ID: {})", name, id))));
        }
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");

    f.render_widget(list, area);
}
