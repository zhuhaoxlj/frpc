use crate::app::{App, LoginState, Screen, Tab};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

/// 处理事件，返回 true 表示退出
pub async fn handle_events(app: &mut App) -> Result<bool, Box<dyn std::error::Error>> {
    // 收集日志和异步任务结果
    app.drain_events();

    // 登录成功或启动后自动刷新隧道列表
    if app.needs_refresh {
        app.needs_refresh = false;
        app.refresh_tunnels().await;
        if app.settings.auto_start_tunnels_enabled {
            app.needs_auto_start = true;
        }
    }

    // 当隧道加载完成后且需要自启时触发
    if app.needs_auto_start && !app.tunnels.is_empty() {
        app.needs_auto_start = false;
        app.start_auto_tunnels().await;
    }

    if !event::poll(Duration::from_millis(100))? {
        return Ok(false);
    }

    let Event::Key(key) = event::read()? else {
        return Ok(false);
    };

    if key.kind != KeyEventKind::Press {
        return Ok(false);
    }

    // 退出确认
    if app.show_confirm_quit {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
            _ => {
                app.show_confirm_quit = false;
                return Ok(false);
            }
        }
    }

    // 全局快捷键处理
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(true);
    }

    match app.screen {
        Screen::Login => handle_login_keys(app, key.code).await,
        Screen::Main => handle_main_keys(app, key.code).await,
    }
}

