use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const FILE_RESUME_CAPABILITY: &str = "file-resume-v1";

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
    pub latency_ms: Option<u64>,
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
#[serde(rename_all = "camelCase")]
pub enum ClipboardContentType {
    Text,
    Image,
    #[serde(alias = "filelist")]
    FileList,
}

impl Default for ClipboardContentType {
    fn default() -> Self {
        Self::Text
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEventVersion {
    pub physical_ms: i64,
    pub logical: u32,
    pub origin_device_id: String,
}

impl Ord for ClipboardEventVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.physical_ms
            .cmp(&other.physical_ms)
            .then_with(|| self.logical.cmp(&other.logical))
            .then_with(|| self.origin_device_id.cmp(&other.origin_device_id))
    }
}

impl PartialOrd for ClipboardEventVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_version: Option<ClipboardEventVersion>,
}

impl ClipboardMessage {
    pub fn effective_event_version(&self) -> ClipboardEventVersion {
        self.event_version
            .clone()
            .unwrap_or_else(|| ClipboardEventVersion {
                physical_ms: self.timestamp.saturating_mul(1000),
                logical: 0,
                origin_device_id: self.source_device_id.clone(),
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AppTheme {
    CopyBlue,
    Win11Dark,
    MacosLight,
    MacosDark,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TranslationEngine {
    Google,
    Ai,
}

impl Default for TranslationEngine {
    fn default() -> Self {
        Self::Google
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TranslateResponse {
    pub source_text: String,
    pub target_text: String,
    pub engine: TranslationEngine,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResponse {
    pub text: String,
    pub preview_base64: String,
    pub image_width: u32,
    pub image_height: u32,
    pub error: Option<String>,
}

fn default_translation_model() -> String {
    "gpt-4o-mini".to_string()
}

fn default_quick_panel_shortcut() -> String {
    "Alt+Shift+V".to_string()
}

fn default_ocr_shortcut() -> String {
    "Alt+Shift+O".to_string()
}

fn default_translate_shortcut() -> String {
    "Alt+Shift+T".to_string()
}

fn default_snippets_shortcut() -> String {
    "Alt+Shift+B".to_string()
}

fn default_toggle_sync_shortcut() -> String {
    "Alt+Shift+S".to_string()
}

pub const MIN_FILE_SIZE_LIMIT_MIB: u32 = 100;
pub const MAX_FILE_SIZE_LIMIT_MIB: u32 = 2048;

fn default_file_size_limit_mib() -> u32 {
    MAX_FILE_SIZE_LIMIT_MIB
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
    #[serde(default = "default_true")]
    pub quick_panel_shortcut_enabled: bool,
    #[serde(default = "default_quick_panel_shortcut")]
    pub quick_panel_shortcut: String,
    #[serde(default)]
    pub ocr_shortcut_enabled: bool,
    #[serde(default = "default_ocr_shortcut")]
    pub ocr_shortcut: String,
    #[serde(default)]
    pub translate_shortcut_enabled: bool,
    #[serde(default = "default_translate_shortcut")]
    pub translate_shortcut: String,
    #[serde(default)]
    pub snippets_shortcut_enabled: bool,
    #[serde(default = "default_snippets_shortcut")]
    pub snippets_shortcut: String,
    #[serde(default)]
    pub toggle_sync_shortcut_enabled: bool,
    #[serde(default = "default_toggle_sync_shortcut")]
    pub toggle_sync_shortcut: String,
    pub save_history: bool,
    pub trusted_devices: Vec<String>,
    pub sync_text: bool,
    pub sync_image: bool,
    pub sync_files: bool,
    #[serde(default = "default_file_size_limit_mib")]
    pub max_send_file_size_mib: u32,
    #[serde(default = "default_file_size_limit_mib")]
    pub max_receive_file_size_mib: u32,
    #[serde(default = "default_true")]
    pub deduplicate_sync_content: bool,
    #[serde(default)]
    pub file_save_dir: Option<String>,
    #[serde(default)]
    pub auto_open_folder_after_save: bool,
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
    #[serde(default)]
    pub translation_engine: TranslationEngine,
    #[serde(default)]
    pub translation_api_url: String,
    #[serde(default)]
    pub translation_api_key: String,
    #[serde(default = "default_translation_model")]
    pub translation_model: String,
    #[serde(default)]
    pub translation_proxy: String,
}

fn default_true() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            config_version: 8,
            device_name: "CopyShare".to_string(),
            device_id: new_device_id(),
            theme: AppTheme::Win11Dark,
            close_action: CloseAction::Ask,
            port: 8765,
            auto_start: false,
            auto_sync: true,
            quick_panel_shortcut_enabled: true,
            quick_panel_shortcut: default_quick_panel_shortcut(),
            ocr_shortcut_enabled: false,
            ocr_shortcut: default_ocr_shortcut(),
            translate_shortcut_enabled: false,
            translate_shortcut: default_translate_shortcut(),
            snippets_shortcut_enabled: false,
            snippets_shortcut: default_snippets_shortcut(),
            toggle_sync_shortcut_enabled: false,
            toggle_sync_shortcut: default_toggle_sync_shortcut(),
            save_history: true,
            trusted_devices: Vec::new(),
            sync_text: true,
            sync_image: true,
            sync_files: true,
            max_send_file_size_mib: default_file_size_limit_mib(),
            max_receive_file_size_mib: default_file_size_limit_mib(),
            deduplicate_sync_content: true,
            file_save_dir: None,
            auto_open_folder_after_save: false,
            discovery_scan_ranges: Vec::new(),
            desktop_notifications: true,
            notify_clipboard: true,
            notify_trust_required: true,
            notify_file_transfer: false,
            notify_device_status: true,
            notify_sync_error: true,
            notification_clipboard_preview: true,
            translation_engine: TranslationEngine::Google,
            translation_api_url: String::new(),
            translation_api_key: String::new(),
            translation_model: default_translation_model(),
            translation_proxy: String::new(),
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
    #[serde(default)]
    pub content_hash: String,
    pub content_type: ClipboardContentType,
    #[serde(default)]
    pub sync_status: SyncStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_transfer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_transfer_status: Option<FileTransferStatus>,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<DateTime<Utc>>,
    pub success: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LibraryRole {
    Saved,
    Snippet,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LibraryAssetKind {
    Image,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAssetRef {
    pub asset_id: String,
    pub kind: LibraryAssetKind,
    pub file_name: String,
    pub relative_path: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItem {
    pub id: String,
    pub role: LibraryRole,
    pub content_type: ClipboardContentType,
    pub title: String,
    #[serde(default)]
    pub content: String,
    pub summary: String,
    #[serde(default)]
    pub assets: Vec<LibraryAssetRef>,
    #[serde(default)]
    pub source_history_id: Option<String>,
    #[serde(default)]
    pub source_content_hash: Option<String>,
    #[serde(default)]
    pub source_device: String,
    pub content_hash: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub is_pinned: bool,
    #[serde(default)]
    pub pin_order: Option<u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    pub items: Vec<LibraryItem>,
    #[serde(default)]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryItemUpdate {
    pub title: String,
    pub content: Option<String>,
    pub tags: Vec<String>,
    pub note: String,
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
    WaitingForPeer,
    Retrying,
    Paused,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
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
    #[serde(default)]
    pub clipboard_sync: bool,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
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
pub enum CopyHistoryResult {
    Copied,
    DownloadStarted,
    Downloading,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileOfferFile {
    pub file_id: String,
    pub file_name: String,
    pub file_size: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<String>,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileResumeOffset {
    pub file_id: String,
    pub offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileResumeGrantFile {
    pub file_id: String,
    pub offset: u64,
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
        #[serde(default)]
        capabilities: Vec<String>,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin_sequence: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event_version: Option<ClipboardEventVersion>,
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
        #[serde(default)]
        clipboard_sync: bool,
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
    FileResumeRequest {
        transfer_id: String,
        receiver_device_id: String,
        files: Vec<FileResumeOffset>,
    },
    FileResumeGrant {
        transfer_id: String,
        sender_device_id: String,
        download_host: String,
        download_port: u16,
        files: Vec<FileResumeGrantFile>,
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
            origin_sequence: value.origin_sequence,
            event_version: value.event_version,
        }
    }
}

#[cfg(test)]
mod file_transfer_wire_tests {
    use chrono::Utc;

    use super::{
        ClipboardContentType, ClipboardMessage, FileCompleteFile, FileOfferFile,
        FileTransferDirection, FileTransferStatus, FileTransferTask, WireMessage,
    };

    #[test]
    fn clipboard_content_type_file_list_serializes_as_frontend_file_list() {
        let json = serde_json::to_string(&ClipboardContentType::FileList).unwrap();

        assert_eq!(json, "\"fileList\"");
    }

    #[test]
    fn clipboard_content_type_accepts_legacy_lowercase_filelist() {
        let decoded: ClipboardContentType = serde_json::from_str("\"filelist\"").unwrap();

        assert_eq!(decoded, ClipboardContentType::FileList);
    }

    #[test]
    fn multi_file_offer_round_trips_as_wire_json() {
        let message = WireMessage::FileOffer {
            transfer_id: "transfer-1".to_string(),
            sender_device_id: "device-a".to_string(),
            sender_device_name: "Laptop A".to_string(),
            clipboard_sync: true,
            files: vec![
                FileOfferFile {
                    file_id: "file-1".to_string(),
                    file_name: "a.txt".to_string(),
                    file_size: 3,
                    sha256: "hash-a".to_string(),
                    thumbnail: None,
                    token: "token-a".to_string(),
                },
                FileOfferFile {
                    file_id: "file-2".to_string(),
                    file_name: "b.txt".to_string(),
                    file_size: 4,
                    sha256: "hash-b".to_string(),
                    thumbnail: None,
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
    fn hello_preserves_file_resume_capability() {
        let json = serde_json::json!({
            "type": "hello",
            "device_id": "device-a",
            "device_name": "Laptop A",
            "app_version": "3.4.0",
            "port": 8765,
            "manual_trust_required": false,
            "capabilities": ["file-resume-v1"]
        });

        let decoded: WireMessage = serde_json::from_value(json).unwrap();
        let encoded = serde_json::to_value(decoded).unwrap();

        assert_eq!(encoded["capabilities"][0], "file-resume-v1");
    }

    #[test]
    fn legacy_clipboard_wire_message_defaults_event_metadata() {
        let legacy = serde_json::json!({
            "type": "clipboard",
            "message_id": "message-1",
            "source_device_id": "device-a",
            "source_device_name": "Laptop A",
            "content_type": "text",
            "content": "hello",
            "content_hash": "hash",
            "timestamp": 1_712_000_000_i64
        });

        let wire: WireMessage = serde_json::from_value(legacy).unwrap();
        let clipboard = ClipboardMessage::try_from(wire).unwrap();

        assert_eq!(clipboard.origin_sequence, None);
        assert_eq!(clipboard.event_version, None);
        assert_eq!(
            clipboard.effective_event_version().physical_ms,
            1_712_000_000_000
        );
    }

    #[test]
    fn file_resume_messages_are_accepted_by_the_wire_protocol() {
        let request = serde_json::json!({
            "type": "fileResumeRequest",
            "transfer_id": "transfer-1",
            "receiver_device_id": "device-b",
            "files": [{ "fileId": "file-1", "offset": 734003200_u64 }]
        });
        let grant = serde_json::json!({
            "type": "fileResumeGrant",
            "transfer_id": "transfer-1",
            "sender_device_id": "device-a",
            "download_host": "192.168.1.10",
            "download_port": 8765,
            "files": [{ "fileId": "file-1", "offset": 734003200_u64, "token": "fresh" }]
        });

        assert!(serde_json::from_value::<WireMessage>(request).is_ok());
        assert!(serde_json::from_value::<WireMessage>(grant).is_ok());
    }

    #[test]
    fn file_transfer_task_serializes_clipboard_sync_as_camel_case() {
        let task = FileTransferTask {
            transfer_id: "transfer-1".to_string(),
            direction: FileTransferDirection::Receive,
            peer_device_id: "device-a".to_string(),
            peer_device_name: "Laptop A".to_string(),
            clipboard_sync: true,
            files: Vec::new(),
            total_size: 0,
            transferred_bytes: 0,
            status: FileTransferStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            error: None,
        };

        let json = serde_json::to_string(&task).unwrap();

        assert!(json.contains(r#""clipboardSync":true"#));
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

    #[test]
    fn file_list_clipboard_message_uses_file_list_wire_value() {
        let message = ClipboardMessage {
            message_id: "message-1".to_string(),
            source_device_id: "device-a".to_string(),
            source_device_name: "Laptop A".to_string(),
            content_type: ClipboardContentType::FileList,
            content: "[]".to_string(),
            content_hash: "hash-a".to_string(),
            timestamp: 1,
            origin_sequence: None,
            event_version: None,
        };

        let json = serde_json::to_string(&WireMessage::from(message)).unwrap();

        assert!(json.contains(r#""fileList""#));
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
                origin_sequence,
                event_version,
            } => Ok(Self {
                message_id,
                source_device_id,
                source_device_name,
                content_type,
                content,
                content_hash,
                timestamp,
                origin_sequence,
                event_version,
            }),
            _ => Err("wire message is not clipboard content"),
        }
    }
}
