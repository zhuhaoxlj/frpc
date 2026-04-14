use std::fs;
use std::path::PathBuf;
use tauri::Manager;

fn background_dir(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    let background_dir = app_dir.join("backgrounds");
    fs::create_dir_all(&background_dir).map_err(|e| e.to_string())?;
    Ok(background_dir)
}

#[tauri::command]
pub async fn copy_background_video(
    app_handle: tauri::AppHandle,
    source_path: String,
) -> Result<String, String> {
    let background_dir = background_dir(&app_handle)?;

    let source = PathBuf::from(&source_path);
    let file_name = source
        .file_name()
        .ok_or_else(|| "无法获取文件名".to_string())?
        .to_string_lossy()
        .to_string();

    let dest_path = background_dir.join(&file_name);

    fs::copy(&source_path, &dest_path).map_err(|e| {
        format!("复制文件失败: {}", e)
    })?;

    Ok(dest_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn copy_background_image(
    app_handle: tauri::AppHandle,
    source_path: String,
) -> Result<String, String> {
    let background_dir = background_dir(&app_handle)?;

    let source = PathBuf::from(&source_path);
    let file_name = source
        .file_name()
        .ok_or_else(|| "无法获取文件名".to_string())?
        .to_string_lossy()
        .to_string();

    let dest_path = background_dir.join(&file_name);

    fs::copy(&source_path, &dest_path).map_err(|e| {
        format!("复制文件失败: {}", e)
    })?;

    Ok(dest_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_background_image_folder(
    app_handle: tauri::AppHandle,
    dir_path: String,
) -> Result<Vec<String>, String> {
    let source_dir = PathBuf::from(&dir_path);
    if !source_dir.is_dir() {
        return Err("选择的路径不是文件夹".to_string());
    }

    let slideshow_dir = background_dir(&app_handle)?.join("slideshow");
    if slideshow_dir.exists() {
        fs::remove_dir_all(&slideshow_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&slideshow_dir).map_err(|e| e.to_string())?;

    let mut imported = Vec::new();
    let mut counter = 0usize;
    let extensions = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];

    let entries = fs::read_dir(&source_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        let ext = ext.to_lowercase();
        if !extensions.contains(&ext.as_str()) {
            continue;
        }

        let file_stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");
        let safe_stem: String = file_stem
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let file_name = format!("{:03}_{}.{}", counter, safe_stem, ext);
        let dest_path = slideshow_dir.join(file_name);

        fs::copy(&path, &dest_path)
            .map_err(|e| format!("复制文件失败 {}: {}", path.to_string_lossy(), e))?;
        imported.push(dest_path.to_string_lossy().to_string());
        counter += 1;
    }

    Ok(imported)
}

#[tauri::command]
pub async fn get_background_video_path(
    app_handle: tauri::AppHandle,
) -> Result<Option<String>, String> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    let background_dir = app_dir.join("backgrounds");
    
    if !background_dir.exists() {
        return Ok(None);
    }

    let entries = fs::read_dir(&background_dir).map_err(|e| e.to_string())?;
    
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if matches!(ext_lower.as_str(), "mp4" | "webm" | "ogv" | "mov") {
                    return Ok(Some(path.to_string_lossy().to_string()));
                }
            }
        }
    }

    Ok(None)
}