async fn handle_login_keys(app: &mut App, key: KeyCode) -> Result<bool, Box<dyn std::error::Error>> {
    match key {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Enter => {
            if app.login_state == LoginState::Idle || app.login_state == LoginState::Error {
                start_device_login(app).await;
            }
        }
        KeyCode::Char('c') => {
            if app.login_state == LoginState::WaitingForAuth {
                let text = app.login_user_code.clone();
                #[cfg(target_os = "linux")]
                {
                    std::thread::spawn(move || {
                        if let Ok(mut cb) = arboard::Clipboard::new() {
                            use arboard::SetExtLinux;
                            let _ = cb.set().wait().text(text);
                        }
                    });
                }
                #[cfg(not(target_os = "linux"))]
                {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(text);
                    }
                }
                app.status_message = "验证码已复制到剪贴板".to_string();
            }
        }
        KeyCode::Char('u') => {
            if app.login_state == LoginState::WaitingForAuth {
                let text = app.login_verification_uri.clone();
                #[cfg(target_os = "linux")]
                {
                    std::thread::spawn(move || {
                        if let Ok(mut cb) = arboard::Clipboard::new() {
                            use arboard::SetExtLinux;
                            let _ = cb.set().wait().text(text);
                        }
                    });
                }
                #[cfg(not(target_os = "linux"))]
                {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        let _ = cb.set_text(text);
                    }
                }
                app.status_message = "授权链接已复制到剪贴板".to_string();
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_main_keys(app: &mut App, key: KeyCode) -> Result<bool, Box<dyn std::error::Error>> {
    match key {
        KeyCode::Char('q') => {
            // 检查是否有运行中的隧道
            if !app.running_tunnels.is_empty() {
                app.show_confirm_quit = true;
                app.status_message = "有隧道运行中，按 y 确认退出（隧道将继续在后台运行）".to_string();
            } else {
                return Ok(true);
            }
        }
        KeyCode::Char('1') => app.tab = Tab::Tunnels,
        KeyCode::Char('2') => app.tab = Tab::Logs,
        KeyCode::Char('3') => app.tab = Tab::Settings,
        KeyCode::Up | KeyCode::Char('k') => {
            if app.tab == Tab::Tunnels && app.selected_tunnel > 0 {
                app.selected_tunnel -= 1;
            }
            if app.tab == Tab::Logs && app.log_scroll > 0 {
                app.log_scroll -= 1;
            }
            if app.tab == Tab::Settings && app.selected_setting > 0 {
                app.selected_setting -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.tab == Tab::Tunnels && app.selected_tunnel + 1 < app.tunnels.len() {
                app.selected_tunnel += 1;
            }
            if app.tab == Tab::Logs {
                app.log_scroll += 1;
            }
            if app.tab == Tab::Settings && app.selected_setting < 1 {
                app.selected_setting += 1;
            }
        }
        KeyCode::Enter => {
            if app.tab == Tab::Tunnels && !app.tunnels.is_empty() {
                let tunnel_id = app.tunnels[app.selected_tunnel].id;
                if app.running_tunnels.contains(&tunnel_id) {
                    app.stop_selected_tunnel();
                } else {
                    app.start_selected_tunnel().await;
                }
            } else if app.tab == Tab::Settings {
                if app.selected_setting == 0 {
                    if app.is_systemd_installed {
                        app.status_message = "请手动执行 sudo systemctl disable --now chmlfrp-tui.service 并删除 service 文件".to_string();
                    } else {
                        // TODO: Implement install_systemd_service call later
                        app.status_message = "请使用 sudo 安装 deb 包获取后台服务支持".to_string();
                    }
                } else if app.selected_setting == 1 {
                    app.settings.auto_start_tunnels_enabled = !app.settings.auto_start_tunnels_enabled;
                    let _ = crate::storage::save_settings(&app.settings);
                    app.status_message = if app.settings.auto_start_tunnels_enabled {
                        "已开启软件启动时自动开启隧道".to_string()
                    } else {
                        "已关闭软件启动时自动开启隧道".to_string()
                    };
                }
            }
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            if app.tab == Tab::Tunnels && !app.tunnels.is_empty() {
                let tunnel_id = app.tunnels[app.selected_tunnel].id;
                if app.settings.auto_start_tunnel_ids.contains(&tunnel_id) {
                    app.settings.auto_start_tunnel_ids.retain(|&id| id != tunnel_id);
                    app.status_message = format!("已取消隧道 {} (ID: {}) 的自启标记", app.tunnels[app.selected_tunnel].name, tunnel_id);
                } else {
                    app.settings.auto_start_tunnel_ids.push(tunnel_id);
                    app.status_message = format!("隧道 {} (ID: {}) 已设为自动启动", app.tunnels[app.selected_tunnel].name, tunnel_id);
                }
                let _ = crate::storage::save_settings(&app.settings);
            }
        }
        KeyCode::Char('r') => {
            app.refresh_tunnels().await;
        }
        KeyCode::Char('d') => {
            if !app.is_downloading {
                start_download(app).await;
            }
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            // 注销
            app.stored_user = None;
            app.tunnels.clear();
            app.running_tunnels.clear();
            app.screen = Screen::Login;
            let _ = crate::storage::clear_user();
            app.status_message = "已注销".to_string();
        }
        KeyCode::Char('o') | KeyCode::Char('O') => {
            if app.tab == Tab::Tunnels && !app.tunnels.is_empty() {
                let tunnel = &app.tunnels[app.selected_tunnel];
                let url = if tunnel.tunnel_type == "http" || tunnel.tunnel_type == "https" {
                    format!("{}://{}", tunnel.tunnel_type, tunnel.dorp)
                } else {
                    format!("http://{}:{}", tunnel.node_ip, tunnel.dorp)
                };
                let _ = open::that(&url);
                app.status_message = format!("在浏览器中打开: {}", url);
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn start_device_login(app: &mut App) {
    app.login_state = LoginState::WaitingForAuth;
    app.login_error.clear();
    app.status_message = "正在申请设备授权...".to_string();

    let auth = match chmlfrp_core::auth::create_device_authorization().await {
        Ok(auth) => auth,
        Err(e) => {
            app.login_state = LoginState::Error;
            app.login_error = format!("申请设备授权失败: {}", e);
            return;
        }
    };

    app.login_user_code = auth.user_code.clone();
    app.login_verification_uri = auth.verification_uri.clone();
    app.login_device_code = auth.device_code.clone();
    app.status_message = "请在浏览器中完成授权".to_string();

    let uri_to_open = auth.verification_uri_complete.unwrap_or(auth.verification_uri.clone());
    let _ = open::that(uri_to_open);

    let interval = auth.interval.unwrap_or(5);
    let expires_in = auth.expires_in.unwrap_or(600);
    let device_code = auth.device_code.clone();

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    app.login_result_rx = Some(rx);

    tokio::spawn(async move {
        let result = chmlfrp_core::auth::poll_device_authorization(
            &device_code,
            interval,
            expires_in,
        )
        .await;

        match result {
            Ok(token_resp) => {
                if let Some(ref access_token) = token_resp.access_token {
                    match chmlfrp_core::auth::login_with_access_token(access_token, &token_resp).await {
                        Ok(user) => {
                            let _ = tx.send(Ok(user));
                        }
                        Err(e) => {
                            let _ = tx.send(Err(format!("获取用户信息失败: {}", e)));
                        }
                    }
                } else {
                    let _ = tx.send(Err("授权成功但未返回 token".to_string()));
                }
            }
            Err(e) => {
                let _ = tx.send(Err(e));
            }
        }
    });
}

async fn start_download(app: &mut App) {
    app.is_downloading = true;
    app.download_progress = 0.0;
    app.status_message = "正在下载 frpc...".to_string();

    let data_dir = app.data_dir.clone();

    let (prog_tx, prog_rx) = tokio::sync::mpsc::unbounded_channel();
    let (res_tx, res_rx) = tokio::sync::mpsc::unbounded_channel();

    app.download_progress_rx = Some(prog_rx);
    app.download_result_rx = Some(res_rx);

    tokio::spawn(async move {
        let result = chmlfrp_core::download::download_frpc(&data_dir, move |progress| {
            let _ = prog_tx.send(progress.percentage);
        })
        .await;

        let _ = res_tx.send(result);
    });
}
