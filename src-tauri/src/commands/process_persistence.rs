use crate::models::FrpcProcesses;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{Manager, State};

const RUNNING_TUNNELS_FILE: &str = "running_tunnels.json";

/// 保存隧道进程信息
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PersistedTunnelInfo {
    pub tunnel_id: i32,
    pub pid: u32,
    pub tunnel_type: String,
    pub original_id: Option<String>,
    pub started_at: String,
}

fn get_persistence_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("获取应用目录失败: {}", e))?;
    Ok(app_dir.join(RUNNING_TUNNELS_FILE))
}

/// 保存运行中的隧道信息
pub fn save_running_tunnel(
    app_handle: &tauri::AppHandle,
    tunnel_id: i32,
    pid: u32,
    tunnel_type: &str,
    original_id: Option<String>,
) -> Result<(), String> {
    let path = get_persistence_path(app_handle)?;
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
pub fn remove_running_tunnel(
    app_handle: &tauri::AppHandle,
    tunnel_id: i32,
) -> Result<(), String> {
    let path = get_persistence_path(app_handle)?;
    let mut tunnels = load_persisted_tunnels_from_file(&path);

    tunnels.remove(&tunnel_id);

    write_persisted_tunnels(&path, &tunnels)
}

/// 从磁盘读取隧道信息
fn load_persisted_tunnels_from_file(
    path: &PathBuf,
) -> HashMap<i32, PersistedTunnelInfo> {
    if !path.exists() {
        return HashMap::new();
    }

    std::fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn write_persisted_tunnels(
    path: &PathBuf,
    tunnels: &HashMap<i32, PersistedTunnelInfo>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    let content = serde_json::to_string_pretty(tunnels)
        .map_err(|e| format!("序列化失败: {}", e))?;

    std::fs::write(path, content)
        .map_err(|e| format!("写入文件失败: {}", e))
}

/// 检查进程是否在运行
fn is_process_alive(pid: u32) -> bool {
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
    OpenProcess(0x1000, 0, pid)
}

#[cfg(target_os = "windows")]
unsafe fn windows_get_exit_code(
    handle: *mut std::ffi::c_void,
    exit_code: &mut u32,
) -> i32 {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetExitCodeProcess(
            hProcess: *mut std::ffi::c_void,
            lpExitCode: *mut u32,
        ) -> i32;
    }
    GetExitCodeProcess(handle, exit_code)
}

#[cfg(target_os = "windows")]
unsafe fn windows_close_handle(handle: *mut std::ffi::c_void) {
    #[link(name = "kernel32")]
    extern "system" {
        fn CloseHandle(hObject: *mut std::ffi::c_void) -> i32;
    }
    CloseHandle(handle);
}

/// 恢复进程状态
pub fn recover_running_tunnels(app_handle: &tauri::AppHandle) -> Vec<PersistedTunnelInfo> {
    let path = match get_persistence_path(app_handle) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    let tunnels = load_persisted_tunnels_from_file(&path);
    let mut still_running = Vec::new();
    let mut updated = HashMap::new();

    for (tunnel_id, info) in tunnels {
        if is_process_alive(info.pid) {
            //eprintln!(
            //    "[进程恢复] 隧道 {} (PID: {}) 仍在运行",
            //    tunnel_id, info.pid
            //);
            still_running.push(info.clone());
            updated.insert(tunnel_id, info);
        } else {
            //eprintln!(
            //    "[进程恢复] 隧道 {} (PID: {}) 已停止，清理记录",
            //    tunnel_id, info.pid
            //);
        }
    }

    // 移除已停止的进程
    let _ = write_persisted_tunnels(&path, &updated);

    still_running
}

/// 获取仍在运行的隧道列表
#[tauri::command]
pub async fn get_persisted_running_tunnels(
    app_handle: tauri::AppHandle,
) -> Result<Vec<PersistedTunnelInfo>, String> {
    let path = get_persistence_path(&app_handle)?;
    let tunnels = load_persisted_tunnels_from_file(&path);
    let mut result = Vec::new();

    for (_tunnel_id, info) in &tunnels {
        if is_process_alive(info.pid) {
            result.push(info.clone());
        }
    }

    Ok(result)
}

/// 停止一个进程
#[tauri::command]
pub async fn stop_orphan_process(
    app_handle: tauri::AppHandle,
    tunnel_id: i32,
    processes: State<'_, FrpcProcesses>,
) -> Result<String, String> {
    // 检查是否在进程管理器中
    {
        let mut procs = processes
            .processes
            .lock()
            .map_err(|e| format!("获取进程锁失败: {}", e))?;

        if let Some(mut child) = procs.remove(&tunnel_id) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = remove_running_tunnel(&app_handle, tunnel_id);
            return Ok("隧道已停止".to_string());
        }
    }

    // 不在进程管理器 尝试通过pid终止
    let path = get_persistence_path(&app_handle)?;
    let tunnels = load_persisted_tunnels_from_file(&path);

    if let Some(info) = tunnels.get(&tunnel_id) {
        let pid = info.pid;
        if is_process_alive(pid) {
            kill_process_by_pid(pid)?;
        }
        let _ = remove_running_tunnel(&app_handle, tunnel_id);
        Ok(format!("已终止进程 (PID: {})", pid))
    } else {
        Err("未找到该隧道的运行记录".to_string())
    }
}

fn kill_process_by_pid(pid: u32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
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

/// 检查进程是否在运行
#[tauri::command]
pub async fn is_tunnel_process_alive(
    app_handle: tauri::AppHandle,
    tunnel_id: i32,
    processes: State<'_, FrpcProcesses>,
) -> Result<bool, String> {
    {
        let mut procs = processes
            .processes
            .lock()
            .map_err(|e| format!("获取进程锁失败: {}", e))?;

        if let Some(child) = procs.get_mut(&tunnel_id) {
            return match child.try_wait() {
                Ok(None) => Ok(true),
                Ok(Some(_)) => {
                    procs.remove(&tunnel_id);
                    let _ = remove_running_tunnel(&app_handle, tunnel_id);
                    Ok(false)
                }
                Err(_) => {
                    procs.remove(&tunnel_id);
                    let _ = remove_running_tunnel(&app_handle, tunnel_id);
                    Ok(false)
                }
            };
        }
    }

    // 检查pid
    let path = get_persistence_path(&app_handle)?;
    let tunnels = load_persisted_tunnels_from_file(&path);

    if let Some(info) = tunnels.get(&tunnel_id) {
        let alive = is_process_alive(info.pid);
        if !alive {
            let _ = remove_running_tunnel(&app_handle, tunnel_id);
        }
        Ok(alive)
    } else {
        Ok(false)
    }
}
