pub mod external;
pub mod mpv;

pub use external::{detect_players, DetectedPlayer};
pub use mpv::{MpvEvent, MpvSession, PlayerSnapshot};

use std::path::PathBuf;

/// 找一个能用的 mpv.exe：用户指定 > 程序自带 > 系统 PATH / 常见目录
pub fn resolve_mpv_path(configured: &str) -> Option<String> {
    if !configured.trim().is_empty() && PathBuf::from(configured).is_file() {
        return Some(configured.to_string());
    }

    // 程序目录下的 mpv（安装包会带一份）
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for candidate in ["mpv/mpv.exe", "mpv.exe", "resources/mpv/mpv.exe"] {
                let p = dir.join(candidate);
                if p.is_file() {
                    return Some(p.to_string_lossy().to_string());
                }
            }
        }
    }

    detect_players()
        .into_iter()
        .find(|p| p.kind == "mpv")
        .map(|p| p.path)
}
