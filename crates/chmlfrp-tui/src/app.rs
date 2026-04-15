use chmlfrp_core::models::{
    FrpcProcesses, LogMessage, StoredUser, Tunnel,
};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Login,
    Main,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Tunnels,
    Logs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginState {
    Idle,
    WaitingForAuth,
    Error,
}

pub struct App {
    pub screen: Screen,
    pub tab: Tab,
    pub stored_user: Option<StoredUser>,
    pub tunnels: Vec<Tunnel>,
    pub running_tunnels: Vec<i32>,
    pub selected_tunnel: usize,
    pub logs: Vec<LogMessage>,
    pub log_scroll: u16,
    pub status_message: String,
    pub login_state: LoginState,
    pub login_user_code: String,
    pub login_verification_uri: String,
    pub login_error: String,
    pub login_device_code: String,
    pub processes: FrpcProcesses,
    pub log_tx: mpsc::UnboundedSender<LogMessage>,
    pub log_rx: mpsc::UnboundedReceiver<LogMessage>,
    pub data_dir: std::path::PathBuf,
    pub is_downloading: bool,
    pub download_progress: f64,
    pub show_confirm_quit: bool,
    pub login_result_rx: Option<mpsc::UnboundedReceiver<Result<StoredUser, String>>>,
    pub download_result_rx: Option<mpsc::UnboundedReceiver<Result<std::path::PathBuf, String>>>,
    pub download_progress_rx: Option<mpsc::UnboundedReceiver<f64>>,
    pub needs_refresh: bool,
}

impl App {
    pub fn new() -> Self {
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        let data_dir = crate::storage::get_data_dir();

        Self {
            screen: Screen::Login,
            tab: Tab::Tunnels,
            stored_user: None,
            tunnels: Vec::new(),
            running_tunnels: Vec::new(),
            selected_tunnel: 0,
            logs: Vec::new(),
            log_scroll: 0,
            status_message: String::new(),
            login_state: LoginState::Idle,
            login_user_code: String::new(),
            login_verification_uri: String::new(),
            login_error: String::new(),
            login_device_code: String::new(),
            processes: FrpcProcesses::new(),
            log_tx,
            log_rx,
            data_dir,
            is_downloading: false,
            download_progress: 0.0,
            show_confirm_quit: false,
            login_result_rx: None,
            download_result_rx: None,
            download_progress_rx: None,
            needs_refresh: false,
        }
    }

    /// 获取有效 token
    pub fn get_token(&self) -> Option<String> {
        let user = self.stored_user.as_ref()?;
        user.access_token
            .clone()
            .or_else(|| user.usertoken.clone())
    }

    /// 刷新隧道列表
    pub async fn refresh_tunnels(&mut self) {
        let token = match self.get_token() {
            Some(t) => t,
            None => {
                self.status_message = "未登录".to_string();
                return;
            }
        };

        match chmlfrp_core::api::fetch_tunnels(&token).await {
            Ok(tunnels) => {
                self.tunnels = tunnels;
                // 刷新运行状态
                match chmlfrp_core::process::get_running_tunnels(&self.data_dir, &self.processes) {
                    Ok(running) => self.running_tunnels = running,
                    Err(_) => self.running_tunnels.clear(),
                }
                self.status_message = format!("已加载 {} 个隧道", self.tunnels.len());
            }
            Err(e) => {
                if e.contains("无效的登录状态") || e.contains("授权已过期") || e.contains("Token无效") {
                    self.status_message = "登录已过期，请按 L 重新登录".to_string();
                    self.stored_user = None;
                    let _ = crate::storage::clear_user();
                    self.tunnels.clear();
                    self.running_tunnels.clear();
                } else {
                    self.status_message = format!("获取隧道失败: {}", e);
                }
            }
        }
    }

    /// 启动选中的隧道
    pub async fn start_selected_tunnel(&mut self) {
        let tunnel = match self.tunnels.get(self.selected_tunnel) {
            Some(t) => t.clone(),
            None => return,
        };

        if self.running_tunnels.contains(&tunnel.id) {
            self.status_message = "隧道已在运行中".to_string();
            return;
        }

        // 检查 frpc 是否存在
        if !chmlfrp_core::download::check_frpc_exists(&self.data_dir) {
            self.status_message = "frpc 未下载，请按 d 下载".to_string();
            return;
        }

        let token = match self.get_token() {
            Some(t) => t,
            None => return,
        };

        // 获取节点信息以构建 TunnelConfig
        let _node_info = match chmlfrp_core::api::fetch_node_info(&tunnel.node, &token).await {
            Ok(info) => info,
            Err(e) => {
                self.status_message = format!("获取节点信息失败: {}", e);
                return;
            }
        };

        let config = chmlfrp_core::models::TunnelConfig {
            tunnel_id: tunnel.id,
            tunnel_name: tunnel.name.clone(),
            user_token: self.stored_user.as_ref().unwrap().usertoken.clone().unwrap_or_default(),
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

        match chmlfrp_core::process::start_frpc(
            &self.data_dir,
            &config,
            &self.processes,
            self.log_tx.clone(),
        ) {
            Ok(pid) => {
                self.running_tunnels.push(tunnel.id);
                self.status_message = format!("隧道 {} 已启动 (PID: {})", tunnel.name, pid);
            }
            Err(e) => {
                self.status_message = format!("启动失败: {}", e);
            }
        }
    }

    /// 停止选中的隧道
    pub fn stop_selected_tunnel(&mut self) {
        let tunnel = match self.tunnels.get(self.selected_tunnel) {
            Some(t) => t.clone(),
            None => return,
        };

        if !self.running_tunnels.contains(&tunnel.id) {
            self.status_message = "隧道未在运行".to_string();
            return;
        }

        match chmlfrp_core::process::stop_frpc(&self.data_dir, tunnel.id, &self.processes) {
            Ok(msg) => {
                self.running_tunnels.retain(|id| *id != tunnel.id);
                self.status_message = format!("{}: {}", tunnel.name, msg);
            }
            Err(e) => {
                self.status_message = format!("停止失败: {}", e);
            }
        }
    }

    /// 收集新日志和后台任务结果
    pub fn drain_events(&mut self) {
        // 处理日志
        while let Ok(log) = self.log_rx.try_recv() {
            self.logs.push(log);
            // 保留最近 5000 条
            if self.logs.len() > 5000 {
                self.logs.drain(..self.logs.len() - 5000);
            }
        }

        // 处理登录结果
        if let Some(ref mut rx) = self.login_result_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(user) => {
                        let _ = crate::storage::save_user(&user);
                        self.stored_user = Some(user);
                        self.screen = Screen::Main;
                        self.login_state = LoginState::Idle;
                        self.login_result_rx = None;
                        self.needs_refresh = true;
                        self.status_message = "登录成功！正在刷新隧道列表...".to_string();
                    }
                    Err(e) => {
                        self.login_state = LoginState::Error;
                        self.login_error = e;
                        self.login_result_rx = None;
                    }
                }
            }
        }

        // 处理下载进度
        if let Some(ref mut rx) = self.download_progress_rx {
            while let Ok(progress) = rx.try_recv() {
                self.download_progress = progress;
            }
        }

        // 处理下载结果
        if let Some(ref mut rx) = self.download_result_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(path) => {
                        self.status_message = format!("frpc 下载完成: {}", path.display());
                    }
                    Err(e) => {
                        self.status_message = format!("下载失败: {}", e);
                    }
                }
                self.is_downloading = false;
                self.download_result_rx = None;
                self.download_progress_rx = None;
            }
        }
    }
}
