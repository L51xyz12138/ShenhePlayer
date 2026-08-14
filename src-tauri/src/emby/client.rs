//! Emby REST API 客户端。
//!
//! 所有网络请求都在 Rust 侧完成：前端拿不到 token，也不受 WebView 的
//! CORS / 混合内容限制，同时 JSON 解析比 JS 快得多。

use super::models::*;
use crate::error::{AppError, Result};
use serde::Serialize;
use serde_json::json;
use std::time::Duration;

const CLIENT_NAME: &str = "ShenhePlayer";
const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 请求 Items 时需要的字段，一次拿全避免二次请求
pub const ITEM_FIELDS: &str = "BasicSyncInfo,Overview,Genres,MediaSources,MediaStreams,ProductionYear,CommunityRating,OfficialRating,RunTimeTicks,Path,Taglines,Studios,People,ProviderIds,DateCreated,Status,ParentId,PremiereDate,EndDate,ChildCount,RecursiveItemCount";

/// 列表场景用的精简字段，减少服务器与网络开销
pub const LIST_FIELDS: &str = "BasicSyncInfo,ProductionYear,CommunityRating,RunTimeTicks,PrimaryImageAspectRatio,Overview";

#[derive(Clone)]
pub struct EmbyClient {
    http: reqwest::Client,
    /// 已归一化：无尾部斜杠、无 /emby 后缀
    base_url: String,
    device_id: String,
    device_name: String,
    token: Option<String>,
    user_id: Option<String>,
}

impl EmbyClient {
    pub fn new(base_url: &str, device_id: String, allow_invalid_certs: bool) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(format!("{CLIENT_NAME}/{CLIENT_VERSION}"))
            .danger_accept_invalid_certs(allow_invalid_certs)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .pool_idle_timeout(Duration::from_secs(90))
            .build()?;

        let device_name = hostname().unwrap_or_else(|| "Windows PC".into());

