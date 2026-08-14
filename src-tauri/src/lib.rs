mod commands;
mod config;
mod emby;
mod error;
mod player;
mod state;
mod update;
mod win;

use crate::config::WindowState;
use crate::emby::{seconds_to_ticks, ProgressReport};
use crate::state::AppState;
use std::sync::Arc;
use tauri::{Emitter, Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let app_state = AppState::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![
            // 登录 / 服务器
            commands::auth::connect_server,
            commands::auth::login,
            commands::auth::restore_session,
            commands::auth::logout,
            commands::auth::saved_servers,
            commands::auth::disconnect,
            commands::auth::forget_server,
            commands::auth::switch_server,
            // 媒体库
            commands::library::get_views,
            commands::library::get_home,
            commands::library::get_items,
            commands::library::get_item,
            commands::library::get_seasons,
            commands::library::get_episodes,
            commands::library::get_similar,
            commands::library::search,
            commands::library::set_favorite,
            commands::library::set_played,
            // 播放
            commands::playback::prepare_playback,
            commands::playback::current_target,
            commands::playback::start_internal,
            commands::playback::play_test_pattern,
            commands::playback::stop_playback,
            commands::playback::player_set_pause,
            commands::playback::player_seek,
            commands::playback::player_seek_relative,
            commands::playback::player_set_volume,
            commands::playback::player_set_muted,
            commands::playback::player_set_speed,
            commands::playback::player_set_track,
            commands::playback::player_snapshot,
            commands::playback::list_external_players,
            commands::playback::start_external,
            commands::playback::report_external_progress,
            // 系统
            commands::system::get_settings,
            commands::system::update_player_settings,
            commands::system::update_ui_settings,
            commands::system::app_info,
            commands::system::set_fullscreen,
            commands::system::is_fullscreen,
            commands::system::player_minimize,
            commands::system::player_toggle_maximize,
            commands::system::player_is_maximized,
            commands::system::player_start_drag,
            // 更新
            update::check_update,
            update::open_release_page,
            update::download_update,
            update::install_update,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // 前端渲染好再显示，避免启动时白屏闪烁
            if let Some(main) = app.get_webview_window("main") {
                // 窗口还没显示，这时候摆位置不会看到跳动
                restore_window_state(&main, &app_state.settings.read().window);

                let handle_for_events = handle.clone();
                let state_for_events = app_state.clone();

                main.on_window_event(move |event| {
                    // 关掉浏览窗口就是退出整个程序，播放窗口和 mpv 一起收掉
                    if let WindowEvent::CloseRequested { .. } = event {
                        save_window_state(&handle_for_events, &state_for_events);
                        let _ = handle_for_events.emit("app:closing", ());
                        shutdown(&state_for_events);
                    }
                });
            }

            spawn_show_fallback(handle.clone());
            update::spawn_startup_check(handle.clone(), app_state.clone());
            spawn_progress_reporter(handle.clone(), app_state.clone());

            // 冒烟自检：SHENHE_SELFTEST=1 启动时直接跑一遍内置播放器，
            // 用来验证 mpv 能否嵌入窗口。正常使用不受影响。
            if std::env::var("SHENHE_SELFTEST").as_deref() == Ok("1") {
                spawn_selftest(handle, app_state.clone());
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动 ShenhePlayer 失败");
}

/// 把上次记住的位置和尺寸套回去。没存过就保持配置文件里的默认值。
fn restore_window_state(window: &tauri::WebviewWindow, state: &WindowState) {
    if !state.saved || state.width == 0 || state.height == 0 {
        return;
    }

    let _ = window.set_size(tauri::PhysicalSize::new(state.width, state.height));
    let _ = window.set_position(tauri::PhysicalPosition::new(state.x, state.y));
    if state.maximized {
        let _ = window.maximize();
    }
}

fn capture_window_state(window: &tauri::WebviewWindow) -> Option<WindowState> {
    // 全屏时的尺寸就是整块屏幕，记下来的话下次开窗就是屏幕那么大，
    // 保持上一次的记录更合理
    if window.is_fullscreen().unwrap_or(false) {
        return None;
    }

    let maximized = window.is_maximized().unwrap_or(false);

    // 最大化时记的必须是还原后的尺寸，否则下次启动会把「最大化的大小」
    // 当成普通窗口尺寸，取消最大化后窗口就撑满屏幕了
    if maximized {
        let _ = window.unmaximize();
    }
    let pos = window.outer_position().ok()?;
    let size = window.inner_size().ok()?;
    if maximized {
        let _ = window.maximize();
    }

    Some(WindowState {
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
        maximized,
        saved: true,
    })
}

/// 关闭前把两个窗口的位置尺寸记下来
fn save_window_state(app: &tauri::AppHandle, state: &Arc<AppState>) {
    let main = app
        .get_webview_window("main")
        .and_then(|w| capture_window_state(&w));
    let player = app
        .get_webview_window("player")
        .and_then(|w| capture_window_state(&w));

    {
        let mut settings = state.settings.write();
        if let Some(m) = main {
            settings.window = m;
        }
        if let Some(p) = player {
            settings.player_window = p;
        }
    }
    if let Err(e) = state.save_settings() {
        log::warn!("保存窗口状态失败: {e}");
    }
}

/// 窗口默认隐藏、由前端渲染完再显示。万一前端初始化抛异常，
/// 用户就会面对一个「进程在跑但看不见窗口」的状态 —— 这里兜底。
fn spawn_show_fallback(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        if let Some(main) = app.get_webview_window("main") {
            if !main.is_visible().unwrap_or(true) {
                log::warn!("前端未在 3 秒内就绪，强制显示窗口");
                let _ = main.show();
            }
        }
    });
}

