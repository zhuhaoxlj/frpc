use crate::app::{App, LoginState, Screen, Tab, TunnelPageMode};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::time::Duration;

/// 处理事件，返回 true 表示退出
pub async fn handle_events(app: &mut App) -> Result<bool, Box<dyn std::error::Error>> {
    app.drain_events();

    if app.needs_refresh {
        app.needs_refresh = false;
        app.refresh_tunnels().await;
        if app.settings.auto_start_tunnels_enabled {
            app.needs_auto_start = true;
        }
    }

    if app.needs_auto_start {
        app.needs_auto_start = false;
        app.start_auto_tunnels().await;
    }

    if !event::poll(Duration::from_millis(100))? {
        return Ok(false);
    }

    let Event::Key(key) = event::read()? else {
        return Ok(false);
    };

    if matches!(key.kind, KeyEventKind::Release) {
        return Ok(false);
    }

    if app.show_confirm_quit {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
            _ => {
                app.show_confirm_quit = false;
                return Ok(false);
            }
        }
    }

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
    if app.is_editing_tunnel() {
        return handle_tunnel_editor_keys(app, key).await;
    }

    if app.tab == Tab::Tunnels {
        match app.tunnel_page_mode {
            TunnelPageMode::OfficialNodeSelect => {
                return handle_official_node_select_keys(app, key).await;
            }
            TunnelPageMode::OfficialForm => {
                return handle_official_form_keys(app, key).await;
            }
            TunnelPageMode::ApiDeleteConfirm => {
                return handle_api_delete_confirm_keys(app, key).await;
            }
            TunnelPageMode::List => {}
        }
    }

    match key {
        KeyCode::Char('q') => {
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
            if app.tab == Tab::Tunnels {
                app.move_tunnel_selection_up();
            }
            if app.tab == Tab::Logs && app.log_scroll > 0 {
                app.log_scroll -= 1;
            }
            if app.tab == Tab::Settings && app.selected_setting > 0 {
                app.selected_setting -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.tab == Tab::Tunnels {
                app.move_tunnel_selection_down();
            }
            if app.tab == Tab::Logs {
                app.log_scroll += 1;
            }
            if app.tab == Tab::Settings && app.selected_setting < 1 {
                app.selected_setting += 1;
            }
        }
        KeyCode::Enter => {
            if app.tab == Tab::Tunnels && !app.tunnel_items.is_empty() {
                let tunnel_id = app.selected_tunnel_id().unwrap_or_default();
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
            if app.tab == Tab::Tunnels {
                if let Some(tunnel_id) = app.selected_tunnel_id() {
                    let name = app
                        .selected_tunnel_item()
                        .map(|item| item.name().to_string())
                        .unwrap_or_default();
                    if app.settings.auto_start_tunnel_ids.contains(&tunnel_id) {
                        app.settings.auto_start_tunnel_ids.retain(|&id| id != tunnel_id);
                        app.status_message = format!("已取消隧道 {} (ID: {}) 的自启标记", name, tunnel_id);
                    } else {
                        app.settings.auto_start_tunnel_ids.push(tunnel_id);
                        app.status_message = format!("隧道 {} (ID: {}) 已设为自动启动", name, tunnel_id);
                    }
                    let _ = crate::storage::save_settings(&app.settings);
                }
            }
        }
        KeyCode::Char('c') if app.tab == Tab::Tunnels => {
            app.start_official_tunnel_creation().await;
        }
        KeyCode::Char('n') if app.tab == Tab::Tunnels => app.start_new_tunnel_editor(),
        KeyCode::Char('e') if app.tab == Tab::Tunnels => app.start_edit_selected_tunnel().await,
        KeyCode::Char('x') if app.tab == Tab::Tunnels => app.request_delete_selected_tunnel(),
        KeyCode::Char('r') => {
            app.refresh_tunnels().await;
        }
        KeyCode::Char('d') => {
            if !app.is_downloading {
                start_download(app).await;
            }
        }
        KeyCode::Char('l') | KeyCode::Char('L') => {
            app.stored_user = None;
            app.tunnels.clear();
            app.tunnel_items.clear();
            app.running_tunnels.clear();
            app.screen = Screen::Login;
            let _ = crate::storage::clear_user();
            app.status_message = "已注销".to_string();
        }
        KeyCode::Char('o') | KeyCode::Char('O') => {
            if app.tab == Tab::Tunnels {
                app.open_selected_tunnel_url();
            }
        }
        _ => {}
    }
    Ok(false)
}

async fn handle_tunnel_editor_keys(
    app: &mut App,
    key: KeyCode,
) -> Result<bool, Box<dyn std::error::Error>> {
    match key {
        KeyCode::Esc => app.cancel_tunnel_editor(),
        KeyCode::Char('s') | KeyCode::Char('S') => app.save_tunnel_editor(),
        KeyCode::Backspace => app.backspace_tunnel_editor(),
        KeyCode::Enter => app.append_tunnel_editor_newline(),
        KeyCode::Tab => {
            for _ in 0..4 {
                app.append_tunnel_editor_char(' ');
            }
        }
        KeyCode::Char(ch) => app.append_tunnel_editor_char(ch),
        _ => {}
    }
    Ok(false)
}

async fn handle_official_node_select_keys(
    app: &mut App,
    key: KeyCode,
) -> Result<bool, Box<dyn std::error::Error>> {
    match key {
        KeyCode::Esc => app.cancel_official_node_select(),
        KeyCode::Up | KeyCode::Char('k') => app.move_official_node_selection_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_official_node_selection_down(),
        KeyCode::Enter => app.start_official_tunnel_form().await,
        _ => {}
    }
    Ok(false)
}

async fn handle_official_form_keys(
    app: &mut App,
    key: KeyCode,
) -> Result<bool, Box<dyn std::error::Error>> {
    match key {
        KeyCode::Esc => app.return_official_form_to_node_select(),
        KeyCode::Up | KeyCode::Char('k') => app.move_official_form_field_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_official_form_field_down(),
        KeyCode::Backspace => app.backspace_official_form(),
        KeyCode::Tab => app.toggle_current_official_form_field(),
        KeyCode::Char(' ') => app.toggle_current_official_form_field(),
        KeyCode::Enter => app.submit_official_tunnel().await,
        KeyCode::Char(ch) => app.append_official_form_char(ch),
        _ => {}
    }
    Ok(false)
}

async fn handle_api_delete_confirm_keys(
    app: &mut App,
    key: KeyCode,
) -> Result<bool, Box<dyn std::error::Error>> {
    match key {
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => app.cancel_api_delete_confirm(),
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_api_delete().await,
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
        let result = match chmlfrp_core::auth::poll_device_authorization(&device_code, interval, expires_in).await {
            Ok(token_response) => {
                if let Some(access_token) = token_response.access_token.as_deref() {
                    chmlfrp_core::auth::login_with_access_token(access_token, &token_response).await
                } else {
                    Err("设备授权成功但未返回 access token".to_string())
                }
            }
            Err(err) => Err(err),
        };
        let _ = tx.send(result);
    });
}

async fn start_download(app: &mut App) {
    app.is_downloading = true;
    app.download_progress = 0.0;
    app.status_message = "开始下载 frpc...".to_string();

    let data_dir = app.data_dir.clone();
    let (result_tx, result_rx) = tokio::sync::mpsc::unbounded_channel();
    let (progress_tx, progress_rx) = tokio::sync::mpsc::unbounded_channel();
    app.download_result_rx = Some(result_rx);
    app.download_progress_rx = Some(progress_rx);

    tokio::spawn(async move {
        let result = chmlfrp_core::download::download_frpc(&data_dir, move |progress| {
            let _ = progress_tx.send(progress.percentage);
        })
        .await;
        let _ = result_tx.send(result);
    });
}
