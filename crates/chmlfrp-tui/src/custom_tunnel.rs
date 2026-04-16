use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

const CUSTOM_TUNNEL_PREFIX: &str = "custom_";
const CONFIG_FILE_PREFIX: &str = "z_";
const CONFIG_FILE_EXT: &str = ".ini";
const TUNNELS_LIST_FILE: &str = "custom_tunnels.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CustomTunnel {
    pub id: String,
    pub name: String,
    pub config_file: String,
    pub server_addr: Option<String>,
    pub server_port: Option<u16>,
    pub tunnels: Vec<String>,
    pub tunnel_type: Option<String>,
    pub custom_domains: Option<String>,
    pub subdomain: Option<String>,
    pub local_ip: Option<String>,
    pub local_port: Option<u16>,
    pub remote_port: Option<u16>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashed_id: Option<i32>,
}

struct IniSplitResult {
    common: String,
    tunnels: Vec<(String, String)>,
}

struct IniParsedInfo {
    server_addr: Option<String>,
    server_port: Option<u16>,
    tunnel_names: Vec<String>,
    tunnel_type: Option<String>,
    custom_domains: Option<String>,
    subdomain: Option<String>,
    local_ip: Option<String>,
    local_port: Option<u16>,
    remote_port: Option<u16>,
}

pub fn get_custom_tunnel_hash(tunnel_id: &str) -> i32 {
    string_to_i32(&format!("{}{}", CUSTOM_TUNNEL_PREFIX, tunnel_id))
}

pub fn get_config_file_name(tunnel_id: &str) -> String {
    format!("{}{}{}", CONFIG_FILE_PREFIX, tunnel_id, CONFIG_FILE_EXT)
}

pub fn save_custom_tunnel(data_dir: &Path, config_content: &str) -> Result<Vec<CustomTunnel>, String> {
    let split = split_ini_config_internal(config_content)?;

    if split.tunnels.is_empty() {
        return Err("配置文件中未找到隧道名称".to_string());
    }

    fs::create_dir_all(data_dir).map_err(|e| format!("创建目录失败: {}", e))?;

    let mut created = Vec::with_capacity(split.tunnels.len());
    for (tunnel_name, tunnel_block) in split.tunnels {
        validate_tunnel_name(&tunnel_name)?;

        let single_ini = if split.common.trim().is_empty() {
            tunnel_block
        } else {
            format!("{}\n\n{}", split.common, tunnel_block)
        };

        let parsed_info = parse_ini_config(&single_ini)?;
        let config_file_name = get_config_file_name(&tunnel_name);
        let config_file_path = data_dir.join(&config_file_name);

        fs::write(&config_file_path, &single_ini)
            .map_err(|e| format!("写入配置文件失败: {}", e))?;

        let custom_tunnel = CustomTunnel {
            id: tunnel_name.clone(),
            name: tunnel_name.clone(),
            config_file: config_file_name,
            server_addr: parsed_info.server_addr,
            server_port: parsed_info.server_port,
            tunnels: parsed_info.tunnel_names,
            tunnel_type: parsed_info.tunnel_type,
            custom_domains: parsed_info.custom_domains,
            subdomain: parsed_info.subdomain,
            local_ip: parsed_info.local_ip,
            local_port: parsed_info.local_port,
            remote_port: parsed_info.remote_port,
            created_at: chrono::Local::now().to_rfc3339(),
            hashed_id: Some(get_custom_tunnel_hash(&tunnel_name)),
        };

        save_custom_tunnel_list(data_dir, &custom_tunnel)?;
        created.push(custom_tunnel);
    }

    Ok(created)
}