fn spawn_selftest(app: tauri::AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let external = player::detect_players();
        log::info!(
            "自检：检测到 {} 个外置播放器 [{}]",
            external.len(),
            external
                .iter()
                .map(|p| format!("{}={}", p.kind, p.path))
                .collect::<Vec<_>>()
                .join(", ")
        );

        if let Err(e) = commands::playback::run_test_pattern(&app, &state).await {
            log::error!("自检失败: {e}");
            return;
        }

        // 等 mpv 把画面配置好，再把它真实的状态打出来
        tokio::time::sleep(std::time::Duration::from_secs(4)).await;
        let snapshot = match state.mpv.lock().await.as_ref() {
            Some(s) => s.snapshot.read().clone(),
            None => {
                log::error!("自检：mpv 会话不存在");
                return;
            }
        };

        log::info!(
            "自检结果: 已载入={} 分辨率={}x{} 帧率={:.2} 解码={} 编码={} 位置={:.2}s 轨道数={}",
            snapshot.file_loaded,
            snapshot.width,
            snapshot.height,
            snapshot.fps,
            if snapshot.hwdec.is_empty() { "未知" } else { &snapshot.hwdec },
            snapshot.video_codec,
            snapshot.position,
            snapshot.tracks.len(),
        );

        if snapshot.file_loaded && snapshot.width > 0 && snapshot.position > 0.0 {
            log::info!("自检：通过 —— mpv 正在向宿主窗口输出画面");
        } else {
            log::error!("自检：mpv 没有正常出画");
        }
    });
}

/// 每 10 秒向 Emby 上报一次播放进度，这样其它设备接着看能对上位置
fn spawn_progress_reporter(app: tauri::AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(10));
        loop {
            ticker.tick().await;

            let Some(active) = state.playback.read().clone() else {
                continue;
            };
            if active.external {
                continue;
            }

            let snapshot = {
                let guard = state.mpv.lock().await;
                match guard.as_ref() {
                    Some(s) => s.snapshot.read().clone(),
                    None => continue,
                }
            };
            if !snapshot.file_loaded || snapshot.position <= 0.0 {
                continue;
            }

            let Ok(client) = state.client() else { continue };
            let report = ProgressReport {
                item_id: active.item_id.clone(),
                media_source_id: active.media_source_id.clone(),
                play_session_id: active.play_session_id.clone(),
                position_ticks: seconds_to_ticks(snapshot.position),
                is_paused: snapshot.paused,
                is_muted: snapshot.muted,
                can_seek: true,
                play_method: if active.is_direct {
                    "DirectStream".into()
                } else {
                    "Transcode".into()
                },
                event_name: Some("timeupdate".into()),
                volume_level: Some(snapshot.volume.round() as i32),
                audio_stream_index: None,
                subtitle_stream_index: None,
            };

            if let Err(e) = client.report_progress(&report).await {
                log::debug!("上报进度失败: {e}");
            }

            if let Some(p) = state.playback.write().as_mut() {
                p.last_reported = snapshot.position;
            }

            let _ = app.emit("player:progress-reported", snapshot.position);
        }
    });
}

/// 关闭时清理：mpv 是独立进程，不主动杀会变成孤儿进程
fn shutdown(state: &Arc<AppState>) {
    // DestroyWindow 必须在创建它的线程（主线程）调用，CloseRequested 正好在主线程
    state.host.write().destroy();

    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(session) = state.mpv.lock().await.take() {
            session.quit().await;
        }
    });
}
