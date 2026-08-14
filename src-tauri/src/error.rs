use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("未连接到服务器，请先登录")]
    NotAuthenticated,

    #[error("网络请求失败: {0}")]
    Http(#[from] reqwest::Error),

    #[error("服务器返回错误 {status}: {body}")]
    Server { status: u16, body: String },

    #[error("用户名或密码错误")]
    InvalidCredentials,

    #[error("数据解析失败: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("文件读写失败: {0}")]
    Io(#[from] std::io::Error),

    #[error("播放器错误: {0}")]
    Player(String),

    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Other(e.to_string())
    }
}

impl Serialize for AppError {
    // 注意：这里必须写全路径，否则会解析到本模块下面的 Result 别名
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
