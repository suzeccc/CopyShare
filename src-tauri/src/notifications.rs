use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::{
    config as app_config,
    models::{
        AppConfig, ClipboardContentType, ClipboardMessage, DeviceInfo, FileTransferDirection,
        FileTransferTask,
    },
};

const NAVIGATE_EVENT: &str = "navigate-to-page";
const HOME_ROUTE: &str = "/";
const DEVICES_ROUTE: &str = "/devices";
const FILES_ROUTE: &str = "/files";
const MOBILE_ROUTE: &str = "/mobile";
const SETTINGS_ROUTE: &str = "/settings";

static NOTIFICATION_COOLDOWNS: OnceLock<Mutex<HashMap<String, Instant>>> = OnceLock::new();

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

pub fn notify_clipboard_received(
    app: &AppHandle,
    config: &AppConfig,
    message: &ClipboardMessage,
) {
    notify_if_enabled(
        app,
        config,
        config.notify_clipboard,
        &format!(
            "clipboard:{}:{}",
            message.source_device_id, message.content_hash
        ),
        Duration::from_secs(10),
        "收到剪贴板内容",
        &clipboard_notification_body(config, message),
        HOME_ROUTE,
    );
}

pub fn notify_mobile_clipboard_received(
    app: &AppHandle,
    config: &AppConfig,
    message: &ClipboardMessage,
) {
    notify_if_enabled(
        app,
        config,
        config.notify_clipboard,
        &format!("mobile-clipboard:{}", message.content_hash),
        Duration::from_secs(10),
        "手机内容已写入剪贴板",
        &mobile_clipboard_notification_body(config, message),
        MOBILE_ROUTE,
    );
}

pub fn notify_trust_required(app: &AppHandle, config: &AppConfig, device: &DeviceInfo) {
    notify_if_enabled(
        app,
        config,
        config.notify_trust_required,
        &format!("trust-required:{}", device.id),
        Duration::from_secs(60 * 10),
        "设备请求信任",
        &format!("{} 等待确认信任", device_display_name(device)),
        DEVICES_ROUTE,
    );
}

pub fn notify_file_transfer_offer(app: &AppHandle, task: &FileTransferTask) {
    let config = load_notification_config(app);
    notify_if_enabled(
        app,
        &config,
        config.notify_file_transfer,
        &format!("file-offer:{}", task.transfer_id),
        Duration::from_secs(60 * 10),
        "收到文件传输请求",
        &format!("来自 {}：{}", task.peer_device_name, file_summary(task)),
        FILES_ROUTE,
    );
}

pub fn notify_device_discovered(app: &AppHandle, device: &DeviceInfo) {
    let config = load_notification_config(app);
    notify_if_enabled(
        app,
        &config,
        config.notify_device_status,
        &format!("device-discovered:{}", device.id),
        Duration::from_secs(60 * 10),
        "发现局域网设备",
        &format!("{} 等待连接", device_display_name(device)),
        DEVICES_ROUTE,
    );
}

pub fn notify_device_offline(app: &AppHandle, device: &DeviceInfo) {
    let config = load_notification_config(app);
    notify_if_enabled(
        app,
        &config,
        config.notify_device_status,
        &format!("device-offline:{}", device.id),
        Duration::from_secs(30),
        "设备已离线",
        &format!("{} 已离线", device_display_name(device)),
        DEVICES_ROUTE,
    );
}

pub fn notify_file_transfer_completed(app: &AppHandle, task: &FileTransferTask) {
    let config = load_notification_config(app);
    let verb = match task.direction {
        FileTransferDirection::Send => "发送完成",
        FileTransferDirection::Receive => "接收完成",
    };
    notify_if_enabled(
        app,
        &config,
        config.notify_file_transfer,
        &format!("file-completed:{}", task.transfer_id),
        Duration::from_secs(60 * 60),
        "文件传输完成",
        &format!("{verb}: {}", file_summary(task)),
        FILES_ROUTE,
    );
}

