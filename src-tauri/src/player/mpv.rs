//! 内置播放器：把 mpv 嵌入宿主窗口，通过命名管道 JSON IPC 控制。
//!
//! 为什么用 mpv 而不是 WebView 的 <video>：
//! WebView2 只能播 H.264/VP9 + AAC 的 MP4/WebM，Emby 库里大量的
//! MKV / HEVC / AV1 / DTS / TrueHD / ASS 字幕全都放不了，只能让服务器
//! 转码 —— 画质有损、服务器 CPU 爆炸。mpv 走 D3D11 硬解，本地零转码。

use crate::config::{PlayerSettings, QualityPreset};
use crate::error::{AppError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 播放器状态快照，节流后推给前端
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSnapshot {
    pub active: bool,
    pub paused: bool,
    pub idle: bool,
    /// 正在缓冲（网络跟不上）
    pub buffering: bool,
    pub seeking: bool,
    pub position: f64,
    pub duration: f64,
    /// 已缓冲到的时间点
    pub cache_time: f64,
    pub volume: f64,
    pub muted: bool,
    pub speed: f64,
    pub width: i64,
    pub height: i64,
    pub fps: f64,
    /// 当前实际使用的硬解方式，no 表示软解
    pub hwdec: String,
    pub video_codec: String,
    pub audio_codec: String,
    pub tracks: Vec<TrackInfo>,
    pub audio_track: i64,
    pub sub_track: i64,
    pub file_loaded: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackInfo {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub lang: String,
    pub codec: String,
    pub selected: bool,
    pub external: bool,
    pub default: bool,
    pub forced: bool,
    /// 音轨声道数 / 视频分辨率描述
    pub detail: String,
}

/// 一个正在运行的 mpv 实例
pub struct MpvSession {
    child: tokio::process::Child,
    tx: UnboundedSender<String>,
    pub snapshot: Arc<RwLock<PlayerSnapshot>>,
    request_id: AtomicU64,
}

impl MpvSession {
    /// 启动 mpv 并嵌入到 `wid` 指定的窗口里
    pub async fn launch<F>(
        mpv_path: &str,
        wid: isize,
        settings: &PlayerSettings,
        on_event: F,
    ) -> Result<Self>
    where
        F: Fn(MpvEvent) + Send + Sync + 'static,
    {
        let pipe_name = format!(r"\\.\pipe\shenhe-mpv-{}", uuid::Uuid::new_v4().simple());

        let mut cmd = tokio::process::Command::new(mpv_path);
        cmd.args(build_args(wid, &pipe_name, settings))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW);

        let child = cmd
            .spawn()
            .map_err(|e| AppError::Player(format!("启动 mpv 失败（{mpv_path}）: {e}")))?;

        // mpv 需要一点时间创建管道
        let pipe = connect_pipe(&pipe_name, Duration::from_secs(8)).await?;

        let snapshot = Arc::new(RwLock::new(PlayerSnapshot {
            volume: settings.volume as f64,
            speed: 1.0,
            ..Default::default()
        }));

        let (tx, mut rx) = unbounded_channel::<String>();
        let (reader, mut writer) = tokio::io::split(pipe);

        // 写线程：串行化所有命令
        tauri::async_runtime::spawn(async move {
            while let Some(line) = rx.recv().await {
                if writer.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
                let _ = writer.flush().await;
            }
        });

        // 读线程：解析事件，更新快照
        let snap = snapshot.clone();
        tauri::async_runtime::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            let mut last_emit = std::time::Instant::now() - Duration::from_secs(1);

            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };

                let mut force_emit = false;
                if let Some(event) = msg.get("event").and_then(Value::as_str) {
                    match event {
                        "property-change" => {
                            apply_property(&snap, &msg);
                        }
                        "file-loaded" => {
                            {
                                let mut s = snap.write();
                                s.file_loaded = true;
                                s.active = true;
                            }
                            force_emit = true;
                            on_event(MpvEvent::FileLoaded);
                        }
                        "end-file" => {
                            let reason = msg
                                .get("reason")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown")
                                .to_string();
                            {
                                let mut s = snap.write();
                                s.file_loaded = false;
                                s.position = 0.0;
                            }
                            force_emit = true;
                            on_event(MpvEvent::EndFile { reason });
                        }
                        "seek" => {
                            snap.write().seeking = true;
                            force_emit = true;
                        }
                        "playback-restart" => {
                            snap.write().seeking = false;
                            force_emit = true;
                        }
                        "shutdown" => {
                            snap.write().active = false;
                            on_event(MpvEvent::Shutdown);
                            break;
                        }
                        _ => {}
                    }
                }

                // 位置更新按 4Hz 推送即可，避免无谓的 IPC 与重渲染
                if force_emit || last_emit.elapsed() >= Duration::from_millis(250) {
                    last_emit = std::time::Instant::now();
                    on_event(MpvEvent::State(snap.read().clone()));
                }
            }
            on_event(MpvEvent::Shutdown);
        });

        let session = Self {
            child,
            tx,
            snapshot,
            request_id: AtomicU64::new(1),
        };

        session.observe_properties()?;
        Ok(session)
    }

    fn observe_properties(&self) -> Result<()> {
        const PROPS: &[(u64, &str)] = &[
            (1, "pause"),
            (2, "time-pos"),
            (3, "duration"),
            (4, "paused-for-cache"),
            (5, "demuxer-cache-time"),
            (6, "volume"),
            (7, "mute"),
            (8, "speed"),
            (9, "track-list"),
            (10, "core-idle"),
            (11, "idle-active"),
            (12, "width"),
            (13, "height"),
            (14, "container-fps"),
            (15, "hwdec-current"),
            (16, "video-format"),
            (17, "audio-codec-name"),
            (18, "aid"),
            (19, "sid"),
        ];
        for (id, name) in PROPS {
            self.send(json!({ "command": ["observe_property", id, name] }))?;
        }
        Ok(())
    }

    pub fn send(&self, mut payload: Value) -> Result<()> {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "request_id".into(),
                json!(self.request_id.fetch_add(1, Ordering::Relaxed)),
            );
        }
        self.tx
            .send(payload.to_string())
            .map_err(|_| AppError::Player("mpv 连接已断开".into()))
    }

    pub fn command(&self, args: Vec<Value>) -> Result<()> {
        self.send(json!({ "command": args }))
    }

    pub fn set_property(&self, name: &str, value: Value) -> Result<()> {
        self.send(json!({ "command": ["set_property", name, value] }))
    }

    /// 载入一个新文件。external_subs 为外挂字幕的完整 URL。
    ///
    /// 起播位置和标题走属性而不是 loadfile 的 options 参数：
    /// mpv 0.38 在 flags 之后插入了一个 index 参数，把 options 直接接在
    /// 第四位会被当成 index，整条命令静默失效。属性写法各版本都一致。
    pub fn load_file(&self, url: &str, start: f64, title: &str) -> Result<()> {
        self.set_property("force-media-title", json!(title))?;
        self.set_property("start", json!(format!("{:.3}", start.max(0.0))))?;
        self.command(vec![json!("loadfile"), json!(url), json!("replace")])
    }

    /// 挂载外挂字幕。select=true 时同时切过去。
    ///
    /// 必须等 file-loaded 之后再调：loadfile 是异步的，紧跟着 sub-add
    /// 会挂到上一个文件上，甚至直接被丢弃。
    pub fn add_subtitle(&self, url: &str, title: &str, select: bool) -> Result<()> {
        self.command(vec![
            json!("sub-add"),
            json!(url),
            json!(if select { "select" } else { "auto" }),
            json!(title),
        ])
    }

    pub fn set_pause(&self, paused: bool) -> Result<()> {
        self.set_property("pause", json!(paused))
    }

    pub fn seek_absolute(&self, seconds: f64) -> Result<()> {
        self.command(vec![
            json!("seek"),
            json!(seconds),
            json!("absolute+keyframes"),
        ])
    }

    pub fn seek_relative(&self, seconds: f64) -> Result<()> {
        self.command(vec![json!("seek"), json!(seconds), json!("relative")])
    }

    pub fn stop(&self) -> Result<()> {
        self.command(vec![json!("stop")])
    }

    pub async fn quit(mut self) {
        let _ = self.command(vec![json!("quit")]);
        // 给 mpv 一点时间自己退出，超时就强杀
        let killed = tokio::time::timeout(Duration::from_millis(1200), self.child.wait()).await;
        if killed.is_err() {
            let _ = self.child.kill().await;
        }
    }
}

