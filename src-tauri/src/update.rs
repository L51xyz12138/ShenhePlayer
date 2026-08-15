//! 检查更新：读 GitHub Releases，比对版本号。
//!
//! 没有用 tauri-plugin-updater —— 那套要签名密钥和自建更新清单，
//! 对一个开源小工具来说太重。这里自己实现：查版本 → 下载安装包 → 起安装程序。
//!
//! 版本号**不走 REST API**：api.github.com 未认证只有 60 次/小时，而且是按
//! 出口 IP 计的。用了 VPN 或公司网关的用户，配额经常已经被别人用光，
//! 实测直接吃 403。改成读 `/releases/latest` 这个网页地址的 302 跳转，
//! 从 Location 里取 tag，不受 API 限额约束。
//!
//! 发布说明仍然只能从 API 拿，所以那一步做成「能拿到就显示，拿不到就算了」，
//! 不影响「有没有新版本」这个核心判断。

use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use tauri::Emitter;
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
    /// Release 的 tag（带 v 前缀），下载安装包时要用
    pub tag: String,
}

#[derive(Deserialize, Default)]
struct GithubRelease {
    #[serde(default)]
    body: String,
    #[serde(default)]
    name: String,
}

fn releases_url() -> String {
    format!("https://github.com/{REPO}/releases")
}

#[tauri::command]
pub async fn check_update() -> Result<UpdateInfo> {
    let client = reqwest::Client::builder()
        .user_agent(format!("ShenhePlayer/{CURRENT}"))
        .timeout(Duration::from_secs(15))
        // 要自己读 Location，不能让 reqwest 跟着跳
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let resp = client
        .get(format!("https://github.com/{REPO}/releases/latest"))
        .send()
        .await?;

    let status = resp.status();

    // 一个 Release 都没发过：GitHub 直接 404
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateInfo {
            current: CURRENT.into(),
            latest: CURRENT.into(),
            available: false,
            no_release: true,
            notes: String::new(),
            url: releases_url(),
            tag: String::new(),
        });
    }

    if !status.is_redirection() {
        return Err(AppError::Server {
            status: status.as_u16(),
            body: "检查更新失败，请稍后再试".into(),
        });
    }

    let location = resp
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();

    let Some(tag) = tag_from_release_url(&location) else {
        return Err(AppError::Other("没能解析出最新版本号".into()));
    };

    let latest = tag.trim_start_matches(['v', 'V']).to_string();
    let available = is_newer(&latest, CURRENT);

    // 发布说明是锦上添花：API 被限流也不该让整个检查失败
    let notes = if available {
        fetch_notes(&client, &tag).await.unwrap_or_default()
    } else {
        String::new()
    };

    Ok(UpdateInfo {
        current: CURRENT.into(),
        latest,
        available,
        no_release: false,
        notes,
        url: if location.is_empty() { releases_url() } else { location },
        tag,
    })
}

/// 启动时的静默检查，有新版本就广播给前端挂角标。
/// 失败一律忽略——没网、被墙、限流都不该在启动时弹错误。
///
/// 不做节流：整个检查就是一次到 github.com 的 302，响应不到 1 KB。
/// 之前按「一天最多一次」限频，结果是发了新版本也要等到第二天才提示，
/// 完全违背用户对「启动时检查更新」的预期。
pub fn spawn_startup_check(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        // 别和启动时那一堆媒体库请求抢带宽
        tokio::time::sleep(Duration::from_secs(6)).await;

        match check_update().await {
            Ok(info) if info.available => {
                log::info!("检查到新版本 {}", info.latest);
                let _ = app.emit("update:available", info);
            }
            Ok(_) => log::info!("已是最新版本"),
            Err(e) => log::debug!("启动检查更新失败: {e}"),
        }
    });
}

// ---------------------------------------------------------------- 应用内更新

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub downloaded: u64,
    /// 服务器给的总长度，未知时为 0
    pub total: u64,
}

