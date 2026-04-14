use chmlfrp_core::models::StoredUser;
use std::path::PathBuf;

const APP_DIR_NAME: &str = "chmlfrp-launcher";
const USER_FILE: &str = "user.json";

/// 获取数据目录
pub fn get_data_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR_NAME);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// 加载用户凭证
pub fn load_user() -> Result<Option<StoredUser>, Box<dyn std::error::Error>> {
    let path = get_data_dir().join(USER_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    let user: StoredUser = serde_json::from_str(&content)?;
    Ok(Some(user))
}

/// 保存用户凭证
pub fn save_user(user: &StoredUser) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_data_dir().join(USER_FILE);
    let content = serde_json::to_string_pretty(user)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// 清除用户凭证
pub fn clear_user() -> Result<(), Box<dyn std::error::Error>> {
    let path = get_data_dir().join(USER_FILE);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}
