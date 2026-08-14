use crate::emby::models::{
    seconds_to_ticks, ticks_to_seconds, ExternalSubtitle, MediaSourceInfo, PlaybackTarget,
};
use crate::emby::ProgressReport;
use crate::error::{AppError, Result};
use crate::player::{self, detect_players, DetectedPlayer, MpvEvent, MpvSession, PlayerSnapshot};
use crate::state::{ActivePlayback, AppState};
use crate::win::VideoHost;
use serde_json::json;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

/// 解析出可以直接送进播放器的 URL 与元信息
#[tauri::command]
pub async fn prepare_playback(
    state: State<'_, Arc<AppState>>,
    item_id: String,
    media_source_id: Option<String>,
    resume: bool,
) -> Result<PlaybackTarget> {
    let client = state.client()?;
    let item = client.item(&item_id).await?;

    let resume_ticks = item
        .user_data
        .as_ref()
        .and_then(|u| u.playback_position_ticks)
        .unwrap_or(0);
    let start_ticks = if resume { resume_ticks } else { 0 };

    let max_bitrate = {
        let mb = state.settings.read().player.max_bitrate;
        if mb > 0 { Some(mb) } else { None }
    };

    let info = client
        .playback_info(&item_id, media_source_id.as_deref(), start_ticks, max_bitrate)
        .await?;

    let play_session_id = info
        .play_session_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().simple().to_string());

    let source: MediaSourceInfo = match &media_source_id {
        Some(msid) => info
            .media_sources
            .iter()
            .find(|s| &s.id == msid)
            .cloned()
            .or_else(|| info.media_sources.first().cloned()),
        None => info.media_sources.first().cloned(),
    }
    .or_else(|| item.media_sources.first().cloned())
    .ok_or_else(|| AppError::Player("服务器没有返回可播放的媒体源".into()))?;

    // 优先直连原始文件：无转码、画质无损、服务器几乎不吃 CPU
    let (url, is_direct) = if source.supports_direct_play || source.supports_direct_stream {
        let url = match &source.direct_stream_url {
            Some(u) if !u.is_empty() => client.absolute_url(u),
            _ => client.direct_stream_url(
                &item_id,
                &source.id,
                source.container.as_deref(),
                &play_session_id,
            ),
        };
        (url, true)
    } else if let Some(t) = &source.transcoding_url {
        (client.absolute_url(t), false)
    } else {
        // 服务器没给结论时仍然试直连，mpv 基本什么都能吃
        (
            client.direct_stream_url(
                &item_id,
                &source.id,
                source.container.as_deref(),
                &play_session_id,
            ),
            true,
        )
    };

    let mut audio_streams = Vec::new();
    let mut subtitle_streams = Vec::new();
    let mut video_stream = None;
    let mut external_subtitles = Vec::new();

    for s in &source.media_streams {
        match s.kind.as_str() {
            "Audio" => audio_streams.push(s.clone()),
            "Subtitle" => {
                if s.is_external {
                    let codec = s.codec.clone().unwrap_or_else(|| "srt".into());
                    let url = match &s.delivery_url {
                        Some(u) if !u.is_empty() => client.absolute_url(u),
                        _ => client.subtitle_url(&item_id, &source.id, s.index, &codec),
                    };
                    external_subtitles.push(ExternalSubtitle {
                        index: s.index,
                        title: s
                            .display_title
                            .clone()
                            .or_else(|| s.title.clone())
                            .unwrap_or_else(|| format!("字幕 {}", s.index)),
                        language: s.language.clone(),
                        url,
                        is_default: s.is_default,
                        codec: s.codec.clone(),
                    });
                }
                subtitle_streams.push(s.clone());
            }
            "Video" if video_stream.is_none() => video_stream = Some(s.clone()),
            _ => {}
        }
    }

    // Emby 给的是文件里的绝对流索引，mpv 的 aid/sid 是「同类轨道里的第几个」，
    // 两者对不上，必须按流索引排序后换算。不做这一步的话 Emby 里设好的
    // 语言偏好就白设了 —— mpv 会按自己的规则挑一条。
    let mpv_aid = source.default_audio_stream_index.and_then(|want| {
        let mut ordered: Vec<i32> = audio_streams.iter().map(|s| s.index).collect();
        ordered.sort_unstable();
        ordered.iter().position(|i| *i == want).map(|p| p as i64 + 1)
    });

    // 内封字幕才有 sid；外挂字幕是 sub-add 进来的，另算
    let embedded_subs: Vec<i32> = {
        let mut v: Vec<i32> = subtitle_streams
            .iter()
            .filter(|s| !s.is_external)
            .map(|s| s.index)
            .collect();
        v.sort_unstable();
        v
    };
    let mpv_sid = source.default_subtitle_stream_index.and_then(|want| {
        embedded_subs.iter().position(|i| *i == want).map(|p| p as i64 + 1)
    });
    let default_external_sub = source
        .default_subtitle_stream_index
        .and_then(|want| external_subtitles.iter().position(|s| s.index == want));

    let title = match item.kind.as_str() {
        "Episode" => item
            .series_name
            .clone()
            .map(|s| format!("{s} - {}", item.name))
            .unwrap_or_else(|| item.name.clone()),
        _ => item.name.clone(),
    };

    let sub_title = match item.kind.as_str() {
        "Episode" => match (item.parent_index_number, item.index_number) {
            (Some(s), Some(e)) => Some(format!("第 {s} 季 第 {e} 集")),
            _ => item.season_name.clone(),
        },
        _ => item.production_year.map(|y| y.to_string()),
    };

    let backdrop_url = item
        .backdrop_image_tags
        .first()
        .map(|tag| client.image_url(&item.id, "Backdrop", Some(tag), Some(720), None))
        .or_else(|| {
            let pid = item.parent_backdrop_item_id.as_ref()?;
            let tag = item.parent_backdrop_image_tags.first()?;
            Some(client.image_url(pid, "Backdrop", Some(tag), Some(720), None))
        });

    let duration = ticks_to_seconds(source.run_time_ticks.or(item.run_time_ticks));

    let target = PlaybackTarget {
        item_id: item.id.clone(),
        media_source_id: source.id.clone(),
        play_session_id,
        url,
        is_direct,
        title,
        sub_title,
        start_position: ticks_to_seconds(Some(start_ticks)),
        duration,
        container: source.container.clone(),
        size: source.size,
        bitrate: source.bitrate,
        default_audio_index: source.default_audio_stream_index,
        default_subtitle_index: source.default_subtitle_stream_index,
        mpv_aid,
        mpv_sid,
        default_external_sub,
        audio_streams,
        subtitle_streams,
        video_stream,
        external_subtitles,
        backdrop_url,
    };

    *state.target.write() = Some(target.clone());
    Ok(target)
}

