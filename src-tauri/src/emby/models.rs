//! Emby 服务端数据模型。
//!
//! 反序列化时使用 Emby 的 PascalCase，序列化给前端时转成 camelCase，
//! 这样 Vue 侧拿到的就是符合 JS 习惯的字段名。
//!
//! 所有 id / tag / 文本字段都走 [`super::de`] 里的宽容反序列化器：不同版本的
//! Emby 和刮削插件会把同一个字段一会儿给字符串一会儿给数字，严格解析会让
//! 整部影片加载不出来。

use super::de::{flex_opt_string, flex_string, flex_string_map, flex_string_vec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 1 tick = 100 纳秒
pub const TICKS_PER_SECOND: i64 = 10_000_000;

pub fn ticks_to_seconds(ticks: Option<i64>) -> f64 {
    ticks.unwrap_or(0) as f64 / TICKS_PER_SECOND as f64
}

pub fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * TICKS_PER_SECOND as f64).round() as i64
}

macro_rules! emby_model {
    ($(#[$meta:meta])* pub struct $name:ident { $($body:tt)* }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, Deserialize, Serialize)]
        #[serde(rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
        #[serde(default)]
        pub struct $name { $($body)* }
    };
}

emby_model! {
    pub struct AuthResult {
        pub user: UserDto,
        #[serde(deserialize_with = "flex_string")]
        pub access_token: String,
        #[serde(deserialize_with = "flex_string")]
        pub server_id: String,
    }
}

emby_model! {
    pub struct UserDto {
        #[serde(deserialize_with = "flex_string")]
        pub id: String,
        #[serde(deserialize_with = "flex_string")]
        pub name: String,
        #[serde(deserialize_with = "flex_string")]
        pub server_id: String,
        #[serde(deserialize_with = "flex_opt_string")]
        pub primary_image_tag: Option<String>,
        pub has_password: bool,
    }
}

emby_model! {
    pub struct SystemInfo {
        #[serde(deserialize_with = "flex_string")]
        pub server_name: String,
        #[serde(deserialize_with = "flex_string")]
        pub version: String,
        #[serde(deserialize_with = "flex_string")]
        pub id: String,
        #[serde(deserialize_with = "flex_opt_string")]
        pub operating_system: Option<String>,
    }
}

emby_model! {
    pub struct UserData {
        pub playback_position_ticks: Option<i64>,
        pub play_count: i32,
        pub is_favorite: bool,
        pub played: bool,
        pub played_percentage: Option<f64>,
        pub unplayed_item_count: Option<i32>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub last_played_date: Option<String>,
    }
}

emby_model! {
    pub struct NameGuidPair {
        #[serde(deserialize_with = "flex_string")]
        pub name: String,
        #[serde(deserialize_with = "flex_opt_string")]
        pub id: Option<String>,
    }
}

emby_model! {
    pub struct PersonDto {
        #[serde(deserialize_with = "flex_string")]
        pub name: String,
        #[serde(deserialize_with = "flex_opt_string")]
        pub id: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub role: Option<String>,
        #[serde(rename(deserialize = "Type", serialize = "type"), deserialize_with = "flex_opt_string")]
        pub kind: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub primary_image_tag: Option<String>,
    }
}

emby_model! {
    pub struct MediaStream {
        #[serde(deserialize_with = "flex_opt_string")]
        pub codec: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub language: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub display_title: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub title: Option<String>,
        #[serde(rename(deserialize = "Type", serialize = "type"), deserialize_with = "flex_string")]
        pub kind: String,
        pub index: i32,
        pub is_default: bool,
        pub is_forced: bool,
        pub is_external: bool,
        pub height: Option<i32>,
        pub width: Option<i32>,
        pub bit_rate: Option<i64>,
        pub channels: Option<i32>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub channel_layout: Option<String>,
        pub sample_rate: Option<i32>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub delivery_url: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub codec_tag: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub video_range: Option<String>,
        pub average_frame_rate: Option<f64>,
        pub real_frame_rate: Option<f64>,
    }
}

emby_model! {
    pub struct MediaSourceInfo {
        #[serde(deserialize_with = "flex_string")]
        pub id: String,
        #[serde(deserialize_with = "flex_opt_string")]
        pub name: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub path: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub protocol: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub container: Option<String>,
        pub size: Option<i64>,
        pub bitrate: Option<i64>,
        pub run_time_ticks: Option<i64>,
        pub is_remote: bool,
        pub supports_direct_play: bool,
        pub supports_direct_stream: bool,
        pub supports_transcoding: bool,
        #[serde(deserialize_with = "flex_opt_string")]
        pub direct_stream_url: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub transcoding_url: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub transcoding_container: Option<String>,
        pub media_streams: Vec<MediaStream>,
        pub default_audio_stream_index: Option<i32>,
        pub default_subtitle_stream_index: Option<i32>,
    }
}

emby_model! {
    pub struct BaseItem {
        #[serde(deserialize_with = "flex_string")]
        pub id: String,
        #[serde(deserialize_with = "flex_string")]
        pub name: String,
        #[serde(deserialize_with = "flex_opt_string")]
        pub original_title: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub server_id: Option<String>,
        #[serde(rename(deserialize = "Type", serialize = "type"), deserialize_with = "flex_string")]
        pub kind: String,
        #[serde(deserialize_with = "flex_opt_string")]
        pub media_type: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub collection_type: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub overview: Option<String>,
        #[serde(deserialize_with = "flex_string_vec")]
        pub taglines: Vec<String>,
        pub production_year: Option<i32>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub premiere_date: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub end_date: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub date_created: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub official_rating: Option<String>,
        pub community_rating: Option<f64>,
        pub critic_rating: Option<f64>,
        pub run_time_ticks: Option<i64>,
        pub index_number: Option<i32>,
        pub parent_index_number: Option<i32>,
        pub child_count: Option<i32>,
        pub recursive_item_count: Option<i32>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub status: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub path: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub container: Option<String>,
        pub width: Option<i32>,
        pub height: Option<i32>,

        #[serde(deserialize_with = "flex_opt_string")]
        pub series_id: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub series_name: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub season_id: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub season_name: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub album_id: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub parent_id: Option<String>,

        #[serde(deserialize_with = "flex_string_map")]
        pub image_tags: HashMap<String, String>,
        #[serde(deserialize_with = "flex_string_vec")]
        pub backdrop_image_tags: Vec<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub parent_backdrop_item_id: Option<String>,
        #[serde(deserialize_with = "flex_string_vec")]
        pub parent_backdrop_image_tags: Vec<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub parent_thumb_item_id: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub parent_thumb_image_tag: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub parent_primary_image_item_id: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub parent_primary_image_tag: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub series_primary_image_tag: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub series_thumb_image_tag: Option<String>,

        #[serde(deserialize_with = "flex_string_vec")]
        pub genres: Vec<String>,
        pub studios: Vec<NameGuidPair>,
        pub people: Vec<PersonDto>,
        pub user_data: Option<UserData>,
        pub media_sources: Vec<MediaSourceInfo>,
        pub media_streams: Vec<MediaStream>,
        /// TMDB / TVDB 的 id 经常是整数，必须宽容处理
        #[serde(deserialize_with = "flex_string_map")]
        pub provider_ids: HashMap<String, String>,
        pub is_folder: bool,
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all(deserialize = "PascalCase", serialize = "camelCase"))]
#[serde(default)]
pub struct QueryResult<T> {
    pub items: Vec<T>,
    pub total_record_count: i64,
}

emby_model! {
    pub struct PlaybackInfoResponse {
        pub media_sources: Vec<MediaSourceInfo>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub play_session_id: Option<String>,
        #[serde(deserialize_with = "flex_opt_string")]
        pub error_code: Option<String>,
    }
}

/// 传给前端 / 播放器的最终播放参数
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackTarget {
    pub item_id: String,
    pub media_source_id: String,
    pub play_session_id: String,
    /// 可直接喂给 mpv / 外置播放器的完整 URL
    pub url: String,
    /// true = 原始文件直连（画质无损），false = 服务器转码
    pub is_direct: bool,
    pub title: String,
    pub sub_title: Option<String>,
    /// 起播位置（秒）
    pub start_position: f64,
    pub duration: f64,
    pub container: Option<String>,
    pub size: Option<i64>,
    pub bitrate: Option<i64>,
    pub audio_streams: Vec<MediaStream>,
    pub subtitle_streams: Vec<MediaStream>,
    pub video_stream: Option<MediaStream>,
    /// 外挂字幕的完整下载地址，按 MediaStream.index 索引
    pub external_subtitles: Vec<ExternalSubtitle>,
    pub default_audio_index: Option<i32>,
    pub default_subtitle_index: Option<i32>,
    pub backdrop_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 复现「详情页报 invalid type: integer '757', expected a string」：
    /// Emby 把 TMDB id 当数字返回，同时 Studios/People 的 Id 也可能是数字。
    #[test]
    fn parses_item_with_numeric_ids() {
        let json = r#"{
            "Name": "某部电影",
            "Id": 4211,
            "Type": "Movie",
            "ProviderIds": { "Tmdb": 757, "Imdb": "tt0120737", "Tvdb": 12345 },
            "Studios": [{ "Name": "New Line", "Id": 88 }],
            "People": [{ "Name": "演员甲", "Id": 991, "Type": "Actor", "Role": "主角" }],
            "ImageTags": { "Primary": 8843 },
            "BackdropImageTags": [770, "abc"],
            "Genres": ["剧情"],
            "RunTimeTicks": 71000000000,
            "CommunityRating": 8.4,
            "MediaSources": [{ "Id": 55, "Container": "mkv", "MediaStreams": [
                { "Type": "Video", "Index": 0, "Codec": "hevc" }
            ]}]
        }"#;

        let item: BaseItem = serde_json::from_str(json).expect("数字 id 也应当能解析");

        assert_eq!(item.id, "4211");
        assert_eq!(item.kind, "Movie");
        assert_eq!(item.provider_ids.get("Tmdb").map(String::as_str), Some("757"));
        assert_eq!(item.provider_ids.get("Imdb").map(String::as_str), Some("tt0120737"));
        assert_eq!(item.studios[0].id.as_deref(), Some("88"));
        assert_eq!(item.people[0].id.as_deref(), Some("991"));
        assert_eq!(item.image_tags.get("Primary").map(String::as_str), Some("8843"));
        assert_eq!(item.backdrop_image_tags, vec!["770".to_string(), "abc".to_string()]);
        assert_eq!(item.media_sources[0].id, "55");
        assert_eq!(item.media_sources[0].media_streams[0].kind, "Video");
    }

    #[test]
    fn parses_plain_string_ids() {
        let json = r#"{
            "Name": "常规条目",
            "Id": "9f2c1a",
            "Type": "Episode",
            "ProviderIds": { "Tmdb": "757" },
            "SeriesId": "abc123"
        }"#;
        let item: BaseItem = serde_json::from_str(json).expect("字符串 id 当然要能解析");
        assert_eq!(item.id, "9f2c1a");
        assert_eq!(item.series_id.as_deref(), Some("abc123"));
        assert_eq!(item.provider_ids.get("Tmdb").map(String::as_str), Some("757"));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSubtitle {
    pub index: i32,
    pub title: String,
    pub language: Option<String>,
    pub url: String,
    pub is_default: bool,
    pub codec: Option<String>,
}
