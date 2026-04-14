mod login;
mod logs;
mod tunnels;

use crate::app::{App, Screen, Tab};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn draw(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Login => login::draw_login(f, app),
        Screen::Main => draw_main(f, app),
    }
}

fn draw_main(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 标题栏 + Tab
            Constraint::Length(3), // 用户信息
            Constraint::Min(5),   // 内容区
            Constraint::Length(3), // 状态栏 + 快捷键
        ])
        .split(f.area());

    // 标题栏 + Tab
    let tabs = Tabs::new(vec!["[1] 隧道", "[2] 日志"])
        .select(match app.tab {
            Tab::Tunnels => 0,
            Tab::Logs => 1,
        })
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Cyan).bold())
        .divider(" | ");

    let title_block = Block::default()
        .title(" ChmlFrp Launcher TUI ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(tabs.block(title_block), chunks[0]);

    // 用户信息栏
    let user_info = if let Some(ref user) = app.stored_user {
        let frpc_status = if chmlfrp_core::download::check_frpc_exists(&app.data_dir) {
            "已安装"
        } else {
            "未安装"
        };
        format!(
            " 用户: {} | 组: {} | 隧道: {}/{} | 运行中: {} | frpc: {}",
            user.username,
            user.usergroup,
            app.running_tunnels.len(),
            app.tunnels.len(),
            app.running_tunnels.len(),
            frpc_status,
        )
    } else {
        " 未登录".to_string()
    };

    let user_bar = Paragraph::new(user_info)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(user_bar, chunks[1]);

    // 内容区
    match app.tab {
        Tab::Tunnels => tunnels::draw_tunnels(f, app, chunks[2]),
        Tab::Logs => logs::draw_logs(f, app, chunks[2]),
    }

    // 状态栏
    let confirm_text = if app.show_confirm_quit {
        " [y]确认退出 [其他键]取消"
    } else {
        ""
    };

    let status_text = if app.is_downloading {
        format!(" 下载中 {:.1}% | {}", app.download_progress, app.status_message)
    } else {
        format!(" {} {}", app.status_message, confirm_text)
    };

    let help = match app.tab {
        Tab::Tunnels => " ↑↓选择 Enter启动/停止 o打开远程地址 r刷新 d下载frpc l注销 q退出",
        Tab::Logs => " ↑↓滚动 r刷新 q退出",
    };

    let status_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[3]);

    let status_bar = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(status_bar, status_layout[0]);

    let help_bar = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));
    f.render_widget(help_bar, status_layout[1]);
}