/// 控制层窗口用它拿当前播放的元信息（标题、音轨字幕列表等）
#[tauri::command]
pub fn current_target(state: State<'_, Arc<AppState>>) -> Option<PlaybackTarget> {
    state.target.read().clone()
}

// ------------------------------------------------------------------ 内置播放器

/// 开始用内置 mpv 播放 prepare_playback 解析出来的目标
#[tauri::command]
pub async fn start_internal(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<()> {
    let target = state
        .target
        .read()
        .clone()
        .ok_or_else(|| AppError::Player("没有待播放的内容".into()))?;
    open_in_mpv(&app, state.inner(), &target).await?;

    // 通知服务器「开始播放」，其它设备能看到正在播放状态
    let volume = state.settings.read().player.volume;
    *state.playback.write() = Some(ActivePlayback {
        item_id: target.item_id.clone(),
        media_source_id: target.media_source_id.clone(),
        play_session_id: target.play_session_id.clone(),
        is_direct: target.is_direct,
        last_reported: target.start_position,
        external: false,
    });

    let client = state.client()?;
    let report = ProgressReport {
        item_id: target.item_id.clone(),
        media_source_id: target.media_source_id.clone(),
        play_session_id: target.play_session_id.clone(),
        position_ticks: seconds_to_ticks(target.start_position),
        is_paused: false,
        is_muted: false,
        can_seek: true,
        play_method: if target.is_direct { "DirectStream".into() } else { "Transcode".into() },
        event_name: None,
        volume_level: Some(volume),
        audio_stream_index: None,
        subtitle_stream_index: None,
    };
    if let Err(e) = client.report_playing(&report).await {
        log::warn!("上报开始播放失败: {e}");
    }

    Ok(())
}

/// 诊断用：不连服务器，直接让 mpv 播一段合成测试画面。
/// 一次性验证 mpv 能否启动、硬解是否可用、画面能否正确嵌入窗口。
#[tauri::command]
pub async fn play_test_pattern(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<()> {
    run_test_pattern(&app, state.inner()).await
}

pub async fn run_test_pattern(app: &AppHandle, state: &Arc<AppState>) -> Result<()> {
    let target = PlaybackTarget {
        item_id: "__test__".into(),
        media_source_id: "__test__".into(),
        play_session_id: "__test__".into(),
        url: "av://lavfi:testsrc2=size=1920x1080:rate=60".into(),
        is_direct: true,
        title: "测试画面".into(),
        sub_title: Some("用于检查内置播放器是否正常".into()),
        start_position: 0.0,
        duration: 0.0,
        container: None,
        size: None,
        bitrate: None,
        audio_streams: Vec::new(),
        subtitle_streams: Vec::new(),
        video_stream: None,
        external_subtitles: Vec::new(),
        default_audio_index: None,
        default_subtitle_index: None,
        mpv_aid: None,
        mpv_sid: None,
        default_external_sub: None,
        backdrop_url: None,
    };

    *state.target.write() = Some(target.clone());
    open_in_mpv(app, state, &target).await
}

/// 启动/复用 mpv 会话，把目标载入并把画面显示出来
async fn open_in_mpv(
    app: &AppHandle,
    state: &Arc<AppState>,
    target: &PlaybackTarget,
) -> Result<()> {
    let settings = state.settings.read().player.clone();

    let mpv_path = player::resolve_mpv_path(&settings.mpv_path).ok_or_else(|| {
        AppError::Player(
            "没有找到 mpv.exe。请在设置里指定 mpv 路径，或改用外置播放器。".into(),
        )
    })?;

    // 播放窗口是独立的顶层窗口，视频宿主挂在它下面
    let window = ensure_player_window(app, state)?;
    let host = ensure_host_window(app, state, &window).await?;

    let mut guard = state.mpv.lock().await;
    if guard.is_none() {
        let app_for_events = app.clone();
        let session = MpvSession::launch(&mpv_path, host.wid(), &settings, move |event| {
            handle_mpv_event(&app_for_events, event);
        })
        .await?;
        *guard = Some(session);
    }

    let session = guard.as_ref().expect("刚刚已确保存在");

    // 外挂字幕和默认轨道都等 file-loaded 之后再处理，见 apply_default_tracks
    session.load_file(&target.url, target.start_position, &target.title)?;
    session.set_pause(false)?;

    let _ = window.set_title(&format!("{} - ShenhePlayer", target.title));

    // 顺序有讲究：先亮播放窗口，再把视频子窗口贴上去，最后浮出控制层，
    // 中间不会闪出桌面或空白的 WebView2
    window
        .show()
        .map_err(|e| AppError::Player(format!("显示播放窗口失败: {e}")))?;

    if state.settings.read().player.fullscreen_on_play {
        let _ = window.set_fullscreen(true);
    }

    host.show();

    let overlay = ensure_overlay_window(app, &window)?;
    sync_overlay(app);
    overlay
        .show()
        .map_err(|e| AppError::Player(format!("显示控制层失败: {e}")))?;
    let _ = overlay.set_focus();

    Ok(())
}

/// 停止播放，回到浏览界面
#[tauri::command]
pub async fn stop_playback(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<()> {
    let position = {
        let guard = state.mpv.lock().await;
        guard.as_ref().map(|s| s.snapshot.read().position).unwrap_or(0.0)
    };

    report_stop(&state, position).await;

    if let Some(session) = state.mpv.lock().await.as_ref() {
        let _ = session.stop();
    }

    {
        let h = *state.host.read();
        h.hide();
    }

    // 窗口只隐藏不销毁：下次播放能秒开，mpv 会话也继续复用
    if let Some(overlay) = app.get_webview_window("overlay") {
        let _ = overlay.hide();
    }
    if let Some(window) = app.get_webview_window("player") {
        let _ = window.set_fullscreen(false);
        let _ = window.hide();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }

    *state.playback.write() = None;
    *state.target.write() = None;
    let _ = app.emit("player:closed", ());
    Ok(())
}

async fn report_stop(state: &State<'_, Arc<AppState>>, position: f64) {
    let active = state.playback.read().clone();
    let (Some(active), Ok(client)) = (active, state.client()) else {
        return;
    };

    let report = ProgressReport {
        item_id: active.item_id.clone(),
        media_source_id: active.media_source_id.clone(),
        play_session_id: active.play_session_id.clone(),
        position_ticks: seconds_to_ticks(position),
        is_paused: false,
        is_muted: false,
        can_seek: true,
        play_method: if active.is_direct { "DirectStream".into() } else { "Transcode".into() },
        event_name: None,
        volume_level: None,
        audio_stream_index: None,
        subtitle_stream_index: None,
    };
    if let Err(e) = client.report_stopped(&report).await {
        log::warn!("上报停止播放失败: {e}");
    }
}

macro_rules! with_session {
    ($state:expr, $session:ident => $body:expr) => {{
        let guard = $state.mpv.lock().await;
        let $session = guard
            .as_ref()
            .ok_or_else(|| AppError::Player("播放器未运行".into()))?;
        $body
    }};
}

#[tauri::command]
pub async fn player_set_pause(state: State<'_, Arc<AppState>>, paused: bool) -> Result<()> {
    with_session!(state, s => s.set_pause(paused))
}

#[tauri::command]
pub async fn player_seek(state: State<'_, Arc<AppState>>, position: f64) -> Result<()> {
    with_session!(state, s => s.seek_absolute(position))
}

#[tauri::command]
pub async fn player_seek_relative(state: State<'_, Arc<AppState>>, delta: f64) -> Result<()> {
    with_session!(state, s => s.seek_relative(delta))
}

#[tauri::command]
pub async fn player_set_volume(state: State<'_, Arc<AppState>>, volume: f64) -> Result<()> {
    {
        let mut settings = state.settings.write();
        settings.player.volume = volume.round() as i32;
    }
    let _ = state.save_settings();
    with_session!(state, s => s.set_property("volume", json!(volume)))
}

#[tauri::command]
pub async fn player_set_muted(state: State<'_, Arc<AppState>>, muted: bool) -> Result<()> {
    with_session!(state, s => s.set_property("mute", json!(muted)))
}

#[tauri::command]
pub async fn player_set_speed(state: State<'_, Arc<AppState>>, speed: f64) -> Result<()> {
    with_session!(state, s => s.set_property("speed", json!(speed)))
}

/// kind: "aid" 音轨 / "sid" 字幕；id 为 mpv track id，-1 或 0 表示关闭
#[tauri::command]
pub async fn player_set_track(
    state: State<'_, Arc<AppState>>,
    kind: String,
    id: i64,
) -> Result<()> {
    let prop = match kind.as_str() {
        "audio" => "aid",
        "sub" => "sid",
        other => return Err(AppError::Player(format!("未知轨道类型: {other}"))),
    };
    let value = if id < 0 { json!("no") } else { json!(id) };
    with_session!(state, s => s.set_property(prop, value))
}

#[tauri::command]
pub async fn player_snapshot(state: State<'_, Arc<AppState>>) -> Result<PlayerSnapshot> {
    let guard = state.mpv.lock().await;
    Ok(guard
        .as_ref()
        .map(|s| s.snapshot.read().clone())
        .unwrap_or_default())
}

// ------------------------------------------------------------------ 外置播放器

#[tauri::command]
pub fn list_external_players() -> Vec<DetectedPlayer> {
    detect_players()
}

#[tauri::command]
pub async fn start_external(state: State<'_, Arc<AppState>>) -> Result<()> {
    let target = state
        .target
        .read()
        .clone()
        .ok_or_else(|| AppError::Player("没有待播放的内容".into()))?;

    let (exe, kind, custom) = {
        let s = state.settings.read();
        (
            s.player.external_path.clone(),
            s.player.external_kind.clone(),
            s.player.external_args.clone(),
        )
    };

    // 没配置就自动挑一个装了的
    let (exe, kind) = if exe.trim().is_empty() {
        let found = detect_players()
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Player("没有检测到外置播放器，请在设置里指定".into()))?;
        (found.path, found.kind)
    } else {
        (exe, kind)
    };

    let subs: Vec<String> = target
        .external_subtitles
        .iter()
        .map(|s| s.url.clone())
        .collect();

    player::external::launch(
        &exe,
        &kind,
        &target.url,
        &target.title,
        target.start_position,
        &custom,
        &subs,
    )?;

    *state.playback.write() = Some(ActivePlayback {
        item_id: target.item_id.clone(),
        media_source_id: target.media_source_id.clone(),
        play_session_id: target.play_session_id.clone(),
        is_direct: target.is_direct,
        last_reported: target.start_position,
        external: true,
    });

    Ok(())
}

/// 外置播放器播完后，由前端手动回报观看进度
#[tauri::command]
pub async fn report_external_progress(
    state: State<'_, Arc<AppState>>,
    position: f64,
    finished: bool,
) -> Result<()> {
    let active = state.playback.read().clone();
    let Some(active) = active else {
        return Ok(());
    };
    let client = state.client()?;

    if finished {
        client.set_played(&active.item_id, true).await?;
    }
    report_stop(&state, position).await;
    *state.playback.write() = None;
    Ok(())
}

// ------------------------------------------------------------------ 窗口

/// 独立的播放器由两个顶层窗口组成：
///
/// ```text
/// player 窗口（不透明、无边框）      ← 全屏 / 最大化 / 关闭都作用在它身上
///  ├─ WebView2                      闲置，被视频完全盖住
///  └─ ShenheVideoHost（置顶）        mpv 在这里渲染
///
/// overlay 窗口（透明，归属于 player） 控制条，由 DWM 合成到视频之上
/// ```
///
/// 为什么不能合成一个窗口：Tauri 的透明窗口带 WS_EX_NOREDIRECTIONBITMAP，
/// 没有重定向表面，子窗口不参与合成 —— 把视频放在透明 WebView2 底下的话
/// 画面根本不显示。所以视频盖住 WebView2，控制条另开一个透明窗口。
///
/// 两个窗口都挂在播放器自己身上（而不是浏览窗口），所以全屏时只要在
/// Resized 里把视频宿主和控制层一起对齐，就不会露出底下的东西。
fn ensure_player_window(app: &AppHandle, state: &Arc<AppState>) -> Result<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window("player") {
        return Ok(w);
    }

    let window = tauri::WebviewWindowBuilder::new(
        app,
        "player",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .title("ShenhePlayer")
    .inner_size(1280.0, 720.0)
    .min_inner_size(640.0, 380.0)
    .decorations(false)
    .background_color(tauri::window::Color(0, 0, 0, 255))
    .resizable(true)
    .center()
    .visible(false)
    .build()
    .map_err(|e| AppError::Player(format!("创建播放窗口失败: {e}")))?;

    // 窗口还没显示，这时候摆位置不会看到跳动
    {
        let saved = state.settings.read().player_window.clone();
        if saved.saved && saved.width > 0 && saved.height > 0 {
            let _ = window.set_size(tauri::PhysicalSize::new(saved.width, saved.height));
            let _ = window.set_position(tauri::PhysicalPosition::new(saved.x, saved.y));
            if saved.maximized {
                let _ = window.maximize();
            }
        }
    }

    let state_for_events = state.clone();
    let app_for_events = app.clone();
    window.on_window_event(move |event| match event {
        // 尺寸/位置一变，视频宿主和控制层都要跟上，否则会露出底层
        tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
            state_for_events.host.read().fit_to_parent();
            sync_overlay(&app_for_events);
        }
        // 用户直接关掉播放窗口 == 停止播放，进度照常回写
        tauri::WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            let app = app_for_events.clone();
            tauri::async_runtime::spawn(async move {
                let state = app.state::<Arc<AppState>>();
                if let Err(e) = stop_playback(app.clone(), state).await {
                    log::warn!("关闭播放窗口时停止播放失败: {e}");
                }
            });
        }
        _ => {}
    });

    Ok(window)
}