pub fn get_custom_tunnels(data_dir: &Path) -> Result<Vec<CustomTunnel>, String> {
    let list_file = data_dir.join(TUNNELS_LIST_FILE);
    if !list_file.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&list_file)
        .map_err(|e| format!("读取自定义隧道列表失败: {}", e))?;
    let tunnels: Vec<CustomTunnel> = serde_json::from_str(&content)
        .map_err(|e| format!("解析自定义隧道列表失败: {}", e))?;

    Ok(tunnels
        .into_iter()
        .map(|mut tunnel| {
            let config_path = data_dir.join(&tunnel.config_file);
            if let Ok(config) = fs::read_to_string(&config_path) {
                if let Ok(parsed) = parse_ini_config(&config) {
                    tunnel.server_addr = parsed.server_addr.or(tunnel.server_addr);
                    tunnel.server_port = parsed.server_port.or(tunnel.server_port);
                    if !parsed.tunnel_names.is_empty() {
                        tunnel.tunnels = parsed.tunnel_names;
                    }
                    tunnel.tunnel_type = parsed.tunnel_type.or(tunnel.tunnel_type);
                    tunnel.custom_domains = parsed.custom_domains.or(tunnel.custom_domains);
                    tunnel.subdomain = parsed.subdomain.or(tunnel.subdomain);
                    tunnel.local_ip = parsed.local_ip.or(tunnel.local_ip);
                    tunnel.local_port = parsed.local_port.or(tunnel.local_port);
                    tunnel.remote_port = parsed.remote_port.or(tunnel.remote_port);
                }
            }
            tunnel.hashed_id = Some(get_custom_tunnel_hash(&tunnel.id));
            tunnel
        })
        .collect())
}

pub fn get_custom_tunnel_config(data_dir: &Path, tunnel_id: &str) -> Result<String, String> {
    let config_path = data_dir.join(get_config_file_name(tunnel_id));
    if !config_path.exists() {
        return Err("配置文件不存在".to_string());
    }

    fs::read_to_string(&config_path).map_err(|e| format!("读取配置文件失败: {}", e))
}