#[derive(Debug, Clone)]
pub enum MpvEvent {
    State(PlayerSnapshot),
    FileLoaded,
    EndFile { reason: String },
    Shutdown,
}

async fn connect_pipe(name: &str, timeout: Duration) -> Result<NamedPipeClient> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match ClientOptions::new().open(name) {
            Ok(client) => return Ok(client),
            Err(e) if std::time::Instant::now() < deadline => {
                log::debug!("等待 mpv IPC 管道: {e}");
                tokio::time::sleep(Duration::from_millis(60)).await;
            }
            Err(e) => {
                return Err(AppError::Player(format!(
                    "无法连接 mpv 控制管道: {e}（mpv 可能启动失败）"
                )))
            }
        }
    }
}

fn apply_property(snap: &Arc<RwLock<PlayerSnapshot>>, msg: &Value) {
    let Some(name) = msg.get("name").and_then(Value::as_str) else {
        return;
    };
    let data = msg.get("data").unwrap_or(&Value::Null);
    let mut s = snap.write();

    match name {
        "pause" => s.paused = data.as_bool().unwrap_or(false),
        "time-pos" => {
            if let Some(v) = data.as_f64() {
                s.position = v;
            }
        }
        "duration" => {
            if let Some(v) = data.as_f64() {
                s.duration = v;
            }
        }
        "paused-for-cache" => s.buffering = data.as_bool().unwrap_or(false),
        "demuxer-cache-time" => s.cache_time = data.as_f64().unwrap_or(0.0),
        "volume" => s.volume = data.as_f64().unwrap_or(100.0),
        "mute" => s.muted = data.as_bool().unwrap_or(false),
        "speed" => s.speed = data.as_f64().unwrap_or(1.0),
        "core-idle" => s.idle = data.as_bool().unwrap_or(false),
        "idle-active" => {
            if data.as_bool().unwrap_or(false) {
                s.file_loaded = false;
            }
        }
        "width" => s.width = data.as_i64().unwrap_or(0),
        "height" => s.height = data.as_i64().unwrap_or(0),
        "container-fps" => s.fps = data.as_f64().unwrap_or(0.0),
        "hwdec-current" => s.hwdec = data.as_str().unwrap_or("no").to_string(),
        "video-format" => s.video_codec = data.as_str().unwrap_or("").to_string(),
        "audio-codec-name" => s.audio_codec = data.as_str().unwrap_or("").to_string(),
        "aid" => s.audio_track = data.as_i64().unwrap_or(-1),
        "sid" => s.sub_track = data.as_i64().unwrap_or(-1),
        "track-list" => {
            if let Some(arr) = data.as_array() {
                s.tracks = arr.iter().filter_map(parse_track).collect();
            }
        }
        _ => {}
    }
}

