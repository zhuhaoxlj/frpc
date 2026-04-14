use crate::app::App;
use crate::storage;
use chmlfrp_core::models::TunnelConfig;
use chmlfrp_core::process::{get_running_tunnels, start_frpc, stop_frpc};
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
            // 在真正的实现中，我们应该从 `tunnel_auto_start.json` 读取需要自启的隧道。
            // 这里为了简单，我们启动**所有**隧道。你可以修改为仅启动指定隧道。
            let running = get_running_tunnels(&app.data_dir, &app.processes).unwrap_or_default();

            for tunnel in tunnels {
                if !running.contains(&tunnel.id) {
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
        }

        // 睡眠一段时间再检查 (比如每 60 秒检查一次保活)
        sleep(Duration::from_secs(60)).await;
    }
}
