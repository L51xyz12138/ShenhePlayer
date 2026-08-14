//! 外置播放器：把直连 URL 交给系统里已装的播放器。
//!
//! 适用场景：用户有调教好的 mpv/PotPlayer 配置（SVP 补帧、madVR、
//! 自定义字幕样式），或者想在播放时继续用本程序浏览媒体库。

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedPlayer {
    /// mpv / potplayer / vlc / mpc-hc / mpc-be / custom
    pub kind: String,
    pub name: String,
    pub path: String,
}

/// 扫描常见安装位置与 PATH，找出可用的外置播放器
pub fn detect_players() -> Vec<DetectedPlayer> {
    let mut found: Vec<DetectedPlayer> = Vec::new();
    let mut push = |kind: &str, name: &str, path: PathBuf| {
        let p = path.to_string_lossy().to_string();
        if path.is_file() && !found.iter().any(|f| f.path.eq_ignore_ascii_case(&p)) {
            found.push(DetectedPlayer {
                kind: kind.into(),
                name: name.into(),
                path: p,
            });
        }
    };

    let pf = env_dir("ProgramFiles");
    let pf86 = env_dir("ProgramFiles(x86)");
    let local = env_dir("LOCALAPPDATA");

    // 逐段 join，避免拼出 "C:\Program Files\DAUM/PotPlayer/..." 这种混合分隔符
    let under = |base: &PathBuf, parts: &[&str]| -> PathBuf {
        parts.iter().fold(base.clone(), |acc, p| acc.join(p))
    };

    // mpv：优先 PATH，其次常见目录
    for p in which("mpv.exe") {
        push("mpv", "mpv", p);
    }

    for base in [&pf, &pf86, &local].into_iter().flatten() {
        push("mpv", "mpv", under(base, &["mpv", "mpv.exe"]));
        push("mpv", "mpv", under(base, &["mpv-player", "mpv.exe"]));
    }

    for base in [&pf, &pf86].into_iter().flatten() {
        // PotPlayer
        push("potplayer", "PotPlayer", under(base, &["DAUM", "PotPlayer", "PotPlayerMini64.exe"]));
        push("potplayer", "PotPlayer", under(base, &["DAUM", "PotPlayer", "PotPlayerMini.exe"]));
        // VLC
        push("vlc", "VLC", under(base, &["VideoLAN", "VLC", "vlc.exe"]));
        // MPC-HC / MPC-BE
        push("mpc-hc", "MPC-HC", under(base, &["MPC-HC", "mpc-hc64.exe"]));
        push("mpc-hc", "MPC-HC", under(base, &["MPC-HC", "mpc-hc.exe"]));
        push("mpc-hc", "MPC-HC", under(base, &["MPC-HC64", "mpc-hc64.exe"]));
        push("mpc-be", "MPC-BE", under(base, &["MPC-BE x64", "mpc-be64.exe"]));
        push("mpc-be", "MPC-BE", under(base, &["MPC-BE", "mpc-be.exe"]));
    }

    found
}

/// 启动外置播放器。start 为起播秒数。
pub fn launch(
    exe: &str,
    kind: &str,
    url: &str,
    title: &str,
    start: f64,
    custom_args: &str,
    subtitles: &[String],
) -> Result<()> {
    let path = Path::new(exe);
    if !path.is_file() {
        return Err(AppError::Player(format!("外置播放器不存在: {exe}")));
    }

    let args = build_args(kind, url, title, start, custom_args, subtitles);

    std::process::Command::new(path)
        .args(&args)
        .current_dir(path.parent().unwrap_or(Path::new(".")))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| AppError::Player(format!("启动外置播放器失败: {e}")))?;

    Ok(())
}

fn build_args(
    kind: &str,
    url: &str,
    title: &str,
    start: f64,
    custom_args: &str,
    subtitles: &[String],
) -> Vec<String> {
    let start_i = start.max(0.0) as u64;

    if !custom_args.trim().is_empty() {
        return custom_args
            .split_whitespace()
            .map(|a| {
                a.replace("{url}", url)
                    .replace("{title}", title)
                    .replace("{start}", &start_i.to_string())
                    .replace("{start_ms}", &(start_i * 1000).to_string())
                    .replace("{start_hms}", &hms(start_i))
            })
            .collect();
    }

    match kind {
        "mpv" => {
            let mut a = vec![
                format!("--force-media-title={title}"),
                "--user-agent=ShenhePlayer/0.1.0".into(),
            ];
            if start_i > 0 {
                a.push(format!("--start={start_i}"));
            }
            for s in subtitles {
                a.push(format!("--sub-file={s}"));
            }
            a.push(url.to_string());
            a
        }
        "potplayer" => {
            let mut a = vec![url.to_string(), format!("/title={title}")];
            if start_i > 0 {
                a.push(format!("/seek={}", hms(start_i)));
            }
            a
        }
        "vlc" => {
            let mut a = vec![url.to_string(), format!("--meta-title={title}")];
            if start_i > 0 {
                a.push(format!("--start-time={start_i}"));
            }
            for s in subtitles {
                a.push(format!("--sub-file={s}"));
            }
            a.push("--no-video-title-show".into());
            a
        }
        "mpc-hc" | "mpc-be" => {
            let mut a = vec![url.to_string()];
            if start_i > 0 {
                a.push("/start".into());
                a.push((start_i * 1000).to_string());
            }
            a
        }
        // 未知播放器：只传 URL，最大兼容
        _ => vec![url.to_string()],
    }
}

fn hms(total: u64) -> String {
    format!("{:02}:{:02}:{:02}", total / 3600, (total % 3600) / 60, total % 60)
}

fn env_dir(key: &str) -> Option<PathBuf> {
    std::env::var(key).ok().map(PathBuf::from).filter(|p| p.exists())
}

/// 在 PATH 里找可执行文件
fn which(exe: &str) -> Vec<PathBuf> {
    std::env::var("PATH")
        .into_iter()
        .flat_map(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(exe))
                .filter(|p| p.is_file())
                .collect::<Vec<_>>()
        })
        .collect()
}

use std::os::windows::process::CommandExt as _;
