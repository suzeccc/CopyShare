use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SyncState {
    Stopped,
    Running,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub running: bool,
    pub device_name: String,
    pub device_id: String,
    pub local_ip: Option<String>,
    pub port: u16,
    pub connected_count: usize,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub state: SyncState,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceStatus {
    Online,
    Connecting,
    Offline,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub connected: bool,
    pub trusted: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub status: DeviceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClipboardContentType {
    Text,
    Image,
    FileList,
}

impl Default for ClipboardContentType {
    fn default() -> Self {
        Self::Text
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardMessage {
    pub message_id: String,
    pub source_device_id: String,
    pub source_device_name: String,
    pub content_type: ClipboardContentType,
    pub content: String,
    pub content_hash: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub device_name: String,
    pub port: u16,
    pub auto_start: bool,
    pub auto_sync: bool,
    pub save_history: bool,
    pub trusted_devices: Vec<String>,
    pub sync_text: bool,
    pub sync_image: bool,
    pub sync_files: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            device_name: "Copy-Sharer".to_string(),
            port: 8765,
            auto_start: false,
            auto_sync: true,
            save_history: true,
            trusted_devices: Vec::new(),
            sync_text: true,
            sync_image: false,
            sync_files: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HistoryDirection {
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub direction: HistoryDirection,
    pub source_device: String,
    pub summary: String,
    pub content_type: ClipboardContentType,
    pub success: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WireMessage {
    Hello {
        device_id: String,
        device_name: String,
        app_version: String,
        port: u16,
    },
    Clipboard {
        message_id: String,
        source_device_id: String,
        source_device_name: String,
        content_type: ClipboardContentType,
        content: String,
        content_hash: String,
        timestamp: i64,
    },
    Ping {
        device_id: String,
        timestamp: i64,
    },
    Pong {
        device_id: String,
        timestamp: i64,
    },
    Error {
        code: String,
        message: String,
    },
}

impl From<ClipboardMessage> for WireMessage {
    fn from(value: ClipboardMessage) -> Self {
        Self::Clipboard {
            message_id: value.message_id,
            source_device_id: value.source_device_id,
            source_device_name: value.source_device_name,
            content_type: value.content_type,
            content: value.content,
            content_hash: value.content_hash,
            timestamp: value.timestamp,
        }
    }
}

impl TryFrom<WireMessage> for ClipboardMessage {
    type Error = &'static str;

    fn try_from(value: WireMessage) -> Result<Self, Self::Error> {
        match value {
            WireMessage::Clipboard {
                message_id,
                source_device_id,
                source_device_name,
                content_type,
                content,
                content_hash,
                timestamp,
            } => Ok(Self {
                message_id,
                source_device_id,
                source_device_name,
                content_type,
                content,
                content_hash,
                timestamp,
            }),
            _ => Err("wire message is not clipboard content"),
        }
    }
}