        Ok(Self {
            http,
            base_url: normalize_base_url(base_url),
            device_id,
            device_name,
            token: None,
            user_id: None,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn user_id(&self) -> Result<&str> {
        self.user_id.as_deref().ok_or(AppError::NotAuthenticated)
    }

    pub fn set_session(&mut self, token: String, user_id: String) {
        self.token = Some(token);
        self.user_id = Some(user_id);
    }

    fn url(&self, path: &str) -> String {
        format!("{}/emby{}", self.base_url, path)
    }

    /// Emby 的鉴权头。未登录时不带 Token 字段。
    fn auth_header(&self) -> String {
        let mut v = format!(
            "MediaBrowser Client=\"{}\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\"",
            CLIENT_NAME, self.device_name, self.device_id, CLIENT_VERSION
        );
        if let Some(t) = &self.token {
            v.push_str(&format!(", Token=\"{t}\""));
        }
        v
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let mut req = self
            .http
            .request(method, self.url(path))
            .header("X-Emby-Authorization", self.auth_header())
            .header("Accept", "application/json");
        if let Some(t) = &self.token {
            req = req.header("X-Emby-Token", t);
        }
        req
    }

    async fn send_json<T: serde::de::DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T> {
        let resp = req.send().await?;
        let status = resp.status();
        let body = resp.text().await?;

        if !status.is_success() {
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(AppError::InvalidCredentials);
            }
            return Err(AppError::Server {
                status: status.as_u16(),
                body: body.chars().take(400).collect(),
            });
        }

        // 部分接口成功时返回空 body
        if body.trim().is_empty() {
            return Ok(serde_json::from_str("null").unwrap_or_else(|_| {
                serde_json::from_value(serde_json::Value::Object(Default::default())).unwrap()
            }));
        }

        Ok(serde_json::from_str(&body)?)
    }

    async fn send_ok(&self, req: reqwest::RequestBuilder) -> Result<()> {
        let resp = req.send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Server {
                status: status.as_u16(),
                body: body.chars().take(400).collect(),
            });
        }
        Ok(())
    }

    // ---------------------------------------------------------------- 系统 / 登录

    /// 探测服务器是否可用（无需登录）
    pub async fn public_system_info(&self) -> Result<SystemInfo> {
        self.send_json(self.request(reqwest::Method::GET, "/System/Info/Public"))
            .await
    }

    pub async fn authenticate(&mut self, username: &str, password: &str) -> Result<AuthResult> {
        let req = self
            .request(reqwest::Method::POST, "/Users/AuthenticateByName")
            .json(&json!({ "Username": username, "Pw": password }));

        let auth: AuthResult = self.send_json(req).await?;
        if auth.access_token.is_empty() {
            return Err(AppError::InvalidCredentials);
        }
        self.set_session(auth.access_token.clone(), auth.user.id.clone());
        Ok(auth)
    }

    /// 用已保存的 token 校验会话是否仍然有效
    pub async fn validate_session(&self) -> Result<UserDto> {
        let uid = self.user_id()?.to_string();
        self.send_json(self.request(reqwest::Method::GET, &format!("/Users/{uid}")))
            .await
    }

    pub async fn logout(&self) -> Result<()> {
        self.send_ok(self.request(reqwest::Method::POST, "/Sessions/Logout"))
            .await
    }

    // ---------------------------------------------------------------- 媒体库

    pub async fn views(&self) -> Result<QueryResult<BaseItem>> {
        let uid = self.user_id()?;
        self.send_json(self.request(reqwest::Method::GET, &format!("/Users/{uid}/Views")))
            .await
    }

    pub async fn items(&self, query: &ItemsQuery) -> Result<QueryResult<BaseItem>> {
        let uid = self.user_id()?;
        let req = self
            .request(reqwest::Method::GET, &format!("/Users/{uid}/Items"))
            .query(query);
        self.send_json(req).await
    }

    pub async fn item(&self, item_id: &str) -> Result<BaseItem> {
        let uid = self.user_id()?;
        let req = self
            .request(reqwest::Method::GET, &format!("/Users/{uid}/Items/{item_id}"))
            .query(&[("Fields", ITEM_FIELDS)]);
        self.send_json(req).await
    }

    /// 继续观看
    pub async fn resume(&self, limit: u32) -> Result<QueryResult<BaseItem>> {
        let uid = self.user_id()?;
        let req = self
            .request(reqwest::Method::GET, &format!("/Users/{uid}/Items/Resume"))
            .query(&[
                ("Limit", limit.to_string().as_str()),
                ("Recursive", "true"),
                ("MediaTypes", "Video"),
                ("Fields", LIST_FIELDS),
                ("EnableImageTypes", "Primary,Backdrop,Thumb"),
            ]);
        self.send_json(req).await
    }

    /// 下一集
    pub async fn next_up(&self, limit: u32) -> Result<QueryResult<BaseItem>> {
        let uid = self.user_id()?;
        let req = self
            .request(reqwest::Method::GET, "/Shows/NextUp")
            .query(&[
                ("UserId", uid),
                ("Limit", limit.to_string().as_str()),
                ("Fields", LIST_FIELDS),
                ("EnableImageTypes", "Primary,Backdrop,Thumb"),
            ]);
        self.send_json(req).await
    }

    /// 最新添加（Latest 接口直接返回数组而非 QueryResult）
    pub async fn latest(&self, parent_id: Option<&str>, limit: u32) -> Result<Vec<BaseItem>> {
        let uid = self.user_id()?;
        let limit_s = limit.to_string();
        let mut q: Vec<(&str, &str)> = vec![
            ("Limit", limit_s.as_str()),
            ("Fields", LIST_FIELDS),
            ("EnableImageTypes", "Primary,Backdrop,Thumb"),
            ("IsPlayed", "false"),
            ("GroupItems", "true"),
        ];
        if let Some(pid) = parent_id {
            q.push(("ParentId", pid));
        }
        let req = self
            .request(reqwest::Method::GET, &format!("/Users/{uid}/Items/Latest"))
            .query(&q);
        self.send_json(req).await
    }

    pub async fn seasons(&self, series_id: &str) -> Result<QueryResult<BaseItem>> {
        let uid = self.user_id()?;
        let req = self
            .request(reqwest::Method::GET, &format!("/Shows/{series_id}/Seasons"))
            .query(&[("UserId", uid), ("Fields", LIST_FIELDS)]);
        self.send_json(req).await
    }

    pub async fn episodes(&self, series_id: &str, season_id: Option<&str>) -> Result<QueryResult<BaseItem>> {
        let uid = self.user_id()?;
        let mut q: Vec<(&str, &str)> = vec![("UserId", uid), ("Fields", ITEM_FIELDS)];
        if let Some(sid) = season_id {
            q.push(("SeasonId", sid));
        }
        let req = self
            .request(reqwest::Method::GET, &format!("/Shows/{series_id}/Episodes"))
            .query(&q);
        self.send_json(req).await
    }

    pub async fn similar(&self, item_id: &str, limit: u32) -> Result<QueryResult<BaseItem>> {
        let uid = self.user_id()?;
        let req = self
            .request(reqwest::Method::GET, &format!("/Items/{item_id}/Similar"))
            .query(&[
                ("UserId", uid),
                ("Limit", limit.to_string().as_str()),
                ("Fields", LIST_FIELDS),
            ]);
        self.send_json(req).await
    }

    // ---------------------------------------------------------------- 用户数据

    pub async fn set_favorite(&self, item_id: &str, favorite: bool) -> Result<()> {
        let uid = self.user_id()?;
        let method = if favorite {
            reqwest::Method::POST
        } else {
            reqwest::Method::DELETE
        };
        self.send_ok(self.request(method, &format!("/Users/{uid}/FavoriteItems/{item_id}")))
            .await
    }

    pub async fn set_played(&self, item_id: &str, played: bool) -> Result<()> {
        let uid = self.user_id()?;
        let method = if played {
            reqwest::Method::POST
        } else {
            reqwest::Method::DELETE
        };
        self.send_ok(self.request(method, &format!("/Users/{uid}/PlayedItems/{item_id}")))
            .await
    }

    // ---------------------------------------------------------------- 播放

    /// 询问服务器该用什么方式播放，并拿到 PlaySessionId。
    /// DeviceProfile 声明「什么都能直接播」，因为内置播放器是 mpv。
    pub async fn playback_info(
        &self,
        item_id: &str,
        media_source_id: Option<&str>,
        start_ticks: i64,
        max_bitrate: Option<i64>,
    ) -> Result<PlaybackInfoResponse> {
        let uid = self.user_id()?;
        let mut body = json!({
            "UserId": uid,
            "StartTimeTicks": start_ticks,
            "IsPlayback": true,
            "AutoOpenLiveStream": true,
            "MaxStreamingBitrate": max_bitrate.unwrap_or(2_000_000_000i64),
            "DeviceProfile": direct_play_profile(max_bitrate.unwrap_or(2_000_000_000i64)),
        });
        if let Some(msid) = media_source_id {
            body["MediaSourceId"] = json!(msid);
        }

        let req = self
            .request(reqwest::Method::POST, &format!("/Items/{item_id}/PlaybackInfo"))
            .query(&[("UserId", uid)])
            .json(&body);
        self.send_json(req).await
    }

    pub async fn report_playing(&self, p: &ProgressReport) -> Result<()> {
        self.send_ok(
            self.request(reqwest::Method::POST, "/Sessions/Playing")
                .json(p),
        )
        .await
    }

    pub async fn report_progress(&self, p: &ProgressReport) -> Result<()> {
        self.send_ok(
            self.request(reqwest::Method::POST, "/Sessions/Playing/Progress")
                .json(p),
        )
        .await
    }

    pub async fn report_stopped(&self, p: &ProgressReport) -> Result<()> {
        self.send_ok(
            self.request(reqwest::Method::POST, "/Sessions/Playing/Stopped")
                .json(p),
        )
        .await
    }

    // ---------------------------------------------------------------- URL 构造

    /// 图片地址。前端 <img> 直接用，走 WebView 自带的图片解码与缓存。
    pub fn image_url(
        &self,
        item_id: &str,
        image_type: &str,
        tag: Option<&str>,
        max_height: Option<u32>,
        max_width: Option<u32>,
    ) -> String {
        let mut url = format!(
            "{}/emby/Items/{}/Images/{}",
            self.base_url, item_id, image_type
        );
        let mut params: Vec<String> = vec!["quality=90".into()];
        if let Some(t) = tag {
            params.push(format!("tag={}", urlencoding::encode(t)));
        }
        if let Some(h) = max_height {
            params.push(format!("maxHeight={h}"));
        }
        if let Some(w) = max_width {
            params.push(format!("maxWidth={w}"));
        }
        if let Some(t) = &self.token {
            params.push(format!("api_key={t}"));
        }
        url.push('?');
        url.push_str(&params.join("&"));
        url
    }

    /// 原始文件直连地址：服务器不转码，画质无损，mpv / 外置播放器首选。
    pub fn direct_stream_url(
        &self,
        item_id: &str,
        media_source_id: &str,
        container: Option<&str>,
        play_session_id: &str,
    ) -> String {
        let ext = container.unwrap_or("mkv");
        format!(
            "{}/emby/Videos/{}/stream.{}?static=true&MediaSourceId={}&PlaySessionId={}&DeviceId={}&api_key={}",
            self.base_url,
            item_id,
            ext,
            media_source_id,
            play_session_id,
            self.device_id,
            self.token.as_deref().unwrap_or(""),
        )
    }

    /// 外挂字幕下载地址
    pub fn subtitle_url(
        &self,
        item_id: &str,
        media_source_id: &str,
        stream_index: i32,
        codec: &str,
    ) -> String {
        format!(
            "{}/emby/Videos/{}/{}/Subtitles/{}/Stream.{}?api_key={}",
            self.base_url,
            item_id,
            media_source_id,
            stream_index,
            codec,
            self.token.as_deref().unwrap_or(""),
        )
    }

    /// 把服务器返回的相对地址补全成绝对地址
    pub fn absolute_url(&self, path: &str) -> String {
        if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProgressReport {
    pub item_id: String,
    pub media_source_id: String,
    pub play_session_id: String,
    pub position_ticks: i64,
    pub is_paused: bool,
    pub is_muted: bool,
    pub can_seek: bool,
    pub play_method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_level: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_stream_index: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle_stream_index: Option<i32>,
}

/// Items 查询参数。所有字段 None 时不会出现在 query string 里。
#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct ItemsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_item_types: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_item_types: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_index: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_term: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genres: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub years: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_played: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_image_types: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_type_limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_starts_with: Option<String>,
}