/// 控制层：透明、归属于播放窗口。owned 窗口永远显示在 owner 之上，
/// 并且跟着 owner 一起最小化，不会在任务栏里多出一个条目。
fn ensure_overlay_window(app: &AppHandle, player: &tauri::WebviewWindow) -> Result<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window("overlay") {
        return Ok(w);
    }

    tauri::WebviewWindowBuilder::new(app, "overlay", tauri::WebviewUrl::App("index.html".into()))
        .title("ShenhePlayer Controls")
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .skip_taskbar(true)
        .resizable(false)
        .visible(false)
        .owner(player)
        .map_err(|e| AppError::Player(format!("设置控制层归属失败: {e}")))?
        .build()
        .map_err(|e| AppError::Player(format!("创建控制层失败: {e}")))
}

/// 让控制层严格贴合播放窗口的客户区。播放窗口无边框，外框即客户区。
pub fn sync_overlay(app: &AppHandle) {
    let (Some(player), Some(overlay)) = (
        app.get_webview_window("player"),
        app.get_webview_window("overlay"),
    ) else {
        return;
    };

    if let (Ok(pos), Ok(size)) = (player.outer_position(), player.inner_size()) {
        let _ = overlay.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
        let _ = overlay.set_size(tauri::PhysicalSize::new(size.width, size.height));
    }
}

