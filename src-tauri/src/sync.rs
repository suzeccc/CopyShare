use std::collections::HashSet;
use std::time::Duration;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, watch};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message, WebSocketStream};
use uuid::Uuid;

use crate::{
    clipboard,
    error::AppResult,
    history,
    models::{
        ClipboardContentType, ClipboardMessage, DeviceInfo, DeviceStatus, HistoryDirection,
        WireMessage,
    },
    network,
    state::AppState,
};

pub fn content_hash(content_type: &ClipboardContentType, content: &str) -> String {
    let mut hasher = Sha256::new();
    let format = match content_type {
        ClipboardContentType::Text => "text",
        ClipboardContentType::Image => "image",
        ClipboardContentType::FileList => "fileList",
    };
    hasher.update(format.as_bytes());
    hasher.update([0]);
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone)]
pub struct SyncEngine {
    device_id: String,
    device_name: String,
    seen_message_ids: HashSet<String>,
    last_local_hash: Option<String>,
    last_remote_hash: Option<String>,
    pending_remote_echo_hashes: HashSet<String>,
}

impl SyncEngine {
    pub fn new(device_id: impl Into<String>, device_name: impl Into<String>) -> Self {
        Self {
            device_id: device_id.into(),
            device_name: device_name.into(),
            seen_message_ids: HashSet::new(),
            last_local_hash: None,
            last_remote_hash: None,
            pending_remote_echo_hashes: HashSet::new(),
        }
    }

    pub fn set_device(&mut self, device_id: impl Into<String>, device_name: impl Into<String>) {
        self.device_id = device_id.into();
        self.device_name = device_name.into();
    }

    pub fn observe_local_text(&mut self, text: impl Into<String>) -> Option<ClipboardMessage> {
        let content = text.into();
        let content_type = ClipboardContentType::Text;
        let local_hash = content_hash(&content_type, &content);

        if self.pending_remote_echo_hashes.remove(&local_hash) {
            self.last_local_hash = Some(local_hash);
            return None;
        }

        if self.last_local_hash.as_deref() == Some(&local_hash) {
            return None;
        }

        let message = ClipboardMessage {
            message_id: Uuid::new_v4().to_string(),
            source_device_id: self.device_id.clone(),
            source_device_name: self.device_name.clone(),
            content_type,
            content,
            content_hash: local_hash.clone(),
            timestamp: Utc::now().timestamp(),
        };
        self.last_local_hash = Some(local_hash);
        self.seen_message_ids.insert(message.message_id.clone());
        Some(message)
    }

    pub fn apply_remote_message(&mut self, message: &ClipboardMessage) -> bool {
        if message.source_device_id == self.device_id {
            return false;
        }

        if self.seen_message_ids.contains(&message.message_id) {
            return false;
        }

        self.seen_message_ids.insert(message.message_id.clone());
        if self.last_remote_hash.as_deref() == Some(&message.content_hash)
            || self.last_local_hash.as_deref() == Some(&message.content_hash)
        {
            return false;
        }

        self.last_remote_hash = Some(message.content_hash.clone());
        self.last_local_hash = Some(message.content_hash.clone());
        self.pending_remote_echo_hashes
            .insert(message.content_hash.clone());
        true
    }

    #[cfg(test)]
    pub fn should_suppress_watcher_echo(&self, content: &str) -> bool {
        let hash = content_hash(&ClipboardContentType::Text, content);
        self.pending_remote_echo_hashes.contains(&hash)
    }
}

pub async fn run_sync_runtime(app: AppHandle, state: AppState, mut stop_rx: watch::Receiver<bool>) {
    let config = state.config().await;
    let local_ip = local_ip_address::local_ip().ok().map(|ip| ip.to_string());

    let listener = match TcpListener::bind(("0.0.0.0", config.port)).await {
        Ok(listener) => listener,
        Err(error) => {
            let message = format!("无法监听端口 {}：{}", config.port, error);
            state.set_error(message.clone()).await;
            let _ = app.emit("sync-error", message);
            return;
        }
    };

    state
        .set_running(true, local_ip, format!("正在监听端口 {}", config.port))
        .await;
    emit_status(&app, &state).await;

    let mut interval = tokio::time::interval(Duration::from_millis(450));
    loop {
        tokio::select! {
            _ = stop_rx.changed() => {
                if *stop_rx.borrow() {
                    break;
                }
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, address)) => {
                        let connection_id = address.to_string();
                        let app_for_task = app.clone();
                        let state_for_task = state.clone();
                        tauri::async_runtime::spawn(async move {
                            match accept_async(stream).await {
                                Ok(socket) => {
                                    let _ = spawn_socket(app_for_task, state_for_task, connection_id, socket).await;
                                }
                                Err(error) => {
                                    let _ = app_for_task.emit("sync-error", format!("WebSocket 握手失败：{error}"));
                                }
                            }
                        });
                    }
                    Err(error) => {
                        let _ = app.emit("sync-error", format!("设备连接失败：{error}"));
                    }
                }
            }
            _ = interval.tick() => {
                if let Err(error) = poll_local_clipboard(&app, &state).await {
                    let _ = app.emit("sync-error", error.to_string());
                }
            }
        }
    }

    state
        .set_running(false, None, "同步已停止".to_string())
        .await;
    emit_status(&app, &state).await;
}

