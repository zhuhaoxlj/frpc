use crate::app::{App, LoginState};
use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn draw_login(f: &mut Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Length(15),
            Constraint::Percentage(25),
        ])
        .split(area);

    let center = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(chunks[1]);

    let block = Block::default()
        .title(" ChmlFrp Launcher - 登录 ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let content = match app.login_state {
        LoginState::Idle => {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "欢迎使用 ChmlFrp Launcher TUI",
                    Style::default().fg(Color::Cyan).bold(),
                )),
                Line::from(""),
                Line::from("使用 OAuth2 设备码授权登录"),
                Line::from(""),
                Line::from(Span::styled(
                    "按 Enter 开始登录",
                    Style::default().fg(Color::Green).bold(),
                )),
                Line::from(Span::styled(
                    "按 q 退出",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        LoginState::WaitingForAuth => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "请在浏览器中完成授权",
                    Style::default().fg(Color::Yellow).bold(),
                )),
                Line::from(""),
            ];

            if !app.login_verification_uri.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("访问: "),
                    Span::styled(
                        app.login_verification_uri.as_str(),
                        Style::default().fg(Color::Cyan).underlined(),
                    ),
                ]));
            }

            if !app.login_user_code.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::raw("输入验证码: "),
                    Span::styled(
                        app.login_user_code.as_str(),
                        Style::default().fg(Color::Green).bold(),
                    ),
                ]));
            }

            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "等待授权中...",
                Style::default().fg(Color::DarkGray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "快捷键: [c]复制验证码  [u]复制授权链接  [q]退出",
                Style::default().fg(Color::Gray),
            )));

            lines
        }
        LoginState::Error => {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "登录失败",
                    Style::default().fg(Color::Red).bold(),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    app.login_error.as_str(),
                    Style::default().fg(Color::Red),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "按 Enter 重试 | 按 q 退出",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
    };

    let paragraph = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center);

    f.render_widget(paragraph, center[1]);
}