/// 视频宿主窗口只能在创建它的线程（主线程）里建，这里做一次跨线程调度
async fn ensure_host_window(
    app: &AppHandle,
    state: &Arc<AppState>,
    parent: &tauri::WebviewWindow,
) -> Result<VideoHost> {
    {
        let existing = *state.host.read();
        if existing.is_valid() {
            existing.fit_to_parent();
            return Ok(existing);
        }
    }

    let hwnd = parent
        .hwnd()
        .map_err(|e| AppError::Player(format!("获取窗口句柄失败: {e}")))?;

    let (tx, rx) = std::sync::mpsc::channel();
    let hwnd_raw = hwnd.0 as isize;
    app.run_on_main_thread(move || {
        let parent = windows::Win32::Foundation::HWND(hwnd_raw as *mut core::ffi::c_void);
        let _ = tx.send(VideoHost::create(parent));
    })
    .map_err(|e| AppError::Player(format!("主线程调度失败: {e}")))?;

    let host = rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .map_err(|_| AppError::Player("创建视频窗口超时".into()))?
        .map_err(AppError::Player)?;

    *state.host.write() = host;
    Ok(host)
}

// ------------------------------------------------------------------ 事件

fn handle_mpv_event(app: &AppHandle, event: MpvEvent) {
    match event {
        MpvEvent::State(snapshot) => {
            let _ = app.emit("player:state", snapshot);
        }
        MpvEvent::FileLoaded => {
            apply_default_tracks(app);
            let _ = app.emit("player:loaded", ());
        }
        MpvEvent::EndFile { reason } => {
            let _ = app.emit("player:endfile", reason);
        }
        MpvEvent::Shutdown => {
            let _ = app.emit("player:shutdown", ());
        }
    }
}


