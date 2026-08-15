//! 配置持久化：%APPDATA%\ShenhePlayer\settings.json

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub url: String,
    pub username: String,
    pub user_id: String,
    pub token: String,
    pub allow_invalid_certs: bool,
    /// 上次登录时间（用于多服务器排序）
    pub last_used: i64,
}

impl Default for ServerProfile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            url: String::new(),
            username: String::new(),
            user_id: String::new(),
            token: String::new(),
            allow_invalid_certs: false,
            last_used: 0,
        }
    }
}

/// 画质档位：低端机选 Performance，独显机器选 Quality
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityPreset {
    Performance,
    Balanced,
    Quality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlayerSettings {
    /// "internal" | "external"
    pub mode: String,
    /// mpv.exe 路径，留空则自动探测 / 用内置
    pub mpv_path: String,
    /// 外置播放器可执行文件路径
    pub external_path: String,
    /// 外置播放器类型：mpv / potplayer / vlc / mpc-hc / custom
    pub external_kind: String,
    /// 自定义参数模板，{url} {title} {start} 会被替换
    pub external_args: String,
    pub quality: QualityPreset,
    /// auto / d3d11va / d3d11va-copy / nvdec / no
    pub hwdec: String,
    /// 是否使用 gpu-next（画质更好，老显卡可能不稳）
    pub gpu_next: bool,
    /// 音量 0-130
    pub volume: i32,
    /// 字幕字号（mpv sub-font-size）
    pub sub_font_size: i32,
    /// 跳过片头秒数
    pub skip_intro_seconds: i32,
    /// 播放结束自动播下一集
    pub auto_next: bool,
    /// 播放时自动全屏
    pub fullscreen_on_play: bool,
    /// 最大码率限制（bps），0 = 不限
    pub max_bitrate: i64,
}

impl Default for PlayerSettings {
    fn default() -> Self {
        Self {
            mode: "internal".into(),
            mpv_path: String::new(),
            external_path: String::new(),
            external_kind: "mpv".into(),
            external_args: String::new(),
            quality: QualityPreset::Balanced,
            hwdec: "auto-safe".into(),
            gpu_next: true,
            volume: 100,
            sub_font_size: 46,
            skip_intro_seconds: 85,
            auto_next: true,
            fullscreen_on_play: false,
            max_bitrate: 0,
        }
    }
}

/// 界面主题。system = 跟随 Windows 的浅色/深色设置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiSettings {
    pub theme: Theme,
    /// 主题强调色
    pub accent: String,
    /// 海报网格每行张数（0 = 自适应）
    pub grid_size: u32,
    /// 是否显示背景大图
    pub show_backdrop: bool,
    /// 降低动画（低端机）
    pub reduce_motion: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            accent: "#0a84ff".into(),
            grid_size: 0,
            show_backdrop: true,
            reduce_motion: false,
        }
    }
}

/// 记住窗口摆在哪、多大。不记的话每次开都是居中的默认尺寸，
/// 双屏或者习惯最大化的人每次都要重新摆一遍。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
    /// 没存过时不要去套用全 0 的尺寸
    pub saved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub device_id: String,
    pub servers: Vec<ServerProfile>,
    pub active_server: String,
    pub player: PlayerSettings,
    pub ui: UiSettings,
    pub window: WindowState,
    pub player_window: WindowState,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            device_id: uuid::Uuid::new_v4().to_string(),
            servers: Vec::new(),
            active_server: String::new(),
            player: PlayerSettings::default(),
            ui: UiSettings::default(),
            window: WindowState::default(),
            player_window: WindowState::default(),
        }
    }
}

impl Settings {
    pub fn active(&self) -> Option<&ServerProfile> {
        self.servers.iter().find(|s| s.id == self.active_server)
    }

    pub fn upsert_server(&mut self, profile: ServerProfile) {
        match self.servers.iter_mut().find(|s| s.id == profile.id) {
            Some(existing) => *existing = profile,
            None => self.servers.push(profile),
        }
    }

    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                log::warn!("配置文件损坏，使用默认配置: {e}");
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self)?;
        // 先写临时文件再重命名，避免写入中断导致配置丢失
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

pub fn config_dir() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("ShenhePlayer")
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}
