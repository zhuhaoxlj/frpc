use crate::models::{PersistedTunnelInfo};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const RUNNING_TUNNELS_FILE: &str = "running_tunnels.json";

/// 获取持久化文件路径
pub fn get_persistence_path(data_dir: &Path) -> PathBuf {
    data_dir.join(RUNNING_TUNNELS_FILE)
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
