use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

/// 下载进度
#[derive(Serialize, Clone, Debug)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percentage: f64,
}

/// API 响应
#[derive(Deserialize, Debug)]
pub struct FrpcInfoResponse {
    pub msg: String,
    pub state: String,
    pub code: u32,
    pub data: FrpcInfoData,
}

#[derive(Deserialize, Debug)]
pub struct FrpcInfoData {
    pub downloads: Vec<FrpcDownload>,
    #[allow(dead_code)]
    pub version: String,
    #[allow(dead_code)]
    pub release_notes: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct FrpcDownload {
    pub hash: String,
    pub os: String,
    #[allow(dead_code)]
    pub hash_type: String,
    pub platform: String,
    pub link: String,
    pub arch: String,
    pub size: u64,
}

/// 下载信息
pub struct DownloadInfo {
    pub url: String,
    pub hash: String,
    pub size: u64,
}

/// 运行中的 frpc 进程管理
#[derive(Clone)]
pub struct FrpcProcesses {
    pub processes: Arc<Mutex<HashMap<i32, Child>>>,
}

impl FrpcProcesses {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for FrpcProcesses {
    fn default() -> Self {
        Self::new()
    }
}

/// 隧道类型
#[derive(Clone, Debug)]
pub enum TunnelType {
    Api { config: TunnelConfig },
    Custom { original_id: String },
}

/// 进程守护信息
#[derive(Clone)]
pub struct ProcessGuardInfo {
    pub tunnel_id: i32,
    pub tunnel_type: TunnelType,
}

/// 守护进程状态
pub struct ProcessGuardState {
    pub enabled: Arc<AtomicBool>,
    pub guarded_processes: Arc<Mutex<HashMap<i32, ProcessGuardInfo>>>,
    pub manually_stopped: Arc<Mutex<std::collections::HashSet<i32>>>,
}

impl ProcessGuardState {
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(AtomicBool::new(false)),
            guarded_processes: Arc::new(Mutex::new(HashMap::new())),
            manually_stopped: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }
}

impl Default for ProcessGuardState {
    fn default() -> Self {
        Self::new()
    }
}

/// 日志消息
#[derive(Serialize, Clone, Debug)]
pub struct LogMessage {
    pub tunnel_id: i32,
    pub message: String,
    pub timestamp: String,
}

/// 隧道配置
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TunnelConfig {
    pub tunnel_id: i32,
    pub tunnel_name: String,
    pub user_token: String,
    pub server_addr: String,
    pub server_port: u16,
    pub node_token: String,
    pub tunnel_type: String,
    pub local_ip: String,
    pub local_port: u16,
    pub remote_port: Option<u16>,
    pub custom_domains: Option<String>,
    pub http_proxy: Option<String>,
    pub log_level: String,
    pub force_tls: bool,
    pub kcp_optimization: bool,
}

/// API 隧道信息 (从服务器获取)
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Tunnel {
    pub id: i32,
    pub name: String,
    pub localip: String,
    #[serde(rename = "type")]
    pub tunnel_type: String,
    pub nport: i32,
    pub dorp: String,
    pub node: String,
    pub ap: String,
    pub uptime: Option<String>,
    pub client_version: Option<String>,
    pub today_traffic_in: Option<f64>,
    pub today_traffic_out: Option<f64>,
    pub cur_conns: Option<i32>,
    pub nodestate: String,
    pub ip: String,
    pub node_ip: String,
    pub node_ipv6: Option<String>,
    pub server_port: u16,
    pub node_token: String,
}

/// 用户信息
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UserInfo {
    pub id: i32,
    pub username: String,
    pub userimg: String,
    pub usertoken: String,
    pub usergroup: String,
    pub bandwidth: i32,
    pub tunnel: i32,
    pub realname: String,
    pub integral: i32,
    pub term: String,
    #[serde(rename = "tunnelCount")]
    pub tunnel_count: i32,
    #[serde(rename = "totalCurConns")]
    pub total_cur_conns: Option<i32>,
}

/// 节点信息
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Node {
    pub id: i32,
    pub name: String,
    pub area: String,
    pub nodegroup: String,
    pub china: String,
    pub web: String,
    pub udp: String,
    pub fangyu: String,
    pub notes: String,
}

/// 节点详细信息
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NodeInfo {
    pub id: i32,
    pub name: String,
    pub area: String,
    pub nodegroup: String,
    pub china: String,
    pub web: String,
    pub udp: String,
    pub fangyu: String,
    pub notes: String,
    pub ip: String,
    pub port: u16,
    #[serde(rename = "adminPort")]
    pub admin_port: Option<u16>,
    pub rport: String,
    pub state: String,
    pub nodetoken: String,
    #[serde(rename = "real_IP")]
    pub real_ip: Option<String>,
    pub ipv6: Option<String>,
    pub version: Option<String>,
}

/// 存储的用户凭证
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StoredUser {
    pub username: String,
    pub usergroup: String,
    pub userimg: Option<String>,
    pub usertoken: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub access_token_expires_at: Option<i64>,
    pub token_type: Option<String>,
    pub tunnel_count: Option<i32>,
    pub tunnel: Option<i32>,
}

/// 持久化的隧道进程信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PersistedTunnelInfo {
    pub tunnel_id: i32,
    pub pid: u32,
    pub tunnel_type: String,
    pub original_id: Option<String>,
    pub started_at: String,
}
