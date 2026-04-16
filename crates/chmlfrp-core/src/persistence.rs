use crate::models::{PersistedTunnelInfo, LogMessage};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;
use std::io::Write;

const RUNNING_TUNNELS_FILE: &str = "running_tunnels.json";
const TUNNEL_LOGS_FILE: &str = "tunnel_logs.jsonl";

/// 获取持久化文件路径
pub fn get_persistence_path(data_dir: &Path) -> PathBuf {
    data_dir.join(RUNNING_TUNNELS_FILE)
}

/// 获取日志持久化文件路径
pub fn get_logs_persistence_path(data_dir: &Path) -> PathBuf {
    data_dir.join(TUNNEL_LOGS_FILE)
}

/// 保存一条日志
pub fn save_log(data_dir: &Path, log: &LogMessage) -> Result<(), String> {
    let path = get_logs_persistence_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("打开日志文件失败: {}", e))?;
        
    let json = serde_json::to_string(log).map_err(|e| format!("序列化日志失败: {}", e))?;
    writeln!(file, "{}", json).map_err(|e| format!("写入日志失败: {}", e))?;
    
    // 简单截断处理，避免日志无限增长
    if let Ok(metadata) = std::fs::metadata(&path) {
        // 限制在 100MB 左右
        if metadata.len() > 100 * 1024 * 1024 {
            let _ = rotate_logs(&path);
        }
    }
    
    Ok(())
}

fn rotate_logs(path: &Path) -> Result<(), std::io::Error> {
    use std::io::{BufRead, BufReader};
    use std::fs::File;
    
    // 只保留最后 50000 行
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();
    
    if lines.len() > 50000 {
        lines.drain(0..lines.len() - 50000);
        let mut new_file = File::create(path)?;
        for line in lines {
            writeln!(new_file, "{}", line)?;
        }
    }
    Ok(())
}

/// 加载持久化的日志
pub fn load_persisted_logs(data_dir: &Path, max_lines: usize) -> Vec<LogMessage> {
    let path = get_logs_persistence_path(data_dir);
    if !path.exists() {
        return Vec::new();
    }
    
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    
    use std::io::{BufRead, BufReader};
    let reader = BufReader::new(file);
    
    let mut logs: Vec<LogMessage> = reader.lines()
        .filter_map(|l| l.ok())
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect();
        
    if logs.len() > max_lines {
        logs.drain(0..logs.len() - max_lines);
    }
    
    logs
}

/// 保存运行中的隧道信息
pub fn save_running_tunnel(
    data_dir: &Path,
    tunnel_id: i32,
    pid: u32,
    tunnel_type: &str,
    original_id: Option<String>,
) -> Result<(), String> {
    let path = get_persistence_path(data_dir);
    let mut tunnels = load_persisted_tunnels_from_file(&path);

    tunnels.insert(
        tunnel_id,
        PersistedTunnelInfo {
            tunnel_id,
            pid,
            tunnel_type: tunnel_type.to_string(),
            original_id,
            started_at: chrono::Local::now().to_rfc3339(),
        },
    );

    write_persisted_tunnels(&path, &tunnels)
}

/// 移除隧道信息
pub fn remove_running_tunnel(data_dir: &Path, tunnel_id: i32) -> Result<(), String> {
    let path = get_persistence_path(data_dir);
    let mut tunnels = load_persisted_tunnels_from_file(&path);
    tunnels.remove(&tunnel_id);
    write_persisted_tunnels(&path, &tunnels)
}

/// 从磁盘读取隧道信息
fn load_persisted_tunnels_from_file(path: &Path) -> HashMap<i32, PersistedTunnelInfo> {
    if !path.exists() {
        return HashMap::new();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn write_persisted_tunnels(
    path: &Path,
    tunnels: &HashMap<i32, PersistedTunnelInfo>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
    }
    let content =
        serde_json::to_string_pretty(tunnels).map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(path, content).map_err(|e| format!("写入文件失败: {}", e))
}

/// 检查进程是否存活
pub fn is_process_alive(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        unsafe {
            let handle = windows_open_process(pid);
            if handle.is_null() {
                return false;
            }
            let mut exit_code: u32 = 0;
            let result = windows_get_exit_code(handle, &mut exit_code);
            windows_close_handle(handle);
            result != 0 && exit_code == 259
        }
    }

    #[cfg(unix)]
    {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
}

#[cfg(target_os = "windows")]
unsafe fn windows_open_process(pid: u32) -> *mut std::ffi::c_void {
    #[link(name = "kernel32")]
    extern "system" {
        fn OpenProcess(
            dwDesiredAccess: u32,
            bInheritHandle: i32,
            dwProcessId: u32,
        ) -> *mut std::ffi::c_void;
    }
    unsafe { OpenProcess(0x1000, 0, pid) }
}

#[cfg(target_os = "windows")]
unsafe fn windows_get_exit_code(handle: *mut std::ffi::c_void, exit_code: &mut u32) -> i32 {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetExitCodeProcess(hProcess: *mut std::ffi::c_void, lpExitCode: *mut u32) -> i32;
    }
    unsafe { GetExitCodeProcess(handle, exit_code) }
}

#[cfg(target_os = "windows")]
unsafe fn windows_close_handle(handle: *mut std::ffi::c_void) {
    #[link(name = "kernel32")]
    extern "system" {
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    }
    unsafe { CloseHandle(handle) };
}

/// 恢复运行中的隧道进程
pub fn recover_running_tunnels(data_dir: &Path) -> Vec<PersistedTunnelInfo> {
    let path = get_persistence_path(data_dir);
    let tunnels = load_persisted_tunnels_from_file(&path);
    let mut still_running = Vec::new();
    let mut updated = HashMap::new();

    for (tunnel_id, info) in tunnels {
        if is_process_alive(info.pid) {
            still_running.push(info.clone());
            updated.insert(tunnel_id, info);
        }
    }

    let _ = write_persisted_tunnels(&path, &updated);
    still_running
}

/// 通过 PID 终止进程
pub fn kill_process_by_pid(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| format!("终止进程失败: {}", e))?;
        Ok(())
    }

    #[cfg(unix)]
    {
        unsafe {
            if libc::kill(pid as i32, libc::SIGTERM) != 0 {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }
        Ok(())
    }
}
