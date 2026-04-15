use crate::app::App;
use crate::storage;
use chmlfrp_core::models::TunnelConfig;
use chmlfrp_core::process::{get_running_tunnels, start_frpc};
use std::time::Duration;
use tokio::time::sleep;

pub async fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    println!("ChmlFrp TUI 守护进程已启动...");

    let mut app = App::new();

    // 1. 加载用户
    let user = match storage::load_user()? {
        Some(u) => u,
        None => {
            eprintln!("错误: 尚未登录，守护进程无法启动。请先运行 `chmlfrp-tui` 并完成登录。");
            return Ok(());
        }
    };
    app.stored_user = Some(user);

    // 2. 无限循环维护后台
    loop {
        // 尝试刷新隧道列表
        app.refresh_tunnels().await;

        let tunnels = app.tunnels.clone();

        if tunnels.is_empty() {
            println!("警告: 未找到任何隧道。将会在 60 秒后重试。");
        } else {
            // 加载最新设置以响应前台的修改
            if let Ok(settings) = storage::load_settings() {
                app.settings = settings;
            }

            if app.settings.auto_start_tunnels_enabled {
                let running = get_running_tunnels(&app.data_dir, &app.processes).unwrap_or_default();

                for tunnel in tunnels {
                    if !running.contains(&tunnel.id) && app.settings.auto_start_tunnel_ids.contains(&tunnel.id) {
                        println!("尝试自动启动隧道: {} (ID: {})", tunnel.name, tunnel.id);
                        // 构造 TunnelConfig
                        let config = TunnelConfig {
                            tunnel_id: tunnel.id,
                            tunnel_name: tunnel.name.clone(),
                            user_token: app.stored_user.as_ref().unwrap().usertoken.clone().unwrap_or_default(),
                            server_addr: tunnel.node_ip.clone(),
                            server_port: tunnel.server_port,
                            node_token: tunnel.node_token.clone(),
                            tunnel_type: tunnel.tunnel_type.clone(),
                            local_ip: tunnel.localip.clone(),
                            local_port: tunnel.nport as u16,
                            remote_port: if tunnel.tunnel_type == "tcp" || tunnel.tunnel_type == "udp" {
                                tunnel.dorp.parse().ok()
                            } else {
                                None
                            },
                            custom_domains: if tunnel.tunnel_type == "http" || tunnel.tunnel_type == "https" {
                                Some(tunnel.dorp.clone())
                            } else {
                                None
                            },
                            http_proxy: None,
                            log_level: "info".to_string(),
                            force_tls: true,
                            kcp_optimization: false,
                        };

                        match start_frpc(&app.data_dir, &config, &app.processes, app.log_tx.clone()) {
                            Ok(pid) => {
                                println!("隧道 {} 已启动，PID: {}", tunnel.name, pid);
                            }
                            Err(e) => {
                                eprintln!("启动隧道 {} 失败: {}", tunnel.name, e);
                            }
                        }
                    }
                }
            } else {
                // 如果关了自启，是否应该停掉那些进程？
                // 根据常见的行为，如果是关了，一般不主动去杀原有的，只是不去启动新开机或者挂掉的
                // 这里为了简单，我们就不杀已有进程，如果用户需要可以通过TUI或者kill关闭。
            }
        }

        // 睡眠一段时间再检查 (比如每 60 秒检查一次保活)
        sleep(Duration::from_secs(60)).await;
    }
}
