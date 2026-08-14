//! 检查更新：读 GitHub Releases，比对版本号。
//!
//! 没有用 tauri-plugin-updater —— 那套要签名密钥和自建更新清单，
//! 对一个开源小工具来说太重。这里只做「有没有新版本」的判断，
//! 真正的下载安装交给用户点开 Release 页面。

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const REPO: &str = "L51xyz12138/ShenhePlayer";
const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    /// 远端版本比当前新
    pub available: bool,
    /// 仓库还没有发布任何 Release
    pub no_release: bool,
    pub notes: String,
    pub url: String,
    pub published_at: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    #[serde(default)]
    tag_name: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    published_at: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

#[tauri::command]
pub async fn check_update() -> Result<UpdateInfo> {
    let none = UpdateInfo {
        current: CURRENT.into(),
        latest: CURRENT.into(),
        available: false,
        no_release: true,
        notes: String::new(),
        url: format!("https://github.com/{REPO}/releases"),
        published_at: String::new(),
    };

    let client = reqwest::Client::builder()
        .user_agent(format!("ShenhePlayer/{CURRENT}"))
        .timeout(Duration::from_secs(15))
        .build()?;

    let resp = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    // 还没发过 Release，不算错误
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(none);
    }
    if !resp.status().is_success() {
        return Err(AppError::Server {
            status: resp.status().as_u16(),
            body: "检查更新失败，请稍后再试".into(),
        });
    }

    let release: GithubRelease = resp.json().await?;
    if release.draft || release.prerelease || release.tag_name.is_empty() {
        return Ok(none);
    }

    let latest = release.tag_name.trim_start_matches(['v', 'V']).to_string();

    Ok(UpdateInfo {
        available: is_newer(&latest, CURRENT),
        current: CURRENT.into(),
        latest: latest.clone(),
        no_release: false,
        notes: if release.body.trim().is_empty() {
            release.name
        } else {
            release.body
        },
        url: if release.html_url.is_empty() {
            format!("https://github.com/{REPO}/releases")
        } else {
            release.html_url
        },
        published_at: release.published_at,
    })
}

/// 用系统默认浏览器打开发布页
#[tauri::command]
pub fn open_release_page(url: String) -> Result<()> {
    // 只允许打开本项目的 GitHub 地址，避免这个命令被当成任意 URL 启动器
    let allowed = format!("https://github.com/{REPO}");
    if !url.starts_with(&allowed) {
        return Err(AppError::Other("不允许打开该地址".into()));
    }

    std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .spawn()
        .map_err(|e| AppError::Other(format!("打开浏览器失败: {e}")))?;
    Ok(())
}

use std::os::windows::process::CommandExt as _;

/// 语义化版本比较。解析失败时保守返回 false，宁可不提示也不误报。
fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

fn parse_version(v: &str) -> Option<(u32, u32, u32)> {
    // 去掉 1.2.3-beta.1 这种预发布后缀和 +build 元数据
    let core = v.trim().split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.trim().parse().ok()?;
    let minor = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
    let patch = parts.next().unwrap_or("0").trim().parse().unwrap_or(0);
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }

    #[test]
    fn tolerates_loose_version_strings() {
        assert_eq!(parse_version("v1.2.3"), None, "调用方需要先去掉 v 前缀");
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_version("2"), Some((2, 0, 0)));
        assert_eq!(parse_version("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(parse_version("not-a-version"), None);
    }

    #[test]
    fn unparsable_versions_never_prompt() {
        assert!(!is_newer("latest", "0.1.0"));
        assert!(!is_newer("", "0.1.0"));
    }
}
