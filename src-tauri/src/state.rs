use crate::config::{settings_path, Settings};
use crate::emby::EmbyClient;
use crate::error::{AppError, Result};
use crate::player::MpvSession;
use crate::win::VideoHost;
use parking_lot::RwLock;
use std::sync::Arc;

/// 当前正在播放的条目，用于向 Emby 上报进度
#[derive(Debug, Clone, Default)]
pub struct ActivePlayback {
    pub item_id: String,
    pub media_source_id: String,
    pub play_session_id: String,
    pub is_direct: bool,
    /// 上次上报的位置，用于节流
    pub last_reported: f64,
    pub external: bool,
}

pub struct AppState {
    pub settings: RwLock<Settings>,
    pub client: RwLock<Option<EmbyClient>>,
    pub mpv: tokio::sync::Mutex<Option<MpvSession>>,
    pub host: RwLock<VideoHost>,
    pub playback: RwLock<Option<ActivePlayback>>,
    /// prepare_playback 解析出来的播放目标。控制层窗口是独立的 JS 环境，
    /// 从这里取比在两个窗口之间传大 JSON 更稳。
    pub target: RwLock<Option<crate::emby::models::PlaybackTarget>>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        let settings = Settings::load(&settings_path());
        Arc::new(Self {
            settings: RwLock::new(settings),
            client: RwLock::new(None),
            mpv: tokio::sync::Mutex::new(None),
            host: RwLock::new(VideoHost::default()),
            playback: RwLock::new(None),
            target: RwLock::new(None),
        })
    }

    /// 取一份已登录的客户端副本（EmbyClient 内部是 Arc，克隆很轻）
    pub fn client(&self) -> Result<EmbyClient> {
        self.client
            .read()
            .clone()
            .ok_or(AppError::NotAuthenticated)
    }

    pub fn save_settings(&self) -> Result<()> {
        self.settings.read().save(&settings_path())
    }
}
