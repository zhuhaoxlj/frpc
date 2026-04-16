use crate::config::generate_frpc_config;
use crate::models::{FrpcProcesses, LogMessage, TunnelConfig};
use crate::utils::{frpc_file_name, sanitize_log};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};
use std::thread;
use tokio::sync::mpsc;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// 获取 frpc 路径
pub fn resolve_frpc_path(data_dir: &Path) -> PathBuf {
    data_dir.join(frpc_file_name())
}

fn ensure_frpc_executable(frpc_path: &Path) -> Result<(), String> {
    if !frpc_path.exists() {
        return Err("frpc 未找到，请先下载".to_string());
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(frpc_path).map_err(|e| e.to_string())?;
        let mut perms = metadata.permissions();
        if perms.mode() & 0o111 == 0 {
            perms.set_mode(0o755);
            std::fs::set_permissions(frpc_path, perms).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn spawn_log_reader(
    log_tx: mpsc::UnboundedSender<LogMessage>,
    tunnel_id: i32,
    user_token: String,
    node_token: String,
    reader: impl std::io::Read + Send + 'static,
    is_stderr: bool,
    data_dir: PathBuf,
) {
    let thread_name = if is_stderr {
        format!("frpc-stderr-{}", tunnel_id)
    } else {
        format!("frpc-stdout-{}", tunnel_id)
    };

    if let Err(e) = thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let reader = std::io::BufReader::new(reader);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let clean_line = strip_ansi_escapes::strip_str(&line);
                let sanitized_line =
                    sanitize_log(&clean_line, &[user_token.as_str(), node_token.as_str()]);
                let timestamp = chrono::Local::now().format("%Y/%m/%d %H:%M:%S").to_string();

                let message = if is_stderr {
                    format!("[ERR] {}", sanitized_line)
                } else {
                    sanitized_line
                };

                let msg = LogMessage {
                    tunnel_id,
                    message,
                    timestamp,
                };

                let _ = crate::persistence::save_log(&data_dir, &msg);

                if log_tx.send(msg).is_err() {
                    break;
                }
            }
        })
    {
        eprintln!("[错误] 创建日志监听线程失败: {}", e);
    }
}

fn spawn_frpc_process(
    data_dir: &Path,
    tunnel_id: i32,
    config_arg: &Path,
    user_token: String,
    node_token: String,
    processes: &FrpcProcesses,
    log_tx: mpsc::UnboundedSender<LogMessage>,
    tunnel_type: &str,
    original_id: Option<String>,
    start_message: String,
) -> Result<u32, String> {
    {
        let procs = processes
            .processes
            .lock()
            .map_err(|e| format!("获取进程锁失败: {}", e))?;
        if procs.contains_key(&tunnel_id) {
            return Err("该隧道已在运行中".to_string());
        }
    }

    std::fs::create_dir_all(data_dir).map_err(|e| format!("创建应用目录失败: {}", e))?;

    let frpc_path = resolve_frpc_path(data_dir);
    ensure_frpc_executable(&frpc_path)?;

    let mut cmd = StdCommand::new(&frpc_path);
    cmd.current_dir(data_dir)
        .arg("-c")
        .arg(config_arg)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd.spawn().map_err(|e| format!("启动 frpc 失败: {}", e))?;
    let pid = child.id();

    let timestamp = chrono::Local::now().format("%Y/%m/%d %H:%M:%S").to_string();
    let msg = LogMessage {
        tunnel_id,
        message: start_message.replace("{pid}", &pid.to_string()),
        timestamp,
    };
    let _ = crate::persistence::save_log(data_dir, &msg);
    let _ = log_tx.send(msg);

    if let Some(stdout) = child.stdout.take() {
        spawn_log_reader(
            log_tx.clone(),
            tunnel_id,
            user_token.clone(),
            node_token.clone(),
            stdout,
            false,
            data_dir.to_path_buf(),
        );
    }

    if let Some(stderr) = child.stderr.take() {
        spawn_log_reader(
            log_tx,
            tunnel_id,
            user_token,
            node_token,
            stderr,
            true,
            data_dir.to_path_buf(),
        );
    }

    {
        let mut procs = processes
            .processes
            .lock()
            .map_err(|e| format!("获取进程锁失败: {}", e))?;
        procs.insert(tunnel_id, child);
    }

    let _ = crate::persistence::save_running_tunnel(data_dir, tunnel_id, pid, tunnel_type, original_id);

    Ok(pid)
}

/// 启动 frpc 进程
pub fn start_frpc(
    data_dir: &Path,
    config: &TunnelConfig,
    processes: &FrpcProcesses,
    log_tx: mpsc::UnboundedSender<LogMessage>,
) -> Result<u32, String> {
    let tunnel_id = config.tunnel_id;
    let user_token = config.user_token.clone();
    let node_token = config.node_token.clone();

    let config_path = data_dir.join(format!("g_{}.ini", tunnel_id));
    let config_content = generate_frpc_config(config)?;

    std::fs::write(&config_path, &config_content)
        .map_err(|e| format!("写入配置文件失败: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&config_path)
            .map_err(|e| format!("获取配置文件权限失败: {}", e))?
            .permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&config_path, perms)
            .map_err(|e| format!("设置配置文件权限失败: {}", e))?;
    }

    spawn_frpc_process(
        data_dir,
        tunnel_id,
        &config_path,
        user_token,
        node_token,
        processes,
        log_tx,
        "api",
        None,
        "[I] [ChmlFrpLauncher] frpc 进程已启动 (PID: {pid}), 开始连接服务器...".to_string(),
    )
}

