use std::{collections::HashMap, sync::Arc};

use chrono::Utc;
use tokio::{
    sync::{mpsc, watch, Mutex, RwLock},
};
use tauri::async_runtime::JoinHandle;

use crate::{
    config,
    error::{AppError, AppResult},
    history,
    models::{
        AppConfig, AppStatus, DeviceInfo, DeviceStatus, HistoryItem, SyncState, WireMessage,
    },
    sync::SyncEngine,
};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    status: RwLock<AppStatus>,
    config: RwLock<AppConfig>,
    devices: RwLock<HashMap<String, DeviceInfo>>,
    history: RwLock<Vec<HistoryItem>>,
    sync_engine: Mutex<SyncEngine>,
    runtime: Mutex<Option<RuntimeHandle>>,
    peers: Mutex<HashMap<String, PeerHandle>>,
}

struct RuntimeHandle {
    stop: watch::Sender<bool>,
    join: JoinHandle<()>,
}

struct PeerHandle {
    sender: mpsc::UnboundedSender<WireMessage>,
    join: JoinHandle<()>,
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::default();
        let device_id = device_id_from_name(&config.device_name);
        Self {
            inner: Arc::new(AppStateInner {
                status: RwLock::new(AppStatus {
                    running: false,
                    device_name: config.device_name.clone(),
                    device_id: device_id.clone(),
                    local_ip: None,
                    port: config.port,
                    connected_count: 0,
                    last_sync_at: None,
                    state: SyncState::Stopped,
                    message: Some("等待启动同步".to_string()),
                }),
                config: RwLock::new(config.clone()),
                devices: RwLock::new(HashMap::new()),
                history: RwLock::new(Vec::new()),
                sync_engine: Mutex::new(SyncEngine::new(device_id, config.device_name)),
                runtime: Mutex::new(None),
                peers: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub async fn load_from_disk(&self, app: &tauri::AppHandle) -> AppResult<()> {
        let config = config::load_config(app)?;
        let device_id = device_id_from_name(&config.device_name);
        let history = history::load_history(app)?;

        *self.inner.config.write().await = config.clone();
        *self.inner.history.write().await = history;
        *self.inner.sync_engine.lock().await =
            SyncEngine::new(device_id.clone(), config.device_name.clone());

        let mut status = self.inner.status.write().await;
        status.device_name = config.device_name;
        status.device_id = device_id;
        status.port = config.port;
        Ok(())
    }

    pub async fn status(&self) -> AppStatus {
        let mut status = self.inner.status.read().await.clone();
        status.connected_count = self.inner.peers.lock().await.len();
        status
    }

    pub async fn config(&self) -> AppConfig {
        self.inner.config.read().await.clone()
    }

    pub async fn set_config(&self, config: AppConfig) {
        let device_id = device_id_from_name(&config.device_name);
        *self.inner.config.write().await = config.clone();
        self.inner
            .sync_engine
            .lock()
            .await
            .set_device(device_id.clone(), config.device_name.clone());

        let mut status = self.inner.status.write().await;
        status.device_name = config.device_name;
        status.device_id = device_id;
        status.port = config.port;
    }

    pub async fn history(&self) -> Vec<HistoryItem> {
        self.inner.history.read().await.clone()
    }

    pub async fn replace_history(&self, items: Vec<HistoryItem>) {
        *self.inner.history.write().await = items;
    }

    pub async fn push_history(&self, item: HistoryItem) {
        let mut items = self.inner.history.write().await;
        history::push_history(&mut items, item);
    }

    pub async fn devices(&self) -> Vec<DeviceInfo> {
        self.inner.devices.read().await.values().cloned().collect()
    }

    pub async fn upsert_device(&self, device: DeviceInfo) {
        self.inner.devices.write().await.insert(device.id.clone(), device);
    }

    pub async fn mark_device_disconnected(&self, device_id: &str) -> Option<DeviceInfo> {
        let mut devices = self.inner.devices.write().await;
        let device = devices.get_mut(device_id)?;
        device.connected = false;
        device.status = DeviceStatus::Offline;
        device.last_seen_at = Some(Utc::now());
        Some(device.clone())
    }

    pub async fn apply_remote_clipboard(&self, message: &crate::models::ClipboardMessage) -> bool {
        self.inner.sync_engine.lock().await.apply_remote_message(message)
    }

    pub async fn observe_local_text(&self, text: String) -> Option<crate::models::ClipboardMessage> {
        self.inner.sync_engine.lock().await.observe_local_text(text)
    }

    pub async fn set_running(&self, running: bool, local_ip: Option<String>, message: String) {
        let config = self.config().await;
        let mut status = self.inner.status.write().await;
        status.running = running;
        status.local_ip = local_ip;
        status.port = config.port;
        status.state = if running { SyncState::Running } else { SyncState::Stopped };
        status.message = Some(message);
    }

    pub async fn set_error(&self, message: String) {
        let mut status = self.inner.status.write().await;
        status.running = false;
        status.state = SyncState::Error;
        status.message = Some(message);
    }

    pub async fn touch_last_sync(&self) {
        self.inner.status.write().await.last_sync_at = Some(Utc::now());
    }

    pub async fn start_runtime(&self, stop: watch::Sender<bool>, join: JoinHandle<()>) -> AppResult<()> {
        let mut runtime = self.inner.runtime.lock().await;
        if runtime.is_some() {
            return Err(AppError::AlreadyRunning);
        }
        *runtime = Some(RuntimeHandle { stop, join });
        Ok(())
    }

    pub async fn stop_runtime(&self) -> AppResult<()> {
        let handle = self.inner.runtime.lock().await.take();
        let Some(handle) = handle else {
            return Err(AppError::NotRunning);
        };

        let _ = handle.stop.send(true);
        let _ = handle.join.await;

        let mut peers = self.inner.peers.lock().await;
        for (_, peer) in peers.drain() {
            peer.join.abort();
        }
        Ok(())
    }

    pub async fn register_peer(
        &self,
        connection_id: String,
        sender: mpsc::UnboundedSender<WireMessage>,
        join: JoinHandle<()>,
    ) {
        self.inner
            .peers
            .lock()
            .await
            .insert(connection_id, PeerHandle { sender, join });
    }

    pub async fn remove_peer(&self, connection_id: &str) {
        if let Some(peer) = self.inner.peers.lock().await.remove(connection_id) {
            peer.join.abort();
        }
    }

    pub async fn forget_peer(&self, connection_id: &str) {
        self.inner.peers.lock().await.remove(connection_id);
    }

    pub async fn broadcast(&self, message: WireMessage) {
        let peers = self.inner.peers.lock().await;
        for peer in peers.values() {
            let _ = peer.sender.send(message.clone());
        }
    }
}

pub fn device_id_from_name(name: &str) -> String {
    let normalized: String = name
        .trim()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '-' })
        .collect();
    let compact = normalized
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if compact.is_empty() {
        "copy-sharer".to_string()
    } else {
        compact
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::device_id_from_name;

    #[test]
    fn device_id_is_stable_and_url_safe() {
        assert_eq!(device_id_from_name(" Office PC "), "office-pc");
        assert_eq!(device_id_from_name("中文设备"), "copy-sharer");
    }
}
