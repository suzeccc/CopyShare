use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("clipboard error: {0}")]
    Clipboard(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("sync is already running")]
    AlreadyRunning,
    #[error("sync is not running")]
    NotRunning,
    #[error("{0}")]
    ConnectionTimeout(String),
    #[error("unknown device: {0}")]
    UnknownDevice(String),
    #[error("tauri error: {0}")]
    Tauri(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

impl From<tauri::Error> for AppError {
    fn from(value: tauri::Error) -> Self {
        Self::Tauri(value.to_string())
    }
}
