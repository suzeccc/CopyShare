use std::collections::HashSet;
use std::time::Duration;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, watch};
use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message, WebSocketStream};
use uuid::Uuid;

use crate::{
    clipboard,
    config as app_config,
    error::{AppError, AppResult},
    history,
    models::{
        ClipboardContentType, ClipboardMessage, DeviceInfo, DeviceStatus, HistoryDirection,
        SyncState, WireMessage,
    },
    network,
    security,
    state::AppState,
};
use crate::models::AppConfig;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(6);

fn heartbeat_timed_out(
    last_seen_at: tokio::time::Instant,
    now: tokio::time::Instant,
) -> bool {
    now.duration_since(last_seen_at) >= HEARTBEAT_TIMEOUT
}

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
        self.observe_local_content(ClipboardContentType::Text, text)
    }

    pub fn observe_local_image(
        &mut self,
        image_base64: impl Into<String>,
    ) -> Option<ClipboardMessage> {
        self.observe_local_content(ClipboardContentType::Image, image_base64)
    }

    fn observe_local_content(
        &mut self,
        content_type: ClipboardContentType,
        content: impl Into<String>,
    ) -> Option<ClipboardMessage> {
        let content = content.into();
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

    pub fn reset_local_observation(&mut self) {
        self.last_local_hash = None;
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
    let local_ip = network::preferred_local_ip(&config.trusted_devices, config.port)
        .map(|ip| ip.to_string());

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

pub async fn start_sync_runtime(app: AppHandle, state: AppState) -> AppResult<()> {
    let (stop, stop_rx) = watch::channel(false);
    let (ready, ready_rx) = oneshot::channel();
    let runtime_id = Uuid::new_v4().to_string();
    let state_for_runtime = state.clone();
    let runtime_id_for_task = runtime_id.clone();
    let join = tauri::async_runtime::spawn(async move {
        if ready_rx.await.is_err() {
            return;
        }
        run_sync_runtime(app, state_for_runtime.clone(), stop_rx).await;
        state_for_runtime.clear_runtime(&runtime_id_for_task).await;
    });
    if let Err(error) = state.start_runtime(runtime_id, stop, join).await {
        return Err(error);
    }
    let _ = ready.send(());
    Ok(())
}

pub fn should_auto_start_sync(config: &crate::models::AppConfig, running: bool) -> bool {
    config.auto_sync && !running
}

pub fn should_start_sync_for_manual_connect(running: bool) -> bool {
    !running
}

pub async fn wait_for_sync_ready(state: &AppState, timeout: Duration) -> AppResult<()> {
    let start = tokio::time::Instant::now();

    loop {
        let status = state.status().await;
        match status.state {
            SyncState::Running => return Ok(()),
            SyncState::Error => {
                return Err(AppError::InvalidInput(
                    status.message.unwrap_or_else(|| "同步启动失败".to_string()),
                ));
            }
            SyncState::Stopped => {}
        }

        if start.elapsed() >= timeout {
            return Err(AppError::ConnectionTimeout(
                "同步启动超时：请确认端口未被占用，并允许 Windows 防火墙放行".to_string(),
            ));
        }

        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

pub async fn connect_to_peer(
    app: AppHandle,
    state: AppState,
    ip: String,
    port: u16,
) -> AppResult<DeviceInfo> {
    if let Some(device) = state.connected_device_for_endpoint(&ip, port).await? {
        return Ok(device);
    }

    let url = network::normalize_peer_endpoint(&ip, port)?;
    let connect_result = tokio::time::timeout(CONNECT_TIMEOUT, connect_async(url.clone())).await;
    let (socket, _) = match connect_result {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => {
            return Err(AppError::ConnectionTimeout(peer_connection_failure_message(
                &ip,
                port,
                &error.to_string(),
            )));
        }
        Err(_) => {
            return Err(AppError::ConnectionTimeout(peer_connection_failure_message(
                &ip,
                port,
                "连接超时",
            )));
        }
    };
    let connection_id = url.clone();
    state.mark_manual_trust_required(&connection_id).await;
    spawn_socket(app.clone(), state.clone(), connection_id.clone(), socket).await?;

    if let Some(local_ip) = network::preferred_local_ip_for_peer(network::peer_ip_hint(&ip, port)) {
        state.set_local_ip(Some(local_ip.to_string())).await;
    }

    let display_ip = network::display_host_from_connection_id(&connection_id);
    let device = DeviceInfo {
        id: connection_id.clone(),
        name: connection_id.clone(),
        ip: display_ip,
        port,
        connected: true,
        trusted: false,
        last_seen_at: Some(Utc::now()),
        status: DeviceStatus::Online,
    };
    let device = state.upsert_device(device).await;
    let _ = app.emit("device-connected", device.clone());
    emit_status(&app, &state).await;
    Ok(device)
}

pub async fn notify_peer_trusted(state: &AppState, config: &AppConfig, trusted_device_id: &str) {
    let status = state.status().await;
    let trusted_device_ids = state.trust_keys_for_device(trusted_device_id).await;
    state
        .broadcast_trusted(
            config,
            WireMessage::TrustGranted {
                source_device_id: status.device_id,
                source_device_name: status.device_name,
                port: status.port,
                trusted_device_ids,
            },
        )
        .await;
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

        let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut last_seen_at = tokio::time::Instant::now();

        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    let now = tokio::time::Instant::now();
                    if heartbeat_timed_out(last_seen_at, now) {
                        let _ = app_for_task.emit("sync-error", "设备连接超时，已标记离线".to_string());
                        break;
                    }
                    if sink.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                }
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
                            last_seen_at = tokio::time::Instant::now();
                            handle_wire_text(&app_for_task, &state_for_task, &connection_id_for_task, &text.to_string()).await;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Ok(Message::Ping(payload))) => {
                            last_seen_at = tokio::time::Instant::now();
                            if sink.send(Message::Pong(payload)).await.is_err() {
                                break;
                            }
                        }
                        Some(Ok(Message::Pong(_))) => {
                            last_seen_at = tokio::time::Instant::now();
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

        let device_id = state_for_task
            .forget_peer(&connection_id_for_task)
            .await
            .unwrap_or_else(|| connection_id_for_task.clone());
        if let Some(device) = state_for_task.mark_device_disconnected(&device_id).await {
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
            let mut config = state.config().await;
            let endpoint = network::endpoint_from_connection_id(connection_id, port)
                .unwrap_or_else(|_| connection_id.to_string());
            state
                .attach_peer_device(connection_id, device_id.clone(), Some(endpoint.clone()))
                .await;
            let manual_trust_required = state
                .manual_trust_required_for_peer(connection_id, &device_id, &endpoint)
                .await;
            if !manual_trust_required && security::is_device_id_trusted(&config, &device_id) {
                for key in [connection_id, &device_id, &endpoint] {
                    security::trust_device(&mut config, key);
                }
                match app_config::save_config(app, &config) {
                    Ok(()) => {
                        state.set_config(config.clone()).await;
                        let _ = app.emit("config-updated", config.clone());
                    }
                    Err(error) => {
                        let _ = app.emit("sync-error", error.to_string());
                    }
                }
            }
            let trusted = !manual_trust_required
                && reset_local_observation_if_peer_is_trusted(
                    state,
                    &config,
                    connection_id,
                    &device_id,
                    &endpoint,
                )
                .await;
            let device = DeviceInfo {
                id: device_id.clone(),
                name: device_name,
                ip: network::display_host_from_connection_id(connection_id),
                port,
                connected: true,
                trusted,
                last_seen_at: Some(Utc::now()),
                status: DeviceStatus::Online,
            };
            let device = state.upsert_device(device).await;
            let _ = app.emit("device-connected", device);
            emit_status(app, state).await;
        }
        WireMessage::TrustGranted {
            source_device_id,
            source_device_name: _,
            port: _,
            trusted_device_ids,
        } => {
            let mut config = state.config().await;
            if apply_peer_trust_grant(
                state,
                &mut config,
                connection_id,
                &source_device_id,
                &trusted_device_ids,
            )
            .await
            {
                match app_config::save_config(app, &config) {
                    Ok(()) => {
                        state.set_config(config.clone()).await;
                        let _ = app.emit("config-updated", config);
                    }
                    Err(error) => {
                        let _ = app.emit("sync-error", error.to_string());
                    }
                }
                emit_status(app, state).await;
            }
        }
        WireMessage::Clipboard { .. } => {
            let clipboard = match ClipboardMessage::try_from(message) {
                Ok(message) => message,
                Err(error) => {
                    let _ = app.emit("sync-error", error);
                    return;
                }
            };
            let config = state.config().await;
            if !should_accept_clipboard_type(&config, &clipboard.content_type) {
                return;
            }
            if !state
                .clipboard_sender_is_trusted(&config, connection_id, &clipboard.source_device_id)
                .await
            {
                return;
            }
            if state.apply_remote_clipboard(&clipboard).await {
                if let Err(error) = write_remote_clipboard(app, &clipboard) {
                    let _ = app.emit("sync-error", error.to_string());
                    return;
                }
                state.touch_last_sync().await;
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
                if should_forward_applied_remote_clipboard() {
                    state.broadcast_trusted(&config, clipboard.into()).await;
                }
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
    if !should_read_local_clipboard(state, &config).await {
        return Ok(());
    }

    let mut changed = false;

    if config.sync_text {
        if let Ok(text) = clipboard::read_clipboard_text(app) {
            if let Some(message) = publish_local_text_if_needed(state, &config, text).await {
                record_local_history(app, state, &config, &message).await?;
                changed = true;
            }
        }
    }

    if config.sync_image {
        if let Some(image_base64) = clipboard::read_clipboard_image_base64(app)? {
            if let Some(message) =
                publish_local_image_if_needed(state, &config, image_base64).await
            {
                record_local_history(app, state, &config, &message).await?;
                changed = true;
            }
        }
    }

    if changed {
        emit_status(app, state).await;
    }
    Ok(())
}

async fn should_read_local_clipboard(state: &AppState, config: &AppConfig) -> bool {
    (config.sync_text || config.sync_image) && state.has_trusted_peers(config).await
}

async fn publish_local_text_if_needed(
    state: &AppState,
    config: &AppConfig,
    text: String,
) -> Option<ClipboardMessage> {
    if !config.sync_text || !state.has_trusted_peers(config).await {
        return None;
    }

    let message = state.observe_local_text(text).await?;
    state.touch_last_sync().await;
    state.broadcast_trusted(config, message.clone().into()).await;
    Some(message)
}

async fn publish_local_image_if_needed(
    state: &AppState,
    config: &AppConfig,
    image_base64: String,
) -> Option<ClipboardMessage> {
    if !config.sync_image || !state.has_trusted_peers(config).await {
        return None;
    }

    let message = state.observe_local_image(image_base64).await?;
    state.touch_last_sync().await;
    state.broadcast_trusted(config, message.clone().into()).await;
    Some(message)
}

async fn record_local_history(
    app: &AppHandle,
    state: &AppState,
    config: &AppConfig,
    message: &ClipboardMessage,
) -> AppResult<()> {
    if !config.save_history {
        return Ok(());
    }

    let item = history::make_history_item(
        HistoryDirection::Local,
        message.source_device_name.clone(),
        message,
    );
    state.push_history(item.clone()).await;
    history::save_history(app, &state.history().await)?;
    let _ = app.emit("clipboard-synced", item);
    Ok(())
}

fn should_accept_clipboard_type(config: &AppConfig, content_type: &ClipboardContentType) -> bool {
    match content_type {
        ClipboardContentType::Text => config.sync_text,
        ClipboardContentType::Image => config.sync_image,
        ClipboardContentType::FileList => false,
    }
}

fn write_remote_clipboard(app: &AppHandle, message: &ClipboardMessage) -> AppResult<()> {
    match message.content_type {
        ClipboardContentType::Text => clipboard::write_clipboard_text(app, &message.content),
        ClipboardContentType::Image => {
            clipboard::write_clipboard_image_base64(app, &message.content)
        }
        ClipboardContentType::FileList => Err(AppError::InvalidInput(
            "暂不支持文件剪贴板同步".to_string(),
        )),
    }
}

async fn reset_local_observation_if_peer_is_trusted(
    state: &AppState,
    config: &AppConfig,
    connection_id: &str,
    device_id: &str,
    endpoint: &str,
) -> bool {
    let _ = (connection_id, endpoint);
    let trusted = security::is_device_id_trusted(config, device_id);
    if trusted {
        state.reset_local_clipboard_observation().await;
    }

    trusted
}

async fn apply_peer_trust_grant(
    state: &AppState,
    config: &mut AppConfig,
    connection_id: &str,
    source_device_id: &str,
    trusted_device_ids: &[String],
) -> bool {
    if !trusted_device_ids
        .iter()
        .any(|device_id| device_id == &config.device_id)
    {
        return false;
    }

    if !state
        .peer_connection_matches_device(connection_id, source_device_id)
        .await
    {
        return false;
    }

    false
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

fn should_forward_applied_remote_clipboard() -> bool {
    false
}

fn peer_connection_failure_message(ip: &str, port: u16, reason: &str) -> String {
    format!(
        "连接失败：无法连接到 {ip}:{port}。请确认对方已开启同步，并使用对方主面板显示的本机地址和端口连接；同时允许 Windows 防火墙放行端口 {port}。原因：{reason}"
    )
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

    fn remote_image_message(id: &str, content: &str) -> ClipboardMessage {
        let content_type = ClipboardContentType::Image;
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
    fn local_image_change_creates_image_message() {
        let mut engine = SyncEngine::new("device-a", "Laptop A");

        let first = engine.observe_local_image("png-base64");
        let duplicate = engine.observe_local_image("png-base64");

        let first = first.expect("first local image should create a message");
        assert_eq!(first.source_device_id, "device-a");
        assert_eq!(first.source_device_name, "Laptop A");
        assert_eq!(first.content, "png-base64");
        assert_eq!(first.content_type, ClipboardContentType::Image);
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
    fn remote_image_message_writes_once_and_suppresses_echo() {
        let mut engine = SyncEngine::new("device-a", "Laptop A");
        let remote = remote_image_message("remote-image-1", "remote-image-base64");

        assert!(engine.apply_remote_message(&remote));
        assert!(engine.observe_local_image("remote-image-base64").is_none());
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

    #[test]
    fn applied_remote_clipboard_messages_are_not_forwarded_again() {
        assert!(!should_forward_applied_remote_clipboard());
    }

    #[test]
    fn heartbeat_timeout_only_trips_after_deadline() {
        let last_seen = tokio::time::Instant::now();

        assert!(!heartbeat_timed_out(
            last_seen,
            last_seen + HEARTBEAT_TIMEOUT - Duration::from_millis(1)
        ));
        assert!(heartbeat_timed_out(
            last_seen,
            last_seen + HEARTBEAT_TIMEOUT
        ));
    }

    #[test]
    fn two_devices_can_take_turns_sending_clipboard_updates() {
        let mut device_a = SyncEngine::new("device-a", "Laptop A");
        let mut device_b = SyncEngine::new("device-b", "Laptop B");

        let from_a = device_a
            .observe_local_text("from A")
            .expect("device A should publish its local change");
        assert!(device_b.apply_remote_message(&from_a));
        assert!(device_b.observe_local_text("from A").is_none());

        let from_b = device_b
            .observe_local_text("from B")
            .expect("device B should publish its next local change");
        assert!(device_a.apply_remote_message(&from_b));
        assert!(device_a.observe_local_text("from B").is_none());
    }

    #[tokio::test]
    async fn websocket_connection_carries_clipboard_updates_both_directions() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tauri::async_runtime::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let mut device_b = SyncEngine::new("device-b", "Laptop B");

            let client_hello = read_wire_message(&mut socket).await;
            assert_eq!(
                client_hello,
                WireMessage::Hello {
                    device_id: "device-a".to_string(),
                    device_name: "Laptop A".to_string(),
                    app_version: "test".to_string(),
                    port: 8765,
                }
            );
            send_wire_message(
                &mut socket,
                WireMessage::Hello {
                    device_id: "device-b".to_string(),
                    device_name: "Laptop B".to_string(),
                    app_version: "test".to_string(),
                    port: 8766,
                },
            )
            .await;

            let from_a = ClipboardMessage::try_from(read_wire_message(&mut socket).await).unwrap();
            assert_eq!(from_a.source_device_id, "device-a");
            assert_eq!(from_a.content, "from A over websocket");
            assert!(device_b.apply_remote_message(&from_a));
            assert!(device_b.observe_local_text("from A over websocket").is_none());

            let from_b = device_b
                .observe_local_text("from B over websocket")
                .expect("device B should publish its local clipboard");
            send_wire_message(&mut socket, from_b.into()).await;
        });

        let (mut client, _) = connect_async(format!("ws://{address}/")).await.unwrap();
        let mut device_a = SyncEngine::new("device-a", "Laptop A");
        send_wire_message(
            &mut client,
            WireMessage::Hello {
                device_id: "device-a".to_string(),
                device_name: "Laptop A".to_string(),
                app_version: "test".to_string(),
                port: 8765,
            },
        )
        .await;

        let server_hello = read_wire_message(&mut client).await;
        assert_eq!(
            server_hello,
            WireMessage::Hello {
                device_id: "device-b".to_string(),
                device_name: "Laptop B".to_string(),
                app_version: "test".to_string(),
                port: 8766,
            }
        );

        let from_a = device_a
            .observe_local_text("from A over websocket")
            .expect("device A should publish its local clipboard");
        send_wire_message(&mut client, from_a.into()).await;

        let from_b = ClipboardMessage::try_from(read_wire_message(&mut client).await).unwrap();
        assert_eq!(from_b.source_device_id, "device-b");
        assert_eq!(from_b.content, "from B over websocket");
        assert!(device_a.apply_remote_message(&from_b));
        assert!(device_a.observe_local_text("from B over websocket").is_none());

        server.await.unwrap();
    }

    #[tokio::test]
    async fn websocket_connection_carries_trust_grant_before_reverse_clipboard() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tauri::async_runtime::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let mut device_b = SyncEngine::new("device-b", "Laptop B");

            let client_hello = read_wire_message(&mut socket).await;
            assert!(matches!(
                client_hello,
                WireMessage::Hello {
                    device_id,
                    port: 8766,
                    ..
                } if device_id == "device-a"
            ));
            send_wire_message(
                &mut socket,
                WireMessage::Hello {
                    device_id: "device-b".to_string(),
                    device_name: "Laptop B".to_string(),
                    app_version: "test".to_string(),
                    port: 8765,
                },
            )
            .await;

            let trust_grant = read_wire_message(&mut socket).await;
            assert!(matches!(
                trust_grant,
                WireMessage::TrustGranted {
                    source_device_id,
                    trusted_device_ids,
                    ..
                } if source_device_id == "device-a"
                    && trusted_device_ids.contains(&"device-b".to_string())
            ));

            let from_b = device_b
                .observe_local_text("from B after trust grant")
                .expect("device B should publish after receiving reciprocal trust grant");
            send_wire_message(&mut socket, from_b.into()).await;
        });

        let (mut client, _) = connect_async(format!("ws://{address}/")).await.unwrap();
        let mut device_a = SyncEngine::new("device-a", "Laptop A");
        send_wire_message(
            &mut client,
            WireMessage::Hello {
                device_id: "device-a".to_string(),
                device_name: "Laptop A".to_string(),
                app_version: "test".to_string(),
                port: 8766,
            },
        )
        .await;

        let server_hello = read_wire_message(&mut client).await;
        assert!(matches!(
            server_hello,
            WireMessage::Hello {
                device_id,
                port: 8765,
                ..
            } if device_id == "device-b"
        ));

        send_wire_message(
            &mut client,
            WireMessage::TrustGranted {
                source_device_id: "device-a".to_string(),
                source_device_name: "Laptop A".to_string(),
                port: 8766,
                trusted_device_ids: vec![
                    "device-b".to_string(),
                    "ws://127.0.0.1:8765/".to_string(),
                ],
            },
        )
        .await;

        let from_b = ClipboardMessage::try_from(read_wire_message(&mut client).await).unwrap();
        assert_eq!(from_b.source_device_id, "device-b");
        assert_eq!(from_b.content, "from B after trust grant");
        assert!(device_a.apply_remote_message(&from_b));
        assert!(device_a.observe_local_text("from B after trust grant").is_none());

        server.await.unwrap();
    }

    #[tokio::test]
    async fn local_polling_publishes_clipboard_updates_both_directions_for_trusted_peers() {
        let state_a = AppState::new();
        let mut config_a = crate::models::AppConfig::default();
        config_a.device_id = "device-a".to_string();
        config_a.device_name = "Laptop A".to_string();
        config_a.trusted_devices.push("device-b".to_string());
        state_a.set_config(config_a.clone()).await;

        let state_b = AppState::new();
        let mut config_b = crate::models::AppConfig::default();
        config_b.device_id = "device-b".to_string();
        config_b.device_name = "Laptop B".to_string();
        config_b.trusted_devices.push("device-a".to_string());
        state_b.set_config(config_b.clone()).await;

        let (sender_a_to_b, mut outbound_a_to_b) = mpsc::unbounded_channel();
        let join_a = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state_a
            .register_peer("a-to-b".to_string(), sender_a_to_b, join_a)
            .await;
        state_a
            .attach_peer_device("a-to-b", "device-b".to_string(), None)
            .await;

        let (sender_b_to_a, mut outbound_b_to_a) = mpsc::unbounded_channel();
        let join_b = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state_b
            .register_peer("b-to-a".to_string(), sender_b_to_a, join_b)
            .await;
        state_b
            .attach_peer_device("b-to-a", "device-a".to_string(), None)
            .await;

        let from_a = publish_local_text_if_needed(&state_a, &config_a, "poll from A".to_string())
            .await
            .expect("device A polling should publish local clipboard text");
        let delivered_to_b =
            ClipboardMessage::try_from(outbound_a_to_b.try_recv().unwrap()).unwrap();
        assert_eq!(delivered_to_b, from_a);
        assert!(state_b.apply_remote_clipboard(&delivered_to_b).await);
        assert!(state_b.observe_local_text("poll from A".to_string()).await.is_none());

        let from_b = publish_local_text_if_needed(&state_b, &config_b, "poll from B".to_string())
            .await
            .expect("device B polling should publish local clipboard text");
        let delivered_to_a =
            ClipboardMessage::try_from(outbound_b_to_a.try_recv().unwrap()).unwrap();
        assert_eq!(delivered_to_a, from_b);
        assert!(state_a.apply_remote_clipboard(&delivered_to_a).await);
        assert!(state_a.observe_local_text("poll from B".to_string()).await.is_none());
    }

    #[tokio::test]
    async fn manual_trust_notifies_connected_peer_that_it_was_trusted() {
        let state = AppState::new();
        let mut config = crate::models::AppConfig::default();
        config.device_id = "device-a".to_string();
        config.device_name = "Laptop A".to_string();
        config.port = 8766;
        config.trusted_devices.push("device-b".to_string());
        state.set_config(config.clone()).await;

        let (sender, mut outbound) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.33.156:51051".to_string(), sender, join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51051",
                "device-b".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        notify_peer_trusted(&state, &config, "device-b").await;

        let WireMessage::TrustGranted {
            source_device_id,
            source_device_name,
            port,
            trusted_device_ids,
        } = outbound.try_recv().unwrap() else {
            panic!("manual trust should send a trust grant");
        };
        assert_eq!(source_device_id, "device-a");
        assert_eq!(source_device_name, "Laptop A");
        assert_eq!(port, 8766);
        assert_eq!(
            trusted_device_ids,
            vec![
                "device-b".to_string(),
                "10.194.33.156:51051".to_string(),
                "ws://10.194.33.156:8765/".to_string(),
            ]
        );
        assert!(
            trusted_device_ids.contains(&"device-b".to_string()),
            "receiver must see its stable device id in trust aliases"
        );
    }

    #[tokio::test]
    async fn trust_grant_requires_local_user_confirmation_before_reverse_publish() {
        let state = AppState::new();
        let mut config = crate::models::AppConfig::default();
        config.device_id = "device-b".to_string();
        config.device_name = "Laptop B".to_string();
        state.set_config(config.clone()).await;

        let (sender, mut outbound) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.34.119:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device(
                "10.194.34.119:51234",
                "device-a".to_string(),
                Some("ws://10.194.34.119:8766/".to_string()),
            )
            .await;

        assert!(
            publish_local_text_if_needed(&state, &config, "from B before grant".to_string())
                .await
                .is_none()
        );

        assert!(!apply_peer_trust_grant(
            &state,
            &mut config,
            "10.194.34.119:51234",
            "device-a",
            &["device-b".to_string()],
        )
        .await);
        state.set_config(config.clone()).await;

        assert!(
            publish_local_text_if_needed(&state, &config, "from B after grant".to_string())
                .await
                .is_none()
        );
        assert!(outbound.try_recv().is_err());
        assert!(!config.trusted_devices.contains(&"device-a".to_string()));
    }

    #[tokio::test]
    async fn local_clipboard_is_read_only_when_text_sync_has_trusted_peers() {
        let state = AppState::new();
        let mut config = crate::models::AppConfig::default();

        assert!(!should_read_local_clipboard(&state, &config).await);

        config.trusted_devices.push("device-b".to_string());
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("a-to-b".to_string(), sender, join)
            .await;
        state
            .attach_peer_device("a-to-b", "device-b".to_string(), None)
            .await;

        assert!(should_read_local_clipboard(&state, &config).await);

        config.sync_text = false;
        assert!(!should_read_local_clipboard(&state, &config).await);

        config.sync_image = true;
        assert!(should_read_local_clipboard(&state, &config).await);
    }

    #[tokio::test]
    async fn trusting_connected_peer_allows_current_clipboard_to_publish() {
        let state = AppState::new();
        let mut config = crate::models::AppConfig::default();
        config.device_id = "device-a".to_string();
        config.device_name = "Laptop A".to_string();
        state.set_config(config.clone()).await;

        let (sender, mut outbound) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("a-to-b".to_string(), sender, join)
            .await;
        state
            .attach_peer_device("a-to-b", "device-b".to_string(), None)
            .await;

        assert!(
            publish_local_text_if_needed(&state, &config, "ready after trust".to_string())
                .await
                .is_none()
        );
        assert!(outbound.try_recv().is_err());

        config.trusted_devices.push("device-b".to_string());
        state.set_config(config.clone()).await;
        state.reset_local_clipboard_observation().await;

        let message =
            publish_local_text_if_needed(&state, &config, "ready after trust".to_string())
                .await
                .expect("current clipboard should publish after trusting connected peer");
        let delivered = ClipboardMessage::try_from(outbound.try_recv().unwrap()).unwrap();

        assert_eq!(delivered, message);
        assert_eq!(delivered.content, "ready after trust");
    }

    #[tokio::test]
    async fn trusted_hello_allows_current_clipboard_to_publish_to_new_peer() {
        let state = AppState::new();
        let mut config = crate::models::AppConfig::default();
        config.device_id = "device-a".to_string();
        config.device_name = "Laptop A".to_string();
        config.trusted_devices.push("device-old".to_string());
        config.trusted_devices.push("device-new".to_string());
        state.set_config(config.clone()).await;

        let (old_sender, mut old_outbound) = mpsc::unbounded_channel();
        let old_join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("old-connection".to_string(), old_sender, old_join)
            .await;
        state
            .attach_peer_device("old-connection", "device-old".to_string(), None)
            .await;

        let first =
            publish_local_text_if_needed(&state, &config, "current clipboard".to_string())
                .await
                .expect("existing trusted peer should receive current clipboard");
        assert_eq!(first.content, "current clipboard");
        assert!(old_outbound.try_recv().is_ok());

        let (new_sender, mut new_outbound) = mpsc::unbounded_channel();
        let new_join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.33.156:51234".to_string(), new_sender, new_join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-new".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        assert!(
            publish_local_text_if_needed(&state, &config, "current clipboard".to_string())
                .await
                .is_none()
        );
        assert!(new_outbound.try_recv().is_err());

        assert!(
            reset_local_observation_if_peer_is_trusted(
                &state,
                &config,
                "10.194.33.156:51234",
                "device-new",
                "ws://10.194.33.156:8765/",
            )
            .await
        );

        let republished =
            publish_local_text_if_needed(&state, &config, "current clipboard".to_string())
                .await
                .expect("trusted Hello should allow current clipboard to reach new peer");
        let delivered_to_new = ClipboardMessage::try_from(new_outbound.try_recv().unwrap())
            .expect("new peer should receive current clipboard after trusted Hello");

        assert_eq!(republished.content, "current clipboard");
        assert_eq!(delivered_to_new, republished);
    }

    #[test]
    fn reset_local_observation_allows_current_clipboard_to_publish_after_trust() {
        let mut engine = SyncEngine::new("device-a", "Laptop A");

        let first = engine
            .observe_local_text("ready before trust")
            .expect("initial local content is observed");
        assert_eq!(first.content, "ready before trust");
        assert!(engine.observe_local_text("ready before trust").is_none());

        engine.reset_local_observation();

        let after_reset = engine
            .observe_local_text("ready before trust")
            .expect("same content should publish again after trust reset");
        assert_eq!(after_reset.content, "ready before trust");
    }

    #[test]
    fn auto_sync_starts_only_when_enabled_and_stopped() {
        let mut config = crate::models::AppConfig::default();

        assert!(should_auto_start_sync(&config, false));
        assert!(!should_auto_start_sync(&config, true));

        config.auto_sync = false;
        assert!(!should_auto_start_sync(&config, false));
    }

    #[test]
    fn manual_connect_starts_sync_runtime_when_stopped() {
        assert!(should_start_sync_for_manual_connect(false));
        assert!(!should_start_sync_for_manual_connect(true));
    }

    #[test]
    fn connection_failure_message_points_to_remote_sync_and_firewall() {
        let message = peer_connection_failure_message(
            "10.194.33.156",
            8765,
            "connection refused",
        );

        assert!(message.contains("10.194.33.156:8765"));
        assert!(message.contains("对方已开启同步"));
        assert!(message.contains("对方主面板显示的本机地址"));
        assert!(message.contains("Windows 防火墙"));
        assert!(message.contains("connection refused"));
    }

    #[tokio::test]
    async fn wait_for_sync_ready_returns_when_runtime_is_running() {
        let state = AppState::new();
        state
            .set_running(true, Some("127.0.0.1".to_string()), "ready".to_string())
            .await;

        assert!(wait_for_sync_ready(&state, Duration::from_millis(10))
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn wait_for_sync_ready_returns_runtime_error() {
        let state = AppState::new();
        state.set_error("bind failed".to_string()).await;

        let error = wait_for_sync_ready(&state, Duration::from_millis(10))
            .await
            .expect_err("runtime error should be returned");

        assert!(error.to_string().contains("bind failed"));
    }

    async fn send_wire_message<S>(socket: &mut WebSocketStream<S>, message: WireMessage)
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let text = network::encode_wire_message(&message).unwrap();
        socket.send(Message::Text(text.into())).await.unwrap();
    }

    async fn read_wire_message<S>(socket: &mut WebSocketStream<S>) -> WireMessage
    where
        S: AsyncRead + AsyncWrite + Unpin,
    {
        let message = socket.next().await.unwrap().unwrap();
        let text = message.into_text().unwrap();
        network::decode_wire_message(&text).unwrap()
    }
}