pub async fn connect_to_peer(
    app: AppHandle,
    state: AppState,
    ip: String,
    port: u16,
) -> AppResult<DeviceInfo> {
    let url = network::normalize_peer_url(&format!("{ip}:{port}"), port)?;
    let (socket, _) = connect_async(url.clone()).await?;
    let connection_id = url.clone();
    spawn_socket(app.clone(), state.clone(), connection_id.clone(), socket).await?;

    let device = DeviceInfo {
        id: connection_id.clone(),
        name: connection_id.clone(),
        ip,
        port,
        connected: true,
        trusted: false,
        last_seen_at: Some(Utc::now()),
        status: DeviceStatus::Online,
    };
    state.upsert_device(device.clone()).await;
    let _ = app.emit("device-connected", device.clone());
    emit_status(&app, &state).await;
    Ok(device)
}

async fn spawn_socket<S>(
    app: AppHandle,
    state: AppState,
    connection_id: String,
    socket: WebSocketStream<S>,
) -> AppResult<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sink, mut stream) = socket.split();
    let (sender, mut receiver) = mpsc::unbounded_channel::<WireMessage>();
    let hello = build_hello(&state).await;
    let connection_id_for_task = connection_id.clone();
    let app_for_task = app.clone();
    let state_for_task = state.clone();

    let join = tauri::async_runtime::spawn(async move {
        if let Ok(text) = network::encode_wire_message(&hello) {
            let _ = sink.send(Message::Text(text.into())).await;
        }

        loop {
            tokio::select! {
                outbound = receiver.recv() => {
                    let Some(outbound) = outbound else {
                        break;
                    };
                    match network::encode_wire_message(&outbound) {
                        Ok(text) => {
                            if sink.send(Message::Text(text.into())).await.is_err() {
                                break;
                            }
                        }
                        Err(error) => {
                            let _ = app_for_task.emit("sync-error", error.to_string());
                        }
                    }
                }
                inbound = stream.next() => {
                    match inbound {
                        Some(Ok(Message::Text(text))) => {
                            handle_wire_text(&app_for_task, &state_for_task, &connection_id_for_task, &text.to_string()).await;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Ok(Message::Ping(payload))) => {
                            let _ = sink.send(Message::Pong(payload)).await;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(error)) => {
                            let _ = app_for_task.emit("sync-error", format!("设备消息读取失败：{error}"));
                            break;
                        }
                    }
                }
            }
        }

        state_for_task.forget_peer(&connection_id_for_task).await;
        if let Some(device) = state_for_task.mark_device_disconnected(&connection_id_for_task).await {
            let _ = app_for_task.emit("device-disconnected", device);
        }
        emit_status(&app_for_task, &state_for_task).await;
    });

    state.register_peer(connection_id, sender, join).await;
    Ok(())
}

async fn handle_wire_text(app: &AppHandle, state: &AppState, connection_id: &str, text: &str) {
    let message = match network::decode_wire_message(text) {
        Ok(message) => message,
        Err(error) => {
            let _ = app.emit("sync-error", format!("忽略无效消息：{error}"));
            return;
        }
    };

    match message {
        WireMessage::Hello {
            device_id,
            device_name,
            port,
            ..
        } => {
            let device = DeviceInfo {
                id: connection_id.to_string(),
                name: device_name,
                ip: connection_id.to_string(),
                port,
                connected: true,
                trusted: crate::security::is_trusted(&state.config().await, &device_id),
                last_seen_at: Some(Utc::now()),
                status: DeviceStatus::Online,
            };
            state.upsert_device(device.clone()).await;
            let _ = app.emit("device-connected", device);
            emit_status(app, state).await;
        }
        WireMessage::Clipboard { .. } => {
            let clipboard = match ClipboardMessage::try_from(message) {
                Ok(message) => message,
                Err(error) => {
                    let _ = app.emit("sync-error", error);
                    return;
                }
            };
            if clipboard.content_type != ClipboardContentType::Text {
                let _ = app.emit("sync-error", "MVP 仅支持文本剪贴板同步");
                return;
            }
            if state.apply_remote_clipboard(&clipboard).await {
                if let Err(error) = clipboard::write_clipboard_text(app, &clipboard.content) {
                    let _ = app.emit("sync-error", error.to_string());
                    return;
                }
                state.touch_last_sync().await;
                let config = state.config().await;
                if config.save_history {
                    let item = history::make_history_item(
                        HistoryDirection::Remote,
                        clipboard.source_device_name.clone(),
                        &clipboard,
                    );
                    state.push_history(item.clone()).await;
                    let _ = history::save_history(app, &state.history().await);
                    let _ = app.emit("clipboard-synced", item);
                }
                state.broadcast(clipboard.into()).await;
                emit_status(app, state).await;
            }
        }
        WireMessage::Ping { timestamp, .. } => {
            let status = state.status().await;
            state
                .broadcast(WireMessage::Pong {
                    device_id: status.device_id,
                    timestamp,
                })
                .await;
        }
        WireMessage::Pong { .. } | WireMessage::Error { .. } => {}
    }
}

