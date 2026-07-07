use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    #[serde(default)]
    pub remote_trusted: bool,
    #[serde(default)]
    pub has_connected_before: bool,
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
pub enum AppTheme {
    CopyBlue,
    Win11Dark,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self::Win11Dark
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CloseAction {
    Ask,
    Minimize,
    Exit,
}

impl Default for CloseAction {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub config_version: u16,
    pub device_name: String,
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub theme: AppTheme,
    #[serde(default)]
    pub close_action: CloseAction,
    pub port: u16,
    pub auto_start: bool,
    pub auto_sync: bool,
    pub save_history: bool,
    pub trusted_devices: Vec<String>,
    pub sync_text: bool,
    pub sync_image: bool,
    pub sync_files: bool,
    #[serde(default)]
    pub discovery_scan_ranges: Vec<String>,
    #[serde(default = "default_true")]
    pub desktop_notifications: bool,
    #[serde(default = "default_true")]
    pub notify_clipboard: bool,
    #[serde(default = "default_true")]
    pub notify_trust_required: bool,
    #[serde(default = "default_true")]
    pub notify_file_transfer: bool,
    #[serde(default = "default_true")]
    pub notify_device_status: bool,
    #[serde(default = "default_true")]
    pub notify_sync_error: bool,
    #[serde(default)]
    pub notification_clipboard_preview: bool,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: 3,
            device_name: "CopyShare".to_string(),
            device_id: new_device_id(),
            theme: AppTheme::Win11Dark,
            close_action: CloseAction::Ask,
            port: 8765,
            auto_start: false,
            auto_sync: true,
            save_history: true,
            trusted_devices: Vec::new(),
            sync_text: true,
            sync_image: true,
            sync_files: false,
            discovery_scan_ranges: Vec::new(),
            desktop_notifications: true,
            notify_clipboard: true,
            notify_trust_required: true,
            notify_file_transfer: true,
            notify_device_status: true,
            notify_sync_error: true,
            notification_clipboard_preview: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DiscoveryScanStatus {
    Idle,
    Running,
    Done,
    Empty,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryScanProgress {
    pub scan_id: u64,
    pub status: DiscoveryScanStatus,
    pub running: bool,
    pub done: usize,
    pub total: usize,
    pub range_count: usize,
    pub started_at: i64,
    pub finished_at: Option<i64>,
}

pub fn new_device_id() -> String {
    format!("device-{}", Uuid::new_v4().simple())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HistoryDirection {
    Local,
    Remote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Synced,
    Unsynced,
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self::Synced
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub direction: HistoryDirection,
    pub source_device: String,
    pub summary: String,
    #[serde(default)]
    pub content: String,
    pub content_type: ClipboardContentType,
    #[serde(default)]
    pub sync_status: SyncStatus,
    pub success: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardTextItem {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub source_device: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MobileSessionMode {
    SendToMobile,
    ReceiveFromMobile,
    Bidirectional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MobileSessionPhase {
    Waiting,
    Opened,
    Copied,
    Submitted,
    Written,
    Expired,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MobileSessionView {
    pub id: String,
    pub url: String,
    pub mode: MobileSessionMode,
    pub phase: MobileSessionPhase,
    pub expires_at: Option<DateTime<Utc>>,
    pub remaining_seconds: Option<i64>,
    pub summary: String,
    pub submitted_summary: Option<String>,
    pub content_items: Vec<ClipboardTextItem>,
    pub submitted_items: Vec<ClipboardTextItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileTransferStatus {
    Pending,
    Accepted,
    Transferring,
    Completed,
    Failed,
    Canceled,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileTransferDirection {
    Send,
    Receive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileTransferFile {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub saved_path: Option<String>,
    pub transferred_bytes: u64,
    pub status: FileTransferFileStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileTransferFileStatus {
    Pending,
    Transferring,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileTransferTask {
    pub transfer_id: String,
    pub direction: FileTransferDirection,
    pub peer_device_id: String,
    pub peer_device_name: String,
    pub files: Vec<FileTransferFile>,
    pub total_size: u64,
    pub transferred_bytes: u64,
    pub status: FileTransferStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectedTransferFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileTransferProgressEvent {
    pub transfer_id: String,
    pub file_id: String,
    pub file_transferred_bytes: u64,
    pub file_size: u64,
    pub total_transferred_bytes: u64,
    pub total_size: u64,
    pub status: FileTransferStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileOfferFile {
    pub file_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub sha256: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileCompleteFile {
    pub file_id: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WireMessage {
    Hello {
        device_id: String,
        device_name: String,
        app_version: String,
        port: u16,
        #[serde(default)]
        manual_trust_required: bool,
    },
    TrustGranted {
        source_device_id: String,
        source_device_name: String,
        port: u16,
        trusted_device_ids: Vec<String>,
    },
    TrustRejected {
        source_device_id: String,
        source_device_name: String,
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
    FileOffer {
        transfer_id: String,
        sender_device_id: String,
        sender_device_name: String,
        files: Vec<FileOfferFile>,
        total_size: u64,
        file_count: usize,
        download_host: String,
        download_port: u16,
    },
    FileAccept {
        transfer_id: String,
        receiver_device_id: String,
        receiver_device_name: String,
    },
    FileReject {
        transfer_id: String,
        receiver_device_id: String,
        reason: Option<String>,
    },
    FileProgress {
        transfer_id: String,
        device_id: String,
        file_id: String,
        file_transferred_bytes: u64,
        file_size: u64,
        total_transferred_bytes: u64,
        total_size: u64,
    },
    FileComplete {
        transfer_id: String,
        device_id: String,
        files: Vec<FileCompleteFile>,
    },
    FileCancel {
        transfer_id: String,
        device_id: String,
    },
    FileError {
        transfer_id: String,
        file_id: Option<String>,
        device_id: String,
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

#[cfg(test)]
mod file_transfer_wire_tests {
    use super::{FileCompleteFile, FileOfferFile, WireMessage};

    #[test]
    fn multi_file_offer_round_trips_as_wire_json() {
        let message = WireMessage::FileOffer {
            transfer_id: "transfer-1".to_string(),
            sender_device_id: "device-a".to_string(),
            sender_device_name: "Laptop A".to_string(),
            files: vec![
                FileOfferFile {
                    file_id: "file-1".to_string(),
                    file_name: "a.txt".to_string(),
                    file_size: 3,
                    sha256: "hash-a".to_string(),
                    token: "token-a".to_string(),
                },
                FileOfferFile {
                    file_id: "file-2".to_string(),
                    file_name: "b.txt".to_string(),
                    file_size: 4,
                    sha256: "hash-b".to_string(),
                    token: "token-b".to_string(),
                },
            ],
            total_size: 7,
            file_count: 2,
            download_host: "10.0.0.1".to_string(),
            download_port: 49152,
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: WireMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, message);
    }

    #[test]
    fn multi_file_progress_round_trips_as_wire_json() {
        let message = WireMessage::FileProgress {
            transfer_id: "transfer-1".to_string(),
            device_id: "device-b".to_string(),
            file_id: "file-2".to_string(),
            file_transferred_bytes: 2,
            file_size: 4,
            total_transferred_bytes: 5,
            total_size: 7,
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: WireMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, message);
    }

    #[test]
    fn multi_file_complete_round_trips_as_wire_json() {
        let message = WireMessage::FileComplete {
            transfer_id: "transfer-1".to_string(),
            device_id: "device-b".to_string(),
            files: vec![
                FileCompleteFile {
                    file_id: "file-1".to_string(),
                    sha256: "hash-a".to_string(),
                },
                FileCompleteFile {
                    file_id: "file-2".to_string(),
                    sha256: "hash-b".to_string(),
                },
            ],
        };

        let json = serde_json::to_string(&message).unwrap();
        let decoded: WireMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded, message);
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