pub fn notify_file_transfer_failed(app: &AppHandle, task: &FileTransferTask) {
    let config = load_notification_config(app);
    let reason = task.error.as_deref().unwrap_or("传输失败");
    notify_if_enabled(
        app,
        &config,
        config.notify_file_transfer,
        &format!("file-failed:{}:{reason}", task.transfer_id),
        Duration::from_secs(60 * 60),
        "文件传输失败",
        &format!("{}: {reason}", file_summary(task)),
        FILES_ROUTE,
    );
}

pub fn notify_sync_error(app: &AppHandle, config: &AppConfig, message: &str) {
    notify_if_enabled(
        app,
        config,
        config.notify_sync_error,
        &format!("sync-error:{message}"),
        Duration::from_secs(30),
        "CopyShare 同步异常",
        message,
        SETTINGS_ROUTE,
    );
}

pub fn notify_test(app: &AppHandle) {
    notify(
        app,
        "CopyShare 测试通知",
        "如果你看到这条消息，桌面右下角通知已生效。",
        SETTINGS_ROUTE,
    );
}

fn notify_if_enabled(
    app: &AppHandle,
    config: &AppConfig,
    category_enabled: bool,
    dedupe_key: &str,
    cooldown: Duration,
    title: &str,
    body: &str,
    route: &'static str,
) {
    if !config.desktop_notifications || !category_enabled {
        return;
    }
    if !claim_notification_slot(dedupe_key, cooldown) {
        return;
    }

    notify(app, title, body, route);
}

fn claim_notification_slot(key: &str, cooldown: Duration) -> bool {
    let now = Instant::now();
    let mut cooldowns = NOTIFICATION_COOLDOWNS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    cooldowns.retain(|_, last_seen| now.duration_since(*last_seen) <= Duration::from_secs(60 * 60));
    if cooldowns
        .get(key)
        .map(|last_seen| now.duration_since(*last_seen) < cooldown)
        .unwrap_or(false)
    {
        return false;
    }

    cooldowns.insert(key.to_string(), now);
    true
}

fn load_notification_config(app: &AppHandle) -> AppConfig {
    app_config::load_config(app).unwrap_or_default()
}

fn notify(app: &AppHandle, title: &str, body: &str, route: &'static str) {
    let _ = (NAVIGATE_EVENT, route);
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
}

#[cfg(test)]
fn notification_response_opens_window(response: &notify_rust::NotificationResponse) -> bool {
    use notify_rust::NotificationResponse;

    match response {
        NotificationResponse::Default => true,
        NotificationResponse::Action(action) => action == "open",
        NotificationResponse::Closed(_) | NotificationResponse::Reply(_) => false,
    }
}

fn clipboard_notification_body(config: &AppConfig, message: &ClipboardMessage) -> String {
    let source = source_display_name(&message.source_device_name);
    match message.content_type {
        ClipboardContentType::Text => {
            if config.notification_clipboard_preview {
                format!("来自 {source}: {}", compact_preview(&message.content, 60))
            } else {
                format!("收到来自 {source} 的文本内容")
            }
        }
        ClipboardContentType::Image => format!("收到来自 {source} 的图片"),
        ClipboardContentType::FileList => format!("收到来自 {source} 的文件列表"),
    }
}

fn mobile_clipboard_notification_body(config: &AppConfig, message: &ClipboardMessage) -> String {
    if config.notification_clipboard_preview {
        format!("来自手机: {}", compact_preview(&message.content, 60))
    } else {
        "收到手机发送的文本内容".to_string()
    }
}

fn compact_preview(content: &str, limit: usize) -> String {
    let compact = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return "空文本".to_string();
    }

    let mut preview = compact.chars().take(limit).collect::<String>();
    if compact.chars().count() > limit {
        preview.push_str("...");
    }
    preview
}

fn source_display_name(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        "对方设备".to_string()
    } else {
        name.to_string()
    }
}

fn device_display_name(device: &DeviceInfo) -> String {
    let name = device.name.trim();
    if name.is_empty() {
        device.ip.clone()
    } else {
        name.to_string()
    }
}