fn parse_track(v: &Value) -> Option<TrackInfo> {
    let kind = v.get("type")?.as_str()?.to_string();
    let id = v.get("id")?.as_i64()?;

    let lang = v
        .get("lang")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let codec = v
        .get("codec")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let detail = match kind.as_str() {
        "video" => {
            let w = v.get("demux-w").and_then(Value::as_i64).unwrap_or(0);
            let h = v.get("demux-h").and_then(Value::as_i64).unwrap_or(0);
            if w > 0 { format!("{w}x{h}") } else { String::new() }
        }
        "audio" => {
            let ch = v.get("demux-channel-count").and_then(Value::as_i64).unwrap_or(0);
            match ch {
                0 => String::new(),
                1 => "单声道".into(),
                2 => "立体声".into(),
                6 => "5.1".into(),
                8 => "7.1".into(),
                n => format!("{n} 声道"),
            }
        }
        _ => String::new(),
    };

    let title = v
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    Some(TrackInfo {
        id,
        kind,
        title,
        lang,
        codec,
        selected: v.get("selected").and_then(Value::as_bool).unwrap_or(false),
        external: v.get("external").and_then(Value::as_bool).unwrap_or(false),
        default: v.get("default").and_then(Value::as_bool).unwrap_or(false),
        forced: v.get("forced").and_then(Value::as_bool).unwrap_or(false),
        detail,
    })
}