/// 归一化服务器地址：补协议、去尾斜杠、去掉用户手填的 /emby 后缀
fn normalize_base_url(input: &str) -> String {
    let mut s = input.trim().to_string();
    if s.is_empty() {
        return s;
    }
    if !s.starts_with("http://") && !s.starts_with("https://") {
        s = format!("http://{s}");
    }
    while s.ends_with('/') {
        s.pop();
    }
    for suffix in ["/emby", "/web/index.html", "/web"] {
        if s.to_lowercase().ends_with(suffix) {
            s.truncate(s.len() - suffix.len());
        }
    }
    while s.ends_with('/') {
        s.pop();
    }
    s
}

fn hostname() -> Option<String> {
    std::env::var("COMPUTERNAME")
        .ok()
        .filter(|s| !s.is_empty())
}

/// 声明「几乎什么都能直接播」的设备配置 —— 内置播放器是 mpv，
/// 外置播放器同样是全能选手，所以尽量让服务器直出原始文件。
fn direct_play_profile(max_bitrate: i64) -> serde_json::Value {
    const VIDEO_CONTAINERS: &str =
        "mkv,mp4,m4v,mov,avi,ts,m2ts,mts,webm,flv,wmv,asf,mpg,mpeg,vob,iso,ogv,3gp,rmvb,rm,divx,f4v,mxf";
    const AUDIO_CONTAINERS: &str =
        "mp3,aac,flac,alac,m4a,ogg,oga,opus,wav,wma,ape,dsf,dff,mka,ac3,eac3,dts,truehd";

    json!({
        "Name": "ShenhePlayer",
        "MaxStreamingBitrate": max_bitrate,
        "MaxStaticBitrate": max_bitrate,
        "MusicStreamingTranscodingBitrate": 1_920_000,
        "DirectPlayProfiles": [
            { "Container": VIDEO_CONTAINERS, "Type": "Video" },
            { "Container": AUDIO_CONTAINERS, "Type": "Audio" }
        ],
        "TranscodingProfiles": [
            {
                "Container": "ts",
                "Type": "Video",
                "AudioCodec": "aac,mp3,ac3",
                "VideoCodec": "h264",
                "Context": "Streaming",
                "Protocol": "hls",
                "MaxAudioChannels": "6",
                "MinSegments": 1,
                "BreakOnNonKeyFrames": true
            },
            {
                "Container": "mp3",
                "Type": "Audio",
                "AudioCodec": "mp3",
                "Context": "Streaming",
                "Protocol": "http"
            }
        ],
        "ContainerProfiles": [],
        "CodecProfiles": [],
        "SubtitleProfiles": [
            { "Format": "srt",    "Method": "External" },
            { "Format": "ass",    "Method": "External" },
            { "Format": "ssa",    "Method": "External" },
            { "Format": "sub",    "Method": "External" },
            { "Format": "vtt",    "Method": "External" },
            { "Format": "pgssub", "Method": "Embed" },
            { "Format": "dvdsub", "Method": "Embed" },
            { "Format": "subrip", "Method": "Embed" },
            { "Format": "ass",    "Method": "Embed" },
            { "Format": "ssa",    "Method": "Embed" }
        ]
    })
}
