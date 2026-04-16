use crate::custom_tunnel::{self, CustomTunnel};
use chmlfrp_core::models::{
    CreateTunnelParams, FrpcProcesses, LogMessage, Node, NodeInfo, StoredUser, Tunnel,
    TunnelConfig, UpdateTunnelParams,
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
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginState {
    Idle,
    WaitingForAuth,
    Error,
}

#[derive(Debug, Clone)]
pub enum TunnelListItem {
    Api(Tunnel),
    Local(CustomTunnel),
}

impl TunnelListItem {
    pub fn id(&self) -> i32 {
        match self {
            Self::Api(tunnel) => tunnel.id,
            Self::Local(tunnel) => tunnel
                .hashed_id
                .unwrap_or_else(|| custom_tunnel::get_custom_tunnel_hash(&tunnel.id)),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Api(tunnel) => &tunnel.name,
            Self::Local(tunnel) => &tunnel.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelEditorMode {
    New,
    Edit { tunnel_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TunnelPageMode {
    List,
    OfficialNodeSelect,
    OfficialForm,
    ApiDeleteConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficialTunnelProtocol {
    Tcp,
    Udp,
    Http,
    Https,
}

impl OfficialTunnelProtocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
            Self::Http => "HTTP",
            Self::Https => "HTTPS",
        }
    }

    pub fn from_tunnel_type(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "udp" => Self::Udp,
            "http" => Self::Http,
            "https" => Self::Https,
            _ => Self::Tcp,
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Self::Tcp => Self::Udp,
            Self::Udp => Self::Http,
            Self::Http => Self::Https,
            Self::Https => Self::Tcp,
        }
    }

    pub fn is_http(self) -> bool {
        matches!(self, Self::Http | Self::Https)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficialTunnelMode {
    Create,
    Edit { tunnel_id: i32 },
}

impl OfficialTunnelMode {
    pub fn is_edit(self) -> bool {
        matches!(self, Self::Edit { .. })
    }
}

#[derive(Debug, Clone)]
pub struct OfficialTunnelForm {
    pub tunnel_name: String,
    pub local_ip: String,
    pub local_port: String,
    pub remote_port: String,
    pub domain: String,
    pub protocol: OfficialTunnelProtocol,
    pub encryption: bool,
    pub compression: bool,
    pub selected_field: usize,
}

impl Default for OfficialTunnelForm {
    fn default() -> Self {
        Self {
            tunnel_name: String::new(),
            local_ip: "127.0.0.1".to_string(),
            local_port: String::new(),
            remote_port: String::new(),
            domain: String::new(),
            protocol: OfficialTunnelProtocol::Tcp,
            encryption: false,
            compression: false,
            selected_field: 0,
        }
    }
}

fn parse_port_bounds(segment: &str) -> Option<(u16, u16)> {
    let segment = segment.trim();
    if segment.is_empty() {
        return None;
    }

    if let Some((start, end)) = segment.split_once('-') {
        let start = start.trim().parse::<u16>().ok()?;
        let end = end.trim().parse::<u16>().ok()?;
        Some((start.min(end), start.max(end)))
    } else {
        let port = segment.parse::<u16>().ok()?;
        Some((port, port))
    }
}

fn first_allowed_port(port_range: &str) -> Option<u16> {
    port_range
        .split(|ch: char| ch == ',' || ch == '，' || ch.is_whitespace())
        .find_map(|segment| parse_port_bounds(segment).map(|(start, _)| start))
}

fn port_is_in_allowed_range(port: u16, port_range: &str) -> bool {
    port_range
        .split(|ch: char| ch == ',' || ch == '，' || ch.is_whitespace())
        .filter_map(parse_port_bounds)
        .any(|(start, end)| (start..=end).contains(&port))
}

pub struct App {
    pub screen: Screen,
    pub tab: Tab,
    pub stored_user: Option<StoredUser>,
    pub tunnels: Vec<Tunnel>,
    pub custom_tunnels: Vec<CustomTunnel>,
    pub tunnel_items: Vec<TunnelListItem>,
    pub tunnel_page_mode: TunnelPageMode,
    pub official_nodes: Vec<Node>,
    pub selected_official_node: usize,
    pub official_tunnel_mode: OfficialTunnelMode,
    pub official_tunnel_form: OfficialTunnelForm,
    pub official_node_info: Option<NodeInfo>,
    pub is_submitting_official_tunnel: bool,
    pub api_delete_target: Option<Tunnel>,
    pub running_tunnels: Vec<i32>,
    pub selected_tunnel: usize,
    pub tunnel_editor: Option<TunnelEditorMode>,
    pub tunnel_editor_content: String,
    pub settings: crate::storage::AppSettings,
    pub selected_setting: usize,
    pub is_systemd_installed: bool,
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
    pub needs_auto_start: bool,
}

impl App {
    pub fn new() -> Self {
        let (log_tx, log_rx) = mpsc::unbounded_channel();
        let data_dir = crate::storage::get_data_dir();
        let settings = crate::storage::load_settings().unwrap_or_default();
        let is_systemd_installed = std::path::Path::new("/etc/systemd/system/chmlfrp-tui.service").exists()
            || std::path::Path::new("/lib/systemd/system/chmlfrp-tui.service").exists();
        let logs = chmlfrp_core::persistence::load_persisted_logs(&data_dir, 5000);

        Self {
            screen: Screen::Login,
            tab: Tab::Tunnels,
            stored_user: None,
            tunnels: Vec::new(),
            custom_tunnels: Vec::new(),
            tunnel_items: Vec::new(),
            tunnel_page_mode: TunnelPageMode::List,
            official_nodes: Vec::new(),
            selected_official_node: 0,
            official_tunnel_mode: OfficialTunnelMode::Create,
            official_tunnel_form: OfficialTunnelForm::default(),
            official_node_info: None,
            is_submitting_official_tunnel: false,
            api_delete_target: None,
            running_tunnels: Vec::new(),
            selected_tunnel: 0,
            tunnel_editor: None,
            tunnel_editor_content: String::new(),
            settings,
            selected_setting: 0,
            is_systemd_installed,
            logs,
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
            needs_auto_start: false,
        }
    }

    pub fn is_editing_tunnel(&self) -> bool {
        self.tunnel_editor.is_some()
    }

    pub fn is_in_tunnel_list_mode(&self) -> bool {
        self.tunnel_page_mode == TunnelPageMode::List && !self.is_editing_tunnel()
    }

    pub fn selected_official_node(&self) -> Option<&Node> {
        self.official_nodes.get(self.selected_official_node)
    }

    pub fn current_official_rport(&self) -> Option<&str> {
        self.official_node_info
            .as_ref()
            .map(|node_info| node_info.rport.as_str())
    }

    pub fn move_official_node_selection_up(&mut self) {
        if self.selected_official_node > 0 {
            self.selected_official_node -= 1;
        }
    }

    pub fn move_official_node_selection_down(&mut self) {
        if self.selected_official_node + 1 < self.official_nodes.len() {
            self.selected_official_node += 1;
        }
    }

    pub fn move_official_form_field_up(&mut self) {
        if self.official_tunnel_form.selected_field > 0 {
            self.official_tunnel_form.selected_field -= 1;
        }
    }

    pub fn move_official_form_field_down(&mut self) {
        if self.official_tunnel_form.selected_field + 1 < self.official_form_field_count() {
            self.official_tunnel_form.selected_field += 1;
        }
    }

    fn official_form_field_count(&self) -> usize {
        if self.official_tunnel_form.protocol.is_http() {
            7
        } else {
            7
        }
    }

    fn reset_official_tunnel_form(&mut self) {
        self.official_tunnel_form = OfficialTunnelForm::default();
    }

    fn reset_official_flow_state(&mut self) {
        self.official_tunnel_mode = OfficialTunnelMode::Create;
        self.official_node_info = None;
        self.api_delete_target = None;
        self.is_submitting_official_tunnel = false;
        self.reset_official_tunnel_form();
    }

    fn apply_official_node_info(&mut self, node_info: NodeInfo) {
        self.official_tunnel_form.remote_port = first_allowed_port(&node_info.rport)
            .map(|port| port.to_string())
            .unwrap_or_default();
        self.official_node_info = Some(node_info);
    }

    pub async fn start_official_tunnel_creation(&mut self) {
        let Some(token) = self.get_token().await else {
            self.status_message = "未登录，无法加载节点列表".to_string();
            return;
        };

        self.reset_official_flow_state();
        self.status_message = "正在加载节点列表...".to_string();
        match chmlfrp_core::api::fetch_nodes(&token).await {
            Ok(nodes) => {
                if nodes.is_empty() {
                    self.status_message = "暂无可用节点".to_string();
                    return;
                }
                self.official_nodes = nodes;
                self.selected_official_node = 0;
                self.official_tunnel_mode = OfficialTunnelMode::Create;
                self.tunnel_page_mode = TunnelPageMode::OfficialNodeSelect;
                self.status_message = "请选择节点并按 Enter 继续创建官方隧道".to_string();
            }
            Err(err) => {
                self.status_message = format!("获取节点列表失败: {}", err);
            }
        }
    }

    pub async fn start_official_tunnel_form(&mut self) {
        let Some(node_name) = self.selected_official_node().map(|node| node.name.clone()) else {
            self.status_message = "暂无可选节点".to_string();
            return;
        };

        let Some(token) = self.get_token().await else {
            self.status_message = "未登录，无法加载节点详情".to_string();
            return;
        };

        self.status_message = format!("正在加载节点 {} 的端口范围...", node_name);
        match chmlfrp_core::api::fetch_node_info(&node_name, &token).await {
            Ok(node_info) => {
                self.apply_official_node_info(node_info);
                self.tunnel_page_mode = TunnelPageMode::OfficialForm;
                self.status_message = if self.official_tunnel_mode.is_edit() {
                    format!("正在编辑节点 {} 上的官方隧道", node_name)
                } else {
                    format!("正在为节点 {} 创建官方隧道", node_name)
                };
            }
            Err(err) => {
                self.status_message = format!("获取节点详情失败: {}", err);
            }
        }
    }

    pub fn cancel_official_node_select(&mut self) {
        let status = if self.official_tunnel_mode.is_edit() {
            "已取消编辑官方隧道"
        } else {
            "已取消创建官方隧道"
        };
        self.tunnel_page_mode = TunnelPageMode::List;
        self.official_nodes.clear();
        self.reset_official_flow_state();
        self.status_message = status.to_string();
    }

    pub fn return_official_form_to_node_select(&mut self) {
        self.tunnel_page_mode = TunnelPageMode::OfficialNodeSelect;
        self.official_node_info = None;
        self.status_message = "已返回节点选择".to_string();
    }

    pub fn toggle_current_official_form_field(&mut self) {
        match self.official_tunnel_form.selected_field {
            4 => {
                self.official_tunnel_form.protocol = self.official_tunnel_form.protocol.cycle();
                if self.official_tunnel_form.protocol.is_http() {
                    self.official_tunnel_form.remote_port.clear();
                } else if self.official_tunnel_form.remote_port.is_empty() {
                    self.official_tunnel_form.remote_port = self
                        .current_official_rport()
                        .and_then(first_allowed_port)
                        .map(|port| port.to_string())
                        .unwrap_or_default();
                }
            }
            5 => self.official_tunnel_form.encryption = !self.official_tunnel_form.encryption,
            6 => self.official_tunnel_form.compression = !self.official_tunnel_form.compression,
            _ => {}
        }
    }

    pub fn append_official_form_char(&mut self, ch: char) {
        match self.official_tunnel_form.selected_field {
            0 => self.official_tunnel_form.tunnel_name.push(ch),
            1 => self.official_tunnel_form.local_ip.push(ch),
            2 => {
                if ch.is_ascii_digit() {
                    self.official_tunnel_form.local_port.push(ch);
                }
            }
            3 => {
                if self.official_tunnel_form.protocol.is_http() {
                    self.official_tunnel_form.domain.push(ch);
                } else if ch.is_ascii_digit() {
                    self.official_tunnel_form.remote_port.push(ch);
                }
            }
            _ => {}
        }
    }

    pub fn backspace_official_form(&mut self) {
        match self.official_tunnel_form.selected_field {
            0 => {
                self.official_tunnel_form.tunnel_name.pop();
            }
            1 => {
                self.official_tunnel_form.local_ip.pop();
            }
            2 => {
                self.official_tunnel_form.local_port.pop();
            }
            3 => {
                if self.official_tunnel_form.protocol.is_http() {
                    self.official_tunnel_form.domain.pop();
                } else {
                    self.official_tunnel_form.remote_port.pop();
                }
            }
            _ => {}
        }
    }

    pub async fn submit_official_tunnel(&mut self) {
        if self.is_submitting_official_tunnel {
            return;
        }

        let Some(node) = self.selected_official_node().cloned() else {
            self.status_message = "请选择节点".to_string();
            return;
        };

        let tunnel_name = self.official_tunnel_form.tunnel_name.trim().to_string();
        if tunnel_name.is_empty() {
            self.status_message = "请输入隧道名称".to_string();
            return;
        }

        let local_ip = self.official_tunnel_form.local_ip.trim().to_string();
        if local_ip.is_empty() {
            self.status_message = "请输入本地 IP".to_string();
            return;
        }

        let local_port = match self.official_tunnel_form.local_port.trim().parse::<u16>() {
            Ok(port) if port > 0 => port,
            _ => {
                self.status_message = "请输入有效的本地端口".to_string();
                return;
            }
        };

        let protocol = self.official_tunnel_form.protocol;
        let allowed_rport = self.current_official_rport().map(str::to_string);
        let (remoteport, banddomain) = if protocol.is_http() {
            let domain = self.official_tunnel_form.domain.trim();
            if domain.is_empty() {
                self.status_message = "请输入域名".to_string();
                return;
            }
            (None, Some(domain.to_string()))
        } else {
            let remote_port = match self.official_tunnel_form.remote_port.trim().parse::<u16>() {
                Ok(port) if port > 0 => port,
                _ => {
                    self.status_message = "请输入有效的远程端口".to_string();
                    return;
                }
            };

            if let Some(rport) = allowed_rport.as_deref() {
                if !port_is_in_allowed_range(remote_port, rport) {
                    self.status_message = format!("远程端口必须在节点允许范围内: {}", rport);
                    return;
                }
            }

            (Some(remote_port), None)
        };

        let Some(token) = self.get_token().await else {
            self.status_message = if self.official_tunnel_mode.is_edit() {
                "未登录，无法更新官方隧道".to_string()
            } else {
                "未登录，无法创建官方隧道".to_string()
            };
            return;
        };

        self.is_submitting_official_tunnel = true;

        match self.official_tunnel_mode {
            OfficialTunnelMode::Create => {
                let params = CreateTunnelParams {
                    tunnelname: tunnel_name.clone(),
                    node: node.name.clone(),
                    localip: local_ip,
                    porttype: protocol.label().to_string(),
                    localport: local_port,
                    encryption: self.official_tunnel_form.encryption,
                    compression: self.official_tunnel_form.compression,
                    extraparams: String::new(),
                    remoteport,
                    banddomain,
                };

                self.status_message = format!("正在创建官方隧道 {}...", params.tunnelname);
                match chmlfrp_core::api::create_tunnel(&params, &token).await {
                    Ok(()) => {
                        self.tunnel_page_mode = TunnelPageMode::List;
                        self.official_nodes.clear();
                        self.reset_official_flow_state();
                        self.refresh_tunnels().await;
                        self.select_api_tunnel_by_name(&params.tunnelname);
                        self.status_message = format!("已在节点 {} 创建官方隧道 {}", node.name, params.tunnelname);
                    }
                    Err(err) => {
                        self.is_submitting_official_tunnel = false;
                        self.status_message = format!("创建官方隧道失败: {}", err);
                    }
                }
            }
            OfficialTunnelMode::Edit { tunnel_id } => {
                let params = UpdateTunnelParams {
                    tunnelid: tunnel_id,
                    tunnelname: tunnel_name.clone(),
                    node: node.name.clone(),
                    localip: local_ip,
                    porttype: protocol.label().to_string(),
                    localport: local_port,
                    encryption: self.official_tunnel_form.encryption,
                    compression: self.official_tunnel_form.compression,
                    extraparams: String::new(),
                    remoteport,
                    banddomain,
                };

                self.status_message = format!("正在更新官方隧道 {}...", params.tunnelname);
                match chmlfrp_core::api::update_tunnel(&params, &token).await {
                    Ok(()) => {
                        self.tunnel_page_mode = TunnelPageMode::List;
                        self.official_nodes.clear();
                        self.reset_official_flow_state();
                        self.refresh_tunnels().await;
                        self.select_api_tunnel(tunnel_id);
                        self.status_message = format!("已更新官方隧道 {}", tunnel_name);
                    }
                    Err(err) => {
                        self.is_submitting_official_tunnel = false;
                        self.status_message = format!("更新官方隧道失败: {}", err);
                    }
                }
            }
        }
    }

    pub fn selected_tunnel_item(&self) -> Option<&TunnelListItem> {
        self.tunnel_items.get(self.selected_tunnel)
    }

    pub fn selected_tunnel_id(&self) -> Option<i32> {
        self.selected_tunnel_item().map(TunnelListItem::id)
    }

    pub fn move_tunnel_selection_up(&mut self) {
        if self.selected_tunnel > 0 {
            self.selected_tunnel -= 1;
        }
    }

    pub fn move_tunnel_selection_down(&mut self) {
        if self.selected_tunnel + 1 < self.tunnel_items.len() {
            self.selected_tunnel += 1;
        }
    }

    pub fn start_new_tunnel_editor(&mut self) {
        self.tunnel_editor = Some(TunnelEditorMode::New);
        self.tunnel_editor_content = default_custom_tunnel_template();
        self.status_message = "已进入新建自定义隧道模式，编辑后按 s 保存".to_string();
    }

    pub async fn start_edit_selected_tunnel(&mut self) {
        let Some(item) = self.selected_tunnel_item().cloned() else {
            return;
        };

        match item {
            TunnelListItem::Api(tunnel) => self.start_edit_selected_api_tunnel(tunnel).await,
            TunnelListItem::Local(tunnel) => self.start_edit_selected_local_tunnel(tunnel),
        }
    }

    fn start_edit_selected_local_tunnel(&mut self, tunnel: CustomTunnel) {
        match custom_tunnel::get_custom_tunnel_config(&self.data_dir, &tunnel.id) {
            Ok(content) => {
                self.tunnel_editor = Some(TunnelEditorMode::Edit {
                    tunnel_id: tunnel.id.clone(),
                });
                self.tunnel_editor_content = content;
                self.status_message = format!("正在编辑本地隧道 {}", tunnel.name);
            }
            Err(err) => {
                self.status_message = format!("读取配置失败: {}", err);
            }
        }
    }

    async fn start_edit_selected_api_tunnel(&mut self, tunnel: Tunnel) {
        let Some(token) = self.get_token().await else {
            self.status_message = "未登录，无法编辑官方隧道".to_string();
            return;
        };

        self.reset_official_flow_state();
        self.status_message = "正在加载节点列表...".to_string();
        let nodes = match chmlfrp_core::api::fetch_nodes(&token).await {
            Ok(nodes) => nodes,
            Err(err) => {
                self.status_message = format!("获取节点列表失败: {}", err);
                return;
            }
        };

        if nodes.is_empty() {
            self.status_message = "暂无可用节点".to_string();
            return;
        }

        let Some(selected_official_node) = nodes.iter().position(|node| node.name == tunnel.node) else {
            self.status_message = format!("当前节点 {} 不在可选节点列表中，无法编辑", tunnel.node);
            return;
        };

        self.official_nodes = nodes;
        self.selected_official_node = selected_official_node;
        self.official_tunnel_mode = OfficialTunnelMode::Edit {
            tunnel_id: tunnel.id,
        };

        self.status_message = format!("正在加载节点 {} 的端口范围...", tunnel.node);
        match chmlfrp_core::api::fetch_node_info(&tunnel.node, &token).await {
            Ok(node_info) => {
                self.apply_official_node_info(node_info);
                self.official_tunnel_form = OfficialTunnelForm {
                    tunnel_name: tunnel.name.clone(),
                    local_ip: tunnel.localip.clone(),
                    local_port: tunnel.nport.to_string(),
                    remote_port: if tunnel.tunnel_type == "http" || tunnel.tunnel_type == "https" {
                        String::new()
                    } else {
                        tunnel.dorp.clone()
                    },
                    domain: if tunnel.tunnel_type == "http" || tunnel.tunnel_type == "https" {
                        tunnel.dorp.clone()
                    } else {
                        String::new()
                    },
                    protocol: OfficialTunnelProtocol::from_tunnel_type(&tunnel.tunnel_type),
                    encryption: false,
                    compression: false,
                    selected_field: 0,
                };
                self.tunnel_page_mode = TunnelPageMode::OfficialForm;
                self.status_message = format!("正在编辑官方隧道 {}", tunnel.name);
            }
            Err(err) => {
                self.status_message = format!("获取节点详情失败: {}", err);
            }
        }
    }

    pub fn save_tunnel_editor(&mut self) {
        let Some(mode) = self.tunnel_editor.clone() else {
            return;
        };

        let result = match mode {
            TunnelEditorMode::New => custom_tunnel::save_custom_tunnel(&self.data_dir, &self.tunnel_editor_content)
                .map(|created| {
                    let selected_id = created.first().map(|t| t.id.clone());
                    (selected_id, format!("已创建 {} 个本地隧道", created.len()))
                }),
            TunnelEditorMode::Edit { tunnel_id } => {
                custom_tunnel::update_custom_tunnel(&self.data_dir, &tunnel_id, &self.tunnel_editor_content)
                    .map(|tunnel| (Some(tunnel.id.clone()), format!("已更新本地隧道 {}", tunnel.name)))
            }
        };

        match result {
            Ok((selected_id, status)) => {
                self.tunnel_editor = None;
                self.tunnel_editor_content.clear();
                if let Err(err) = self.load_local_tunnels() {
                    self.status_message = format!("重新加载本地隧道失败: {}", err);
                    return;
                }
                self.rebuild_tunnel_items();
                if let Some(selected_id) = selected_id {
                    self.select_local_tunnel(&selected_id);
                }
                self.status_message = status;
            }
            Err(err) => {
                self.status_message = format!("保存失败: {}", err);
            }
        }
    }

    pub fn cancel_tunnel_editor(&mut self) {
        self.tunnel_editor = None;
        self.tunnel_editor_content.clear();
        self.status_message = "已取消编辑本地隧道".to_string();
    }

    pub fn append_tunnel_editor_char(&mut self, ch: char) {
        self.tunnel_editor_content.push(ch);
    }

    pub fn append_tunnel_editor_newline(&mut self) {
        self.tunnel_editor_content.push('\n');
    }

    pub fn backspace_tunnel_editor(&mut self) {
        self.tunnel_editor_content.pop();
    }

    pub fn request_delete_selected_tunnel(&mut self) {
        let Some(item) = self.selected_tunnel_item().cloned() else {
            return;
        };

        match item {
            TunnelListItem::Api(tunnel) => {
                self.api_delete_target = Some(tunnel.clone());
                self.tunnel_page_mode = TunnelPageMode::ApiDeleteConfirm;
                self.status_message = format!("确认删除官方隧道 {} 吗？", tunnel.name);
            }
            TunnelListItem::Local(tunnel) => self.delete_selected_local_tunnel(tunnel),
        }
    }

    fn delete_selected_local_tunnel(&mut self, tunnel: CustomTunnel) {
        let tunnel_id = tunnel
            .hashed_id
            .unwrap_or_else(|| custom_tunnel::get_custom_tunnel_hash(&tunnel.id));
        if self.running_tunnels.contains(&tunnel_id) {
            let _ = chmlfrp_core::process::stop_frpc(&self.data_dir, tunnel_id, &self.processes);
            self.running_tunnels.retain(|id| *id != tunnel_id);
        }

        match custom_tunnel::delete_custom_tunnel(&self.data_dir, &tunnel.id) {
            Ok(_) => {
                if let Err(err) = self.load_local_tunnels() {
                    self.status_message = format!("删除后刷新本地隧道失败: {}", err);
                    return;
                }
                self.rebuild_tunnel_items();
                self.status_message = format!("已删除本地隧道 {}", tunnel.name);
            }
            Err(err) => {
                self.status_message = format!("删除失败: {}", err);
            }
        }
    }

    pub fn cancel_api_delete_confirm(&mut self) {
        let tunnel_name = self
            .api_delete_target
            .as_ref()
            .map(|tunnel| tunnel.name.clone())
            .unwrap_or_default();
        self.tunnel_page_mode = TunnelPageMode::List;
        self.api_delete_target = None;
        self.status_message = if tunnel_name.is_empty() {
            "已取消删除官方隧道".to_string()
        } else {
            format!("已取消删除官方隧道 {}", tunnel_name)
        };
    }

    pub async fn confirm_api_delete(&mut self) {
        let Some(tunnel) = self.api_delete_target.clone() else {
            self.tunnel_page_mode = TunnelPageMode::List;
            self.status_message = "没有可删除的官方隧道".to_string();
            return;
        };

        let Some(token) = self.get_token().await else {
            self.status_message = "未登录，无法删除官方隧道".to_string();
            return;
        };

        if self.running_tunnels.contains(&tunnel.id) {
            match chmlfrp_core::process::stop_frpc(&self.data_dir, tunnel.id, &self.processes) {
                Ok(_) => {
                    self.running_tunnels.retain(|id| *id != tunnel.id);
                }
                Err(err) => {
                    self.status_message = format!("停止正在运行的官方隧道失败: {}", err);
                    return;
                }
            }
        }

        self.status_message = format!("正在删除官方隧道 {}...", tunnel.name);
        match chmlfrp_core::api::delete_tunnel(tunnel.id, &token).await {
            Ok(()) => {
                self.tunnel_page_mode = TunnelPageMode::List;
                self.api_delete_target = None;
                self.refresh_tunnels().await;
                self.status_message = format!("已删除官方隧道 {}", tunnel.name);
            }
            Err(err) => {
                self.status_message = format!("删除官方隧道失败: {}", err);
            }
        }
    }

    pub fn open_selected_tunnel_url(&mut self) {
        let Some(item) = self.selected_tunnel_item() else {
            return;
        };

        let url = match item {
            TunnelListItem::Api(tunnel) => {
                if tunnel.tunnel_type == "http" || tunnel.tunnel_type == "https" {
                    format!("{}://{}", tunnel.tunnel_type, tunnel.dorp)
                } else {
                    format!("http://{}:{}", tunnel.node_ip, tunnel.dorp)
                }
            }
            TunnelListItem::Local(tunnel) => {
                let tunnel_type = tunnel.tunnel_type.as_deref().unwrap_or("tcp");
                if tunnel_type == "http" || tunnel_type == "https" {
                    if let Some(domains) = tunnel.custom_domains.as_ref() {
                        let domain = domains.split(',').next().unwrap_or("").trim();
                        if !domain.is_empty() {
                            format!("{}://{}", tunnel_type, domain)
                        } else {
                            self.status_message = "该本地隧道没有可打开的域名".to_string();
                            return;
                        }
                    } else {
                        self.status_message = "该本地隧道没有可打开的域名".to_string();
                        return;
                    }
                } else if let (Some(server_addr), Some(remote_port)) =
                    (tunnel.server_addr.as_ref(), tunnel.remote_port)
                {
                    format!("http://{}:{}", server_addr, remote_port)
                } else {
                    self.status_message = "该本地隧道没有可打开的远程地址".to_string();
                    return;
                }
            }
        };

        let _ = open::that(&url);
        self.status_message = format!("在浏览器中打开: {}", url);
    }

    /// 获取有效 token (自动刷新)
    pub async fn get_token(&mut self) -> Option<String> {
        if let Some(mut user) = self.stored_user.take() {
            let res = chmlfrp_core::auth::ensure_valid_token(&mut user).await;
            self.stored_user = Some(user.clone());
            if let Ok(token) = res {
                let _ = crate::storage::save_user(&user);
                return Some(token);
            }
        }
        None
    }

    pub fn load_local_tunnels(&mut self) -> Result<(), String> {
        self.custom_tunnels = custom_tunnel::get_custom_tunnels(&self.data_dir)?;
        Ok(())
    }

    fn rebuild_tunnel_items(&mut self) {
        self.tunnel_items = self
            .tunnels
            .iter()
            .cloned()
            .map(TunnelListItem::Api)
            .chain(self.custom_tunnels.iter().cloned().map(TunnelListItem::Local))
            .collect();

        if self.tunnel_items.is_empty() {
            self.selected_tunnel = 0;
        } else if self.selected_tunnel >= self.tunnel_items.len() {
            self.selected_tunnel = self.tunnel_items.len() - 1;
        }
    }

    fn select_local_tunnel(&mut self, tunnel_id: &str) {
        if let Some(index) = self.tunnel_items.iter().position(|item| match item {
            TunnelListItem::Local(tunnel) => tunnel.id == tunnel_id,
            TunnelListItem::Api(_) => false,
        }) {
            self.selected_tunnel = index;
        }
    }

    fn select_api_tunnel(&mut self, tunnel_id: i32) {
        if let Some(index) = self.tunnel_items.iter().position(|item| match item {
            TunnelListItem::Api(tunnel) => tunnel.id == tunnel_id,
            TunnelListItem::Local(_) => false,
        }) {
            self.selected_tunnel = index;
        }
    }

    fn select_api_tunnel_by_name(&mut self, tunnel_name: &str) {
        if let Some(index) = self.tunnel_items.iter().position(|item| match item {
            TunnelListItem::Api(tunnel) => tunnel.name == tunnel_name,
            TunnelListItem::Local(_) => false,
        }) {
            self.selected_tunnel = index;
        }
    }

    /// 刷新隧道列表
    pub async fn refresh_tunnels(&mut self) {
        if let Err(err) = self.load_local_tunnels() {
            self.status_message = format!("加载本地隧道失败: {}", err);
        }

        let token = self.get_token().await;
        if let Some(token) = token {
            match chmlfrp_core::api::fetch_tunnels(&token).await {
                Ok(tunnels) => {
                    self.tunnels = tunnels;
                }
                Err(e) => {
                    if e.contains("无效的登录状态")
                        || e.contains("授权已过期")
                        || e.contains("Token无效")
                    {
                        self.status_message = "登录已过期，请按 L 重新登录".to_string();
                        self.stored_user = None;
                        let _ = crate::storage::clear_user();
                        self.tunnels.clear();
                    } else {
                        self.status_message = format!("获取官方隧道失败: {}", e);
                    }
                }
            }
        } else {
            self.tunnels.clear();
        }

        match chmlfrp_core::process::get_running_tunnels(&self.data_dir, &self.processes) {
            Ok(running) => self.running_tunnels = running,
            Err(_) => self.running_tunnels.clear(),
        }

        self.rebuild_tunnel_items();
        if !self.status_message.starts_with("获取官方隧道失败")
            && !self.status_message.starts_with("加载本地隧道失败")
            && !self.status_message.contains("登录已过期")
        {
            self.status_message = format!(
                "已加载 {} 个官方隧道，{} 个本地隧道",
                self.tunnels.len(),
                self.custom_tunnels.len()
            );
        }
    }

    /// 启动选中的隧道
    pub async fn start_selected_tunnel(&mut self) {
        let Some(item) = self.selected_tunnel_item().cloned() else {
            return;
        };

        let tunnel_id = item.id();
        if self.running_tunnels.contains(&tunnel_id) {
            self.status_message = "隧道已在运行中".to_string();
            return;
        }

        if !chmlfrp_core::download::check_frpc_exists(&self.data_dir) {
            self.status_message = "frpc 未下载，请按 d 下载".to_string();
            return;
        }

        match item {
            TunnelListItem::Api(tunnel) => {
                let token = match self.get_token().await {
                    Some(t) => t,
                    None => return,
                };

                let _node_info = match chmlfrp_core::api::fetch_node_info(&tunnel.node, &token).await {
                    Ok(info) => info,
                    Err(e) => {
                        self.status_message = format!("获取节点信息失败: {}", e);
                        return;
                    }
                };

                let config = TunnelConfig {
                    tunnel_id: tunnel.id,
                    tunnel_name: tunnel.name.clone(),
                    user_token: self
                        .stored_user
                        .as_ref()
                        .and_then(|u| u.usertoken.clone())
                        .unwrap_or_default(),
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
            TunnelListItem::Local(tunnel) => {
                match chmlfrp_core::process::start_frpc_with_existing_config(
                    &self.data_dir,
                    tunnel_id,
                    &tunnel.config_file,
                    &tunnel.id,
                    &self.processes,
                    self.log_tx.clone(),
                ) {
                    Ok(pid) => {
                        self.running_tunnels.push(tunnel_id);
                        self.status_message = format!("本地隧道 {} 已启动 (PID: {})", tunnel.name, pid);
                    }
                    Err(e) => {
                        self.status_message = format!("启动失败: {}", e);
                    }
                }
            }
        }
    }

    /// 停止选中的隧道
    pub fn stop_selected_tunnel(&mut self) {
        let Some(item) = self.selected_tunnel_item().cloned() else {
            return;
        };

        let tunnel_id = item.id();
        if !self.running_tunnels.contains(&tunnel_id) {
            self.status_message = "隧道未在运行".to_string();
            return;
        }

        match chmlfrp_core::process::stop_frpc(&self.data_dir, tunnel_id, &self.processes) {
            Ok(msg) => {
                self.running_tunnels.retain(|id| *id != tunnel_id);
                self.status_message = format!("{}: {}", item.name(), msg);
            }
            Err(e) => {
                self.status_message = format!("停止失败: {}", e);
            }
        }
    }

    /// 自动启动被标记的隧道
    pub async fn start_auto_tunnels(&mut self) {
        if !self.settings.auto_start_tunnels_enabled {
            return;
        }

        if !chmlfrp_core::download::check_frpc_exists(&self.data_dir) {
            self.status_message = "frpc 未下载，无法自动启动隧道".to_string();
            return;
        }

        let running = chmlfrp_core::process::get_running_tunnels(&self.data_dir, &self.processes)
            .unwrap_or_default();
        let token = self.get_token().await;

        for item in self.tunnel_items.clone() {
            let tunnel_id = item.id();
            if !self.settings.auto_start_tunnel_ids.contains(&tunnel_id) || running.contains(&tunnel_id) {
                continue;
            }

            match item {
                TunnelListItem::Api(tunnel) => {
                    if token.is_none() {
                        continue;
                    }

                    let config = TunnelConfig {
                        tunnel_id: tunnel.id,
                        tunnel_name: tunnel.name.clone(),
                        user_token: self
                            .stored_user
                            .as_ref()
                            .and_then(|u| u.usertoken.clone())
                            .unwrap_or_default(),
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
                            self.status_message = format!("自动启动隧道 {} (PID: {})", tunnel.name, pid);
                        }
                        Err(e) => {
                            self.status_message = format!("自动启动 {} 失败: {}", tunnel.name, e);
                        }
                    }
                }
                TunnelListItem::Local(tunnel) => {
                    match chmlfrp_core::process::start_frpc_with_existing_config(
                        &self.data_dir,
                        tunnel_id,
                        &tunnel.config_file,
                        &tunnel.id,
                        &self.processes,
                        self.log_tx.clone(),
                    ) {
                        Ok(pid) => {
                            self.running_tunnels.push(tunnel_id);
                            self.status_message = format!("自动启动本地隧道 {} (PID: {})", tunnel.name, pid);
                        }
                        Err(e) => {
                            self.status_message = format!("自动启动 {} 失败: {}", tunnel.name, e);
                        }
                    }
                }
            }
        }
    }

    /// 收集新日志和后台任务结果
    pub fn drain_events(&mut self) {
        while let Ok(log) = self.log_rx.try_recv() {
            self.logs.push(log);
            if self.logs.len() > 5000 {
                self.logs.drain(..self.logs.len() - 5000);
            }
        }

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

        if let Some(ref mut rx) = self.download_progress_rx {
            while let Ok(progress) = rx.try_recv() {
                self.download_progress = progress;
            }
        }

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

fn default_custom_tunnel_template() -> String {
    "[common]\nserver_addr = your.server.com\nserver_port = 7000\n\n[demo_tcp]\ntype = tcp\nlocal_ip = 127.0.0.1\nlocal_port = 8080\nremote_port = 10080\n".to_string()
}