fn build_args(wid: isize, pipe_name: &str, settings: &PlayerSettings) -> Vec<String> {
    let mut args: Vec<String> = vec![
        // 嵌入到我们自己的宿主窗口
        format!("--wid={wid}"),
        format!("--input-ipc-server={pipe_name}"),
        // 不加载用户的 mpv 配置，保证行为可预期
        "--no-config".into(),
        "--idle=yes".into(),
        "--force-window=yes".into(),
        "--keep-open=yes".into(),
        // OSD / 快捷键由我们自己的界面负责
        "--no-osc".into(),
        "--osd-level=0".into(),
        "--no-input-default-bindings".into(),
        "--input-vo-keyboard=no".into(),
        "--no-input-cursor".into(),
        "--cursor-autohide=no".into(),
        "--msg-level=all=warn".into(),
        // 网络播放：足够的预读让拖动和高码率不卡
        "--cache=yes".into(),
        "--cache-secs=60".into(),
        "--demuxer-max-bytes=128MiB".into(),
        "--demuxer-max-back-bytes=48MiB".into(),
        "--demuxer-readahead-secs=20".into(),
        "--stream-lavf-o=reconnect=1,reconnect_streamed=1,reconnect_delay_max=5".into(),
        "--user-agent=ShenhePlayer/0.1.0".into(),
        // 字幕
        "--sub-auto=no".into(),
        format!("--sub-font-size={}", settings.sub_font_size),
        "--sub-border-size=2.6".into(),
        "--sub-shadow-offset=1".into(),
        "--sub-ass-override=scale".into(),
        // 音频
        format!("--volume={}", settings.volume),
        "--volume-max=150".into(),
        "--audio-client-name=ShenhePlayer".into(),
        "--audio-file-auto=no".into(),
        format!("--hwdec={}", settings.hwdec),
        "--gpu-api=d3d11".into(),
    ];

    // 画质档位：低端机默认最省，画质档给独显用户
    match settings.quality {
        QualityPreset::Performance => args.extend([
            "--vo=gpu".into(),
            "--scale=bilinear".into(),
            "--dscale=bilinear".into(),
            "--cscale=bilinear".into(),
            "--dither=no".into(),
            "--correct-downscaling=no".into(),
            "--sigmoid-upscaling=no".into(),
            "--video-sync=audio".into(),
        ]),
        QualityPreset::Balanced => args.extend([
            if settings.gpu_next { "--vo=gpu-next".into() } else { "--vo=gpu".to_string() },
            "--scale=spline36".into(),
            "--dscale=mitchell".into(),
            "--cscale=bilinear".into(),
            "--correct-downscaling=yes".into(),
            "--video-sync=audio".into(),
        ]),
        QualityPreset::Quality => args.extend([
            if settings.gpu_next { "--vo=gpu-next".into() } else { "--vo=gpu".to_string() },
            "--scale=ewa_lanczossharp".into(),
            "--dscale=mitchell".into(),
            "--cscale=spline36".into(),
            "--correct-downscaling=yes".into(),
            "--sigmoid-upscaling=yes".into(),
            "--deband=yes".into(),
            "--dither-depth=auto".into(),
            "--video-sync=display-resample".into(),
        ]),
    }

    args
}
