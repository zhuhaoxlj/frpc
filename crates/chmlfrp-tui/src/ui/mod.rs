mod login;
mod logs;
mod settings;
mod tunnels;

use crate::app::{App, Screen, Tab, TunnelPageMode};
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
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(f.area());

    let tabs = Tabs::new(vec!["[1] 隧道", "[2] 日志", "[3] 设置"])
        .select(match app.tab {
            Tab::Tunnels => 0,
            Tab::Logs => 1,
            Tab::Settings => 2,
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

    let user_info = if let Some(ref user) = app.stored_user {
        let frpc_status = if chmlfrp_core::download::check_frpc_exists(&app.data_dir) {
            "已安装"
        } else {
            "未安装"
        };
        format!(
            " 用户: {} | 组: {} | 隧道总数: {} | 运行中: {} | frpc: {}",
            user.username,
            user.usergroup,
            app.tunnel_items.len(),
            app.running_tunnels.len(),
            frpc_status,
        )
    } else {
        " 未登录".to_string()
    };

    let user_bar = Paragraph::new(user_info).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(user_bar, chunks[1]);

    match app.tab {
        Tab::Tunnels => tunnels::draw_tunnels(f, app, chunks[2]),
        Tab::Logs => logs::draw_logs(f, app, chunks[2]),
        Tab::Settings => settings::draw_settings(f, app, chunks[2]),
    }

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

    let help = if app.is_editing_tunnel() {
        " 输入内容 Enter换行 Backspace删除 s保存 Esc取消"
    } else {
        match app.tab {
            Tab::Tunnels => match app.tunnel_page_mode {
                TunnelPageMode::List => {
                    " ↑↓选择 Enter启动/停止 c官方创建 n新建 e编辑 x删除 a自动启动 o打开 r刷新 d下载 l注销 q退出"
                }
                TunnelPageMode::OfficialNodeSelect => " ↑↓选择节点 Enter继续 Esc返回",
                TunnelPageMode::OfficialForm => {
                    if app.official_tunnel_mode.is_edit() {
                        " ↑↓切字段 输入内容 Backspace删除 Tab/空格切换 Enter保存 Esc返回"
                    } else {
                        " ↑↓切字段 输入内容 Backspace删除 Tab/空格切换 Enter提交 Esc返回"
                    }
                }
                TunnelPageMode::ApiDeleteConfirm => " Enter/y确认删除 Esc/n取消",
            },
            Tab::Logs => " ↑↓滚动 r刷新 q退出",
            Tab::Settings => " ↑↓选择 Enter切换状态 q退出",
        }
    };

    let status_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(chunks[3]);

    let status_bar = Paragraph::new(status_text)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(status_bar, status_layout[0]);

    let help_bar = Paragraph::new(help)
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(help_bar, status_layout[1]);
}
