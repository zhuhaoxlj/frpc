use crate::models::{Node, NodeInfo, Tunnel, UserInfo};
use serde::Deserialize;


const API_BASE_URL: &str = "https://cf-v2.uapis.cn";

#[derive(Deserialize, Debug)]
struct ApiResponse<T> {
    code: u32,
    msg: Option<String>,
    data: Option<T>,
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("ChmlFrpLauncher/1.0")
        .no_proxy()
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))
}

async fn api_get<T: serde::de::DeserializeOwned>(
    endpoint: &str,
    token: &str,
) -> Result<T, String> {
    let client = build_client()?;
    let url = format!("{}{}", API_BASE_URL, endpoint);

    let response = client
        .get(&url)
        .header("authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let text = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let api_resp: ApiResponse<T> =
        serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    if api_resp.code == 200 {
        api_resp
            .data
            .ok_or_else(|| "API 返回空数据".to_string())
    } else {
        Err(api_resp.msg.unwrap_or_else(|| "请求失败".to_string()))
    }
}

async fn api_post<T: serde::de::DeserializeOwned>(
    endpoint: &str,
    token: &str,
    body: &str,
    content_type: &str,
) -> Result<T, String> {
    let client = build_client()?;
    let url = format!("{}{}", API_BASE_URL, endpoint);

    let response = client
        .post(&url)
        .header("authorization", format!("Bearer {}", token))
        .header("Content-Type", content_type)
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let text = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let api_resp: ApiResponse<T> =
        serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    if api_resp.code == 200 {
        api_resp
            .data
            .ok_or_else(|| "API 返回空数据".to_string())
    } else {
        Err(api_resp.msg.unwrap_or_else(|| "请求失败".to_string()))
    }
}

/// 获取用户信息
pub async fn fetch_user_info(token: &str) -> Result<UserInfo, String> {
    api_get("/userinfo", token).await
}

/// 获取隧道列表
pub async fn fetch_tunnels(token: &str) -> Result<Vec<Tunnel>, String> {
    api_get("/tunnel", token).await
}

/// 获取节点列表
pub async fn fetch_nodes(token: &str) -> Result<Vec<Node>, String> {
    api_get("/node", token).await
}

/// 获取节点详细信息
pub async fn fetch_node_info(node_name: &str, token: &str) -> Result<NodeInfo, String> {
    let endpoint = format!("/nodeinfo?node={}", urlencoding::encode(node_name));
    api_get(&endpoint, token).await
}

/// 下线隧道
pub async fn offline_tunnel(tunnel_name: &str, token: &str) -> Result<(), String> {
    let body = format!("tunnel_name={}", urlencoding::encode(tunnel_name));

    #[derive(Deserialize)]
    struct OfflineResponse {
        code: u32,
        state: String,
        msg: Option<String>,
    }

    let client = build_client()?;
    let url = format!("{}/offline_tunnel", API_BASE_URL);

    let response = client
        .post(&url)
        .header("authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    let text = response.text().await.map_err(|e| format!("读取响应失败: {}", e))?;
    let resp: OfflineResponse =
        serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    if resp.code == 200 && resp.state == "success" {
        Ok(())
    } else {
        Err(resp.msg.unwrap_or_else(|| "下线隧道失败".to_string()))
    }
}

/// 删除隧道
pub async fn delete_tunnel(tunnel_id: i32, token: &str) -> Result<(), String> {
    let _: serde_json::Value = api_get(&format!("/delete_tunnel?tunnelid={}", tunnel_id), token).await?;
    Ok(())
}
