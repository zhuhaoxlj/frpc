use crate::models::StoredUser;
use serde::Deserialize;

const ACCOUNT_OAUTH_ISSUER: &str = "https://account-api.qzhua.net";
const ACCOUNT_OAUTH_CLIENT_ID: &str = "019d4334b34972ca9fd41513e5703dfd";
const DEVICE_CODE_DEFAULT_SCOPE: &str = "profile email offline_access chmlfrp_api";

/// 设备授权响应
#[derive(Deserialize, Debug, Clone)]
pub struct DeviceAuthorizationResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: Option<u64>,
    pub interval: Option<u64>,
}

/// Token 响应
#[derive(Deserialize, Debug, Clone)]
pub struct DeviceTokenResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

fn build_oauth_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("ChmlFrpLauncher/1.0")
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))
}

/// 申请设备授权码
pub async fn create_device_authorization() -> Result<DeviceAuthorizationResponse, String> {
    let client = build_oauth_client()?;
    let url = format!("{}/oauth2/device_authorization", ACCOUNT_OAUTH_ISSUER);

    let body = format!(
        "client_id={}&scope={}",
        ACCOUNT_OAUTH_CLIENT_ID, DEVICE_CODE_DEFAULT_SCOPE
    );

    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("请求设备授权失败: {}", e))?;

    let text = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let data: DeviceAuthorizationResponse =
        serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {} (body: {})", e, text))?;

    Ok(data)
}

/// 用设备码换取 token
pub async fn exchange_device_code_for_token(
    device_code: &str,
) -> Result<DeviceTokenResponse, String> {
    let client = build_oauth_client()?;
    let url = format!("{}/oauth2/token", ACCOUNT_OAUTH_ISSUER);

    let body = format!(
        "grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code={}&client_id={}",
        device_code, ACCOUNT_OAUTH_CLIENT_ID
    );

    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("请求 token 失败: {}", e))?;

    let text = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let data: DeviceTokenResponse =
        serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    Ok(data)
}

/// 刷新 access token
pub async fn refresh_access_token(refresh_token: &str) -> Result<DeviceTokenResponse, String> {
    let client = build_oauth_client()?;
    let url = format!("{}/oauth2/token", ACCOUNT_OAUTH_ISSUER);

    let body = format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}",
        refresh_token, ACCOUNT_OAUTH_CLIENT_ID
    );

    let response = client
        .post(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Accept", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("刷新 token 失败: {}", e))?;

    let text = response
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    let data: DeviceTokenResponse =
        serde_json::from_str(&text).map_err(|e| format!("解析响应失败: {}", e))?;

    if data.access_token.is_some() {
        Ok(data)
    } else {
        Err(data
            .error_description
            .or(data.error)
            .unwrap_or_else(|| "刷新 token 失败".to_string()))
    }
}

/// 轮询设备码授权，直到用户完成授权或超时
pub async fn poll_device_authorization(
    device_code: &str,
    interval: u64,
    expires_in: u64,
) -> Result<DeviceTokenResponse, String> {
    let poll_interval = std::time::Duration::from_secs(interval.max(5));
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(expires_in);

    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err("设备授权已超时，请重新登录".to_string());
        }

        tokio::time::sleep(poll_interval).await;

        let resp = exchange_device_code_for_token(device_code).await?;

        if resp.access_token.is_some() {
            return Ok(resp);
        }

        match resp.error.as_deref() {
            Some("authorization_pending") => continue,
            Some("slow_down") => {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
            Some("expired_token") => {
                return Err("设备授权已过期，请重新登录".to_string());
            }
            Some("access_denied") => {
                return Err("用户拒绝了授权".to_string());
            }
            Some(err) => {
                return Err(format!(
                    "授权失败: {}",
                    resp.error_description.unwrap_or_else(|| err.to_string())
                ));
            }
            None => continue,
        }
    }
}

/// 用 access token 登录获取用户信息，返回 StoredUser
pub async fn login_with_access_token(
    access_token: &str,
    token_response: &DeviceTokenResponse,
) -> Result<StoredUser, String> {
    let user_info = crate::api::fetch_user_info(access_token).await?;

    Ok(StoredUser {
        username: user_info.username,
        usergroup: user_info.usergroup,
        userimg: Some(user_info.userimg),
        usertoken: Some(user_info.usertoken),
        access_token: Some(access_token.to_string()),
        refresh_token: token_response.refresh_token.clone(),
        access_token_expires_at: token_response
            .expires_in
            .map(|e| chrono::Utc::now().timestamp() + e as i64),
        token_type: Some(
            token_response
                .token_type
                .clone()
                .unwrap_or_else(|| "Bearer".to_string()),
        ),
        tunnel_count: Some(user_info.tunnel_count),
        tunnel: Some(user_info.tunnel),
    })
}

/// 获取有效的 access token（自动刷新过期 token）
pub async fn ensure_valid_token(user: &mut StoredUser) -> Result<String, String> {
    if let Some(ref token) = user.access_token {
        // 检查是否快过期（提前 60 秒）
        if let Some(expires_at) = user.access_token_expires_at {
            let now = chrono::Utc::now().timestamp();
            if now >= expires_at - 60 {
                // 需要刷新
                if let Some(ref refresh_token) = user.refresh_token {
                    let refreshed = refresh_access_token(refresh_token).await?;
                    if let Some(ref new_token) = refreshed.access_token {
                        user.access_token = Some(new_token.clone());
                        user.refresh_token = refreshed.refresh_token.or(user.refresh_token.clone());
                        user.access_token_expires_at = refreshed
                            .expires_in
                            .map(|e| chrono::Utc::now().timestamp() + e as i64);
                        return Ok(new_token.clone());
                    }
                }
                return Err("登录信息已过期，请重新登录".to_string());
            }
        }
        return Ok(token.clone());
    }

    // 回退到 legacy usertoken
    if let Some(ref usertoken) = user.usertoken {
        return Ok(usertoken.clone());
    }

    Err("登录信息已过期，请重新登录".to_string())
}