/// 挂外挂字幕，并把 Emby 选好的默认音轨/字幕切过去。
///
/// 放在 file-loaded 之后做的原因：loadfile 是异步的，命令发出去时轨道还没
/// 建立，这时候设 aid/sid 或者 sub-add 都会落空。
fn apply_default_tracks(app: &AppHandle) {
    let Some(target) = app.state::<Arc<AppState>>().target.read().clone() else {
        return;
    };

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<Arc<AppState>>();
        let guard = state.mpv.lock().await;
        let Some(session) = guard.as_ref() else {
            return;
        };

        // 外挂字幕：Emby 指定为默认的那条直接选中
        for (i, sub) in target.external_subtitles.iter().enumerate() {
            let select = target.default_external_sub == Some(i);
            if let Err(e) = session.add_subtitle(&sub.url, &sub.title, select) {
                log::warn!("挂载外挂字幕失败: {e}");
            }
        }

        // 转码流里轨道顺序和源文件对不上，索引换算不成立，交给 mpv 自己挑
        if !target.is_direct {
            return;
        }

        if let Some(aid) = target.mpv_aid {
            let _ = session.set_property("aid", json!(aid));
        }
        // 默认字幕是外挂的话，上面 sub-add 已经选中了，别再用 sid 覆盖
        if target.default_external_sub.is_none() {
            if let Some(sid) = target.mpv_sid {
                let _ = session.set_property("sid", json!(sid));
            }
        }
    });
}