fn file_summary(task: &FileTransferTask) -> String {
    if task.files.len() == 1 {
        return task
            .files
            .first()
            .map(|file| file.name.clone())
            .unwrap_or_else(|| "文件".to_string());
    }
    format!("{} 个文件", task.files.len())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use notify_rust::{CloseReason, NotificationResponse};

    use crate::models::{
        AppConfig, ClipboardContentType, ClipboardMessage, FileTransferDirection, FileTransferFile,
        FileTransferFileStatus, FileTransferStatus, FileTransferTask,
    };

    use super::{
        clipboard_notification_body, compact_preview, file_summary,
        mobile_clipboard_notification_body, notification_response_opens_window,
    };

    fn task(files: Vec<FileTransferFile>) -> FileTransferTask {
        FileTransferTask {
            transfer_id: "transfer-a".to_string(),
            direction: FileTransferDirection::Receive,
            peer_device_id: "device-a".to_string(),
            peer_device_name: "Peer".to_string(),
            files,
            total_size: 1,
            transferred_bytes: 0,
            status: FileTransferStatus::Completed,
            created_at: Utc::now(),
            completed_at: None,
            error: None,
        }
    }

    fn file(name: &str) -> FileTransferFile {
        FileTransferFile {
            id: name.to_string(),
            name: name.to_string(),
            size: 1,
            sha256: "hash".to_string(),
            saved_path: None,
            transferred_bytes: 0,
            status: FileTransferFileStatus::Pending,
            error: None,
        }
    }

    fn clipboard_message(content: &str) -> ClipboardMessage {
        ClipboardMessage {
            message_id: "message-a".to_string(),
            source_device_id: "device-a".to_string(),
            source_device_name: "Laptop A".to_string(),
            content_type: ClipboardContentType::Text,
            content: content.to_string(),
            content_hash: "hash-a".to_string(),
            timestamp: 1,
        }
    }

    #[test]
    fn file_summary_uses_name_for_single_file() {
        assert_eq!(file_summary(&task(vec![file("a.txt")])), "a.txt");
    }

    #[test]
    fn file_summary_uses_count_for_multi_file() {
        assert_eq!(file_summary(&task(vec![file("a.txt"), file("b.txt")])), "2 个文件");
    }

    #[test]
    fn notification_default_click_or_open_action_opens_window() {
        assert!(notification_response_opens_window(&NotificationResponse::Default));
        assert!(notification_response_opens_window(&NotificationResponse::Action(
            "open".to_string()
        )));
        assert!(!notification_response_opens_window(
            &NotificationResponse::Action("dismiss".to_string())
        ));
        assert!(!notification_response_opens_window(
            &NotificationResponse::Closed(CloseReason::Dismissed)
        ));
    }

    #[test]
    fn clipboard_notification_hides_text_without_preview() {
        let mut config = AppConfig::default();
        config.notification_clipboard_preview = false;
        let body = clipboard_notification_body(&config, &clipboard_message("secret text"));

        assert_eq!(body, "收到来自 Laptop A 的文本内容");
        assert!(!body.contains("secret text"));
    }

    #[test]
    fn clipboard_notification_can_show_short_preview() {
        let mut config = AppConfig::default();
        config.notification_clipboard_preview = true;

        assert_eq!(
            clipboard_notification_body(&config, &clipboard_message("hello\nworld")),
            "来自 Laptop A: hello world"
        );
    }

    #[test]
    fn mobile_clipboard_notification_hides_text_without_preview() {
        let mut config = AppConfig::default();
        config.notification_clipboard_preview = false;
        let body = mobile_clipboard_notification_body(&config, &clipboard_message("phone secret"));

        assert_eq!(body, "收到手机发送的文本内容");
        assert!(!body.contains("phone secret"));
    }

    #[test]
    fn compact_preview_limits_long_content() {
        assert_eq!(compact_preview(&"a".repeat(65), 60), format!("{}...", "a".repeat(60)));
    }
}