async fn poll_local_clipboard(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let config = state.config().await;
    if !config.sync_text {
        return Ok(());
    }

    let text = clipboard::read_clipboard_text(app)?;
    let Some(message) = state.observe_local_text(text).await else {
        return Ok(());
    };

    state.touch_last_sync().await;
    state.broadcast(message.clone().into()).await;

    if config.save_history {
        let item = history::make_history_item(
            HistoryDirection::Local,
            message.source_device_name.clone(),
            &message,
        );
        state.push_history(item.clone()).await;
        history::save_history(app, &state.history().await)?;
        let _ = app.emit("clipboard-synced", item);
    }

    emit_status(app, state).await;
    Ok(())
}

async fn build_hello(state: &AppState) -> WireMessage {
    let status = state.status().await;
    WireMessage::Hello {
        device_id: status.device_id,
        device_name: status.device_name,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        port: status.port,
    }
}

async fn emit_status(app: &AppHandle, state: &AppState) {
    let _ = app.emit("sync-status-changed", state.status().await);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote_message(id: &str, content: &str) -> ClipboardMessage {
        let content_type = ClipboardContentType::Text;
        ClipboardMessage {
            message_id: id.to_string(),
            source_device_id: "device-b".to_string(),
            source_device_name: "Laptop B".to_string(),
            content_hash: content_hash(&content_type, content),
            content_type,
            content: content.to_string(),
            timestamp: 1_712_000_000,
        }
    }

    #[test]
    fn content_hash_is_stable_and_includes_type() {
        let first = content_hash(&ClipboardContentType::Text, "hello");
        let second = content_hash(&ClipboardContentType::Text, "hello");
        let image = content_hash(&ClipboardContentType::Image, "hello");

        assert_eq!(first, second);
        assert_ne!(first, image);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn local_text_change_creates_one_message() {
        let mut engine = SyncEngine::new("device-a", "Laptop A");

        let first = engine.observe_local_text("hello");
        let duplicate = engine.observe_local_text("hello");

        let first = first.expect("first local change should create a message");
        assert_eq!(first.source_device_id, "device-a");
        assert_eq!(first.source_device_name, "Laptop A");
        assert_eq!(first.content, "hello");
        assert_eq!(first.content_type, ClipboardContentType::Text);
        assert!(duplicate.is_none());
    }

    #[test]
    fn remote_message_writes_once_and_suppresses_echo() {
        let mut engine = SyncEngine::new("device-a", "Laptop A");
        let remote = remote_message("remote-1", "remote text");

        assert!(engine.apply_remote_message(&remote));
        assert!(engine.should_suppress_watcher_echo("remote text"));
        assert!(engine.observe_local_text("remote text").is_none());
        assert!(!engine.apply_remote_message(&remote));
    }

    #[test]
    fn own_device_messages_are_ignored() {
        let mut engine = SyncEngine::new("device-a", "Laptop A");
        let content_type = ClipboardContentType::Text;
        let own = ClipboardMessage {
            message_id: "own-1".to_string(),
            source_device_id: "device-a".to_string(),
            source_device_name: "Laptop A".to_string(),
            content_hash: content_hash(&content_type, "echo"),
            content_type,
            content: "echo".to_string(),
            timestamp: 1_712_000_000,
        };

        assert!(!engine.apply_remote_message(&own));
    }

    #[test]
    fn remote_same_content_with_new_id_is_not_written_again() {
        let mut engine = SyncEngine::new("device-a", "Laptop A");
        let first = remote_message("remote-1", "same");
        let second = remote_message("remote-2", "same");

        assert!(engine.apply_remote_message(&first));
        assert!(!engine.apply_remote_message(&second));
    }
}