pub fn start_frpc_with_existing_config(
    data_dir: &Path,
    tunnel_id: i32,
    config_file_name: &str,
    original_id: &str,
    processes: &FrpcProcesses,
    log_tx: mpsc::UnboundedSender<LogMessage>,
) -> Result<u32, String> {
    let config_path = data_dir.join(config_file_name);
    if !config_path.exists() {
        return Err("配置文件不存在".to_string());
    }

    spawn_frpc_process(
        data_dir,
        tunnel_id,
        Path::new(config_file_name),
        String::new(),
        String::new(),
        processes,
        log_tx,
        "custom",
        Some(original_id.to_string()),
        format!(
            "[I] [ChmlFrpLauncher] 自定义隧道 {} 进程已启动 (PID: {{pid}})",
            original_id
        ),
    )
}

/// 停止 frpc 进程
pub fn stop_frpc(data_dir: &Path, tunnel_id: i32, processes: &FrpcProcesses) -> Result<String, String> {
    {
        let mut procs = processes
            .processes
            .lock()
            .map_err(|e| format!("获取进程锁失败: {}", e))?;

        if let Some(mut child) = procs.remove(&tunnel_id) {
            let result = match child.kill() {
                Ok(_) => {
                    let _ = child.wait();
                    Ok("frpc 已停止".to_string())
                }
                Err(e) => {
                    let _ = child.wait();
                    Err(format!("停止进程失败: {}", e))
                }
            };

            let _ = crate::persistence::remove_running_tunnel(data_dir, tunnel_id);
            cleanup_generated_config(data_dir, tunnel_id);

            return result;
        }
    }

    let persistence_path = crate::persistence::get_persistence_path(data_dir);
    let tunnels: std::collections::HashMap<i32, crate::models::PersistedTunnelInfo> =
        if persistence_path.exists() {
            std::fs::read_to_string(&persistence_path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        };

    if let Some(info) = tunnels.get(&tunnel_id) {
        if crate::persistence::is_process_alive(info.pid) {
            let _ = crate::persistence::kill_process_by_pid(info.pid);
        }
    }

    let _ = crate::persistence::remove_running_tunnel(data_dir, tunnel_id);
    cleanup_generated_config(data_dir, tunnel_id);

    Ok("frpc 已停止".to_string())
}

fn cleanup_generated_config(data_dir: &Path, tunnel_id: i32) {
    let config_path = data_dir.join(format!("g_{}.ini", tunnel_id));
    if config_path.exists() {
        let _ = std::fs::remove_file(&config_path);
    }
}

/// 检查隧道是否运行中
pub fn is_frpc_running(data_dir: &Path, tunnel_id: i32, processes: &FrpcProcesses) -> Result<bool, String> {
    {
        let mut procs = processes
            .processes
            .lock()
            .map_err(|e| format!("获取进程锁失败: {}", e))?;

        if let Some(child) = procs.get_mut(&tunnel_id) {
            return match child.try_wait() {
                Ok(Some(_)) => {
                    procs.remove(&tunnel_id);
                    let _ = crate::persistence::remove_running_tunnel(data_dir, tunnel_id);
                    Ok(false)
                }
                Ok(None) => Ok(true),
                Err(_) => {
                    procs.remove(&tunnel_id);
                    let _ = crate::persistence::remove_running_tunnel(data_dir, tunnel_id);
                    Ok(false)
                }
            };
        }
    }

    let persistence_path = crate::persistence::get_persistence_path(data_dir);
    let tunnels: std::collections::HashMap<i32, crate::models::PersistedTunnelInfo> =
        if persistence_path.exists() {
            std::fs::read_to_string(&persistence_path)
                .ok()
                .and_then(|c| serde_json::from_str(&c).ok())
                .unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        };

    if let Some(info) = tunnels.get(&tunnel_id) {
        let alive = crate::persistence::is_process_alive(info.pid);
        if !alive {
            let _ = crate::persistence::remove_running_tunnel(data_dir, tunnel_id);
        }
        Ok(alive)
    } else {
        Ok(false)
    }
}

/// 获取所有运行中的隧道 ID
pub fn get_running_tunnels(data_dir: &Path, processes: &FrpcProcesses) -> Result<Vec<i32>, String> {
    let mut procs = processes
        .processes
        .lock()
        .map_err(|e| format!("获取进程锁失败: {}", e))?;

    let mut running_tunnels = Vec::new();
    let mut stopped_tunnels = Vec::new();

    for (tunnel_id, child) in procs.iter_mut() {
        match child.try_wait() {
            Ok(None) => running_tunnels.push(*tunnel_id),
            _ => stopped_tunnels.push(*tunnel_id),
        }
    }

    for tunnel_id in &stopped_tunnels {
        procs.remove(tunnel_id);
        let _ = crate::persistence::remove_running_tunnel(data_dir, *tunnel_id);
    }

    let persisted = crate::persistence::recover_running_tunnels(data_dir);
    for info in persisted {
        if !running_tunnels.contains(&info.tunnel_id) {
            running_tunnels.push(info.tunnel_id);
        }
    }

    Ok(running_tunnels)
}