pub fn update_custom_tunnel(data_dir: &Path, tunnel_id: &str, config_content: &str) -> Result<CustomTunnel, String> {
    validate_tunnel_name(tunnel_id)?;
    let parsed_info = parse_ini_config(config_content)?;
    let config_file_name = get_config_file_name(tunnel_id);
    let config_file_path = data_dir.join(&config_file_name);

    fs::create_dir_all(data_dir).map_err(|e| format!("创建目录失败: {}", e))?;
    fs::write(&config_file_path, config_content).map_err(|e| format!("写入配置文件失败: {}", e))?;

    let list_file = data_dir.join(TUNNELS_LIST_FILE);
    let existing_tunnels: Vec<CustomTunnel> = if list_file.exists() {
        let content = fs::read_to_string(&list_file)
            .map_err(|e| format!("读取自定义隧道列表失败: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("解析自定义隧道列表失败: {}", e))?
    } else {
        Vec::new()
    };

    let created_at = existing_tunnels
        .iter()
        .find(|t| t.id == tunnel_id)
        .map(|t| t.created_at.clone())
        .unwrap_or_else(|| chrono::Local::now().to_rfc3339());

    let tunnel = CustomTunnel {
        id: tunnel_id.to_string(),
        name: tunnel_id.to_string(),
        config_file: config_file_name,
        server_addr: parsed_info.server_addr,
        server_port: parsed_info.server_port,
        tunnels: parsed_info.tunnel_names,
        tunnel_type: parsed_info.tunnel_type,
        custom_domains: parsed_info.custom_domains,
        subdomain: parsed_info.subdomain,
        local_ip: parsed_info.local_ip,
        local_port: parsed_info.local_port,
        remote_port: parsed_info.remote_port,
        created_at,
        hashed_id: Some(get_custom_tunnel_hash(tunnel_id)),
    };

    save_custom_tunnel_list(data_dir, &tunnel)?;
    Ok(tunnel)
}

pub fn delete_custom_tunnel(data_dir: &Path, tunnel_id: &str) -> Result<(), String> {
    let config_file = data_dir.join(get_config_file_name(tunnel_id));
    if config_file.exists() {
        fs::remove_file(&config_file).map_err(|e| format!("删除配置文件失败: {}", e))?;
    }

    let list_file = data_dir.join(TUNNELS_LIST_FILE);
    if !list_file.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&list_file)
        .map_err(|e| format!("读取自定义隧道列表失败: {}", e))?;
    let mut tunnels: Vec<CustomTunnel> = serde_json::from_str(&content)
        .map_err(|e| format!("解析自定义隧道列表失败: {}", e))?;

    tunnels.retain(|t| t.id != tunnel_id);

    let content = serde_json::to_string_pretty(&tunnels)
        .map_err(|e| format!("序列化自定义隧道列表失败: {}", e))?;
    fs::write(&list_file, content).map_err(|e| format!("保存自定义隧道列表失败: {}", e))?;
    Ok(())
}

pub fn split_ini_config(content: &str) -> Result<Vec<(String, String)>, String> {
    let split = split_ini_config_internal(content)?;
    Ok(split
        .tunnels
        .into_iter()
        .map(|(name, block)| {
            let content = if split.common.is_empty() {
                block
            } else {
                format!("{}\n\n{}", split.common, block)
            };
            (name, content)
        })
        .collect())
}

pub fn parse_ini_summary(content: &str) -> Result<(Option<String>, Option<u16>, Vec<String>, Option<String>, Option<String>, Option<String>, Option<String>, Option<u16>, Option<u16>), String> {
    let parsed = parse_ini_config(content)?;
    Ok((
        parsed.server_addr,
        parsed.server_port,
        parsed.tunnel_names,
        parsed.tunnel_type,
        parsed.custom_domains,
        parsed.subdomain,
        parsed.local_ip,
        parsed.local_port,
        parsed.remote_port,
    ))
}

fn split_ini_config_internal(content: &str) -> Result<IniSplitResult, String> {
    let mut common_lines: Vec<String> = Vec::new();
    let mut tunnels: Vec<(String, Vec<String>)> = Vec::new();
    let mut current_section: Option<String> = None;

    for raw in content.lines() {
        let trimmed = raw.trim();

        if let Some(name) = parse_section_header(trimmed) {
            current_section = Some(name.clone());
            if name == "common" {
                common_lines.push(format!("[{}]", name));
            } else if !name.is_empty() {
                validate_tunnel_name(&name)?;
                tunnels.push((name.clone(), vec![format!("[{}]", name)]));
            }
            continue;
        }

        match current_section.as_deref() {
            Some("common") => common_lines.push(raw.to_string()),
            Some(sec) if !sec.is_empty() && sec != "common" => {
                if let Some((_, lines)) = tunnels.last_mut() {
                    lines.push(raw.to_string());
                }
            }
            _ => {}
        }
    }

    let common = common_lines.join("\n").trim().to_string();
    let tunnels = tunnels
        .into_iter()
        .map(|(name, lines)| (name, lines.join("\n").trim().to_string()))
        .collect();

    Ok(IniSplitResult { common, tunnels })
}

fn parse_ini_config(content: &str) -> Result<IniParsedInfo, String> {
    let mut info = IniParsedInfo {
        server_addr: None,
        server_port: None,
        tunnel_names: Vec::new(),
        tunnel_type: None,
        custom_domains: None,
        subdomain: None,
        local_ip: None,
        local_port: None,
        remote_port: None,
    };

    let mut current_section = String::new();
    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if let Some(section) = parse_section_header(line) {
            current_section = section.clone();
            if current_section != "common" && !current_section.is_empty() {
                validate_tunnel_name(&current_section)?;
                info.tunnel_names.push(current_section.clone());
            }
            continue;
        }

        if let Some((key, value)) = parse_key_value(line) {
            match current_section.as_str() {
                "common" => match key {
                    "server_addr" => info.server_addr = Some(value.to_string()),
                    "server_port" => info.server_port = value.parse().ok(),
                    _ => {}
                },
                _ if !current_section.is_empty() => match key {
                    "type" => info.tunnel_type = Some(value.to_string()),
                    "custom_domains" => info.custom_domains = Some(value.to_string()),
                    "subdomain" => info.subdomain = Some(value.to_string()),
                    "local_ip" => info.local_ip = Some(value.to_string()),
                    "local_port" => info.local_port = value.parse().ok(),
                    "remote_port" => info.remote_port = value.parse().ok(),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    Ok(info)
}

fn parse_section_header(line: &str) -> Option<String> {
    if line.starts_with('[') && line.ends_with(']') {
        Some(line[1..line.len() - 1].trim().to_string())
    } else {
        None
    }
}

fn parse_key_value(line: &str) -> Option<(&str, &str)> {
    let pos = line.find('=')?;
    Some((line[..pos].trim(), line[pos + 1..].trim()))
}

fn validate_tunnel_name(tunnel_name: &str) -> Result<(), String> {
    if tunnel_name.is_empty()
        || !tunnel_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("配置文件中的隧道名称只能包含字母、数字、下划线和连字符".to_string());
    }

    Ok(())
}

fn save_custom_tunnel_list(data_dir: &Path, tunnel: &CustomTunnel) -> Result<(), String> {
    fs::create_dir_all(data_dir).map_err(|e| format!("创建目录失败: {}", e))?;
    let list_file = data_dir.join(TUNNELS_LIST_FILE);

    let mut tunnels: Vec<CustomTunnel> = if list_file.exists() {
        let content = fs::read_to_string(&list_file)
            .map_err(|e| format!("读取自定义隧道列表失败: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("解析自定义隧道列表失败: {}", e))?
    } else {
        Vec::new()
    };

    if let Some(existing) = tunnels.iter_mut().find(|existing| existing.id == tunnel.id) {
        *existing = tunnel.clone();
    } else {
        tunnels.push(tunnel.clone());
    }

    let content = serde_json::to_string_pretty(&tunnels)
        .map_err(|e| format!("序列化自定义隧道列表失败: {}", e))?;
    fs::write(&list_file, content).map_err(|e| format!("保存自定义隧道列表失败: {}", e))?;
    Ok(())
}

fn string_to_i32(value: &str) -> i32 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    (hasher.finish() as i32).abs()
}

#[cfg(test)]
mod tests {
    use super::{get_config_file_name, get_custom_tunnel_hash, parse_ini_summary, split_ini_config};

    #[test]
    fn split_ini_keeps_common_and_sections() {
        let content = "[common]\nserver_addr = test.example.com\nserver_port = 7000\n\n[tcp_demo]\ntype = tcp\nlocal_ip = 127.0.0.1\nlocal_port = 8080\nremote_port = 9000\n\n[http_demo]\ntype = http\nlocal_ip = 127.0.0.1\nlocal_port = 3000\ncustom_domains = demo.example.com\n";

        let tunnels = split_ini_config(content).unwrap();
        assert_eq!(tunnels.len(), 2);
        assert_eq!(tunnels[0].0, "tcp_demo");
        assert!(tunnels[0].1.contains("[common]"));
        assert!(tunnels[0].1.contains("[tcp_demo]"));
        assert_eq!(tunnels[1].0, "http_demo");
    }

    #[test]
    fn parse_ini_summary_reads_primary_fields() {
        let content = "[common]\nserver_addr = frp.example.com\nserver_port = 7000\n\n[tcp_demo]\ntype = tcp\nlocal_ip = 127.0.0.1\nlocal_port = 8080\nremote_port = 9000\n";

        let (server_addr, server_port, tunnels, tunnel_type, custom_domains, subdomain, local_ip, local_port, remote_port) =
            parse_ini_summary(content).unwrap();

        assert_eq!(server_addr.as_deref(), Some("frp.example.com"));
        assert_eq!(server_port, Some(7000));
        assert_eq!(tunnels, vec!["tcp_demo"]);
        assert_eq!(tunnel_type.as_deref(), Some("tcp"));
        assert_eq!(custom_domains, None);
        assert_eq!(subdomain, None);
        assert_eq!(local_ip.as_deref(), Some("127.0.0.1"));
        assert_eq!(local_port, Some(8080));
        assert_eq!(remote_port, Some(9000));
    }

    #[test]
    fn invalid_tunnel_name_is_rejected() {
        let content = "[common]\nserver_addr = frp.example.com\n\n[bad name]\ntype = tcp\n";
        let err = split_ini_config(content).unwrap_err();
        assert!(err.contains("隧道名称"));
    }

    #[test]
    fn helpers_are_stable() {
        assert_eq!(get_config_file_name("demo"), "z_demo.ini");
        assert_eq!(get_custom_tunnel_hash("demo"), get_custom_tunnel_hash("demo"));
    }
}
