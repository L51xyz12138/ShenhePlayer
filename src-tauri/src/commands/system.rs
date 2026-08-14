use crate::config::{PlayerSettings, Settings, UiSettings};
use crate::error::Result;
use crate::player;
use crate::state::AppState;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<AppState>>) -> Settings {
    let mut s = state.settings.read().clone();
    // token 不下发给前端设置页
    for server in &mut s.servers {
        server.token.clear();
    }
    s
}

#[tauri::command]
pub fn update_player_settings(
    state: State<'_, Arc<AppState>>,
    player: PlayerSettings,
) -> Result<()> {
    state.settings.write().player = player;
    state.save_settings()
}

#[tauri::command]
pub fn update_ui_settings(state: State<'_, Arc<AppState>>, ui: UiSettings) -> Result<()> {
    state.settings.write().ui = ui;
    state.save_settings()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub mpv_path: Option<String>,
    pub mpv_available: bool,
    pub config_path: String,
}

#[tauri::command]
pub fn app_info(state: State<'_, Arc<AppState>>) -> AppInfo {
    let configured = state.settings.read().player.mpv_path.clone();
    let mpv_path = player::resolve_mpv_path(&configured);
    AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        mpv_available: mpv_path.is_some(),
        mpv_path,
        config_path: crate::config::settings_path().to_string_lossy().to_string(),
    }
}

// 控制条跑在 overlay 窗口里，但最小化/最大化/全屏要作用在 player 窗口上。
// 走命令而不是让前端跨窗口操作，省掉一层 capability 配置，也更不容易出错。

/// 全屏针对播放窗口——浏览界面不需要全屏
#[tauri::command]
pub fn set_fullscreen(app: AppHandle, fullscreen: bool) -> Result<()> {
    if let Some(player) = app.get_webview_window("player") {
        let _ = player.set_fullscreen(fullscreen);
    }
    Ok(())
}

#[tauri::command]
pub fn is_fullscreen(app: AppHandle) -> bool {
    app.get_webview_window("player")
        .and_then(|w| w.is_fullscreen().ok())
        .unwrap_or(false)
}

#[tauri::command]
pub fn player_minimize(app: AppHandle) {
    if let Some(player) = app.get_webview_window("player") {
        let _ = player.minimize();
    }
}

#[tauri::command]
pub fn player_toggle_maximize(app: AppHandle) {
    if let Some(player) = app.get_webview_window("player") {
        let maximized = player.is_maximized().unwrap_or(false);
        let _ = if maximized {
            player.unmaximize()
        } else {
            player.maximize()
        };
    }
}

/// 拖动控制层的顶栏时，移动的应该是底下的 player 窗口
#[tauri::command]
pub fn player_start_drag(app: AppHandle) {
    if let Some(player) = app.get_webview_window("player") {
        let _ = player.start_dragging();
    }
}

#[tauri::command]
pub fn player_is_maximized(app: AppHandle) -> bool {
    app.get_webview_window("player")
        .and_then(|w| w.is_maximized().ok())
        .unwrap_or(false)
}
