use crate::config::ServerProfile;
use crate::emby::{EmbyClient, SystemInfo};
use crate::error::{AppError, Result};
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub server_id: String,
    pub server_name: String,
    pub server_url: String,
    pub user_id: String,
    pub user_name: String,
    /// 前端拼图片 URL 需要（图片接口必须带 api_key）
    pub token: String,
    pub avatar_url: Option<String>,
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 只测试连通性，不登录。用于登录页实时校验服务器地址。
#[tauri::command]
pub async fn connect_server(
    state: State<'_, Arc<AppState>>,
    url: String,
    allow_invalid_certs: bool,
) -> Result<SystemInfo> {
    let device_id = state.settings.read().device_id.clone();
    let client = EmbyClient::new(&url, device_id, allow_invalid_certs)?;
    client.public_system_info().await
}

#[tauri::command]
pub async fn login(
    state: State<'_, Arc<AppState>>,
    url: String,
    username: String,
    password: String,
    allow_invalid_certs: bool,
) -> Result<SessionInfo> {
    let device_id = state.settings.read().device_id.clone();
    let mut client = EmbyClient::new(&url, device_id, allow_invalid_certs)?;

    let info = client.public_system_info().await?;
    let auth = client.authenticate(&username, &password).await?;

    let avatar_url = auth.user.primary_image_tag.as_ref().map(|tag| {
        client.image_url(&auth.user.id, "Primary", Some(tag), Some(200), None)
    });

    let profile = ServerProfile {
        id: if info.id.is_empty() { client.base_url().to_string() } else { info.id.clone() },
        name: if info.server_name.is_empty() { url.clone() } else { info.server_name.clone() },
        url: client.base_url().to_string(),
        username: username.clone(),
        user_id: auth.user.id.clone(),
        token: auth.access_token.clone(),
        allow_invalid_certs,
        last_used: now_ts(),
    };

    let session = SessionInfo {
        server_id: profile.id.clone(),
        server_name: profile.name.clone(),
        server_url: profile.url.clone(),
        user_id: auth.user.id.clone(),
        user_name: auth.user.name.clone(),
        token: auth.access_token.clone(),
        avatar_url,
    };

    {
        let mut s = state.settings.write();
        s.active_server = profile.id.clone();
        s.upsert_server(profile);
    }
    state.save_settings()?;
    *state.client.write() = Some(client);

    Ok(session)
}

/// 用保存的 token 恢复会话，失败返回 None（前端跳登录页）
#[tauri::command]
pub async fn restore_session(state: State<'_, Arc<AppState>>) -> Result<Option<SessionInfo>> {
    let (profile, device_id) = {
        let s = state.settings.read();
        match s.active().cloned() {
            Some(p) => (p, s.device_id.clone()),
            None => return Ok(None),
        }
    };

    if profile.token.is_empty() {
        return Ok(None);
    }

    let mut client = EmbyClient::new(&profile.url, device_id, profile.allow_invalid_certs)?;
    client.set_session(profile.token.clone(), profile.user_id.clone());

    // token 可能已被服务器吊销
    let user = match client.validate_session().await {
        Ok(u) => u,
        Err(e) => {
            log::info!("恢复会话失败，需要重新登录: {e}");
            return Ok(None);
        }
    };

    let avatar_url = user
        .primary_image_tag
        .as_ref()
        .map(|tag| client.image_url(&user.id, "Primary", Some(tag), Some(200), None));

    let session = SessionInfo {
        server_id: profile.id.clone(),
        server_name: profile.name.clone(),
        server_url: profile.url.clone(),
        user_id: user.id.clone(),
        user_name: user.name.clone(),
        token: profile.token.clone(),
        avatar_url,
    };

    *state.client.write() = Some(client);
    Ok(Some(session))
}

#[tauri::command]
pub async fn logout(state: State<'_, Arc<AppState>>) -> Result<()> {
    if let Ok(client) = state.client() {
        let _ = client.logout().await;
    }
    *state.client.write() = None;

    {
        let mut s = state.settings.write();
        let active = s.active_server.clone();
        if let Some(p) = s.servers.iter_mut().find(|p| p.id == active) {
            p.token.clear();
        }
        s.active_server.clear();
    }
    state.save_settings()?;
    Ok(())
}

/// 服务器列表项。不下发 token，只告诉前端「有没有凭据」和「是不是当前这台」。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSummary {
    pub id: String,
    pub name: String,
    pub url: String,
    pub username: String,
    pub user_id: String,
    pub allow_invalid_certs: bool,
    pub last_used: i64,
    /// 有保存的登录凭据，可以直接切过去
    pub has_token: bool,
    /// 当前正在使用的服务器
    pub is_active: bool,
}

/// 已保存的服务器列表，按最近使用排序
#[tauri::command]
pub fn saved_servers(state: State<'_, Arc<AppState>>) -> Vec<ServerSummary> {
    let s = state.settings.read();
    let mut list: Vec<ServerSummary> = s
        .servers
        .iter()
        .map(|p| ServerSummary {
            id: p.id.clone(),
            name: p.name.clone(),
            url: p.url.clone(),
            username: p.username.clone(),
            user_id: p.user_id.clone(),
            allow_invalid_certs: p.allow_invalid_certs,
            last_used: p.last_used,
            has_token: !p.token.is_empty(),
            is_active: p.id == s.active_server,
        })
        .collect();
    list.sort_by_key(|s| -s.last_used);
    list
}

/// 断开当前服务器但保留登录凭据，回到服务器列表。
/// 和 logout 的区别：logout 会清掉 token，下次要重新输密码。
#[tauri::command]
pub fn disconnect(state: State<'_, Arc<AppState>>) -> Result<()> {
    *state.client.write() = None;
    state.settings.write().active_server.clear();
    state.save_settings()
}

#[tauri::command]
pub fn forget_server(state: State<'_, Arc<AppState>>, server_id: String) -> Result<()> {
    {
        let mut s = state.settings.write();
        s.servers.retain(|p| p.id != server_id);
        if s.active_server == server_id {
            s.active_server.clear();
        }
    }
    state.save_settings()?;
    Ok(())
}

/// 切换到另一个已保存的服务器
#[tauri::command]
pub async fn switch_server(
    state: State<'_, Arc<AppState>>,
    server_id: String,
) -> Result<Option<SessionInfo>> {
    {
        let mut s = state.settings.write();
        if !s.servers.iter().any(|p| p.id == server_id) {
            return Err(AppError::Other("服务器不存在".into()));
        }
        s.active_server = server_id;
    }
    state.save_settings()?;
    restore_session(state).await
}