/// 下载新版本安装包到临时目录，返回文件路径。
///
/// 不接受前端传来的 URL：内部重新查一次最新版本，URL 完全由代码拼，
/// 避免这个命令变成「下载任意文件并执行」的入口。
#[tauri::command]
pub async fn download_update(app: tauri::AppHandle) -> Result<String> {
    use tokio::io::AsyncWriteExt;

    let info = check_update().await?;
    if !info.available {
        return Err(AppError::Other("当前已是最新版本".into()));
    }

    let file_name = format!("ShenhePlayer_{}_x64-setup.exe", info.latest);
    let url = format!(
        "https://github.com/{REPO}/releases/download/{}/{}",
        info.tag, file_name
    );

    let client = reqwest::Client::builder()
        .user_agent(format!("ShenhePlayer/{CURRENT}"))
        .connect_timeout(Duration::from_secs(15))
        // 不设总超时：安装包几 MB，网慢的时候别中途掐断
        .build()?;

    let mut resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        return Err(AppError::Server {
            status: resp.status().as_u16(),
            body: "下载安装包失败".into(),
        });
    }

    let total = resp.content_length().unwrap_or(0);

    let dir = std::env::temp_dir().join("ShenhePlayer-update");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(&file_name);

    let mut file = tokio::fs::File::create(&path).await?;
    let mut downloaded: u64 = 0;
    let mut last_report = std::time::Instant::now() - Duration::from_secs(1);

    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;

        // 进度按 10Hz 上报就够了，不然光发事件就占掉不少 CPU
        if last_report.elapsed() >= Duration::from_millis(100) {
            last_report = std::time::Instant::now();
            let _ = app.emit("update:progress", DownloadProgress { downloaded, total });
        }
    }
    file.flush().await?;
    drop(file);

    let _ = app.emit("update:progress", DownloadProgress { downloaded, total });

    // 完整性检查。TLS 已经保证传输不被篡改，这里要防的是「下到一半断了」
    // 却把半个安装包运行起来。
    if total > 0 && downloaded != total {
        let _ = std::fs::remove_file(&path);
        return Err(AppError::Other(format!(
            "安装包下载不完整（{downloaded}/{total} 字节），请重试"
        )));
    }
    verify_installer(&path)?;

    Ok(path.to_string_lossy().to_string())
}

/// 起码得是个能运行的 Windows 可执行文件
fn verify_installer(path: &std::path::Path) -> Result<()> {
    let meta = std::fs::metadata(path)?;
    if meta.len() < 512 * 1024 {
        let _ = std::fs::remove_file(path);
        return Err(AppError::Other("安装包异常（体积过小），请重试".into()));
    }

    let head = std::fs::read(path)?;
    if !head.starts_with(b"MZ") {
        let _ = std::fs::remove_file(path);
        return Err(AppError::Other("下载到的不是有效的安装程序，请重试".into()));
    }
    Ok(())
}

/// 启动安装程序并退出本进程 —— 不退出的话安装程序覆盖不了正在运行的文件
#[tauri::command]
pub fn install_update(app: tauri::AppHandle, path: String) -> Result<()> {
    let path = std::path::PathBuf::from(&path);

    // 只允许运行我们自己刚下载到临时目录的那个文件
    let expected_dir = std::env::temp_dir().join("ShenhePlayer-update");
    if path.parent() != Some(expected_dir.as_path()) {
        return Err(AppError::Other("非法的安装包路径".into()));
    }
    verify_installer(&path)?;

    // /S 静默安装、/R 装完自动重启应用。
    // 应用内更新走完整安装向导（欢迎页→下一步→…）没有意义，用户点的是
    // 「立即安装」，预期就是自己更新完再打开。NSIS 会忽略不认识的开关，
    // 所以即使模板不支持 /R，最差也只是装完不自动启动。
    std::process::Command::new(&path)
        .args(["/S", "/R"])
        .spawn()
        .map_err(|e| AppError::Other(format!("启动安装程序失败: {e}")))?;

    // 立刻退出：晚一点安装程序就会检测到本程序还在跑，弹「请先关闭」
    app.exit(0);
    Ok(())
}

async fn fetch_notes(client: &reqwest::Client, tag: &str) -> Option<String> {
    let resp = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/tags/{tag}"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let release: GithubRelease = resp.json().await.ok()?;
    let notes = if release.body.trim().is_empty() {
        release.name
    } else {
        release.body
    };
    Some(notes)
}

/// 用系统默认浏览器打开发布页
#[tauri::command]
pub fn open_release_page(url: String) -> Result<()> {
    // 只允许打开本项目的 GitHub 地址，避免这个命令变成任意 URL 启动器
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

/// 从 .../releases/tag/v1.2.3 里取出 v1.2.3
fn tag_from_release_url(url: &str) -> Option<String> {
    let tag = url.trim_end_matches('/').rsplit('/').next()?;
    if tag.is_empty() || !tag.contains(|c: char| c.is_ascii_digit()) {
        return None;
    }
    Some(tag.to_string())
}

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

    #[test]
    fn extracts_tag_from_redirect_location() {
        assert_eq!(
            tag_from_release_url("https://github.com/o/r/releases/tag/v0.1.0").as_deref(),
            Some("v0.1.0")
        );
        assert_eq!(
            tag_from_release_url("https://github.com/o/r/releases/tag/1.2.3/").as_deref(),
            Some("1.2.3")
        );
        // 没有版本号的地址（比如仓库根本没有 Release）不应该被误认成 tag
        assert_eq!(tag_from_release_url("https://github.com/o/r/releases"), None);
        assert_eq!(tag_from_release_url(""), None);
    }
}
