use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use chrono::Utc;
use tokio::{
    sync::{mpsc, watch, Mutex, RwLock},
};
use tauri::async_runtime::JoinHandle;

use crate::{
    config,
    device_store,
    error::{AppError, AppResult},
    history,
    models::{
        AppConfig, AppStatus, DeviceInfo, DeviceStatus, HistoryItem, SyncState, WireMessage,
    },
    network,
    security,
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
    pending_peer_identities: Mutex<HashMap<String, PendingPeerIdentity>>,
    ignored_peer_connections: Mutex<HashSet<String>>,
    manual_trust_required: Mutex<HashSet<String>>,
}

struct RuntimeHandle {
    id: String,
    stop: watch::Sender<bool>,
    join: JoinHandle<()>,
}

struct PeerHandle {
    sender: mpsc::UnboundedSender<WireMessage>,
    join: JoinHandle<()>,
    device_id: Option<String>,
    endpoint: Option<String>,
}

struct PendingPeerIdentity {
    device_id: String,
    endpoint: Option<String>,
}

impl AppState {
    pub fn new() -> Self {
        let config = AppConfig::default();
        let device_id = config.device_id.clone();
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
                pending_peer_identities: Mutex::new(HashMap::new()),
                ignored_peer_connections: Mutex::new(HashSet::new()),
                manual_trust_required: Mutex::new(HashSet::new()),
            }),
        }
    }

    pub async fn load_from_disk(&self, app: &tauri::AppHandle) -> AppResult<()> {
        let config = config::load_config(app)?;
        let device_id = config.device_id.clone();
        let history = history::load_history(app)?;
        let devices = device_store::load_devices(app)?;

        *self.inner.config.write().await = config.clone();
        *self.inner.history.write().await = history;
        self.replace_devices(devices).await;
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
        let config = self.config().await;
        let peers = self.inner.peers.lock().await;
        status.connected_count = peers.len();
        if status.running && peers.is_empty() {
            status.message = Some("正在监听，等待设备连接".to_string());
        } else if status.running {
            let manual_trust_required = self.inner.manual_trust_required.lock().await;
            status.message = Some(if peers
                .iter()
                .any(|(connection_id, peer)| {
                    peer_is_trusted(&config, connection_id, peer, &manual_trust_required)
                })
            {
                "已信任对方设备，可发送本机剪贴板；要双向同步，请确认对方也信任本机".to_string()
            } else {
                "连接已建立，等待两台电脑互相信任设备".to_string()
            });
        }
        status
    }

    pub async fn config(&self) -> AppConfig {
        self.inner.config.read().await.clone()
    }

    pub async fn set_config(&self, config: AppConfig) {
        let device_id = config.device_id.clone();
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

    pub async fn replace_devices(&self, devices: Vec<DeviceInfo>) {
        let mut device_map = self.inner.devices.write().await;
        device_map.clear();
        for device in devices {
            device_map.insert(device.id.clone(), device);
        }
    }

    pub async fn devices(&self) -> Vec<DeviceInfo> {
        self.inner.devices.read().await.values().cloned().collect()
    }

    pub async fn connected_device_for_endpoint(
        &self,
        ip: &str,
        port: u16,
    ) -> AppResult<Option<DeviceInfo>> {
        let endpoint = network::normalize_peer_endpoint(ip, port)?;
        let endpoint_url = url::Url::parse(&endpoint)?;
        let endpoint_port = endpoint_url.port().unwrap_or(port);
        let display_ip = network::display_host_from_connection_id(&endpoint);
        let candidate = DeviceInfo {
            id: endpoint.clone(),
            name: endpoint,
            ip: display_ip,
            port: endpoint_port,
            connected: true,
            trusted: false,
            last_seen_at: None,
            status: DeviceStatus::Online,
        };

        Ok(self
            .inner
            .devices
            .read()
            .await
            .values()
            .find(|device| device.connected && same_device_endpoint(device, &candidate))
            .cloned())
    }

    pub async fn upsert_device(&self, device: DeviceInfo) -> DeviceInfo {
        let mut devices = self.inner.devices.write().await;
        let existing = devices
            .values()
            .find(|existing| existing.id == device.id || same_device_endpoint(existing, &device))
            .cloned();
        let device = if let Some(existing) = existing {
            merge_device(existing, device)
        } else {
            device
        };
        let duplicate_ids = devices
            .values()
            .filter(|existing| existing.id != device.id && same_device_endpoint(existing, &device))
            .map(|existing| existing.id.clone())
            .collect::<Vec<_>>();

        for duplicate_id in duplicate_ids {
            devices.remove(&duplicate_id);
        }

        devices.insert(device.id.clone(), device.clone());
        device
    }

    pub async fn mark_device_disconnected(&self, device_id: &str) -> Option<DeviceInfo> {
        let default_port = self.config().await.port;
        let mut devices = self.inner.devices.write().await;
        let device_id = device_id_for_key(&devices, device_id, default_port)?;
        let device = devices.get_mut(&device_id)?;
        device.connected = false;
        device.status = DeviceStatus::Offline;
        device.last_seen_at = Some(Utc::now());
        Some(device.clone())
    }

    pub async fn mark_device_trusted(&self, device_id: &str) -> Option<DeviceInfo> {
        let default_port = self.config().await.port;
        let mut devices = self.inner.devices.write().await;
        let device_id = device_id_for_key(&devices, device_id, default_port)?;
        let device = devices.get_mut(&device_id)?;
        device.trusted = true;
        device.last_seen_at = Some(Utc::now());
        Some(device.clone())
    }

    pub async fn mark_manual_trust_required(&self, connection_id: &str) {
        self.inner
            .manual_trust_required
            .lock()
            .await
            .insert(connection_id.to_string());
    }

    pub async fn clear_manual_trust_required(&self, device_key: &str) {
        let mut keys = vec![device_key.to_string()];
        {
            let peers = self.inner.peers.lock().await;
            for (connection_id, peer) in peers.iter() {
                if connection_id == device_key || peer_matches_key(peer, device_key) {
                    push_unique_key(&mut keys, connection_id);
                    if let Some(device_id) = peer.device_id.as_deref() {
                        push_unique_key(&mut keys, device_id);
                    }
                    if let Some(endpoint) = peer.endpoint.as_deref() {
                        push_unique_key(&mut keys, endpoint);
                    }
                }
            }
        }
        {
            let pending = self.inner.pending_peer_identities.lock().await;
            for (connection_id, identity) in pending.iter() {
                if pending_identity_matches_key(connection_id, identity, device_key) {
                    push_unique_key(&mut keys, connection_id);
                    push_unique_key(&mut keys, &identity.device_id);
                    if let Some(endpoint) = identity.endpoint.as_deref() {
                        push_unique_key(&mut keys, endpoint);
                    }
                }
            }
        }

        let mut manual_trust_required = self.inner.manual_trust_required.lock().await;
        for key in keys {
            manual_trust_required.remove(&key);
        }
    }

    pub async fn manual_trust_required_for_peer(
        &self,
        connection_id: &str,
        device_id: &str,
        endpoint: &str,
    ) -> bool {
        let manual_trust_required = self.inner.manual_trust_required.lock().await;
        manual_trust_required_matches_peer(
            &manual_trust_required,
            connection_id,
            Some(device_id),
            Some(endpoint),
        )
    }

    pub async fn remove_device(&self, device_id: &str) -> Option<DeviceInfo> {
        let default_port = self.config().await.port;
        let mut devices = self.inner.devices.write().await;
        let device_id = device_id_for_key(&devices, device_id, default_port)?;
        devices.remove(&device_id)
    }

    pub async fn apply_remote_clipboard(&self, message: &crate::models::ClipboardMessage) -> bool {
        self.inner.sync_engine.lock().await.apply_remote_message(message)
    }

    pub async fn observe_local_text(&self, text: String) -> Option<crate::models::ClipboardMessage> {
        self.inner.sync_engine.lock().await.observe_local_text(text)
    }

    pub async fn observe_local_image(
        &self,
        image_base64: String,
    ) -> Option<crate::models::ClipboardMessage> {
        self.inner
            .sync_engine
            .lock()
            .await
            .observe_local_image(image_base64)
    }

    pub async fn reset_local_clipboard_observation(&self) {
        self.inner.sync_engine.lock().await.reset_local_observation();
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

    pub async fn set_local_ip(&self, local_ip: Option<String>) {
        self.inner.status.write().await.local_ip = local_ip;
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

    pub async fn start_runtime(
        &self,
        id: String,
        stop: watch::Sender<bool>,
        join: JoinHandle<()>,
    ) -> AppResult<()> {
        let mut runtime = self.inner.runtime.lock().await;
        if runtime.is_some() {
            let _ = stop.send(true);
            join.abort();
            return Err(AppError::AlreadyRunning);
        }
        *runtime = Some(RuntimeHandle { id, stop, join });
        Ok(())
    }

    pub async fn clear_runtime(&self, id: &str) {
        let mut runtime = self.inner.runtime.lock().await;
        let should_clear = runtime
            .as_ref()
            .map(|handle| handle.id == id)
            .unwrap_or(false);
        if should_clear {
            let _ = runtime.take();
        }
    }

    pub async fn stop_runtime(&self) -> AppResult<()> {
        let handle = self.inner.runtime.lock().await.take();
        let Some(handle) = handle else {
            return Err(AppError::NotRunning);
        };

        let _ = handle.stop.send(true);
        let _ = handle.join.await;

        let mut peers = self.inner.peers.lock().await;
        let disconnected_ids = peers
            .iter()
            .map(|(connection_id, peer)| {
                peer.device_id
                    .clone()
                    .unwrap_or_else(|| connection_id.clone())
            })
            .collect::<Vec<_>>();
        for (_, peer) in peers.drain() {
            peer.join.abort();
        }
        drop(peers);
        for device_id in disconnected_ids {
            let _ = self.mark_device_disconnected(&device_id).await;
        }
        Ok(())
    }

    pub async fn register_peer(
        &self,
        connection_id: String,
        sender: mpsc::UnboundedSender<WireMessage>,
        join: JoinHandle<()>,
    ) {
        if self
            .inner
            .ignored_peer_connections
            .lock()
            .await
            .remove(&connection_id)
        {
            drop(sender);
            join.abort();
            return;
        }

        let pending = self
            .inner
            .pending_peer_identities
            .lock()
            .await
            .remove(&connection_id);
        let device_id = pending.as_ref().map(|identity| identity.device_id.clone());
        let endpoint = pending.and_then(|identity| identity.endpoint);
        let mut peers = self.inner.peers.lock().await;
        peers.insert(
            connection_id.clone(),
            PeerHandle {
                sender,
                join,
                device_id: device_id.clone(),
                endpoint: endpoint.clone(),
            },
        );
        if let Some(device_id) = device_id.as_deref() {
            remove_duplicate_peer_connections(
                &mut peers,
                &connection_id,
                device_id,
                endpoint.as_deref(),
            );
        }
    }

    pub async fn attach_peer_device(
        &self,
        connection_id: &str,
        device_id: String,
        endpoint: Option<String>,
    ) {
        let attached_to_peer = {
            let mut peers = self.inner.peers.lock().await;
            if let Some(peer) = peers.get_mut(connection_id) {
                peer.device_id = Some(device_id.clone());
                peer.endpoint = endpoint.clone();

                remove_duplicate_peer_connections(
                    &mut peers,
                    connection_id,
                    &device_id,
                    endpoint.as_deref(),
                );

                true
            } else {
                false
            }
        };

        if attached_to_peer {
            let mut pending = self.inner.pending_peer_identities.lock().await;
            remove_pending_peer_identities(
                &mut pending,
                connection_id,
                &device_id,
                endpoint.as_deref(),
            );
            return;
        }

        let mut pending = self.inner.pending_peer_identities.lock().await;
        remove_pending_peer_identities(
            &mut pending,
            connection_id,
            &device_id,
            endpoint.as_deref(),
        );
        pending.insert(connection_id.to_string(), PendingPeerIdentity { device_id, endpoint });
    }

    pub async fn remove_peer(&self, connection_id: &str) {
        let removed_pending_ids = {
            let mut pending = self.inner.pending_peer_identities.lock().await;
            remove_pending_peer_by_key(&mut pending, connection_id)
        };
        if !removed_pending_ids.is_empty() {
            self.inner
                .ignored_peer_connections
                .lock()
                .await
                .extend(removed_pending_ids);
        }
        let mut peers = self.inner.peers.lock().await;
        let mut disconnected_keys = Vec::new();
        if let Some(peer) = peers.remove(connection_id) {
            disconnected_keys.push(peer_disconnect_key(&peer, connection_id));
        } else {
            let matching_ids = peers
                .iter()
                .filter(|(_, peer)| peer_matches_key(peer, connection_id))
                .map(|(peer_id, _)| peer_id.clone())
                .collect::<Vec<_>>();

            for peer_id in matching_ids {
                if let Some(peer) = peers.remove(&peer_id) {
                    disconnected_keys.push(peer_disconnect_key(&peer, &peer_id));
                }
            }
        }
        drop(peers);

        for device_key in disconnected_keys {
            let _ = self.mark_device_disconnected(&device_key).await;
        }
    }

    pub async fn forget_peer(&self, connection_id: &str) -> Option<String> {
        self.inner
            .peers
            .lock()
            .await
            .remove(connection_id)
            .and_then(|peer| peer.device_id)
    }

    pub async fn broadcast(&self, message: WireMessage) {
        let peers = self.inner.peers.lock().await;
        for peer in peers.values() {
            let _ = peer.sender.send(message.clone());
        }
    }

    pub async fn has_trusted_peers(&self, config: &AppConfig) -> bool {
        let peers = self.inner.peers.lock().await;
        let manual_trust_required = self.inner.manual_trust_required.lock().await;

        peers
            .iter()
            .any(|(connection_id, peer)| {
                peer_is_trusted(config, connection_id, peer, &manual_trust_required)
            })
    }

    pub async fn broadcast_trusted(&self, config: &AppConfig, message: WireMessage) {
        let peers = self.inner.peers.lock().await;
        let manual_trust_required = self.inner.manual_trust_required.lock().await;
        for (connection_id, peer) in peers.iter() {
            if peer_is_trusted(config, connection_id, peer, &manual_trust_required) {
                let _ = peer.sender.send(message.clone());
            }
        }
    }

    pub async fn clipboard_sender_is_trusted(
        &self,
        config: &AppConfig,
        connection_id: &str,
        source_device_id: &str,
    ) -> bool {
        if security::is_device_id_trusted(config, source_device_id) {
            return true;
        }

        let peers = self.inner.peers.lock().await;
        let manual_trust_required = self.inner.manual_trust_required.lock().await;
        peers
            .get(connection_id)
            .map(|peer| peer_is_trusted(config, connection_id, peer, &manual_trust_required))
            .unwrap_or(false)
    }

    pub async fn peer_connection_matches_device(
        &self,
        connection_id: &str,
        device_id: &str,
    ) -> bool {
        self.inner
            .peers
            .lock()
            .await
            .get(connection_id)
            .map(|peer| peer.device_id.as_deref() == Some(device_id))
            .unwrap_or(false)
    }

    pub async fn trust_keys_for_device(&self, device_id: &str) -> Vec<String> {
        let mut keys = vec![device_id.to_string()];
        let default_port = self.config().await.port;
        {
            let devices = self.inner.devices.read().await;
            if let Some(device_key) = device_id_for_key(&devices, device_id, default_port) {
                if let Some(device) = devices.get(&device_key) {
                    push_unique_key(&mut keys, &device.id);
                    if let Ok(endpoint) = network::endpoint_from_connection_id(&device.ip, device.port)
                    {
                        push_unique_key(&mut keys, &endpoint);
                    }
                }
            }
        }

        {
            let peers = self.inner.peers.lock().await;
            for (connection_id, peer) in peers.iter() {
                if connection_id == device_id || peer_matches_key(peer, device_id) {
                    push_unique_key(&mut keys, connection_id);
                    if let Some(peer_device_id) = peer.device_id.as_deref() {
                        push_unique_key(&mut keys, peer_device_id);
                    }
                    if let Some(endpoint) = peer.endpoint.as_deref() {
                        push_unique_key(&mut keys, endpoint);
                    }
                }
            }
        }

        {
            let pending = self.inner.pending_peer_identities.lock().await;
            for (connection_id, identity) in pending.iter() {
                if pending_identity_matches_key(connection_id, identity, device_id) {
                    push_unique_key(&mut keys, connection_id);
                    push_unique_key(&mut keys, &identity.device_id);
                    if let Some(endpoint) = identity.endpoint.as_deref() {
                        push_unique_key(&mut keys, endpoint);
                    }
                }
            }
        }

        keys
    }
}

fn peer_is_trusted(
    config: &AppConfig,
    connection_id: &str,
    peer: &PeerHandle,
    manual_trust_required: &HashSet<String>,
) -> bool {
    if manual_trust_required_matches_peer(
        manual_trust_required,
        connection_id,
        peer.device_id.as_deref(),
        peer.endpoint.as_deref(),
    ) {
        return false;
    }

    peer.device_id
        .as_deref()
        .map(|device_id| security::is_device_id_trusted(config, device_id))
        .unwrap_or(false)
}

fn manual_trust_required_matches_peer(
    manual_trust_required: &HashSet<String>,
    connection_id: &str,
    device_id: Option<&str>,
    endpoint: Option<&str>,
) -> bool {
    manual_trust_required.iter().any(|key| {
        key == connection_id
            || device_id.map(|device_id| key == device_id).unwrap_or(false)
            || endpoint.map(|endpoint| key == endpoint).unwrap_or(false)
    })
}

fn push_unique_key(keys: &mut Vec<String>, key: &str) {
    if !keys.iter().any(|existing| existing == key) {
        keys.push(key.to_string());
    }
}

fn peer_matches_identity(peer: &PeerHandle, device_id: &str, endpoint: Option<&str>) -> bool {
    peer.device_id.as_deref() == Some(device_id)
        || endpoint
            .map(|endpoint| peer.endpoint.as_deref() == Some(endpoint))
            .unwrap_or(false)
}

fn peer_matches_key(peer: &PeerHandle, key: &str) -> bool {
    peer.device_id.as_deref() == Some(key) || peer.endpoint.as_deref() == Some(key)
}

fn peer_disconnect_key(peer: &PeerHandle, connection_id: &str) -> String {
    peer.device_id
        .clone()
        .or_else(|| peer.endpoint.clone())
        .unwrap_or_else(|| connection_id.to_string())
}

fn remove_duplicate_peer_connections(
    peers: &mut HashMap<String, PeerHandle>,
    connection_id: &str,
    device_id: &str,
    endpoint: Option<&str>,
) {
    let duplicate_ids = peers
        .iter()
        .filter(|(peer_id, peer)| {
            peer_id.as_str() != connection_id && peer_matches_identity(peer, device_id, endpoint)
        })
        .map(|(peer_id, _)| peer_id.clone())
        .collect::<Vec<_>>();

    for duplicate_id in duplicate_ids {
        if let Some(peer) = peers.remove(&duplicate_id) {
            peer.join.abort();
        }
    }
}

fn pending_identity_matches(
    identity: &PendingPeerIdentity,
    device_id: &str,
    endpoint: Option<&str>,
) -> bool {
    identity.device_id == device_id
        || endpoint
            .map(|endpoint| identity.endpoint.as_deref() == Some(endpoint))
            .unwrap_or(false)
}

fn pending_identity_matches_key(
    connection_id: &str,
    identity: &PendingPeerIdentity,
    key: &str,
) -> bool {
    connection_id == key
        || identity.device_id == key
        || identity.endpoint.as_deref() == Some(key)
}

fn remove_pending_peer_by_key(
    pending: &mut HashMap<String, PendingPeerIdentity>,
    key: &str,
) -> Vec<String> {
    let matching_ids = pending
        .iter()
        .filter(|(connection_id, identity)| {
            pending_identity_matches_key(connection_id, identity, key)
        })
        .map(|(connection_id, _)| connection_id.clone())
        .collect::<Vec<_>>();

    for matching_id in &matching_ids {
        pending.remove(matching_id);
    }

    matching_ids
}

fn remove_pending_peer_identities(
    pending: &mut HashMap<String, PendingPeerIdentity>,
    connection_id: &str,
    device_id: &str,
    endpoint: Option<&str>,
) {
    let duplicate_ids = pending
        .iter()
        .filter(|(peer_id, identity)| {
            peer_id.as_str() != connection_id
                && pending_identity_matches(identity, device_id, endpoint)
        })
        .map(|(peer_id, _)| peer_id.clone())
        .collect::<Vec<_>>();

    for duplicate_id in duplicate_ids {
        pending.remove(&duplicate_id);
    }
}

fn same_device_endpoint(left: &DeviceInfo, right: &DeviceInfo) -> bool {
    left.ip == right.ip && left.port == right.port
}

fn device_id_for_key(
    devices: &HashMap<String, DeviceInfo>,
    key: &str,
    default_port: u16,
) -> Option<String> {
    if devices.contains_key(key) {
        return Some(key.to_string());
    }

    let candidate = device_candidate_from_key(key, default_port)?;
    devices
        .values()
        .find(|device| same_device_endpoint(device, &candidate))
        .map(|device| device.id.clone())
}

fn device_candidate_from_key(key: &str, default_port: u16) -> Option<DeviceInfo> {
    let endpoint = network::normalize_peer_endpoint(key, default_port).ok()?;
    let url = url::Url::parse(&endpoint).ok()?;
    Some(DeviceInfo {
        id: key.to_string(),
        name: key.to_string(),
        ip: network::display_host_from_connection_id(&endpoint),
        port: url.port().unwrap_or(default_port),
        connected: true,
        trusted: false,
        last_seen_at: None,
        status: DeviceStatus::Online,
    })
}

fn merge_device(existing: DeviceInfo, incoming: DeviceInfo) -> DeviceInfo {
    if existing.connected && existing.trusted && !incoming.trusted {
        return DeviceInfo {
            connected: true,
            last_seen_at: incoming.last_seen_at.or(existing.last_seen_at),
            status: DeviceStatus::Online,
            ..existing
        };
    }

    DeviceInfo {
        trusted: incoming.trusted || (existing.connected && existing.trusted),
        connected: incoming.connected || existing.connected,
        status: if incoming.connected || existing.connected {
            DeviceStatus::Online
        } else {
            incoming.status
        },
        ..incoming
    }
}

#[cfg(test)]
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
        "copyshare".to_string()
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
    use super::*;

    #[test]
    fn device_id_is_stable_and_url_safe() {
        assert_eq!(device_id_from_name(" Office PC "), "office-pc");
        assert_eq!(device_id_from_name("中文设备"), "copyshare");
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::*;

    #[tokio::test]
    async fn duplicate_runtime_is_rejected_without_stopping_existing_runtime() {
        let state = AppState::new();
        let (first_stop, mut first_rx) = watch::channel(false);
        let first_join = tauri::async_runtime::spawn(async move {
            let _ = first_rx.changed().await;
        });
        state
            .start_runtime("first".to_string(), first_stop, first_join)
            .await
            .unwrap();

        let (second_stop, mut second_rx) = watch::channel(false);
        let second_join = tauri::async_runtime::spawn(async move {
            let _ = second_rx.changed().await;
        });

        let result = state
            .start_runtime("second".to_string(), second_stop, second_join)
            .await;
        assert!(matches!(result, Err(AppError::AlreadyRunning)));
        assert!(state.stop_runtime().await.is_ok());
    }

    #[tokio::test]
    async fn stop_runtime_marks_connected_peer_devices_offline() {
        let state = AppState::new();
        let (stop, mut stop_rx) = watch::channel(false);
        let runtime_join = tauri::async_runtime::spawn(async move {
            let _ = stop_rx.changed().await;
        });
        state
            .start_runtime("runtime".to_string(), stop, runtime_join)
            .await
            .unwrap();

        let (sender, _receiver) = mpsc::unbounded_channel();
        let peer_join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.33.156:51234".to_string(), sender, peer_join)
            .await;
        state
            .attach_peer_device("10.194.33.156:51234", "device-remote".to_string(), None)
            .await;
        state
            .upsert_device(DeviceInfo {
                id: "device-remote".to_string(),
                name: "CopyShare".to_string(),
                ip: "10.194.33.156".to_string(),
                port: 8765,
                connected: true,
                trusted: true,
                last_seen_at: Some(Utc::now()),
                status: DeviceStatus::Online,
            })
            .await;

        state.stop_runtime().await.unwrap();

        let devices = state.devices().await;
        assert_eq!(state.status().await.connected_count, 0);
        assert_eq!(devices.len(), 1);
        assert!(!devices[0].connected);
        assert_eq!(devices[0].status, DeviceStatus::Offline);
    }

    #[tokio::test]
    async fn running_status_without_peers_reports_waiting_for_connection() {
        let state = AppState::new();
        state
            .set_running(
                true,
                Some("10.194.34.119".to_string()),
                "正在监听端口 8765".to_string(),
            )
            .await;

        let status = state.status().await;

        assert_eq!(status.connected_count, 0);
        assert_eq!(status.message.as_deref(), Some("正在监听，等待设备连接"));
    }

    #[tokio::test]
    async fn connected_device_for_endpoint_requires_same_listening_endpoint() {
        let state = AppState::new();
        state
            .upsert_device(DeviceInfo {
                id: "device-remote".to_string(),
                name: "CopyShare".to_string(),
                ip: "10.194.33.156".to_string(),
                port: 8765,
                connected: true,
                trusted: true,
                last_seen_at: Some(Utc::now()),
                status: DeviceStatus::Online,
            })
            .await;

        let device = state
            .connected_device_for_endpoint("10.194.33.156", 51234)
            .await
            .unwrap();

        assert!(device.is_none());
        assert_eq!(state.devices().await.len(), 1);
    }
}

#[cfg(test)]
mod runtime_clear_tests {
    use super::*;

    #[tokio::test]
    async fn runtime_can_be_registered_again_after_clear() {
        let state = AppState::new();
        let (first_stop, mut first_rx) = watch::channel(false);
        let first_join = tauri::async_runtime::spawn(async move {
            let _ = first_rx.changed().await;
        });
        state
            .start_runtime("first".to_string(), first_stop, first_join)
            .await
            .unwrap();
        state.clear_runtime("first").await;

        let (second_stop, mut second_rx) = watch::channel(false);
        let second_join = tauri::async_runtime::spawn(async move {
            let _ = second_rx.changed().await;
        });

        assert!(state
            .start_runtime("second".to_string(), second_stop, second_join)
            .await
            .is_ok());
        assert!(state.stop_runtime().await.is_ok());
    }
}

#[cfg(test)]
mod runtime_id_tests {
    use super::*;

    #[tokio::test]
    async fn clear_runtime_ignores_other_runtime_ids() {
        let state = AppState::new();
        let (first_stop, mut first_rx) = watch::channel(false);
        let first_join = tauri::async_runtime::spawn(async move {
            let _ = first_rx.changed().await;
        });
        state
            .start_runtime("first".to_string(), first_stop, first_join)
            .await
            .unwrap();

        state.clear_runtime("other").await;

        let (second_stop, mut second_rx) = watch::channel(false);
        let second_join = tauri::async_runtime::spawn(async move {
            let _ = second_rx.changed().await;
        });
        let result = state
            .start_runtime("second".to_string(), second_stop, second_join)
            .await;

        assert!(matches!(result, Err(AppError::AlreadyRunning)));
        assert!(state.stop_runtime().await.is_ok());
    }
}

#[cfg(test)]
mod status_diagnostics_tests {
    use super::*;

    #[tokio::test]
    async fn running_status_explains_when_connected_peer_is_not_trusted() {
        let state = AppState::new();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .set_running(true, Some("127.0.0.1".to_string()), "listening".to_string())
            .await;
        state
            .register_peer("10.0.0.2:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device("10.0.0.2:51234", "device-b".to_string(), None)
            .await;

        let status = state.status().await;

        assert_eq!(status.connected_count, 1);
        assert_eq!(
            status.message.as_deref(),
            Some("连接已建立，等待两台电脑互相信任设备")
        );
    }

    #[tokio::test]
    async fn running_status_reports_syncing_when_connected_peer_is_trusted() {
        let state = AppState::new();
        let mut config = AppConfig::default();
        config.trusted_devices.push("device-b".to_string());
        state.set_config(config).await;
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .set_running(true, Some("127.0.0.1".to_string()), "listening".to_string())
            .await;
        state
            .register_peer("10.0.0.2:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device("10.0.0.2:51234", "device-b".to_string(), None)
            .await;

        let status = state.status().await;

        assert_eq!(status.connected_count, 1);
        assert_eq!(
            status.message.as_deref(),
            Some("已信任对方设备，可发送本机剪贴板；要双向同步，请确认对方也信任本机")
        );
    }
}

#[cfg(test)]
mod device_identity_tests {
    #[test]
    fn default_named_devices_get_distinct_ids() {
        let first = crate::models::AppConfig::default();
        let second = crate::models::AppConfig::default();

        assert_ne!(first.device_id, second.device_id);
        assert_eq!(first.device_name, second.device_name);
    }
}

#[cfg(test)]
mod device_dedup_tests {
    use super::*;

    fn device(id: &str, connected: bool) -> DeviceInfo {
        DeviceInfo {
            id: id.to_string(),
            name: "CopyShare".to_string(),
            ip: "10.194.33.156".to_string(),
            port: 8765,
            connected,
            trusted: false,
            last_seen_at: Some(Utc::now()),
            status: if connected {
                DeviceStatus::Online
            } else {
                DeviceStatus::Offline
            },
        }
    }

    #[tokio::test]
    async fn upsert_device_replaces_same_endpoint_even_when_id_changes() {
        let state = AppState::new();

        state
            .upsert_device(device("10.194.33.156:51234", false))
            .await;
        state.upsert_device(device("device-remote", true)).await;

        let devices = state.devices().await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "device-remote");
        assert!(devices[0].connected);
    }

    #[tokio::test]
    async fn upsert_device_keeps_existing_trusted_connection_on_reconnect() {
        let state = AppState::new();
        let mut existing = device("device-remote", true);
        existing.trusted = true;

        state.upsert_device(existing).await;
        state
            .upsert_device(device("ws://10.194.33.156:8765/", true))
            .await;

        let devices = state.devices().await;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "device-remote");
        assert!(devices[0].trusted);
        assert!(devices[0].connected);
    }

    #[tokio::test]
    async fn upsert_device_returns_existing_trusted_connection_on_reconnect() {
        let state = AppState::new();
        let mut existing = device("device-remote", true);
        existing.trusted = true;

        state.upsert_device(existing).await;
        let merged = state
            .upsert_device(device("ws://10.194.33.156:8765/", true))
            .await;

        assert_eq!(merged.id, "device-remote");
        assert!(merged.trusted);
        assert!(merged.connected);
    }

    #[tokio::test]
    async fn upsert_device_keeps_separate_card_for_different_port_same_host() {
        let state = AppState::new();
        let mut existing = device("device-remote", true);
        existing.trusted = true;

        let mut repeated_pending = device("10.194.33.156:51234", true);
        repeated_pending.port = 51234;

        state.upsert_device(existing).await;
        let merged = state.upsert_device(repeated_pending).await;

        let devices = state.devices().await;
        assert_eq!(devices.len(), 2);
        assert_eq!(merged.id, "10.194.33.156:51234");
        assert!(!merged.trusted);
        assert!(merged.connected);
        assert!(devices.iter().any(|device| device.id == "device-remote"));
        assert!(devices
            .iter()
            .any(|device| device.id == "10.194.33.156:51234" && !device.trusted));
    }

    #[tokio::test]
    async fn upsert_untrusted_connection_does_not_inherit_trust_from_same_host() {
        let state = AppState::new();
        let mut existing = device("old-device", false);
        existing.trusted = true;

        let mut incoming = device("new-device", true);
        incoming.port = 8766;

        state.upsert_device(existing).await;
        let merged = state.upsert_device(incoming).await;

        let devices = state.devices().await;
        assert_eq!(merged.id, "new-device");
        assert!(!merged.trusted);
        assert_eq!(devices.len(), 2);
        assert!(devices.iter().any(|device| device.id == "old-device"));
        assert!(devices
            .iter()
            .any(|device| device.id == "new-device" && !device.trusted));
    }

    #[tokio::test]
    async fn remove_peer_accepts_attached_device_id() {
        let state = AppState::new();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .register_peer("10.194.33.156:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device("10.194.33.156:51234", "device-remote".to_string(), None)
            .await;

        state.remove_peer("device-remote").await;

        assert_eq!(state.status().await.connected_count, 0);
    }

    #[tokio::test]
    async fn remove_peer_marks_attached_device_offline() {
        let state = AppState::new();
        let mut existing = device("device-remote", true);
        existing.trusted = true;
        state.upsert_device(existing).await;

        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .register_peer("10.194.33.156:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device("10.194.33.156:51234", "device-remote".to_string(), None)
            .await;

        state.remove_peer("device-remote").await;

        let devices = state.devices().await;
        assert_eq!(state.status().await.connected_count, 0);
        assert_eq!(devices.len(), 1);
        assert!(!devices[0].connected);
        assert_eq!(devices[0].status, DeviceStatus::Offline);
    }

    #[tokio::test]
    async fn remove_peer_accepts_attached_declared_endpoint() {
        let state = AppState::new();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .register_peer("10.194.33.156:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        state.remove_peer("ws://10.194.33.156:8765/").await;

        assert_eq!(state.status().await.connected_count, 0);
    }

    #[tokio::test]
    async fn remove_peer_accepts_pending_attached_device_id() {
        let state = AppState::new();

        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        state.remove_peer("device-remote").await;

        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.33.156:51234".to_string(), sender, join)
            .await;

        assert_eq!(state.status().await.connected_count, 0);
    }

    #[tokio::test]
    async fn mark_device_trusted_updates_device_list_state() {
        let state = AppState::new();
        state.upsert_device(device("device-remote", true)).await;

        let updated = state
            .mark_device_trusted("device-remote")
            .await
            .expect("device should exist");

        assert!(updated.trusted);
        assert!(state.devices().await[0].trusted);
    }

    #[tokio::test]
    async fn mark_device_trusted_accepts_endpoint_alias() {
        let state = AppState::new();
        state.upsert_device(device("device-remote", true)).await;

        let updated = state
            .mark_device_trusted("ws://10.194.33.156:8765/")
            .await
            .expect("endpoint alias should find the connected device");

        assert_eq!(updated.id, "device-remote");
        assert!(updated.trusted);
        assert!(state.devices().await[0].trusted);
    }

    #[tokio::test]
    async fn mark_device_disconnected_accepts_endpoint_alias() {
        let state = AppState::new();
        let mut existing = device("device-remote", true);
        existing.trusted = true;
        state.upsert_device(existing).await;

        let updated = state
            .mark_device_disconnected("ws://10.194.33.156:8765/")
            .await
            .expect("endpoint alias should find the connected device");

        assert_eq!(updated.id, "device-remote");
        assert!(!updated.connected);
        assert_eq!(updated.status, DeviceStatus::Offline);
    }

    #[tokio::test]
    async fn remove_device_accepts_endpoint_alias() {
        let state = AppState::new();
        state.upsert_device(device("device-remote", true)).await;

        let removed = state
            .remove_device("ws://10.194.33.156:8765/")
            .await
            .expect("endpoint alias should remove the device");

        assert_eq!(removed.id, "device-remote");
        assert!(state.devices().await.is_empty());
    }
}

#[cfg(test)]
mod trusted_broadcast_tests {
    use super::*;
    use crate::models::ClipboardContentType;

    fn clipboard_message() -> WireMessage {
        WireMessage::Clipboard {
            message_id: "message-1".to_string(),
            source_device_id: "device-local".to_string(),
            source_device_name: "Local".to_string(),
            content_type: ClipboardContentType::Text,
            content: "hello".to_string(),
            content_hash: "hash".to_string(),
            timestamp: 1,
        }
    }

    #[tokio::test]
    async fn broadcast_trusted_sends_only_to_trusted_peers() {
        let state = AppState::new();
        let mut config = AppConfig::default();
        config.trusted_devices.push("device-trusted".to_string());
        let (trusted_sender, mut trusted_receiver) = mpsc::unbounded_channel();
        let (pending_sender, mut pending_receiver) = mpsc::unbounded_channel();
        let trusted_join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        let pending_join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .register_peer("trusted-connection".to_string(), trusted_sender, trusted_join)
            .await;
        state
            .attach_peer_device("trusted-connection", "device-trusted".to_string(), None)
            .await;
        state
            .register_peer("pending-connection".to_string(), pending_sender, pending_join)
            .await;
        state
            .attach_peer_device("pending-connection", "device-pending".to_string(), None)
            .await;

        assert!(state.has_trusted_peers(&config).await);
        state.broadcast_trusted(&config, clipboard_message()).await;

        assert!(trusted_receiver.try_recv().is_ok());
        assert!(pending_receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn has_trusted_peers_is_false_for_pending_connections() {
        let state = AppState::new();
        let config = AppConfig::default();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .register_peer("pending-connection".to_string(), sender, join)
            .await;
        state
            .attach_peer_device("pending-connection", "device-pending".to_string(), None)
            .await;

        assert!(!state.has_trusted_peers(&config).await);
    }

    #[tokio::test]
    async fn manual_connect_requires_confirmation_even_for_previously_trusted_device_id() {
        let state = AppState::new();
        let mut config = AppConfig::default();
        config.trusted_devices.push("device-remote".to_string());
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .mark_manual_trust_required("ws://10.194.33.156:8765/")
            .await;
        state
            .register_peer("ws://10.194.33.156:8765/".to_string(), sender, join)
            .await;
        state
            .attach_peer_device(
                "ws://10.194.33.156:8765/",
                "device-remote".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        assert!(!state.has_trusted_peers(&config).await);

        state.clear_manual_trust_required("device-remote").await;

        assert!(state.has_trusted_peers(&config).await);
    }

    #[tokio::test]
    async fn endpoint_only_trust_does_not_make_peer_trusted() {
        let state = AppState::new();
        let mut config = AppConfig::default();
        config.trusted_devices.push("ws://10.194.33.156:8765/".to_string());
        let (sender, mut receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .register_peer("10.194.33.156:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        assert!(!state.has_trusted_peers(&config).await);
        state.broadcast_trusted(&config, clipboard_message()).await;

        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn clipboard_sender_requires_trusted_device_id_not_endpoint_only() {
        let state = AppState::new();
        let mut config = AppConfig::default();
        config.trusted_devices.push("ws://10.194.33.156:8765/".to_string());
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .register_peer("10.194.33.156:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        assert!(!state
            .clipboard_sender_is_trusted(&config, "10.194.33.156:51234", "device-remote")
            .await);
    }

    #[tokio::test]
    async fn trust_keys_fall_back_to_requested_device_id() {
        let state = AppState::new();

        assert_eq!(
            state.trust_keys_for_device("device-remote").await,
            vec!["device-remote".to_string()]
        );
    }

    #[tokio::test]
    async fn trust_keys_include_offline_device_endpoint_alias() {
        let state = AppState::new();
        state
            .upsert_device(DeviceInfo {
                id: "device-remote".to_string(),
                name: "CopyShare".to_string(),
                ip: "10.194.33.156".to_string(),
                port: 8765,
                connected: false,
                trusted: true,
                last_seen_at: Some(Utc::now()),
                status: DeviceStatus::Offline,
            })
            .await;

        assert_eq!(
            state.trust_keys_for_device("device-remote").await,
            vec![
                "device-remote".to_string(),
                "ws://10.194.33.156:8765/".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn trust_keys_include_connection_and_declared_endpoint() {
        let state = AppState::new();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .register_peer("10.194.33.156:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        assert_eq!(
            state.trust_keys_for_device("device-remote").await,
            vec![
                "device-remote".to_string(),
                "10.194.33.156:51234".to_string(),
                "ws://10.194.33.156:8765/".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn trust_keys_accept_declared_endpoint_for_attached_peer() {
        let state = AppState::new();
        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });

        state
            .register_peer("10.194.33.156:51234".to_string(), sender, join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        assert_eq!(
            state
                .trust_keys_for_device("ws://10.194.33.156:8765/")
                .await,
            vec![
                "ws://10.194.33.156:8765/".to_string(),
                "10.194.33.156:51234".to_string(),
                "device-remote".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn trust_keys_include_pending_connection_and_declared_endpoint() {
        let state = AppState::new();

        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        assert_eq!(
            state.trust_keys_for_device("device-remote").await,
            vec![
                "device-remote".to_string(),
                "10.194.33.156:51234".to_string(),
                "ws://10.194.33.156:8765/".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn peer_identity_attached_before_registration_is_not_lost() {
        let state = AppState::new();
        let mut config = AppConfig::default();
        config.trusted_devices.push("device-remote".to_string());

        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                Some("ws://10.194.33.156:8765/".to_string()),
            )
            .await;

        let (sender, _receiver) = mpsc::unbounded_channel();
        let join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.33.156:51234".to_string(), sender, join)
            .await;

        assert!(state.has_trusted_peers(&config).await);
        assert!(state
            .clipboard_sender_is_trusted(&config, "10.194.33.156:51234", "device-remote")
            .await);
    }

    #[tokio::test]
    async fn attaching_same_device_replaces_existing_peer_connection() {
        let state = AppState::new();
        let mut config = AppConfig::default();
        config.trusted_devices.push("device-remote".to_string());
        let endpoint = Some("ws://10.194.33.156:8765/".to_string());

        let (first_sender, mut first_receiver) = mpsc::unbounded_channel();
        let first_join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.33.156:51234".to_string(), first_sender, first_join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                endpoint.clone(),
            )
            .await;

        let (second_sender, mut second_receiver) = mpsc::unbounded_channel();
        let second_join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.33.156:51235".to_string(), second_sender, second_join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51235",
                "device-remote".to_string(),
                endpoint,
            )
            .await;

        assert_eq!(state.status().await.connected_count, 1);
        state.broadcast_trusted(&config, clipboard_message()).await;
        assert!(first_receiver.try_recv().is_err());
        assert!(second_receiver.try_recv().is_ok());
    }

    #[tokio::test]
    async fn pending_same_device_replaces_existing_peer_on_registration() {
        let state = AppState::new();
        let mut config = AppConfig::default();
        config.trusted_devices.push("device-remote".to_string());
        let endpoint = Some("ws://10.194.33.156:8765/".to_string());

        let (first_sender, mut first_receiver) = mpsc::unbounded_channel();
        let first_join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.33.156:51234".to_string(), first_sender, first_join)
            .await;
        state
            .attach_peer_device(
                "10.194.33.156:51234",
                "device-remote".to_string(),
                endpoint.clone(),
            )
            .await;

        state
            .attach_peer_device(
                "10.194.33.156:51235",
                "device-remote".to_string(),
                endpoint,
            )
            .await;

        let (second_sender, mut second_receiver) = mpsc::unbounded_channel();
        let second_join = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state
            .register_peer("10.194.33.156:51235".to_string(), second_sender, second_join)
            .await;

        assert_eq!(state.status().await.connected_count, 1);
        state.broadcast_trusted(&config, clipboard_message()).await;
        assert!(first_receiver.try_recv().is_err());
        assert!(second_receiver.try_recv().is_ok());
    }

    #[tokio::test]
    async fn mutually_trusted_peers_can_take_turns_broadcasting_clipboards() {
        let state_a = AppState::new();
        let mut config_a = AppConfig::default();
        config_a.device_id = "device-a".to_string();
        config_a.device_name = "Laptop A".to_string();
        config_a.trusted_devices.push("device-b".to_string());
        state_a.set_config(config_a.clone()).await;

        let state_b = AppState::new();
        let mut config_b = AppConfig::default();
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
            .attach_peer_device(
                "a-to-b",
                "device-b".to_string(),
                Some("ws://10.0.0.2:8765/".to_string()),
            )
            .await;

        let (sender_b_to_a, mut outbound_b_to_a) = mpsc::unbounded_channel();
        let join_b = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state_b
            .register_peer("b-to-a".to_string(), sender_b_to_a, join_b)
            .await;
        state_b
            .attach_peer_device(
                "b-to-a",
                "device-a".to_string(),
                Some("ws://10.0.0.1:8765/".to_string()),
            )
            .await;

        assert!(state_a.has_trusted_peers(&config_a).await);
        assert!(state_b.has_trusted_peers(&config_b).await);

        let from_a = state_a
            .observe_local_text("from A".to_string())
            .await
            .expect("device A should publish its local clipboard");
        state_a.broadcast_trusted(&config_a, from_a.into()).await;
        let delivered_to_b = outbound_a_to_b
            .try_recv()
            .expect("device B should receive device A clipboard");
        let delivered_to_b =
            crate::models::ClipboardMessage::try_from(delivered_to_b).unwrap();

        assert!(state_b
            .clipboard_sender_is_trusted(&config_b, "b-to-a", &delivered_to_b.source_device_id)
            .await);
        assert!(state_b.apply_remote_clipboard(&delivered_to_b).await);
        assert!(state_b.observe_local_text("from A".to_string()).await.is_none());

        let from_b = state_b
            .observe_local_text("from B".to_string())
            .await
            .expect("device B should publish its next local clipboard");
        state_b.broadcast_trusted(&config_b, from_b.into()).await;
        let delivered_to_a = outbound_b_to_a
            .try_recv()
            .expect("device A should receive device B clipboard");
        let delivered_to_a =
            crate::models::ClipboardMessage::try_from(delivered_to_a).unwrap();

        assert!(state_a
            .clipboard_sender_is_trusted(&config_a, "a-to-b", &delivered_to_a.source_device_id)
            .await);
        assert!(state_a.apply_remote_clipboard(&delivered_to_a).await);
        assert!(state_a.observe_local_text("from B".to_string()).await.is_none());
    }

    #[tokio::test]
    async fn endpoint_only_trust_cannot_sync_without_device_confirmation() {
        let state_a = AppState::new();
        let mut config_a = AppConfig::default();
        config_a.device_id = "device-a".to_string();
        config_a.device_name = "Laptop A".to_string();
        config_a
            .trusted_devices
            .push("ws://10.0.0.2:8765/".to_string());
        state_a.set_config(config_a.clone()).await;

        let state_b = AppState::new();
        let mut config_b = AppConfig::default();
        config_b.device_id = "device-b".to_string();
        config_b.device_name = "Laptop B".to_string();
        config_b
            .trusted_devices
            .push("ws://10.0.0.1:8765/".to_string());
        state_b.set_config(config_b.clone()).await;

        let (sender_a_to_b, mut outbound_a_to_b) = mpsc::unbounded_channel();
        let join_a = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state_a
            .register_peer("ws://10.0.0.2:8765/".to_string(), sender_a_to_b, join_a)
            .await;
        state_a
            .attach_peer_device(
                "ws://10.0.0.2:8765/",
                "device-b".to_string(),
                Some("ws://10.0.0.2:8765/".to_string()),
            )
            .await;

        let (sender_b_to_a, mut outbound_b_to_a) = mpsc::unbounded_channel();
        let join_b = tauri::async_runtime::spawn(async {
            futures_util::future::pending::<()>().await;
        });
        state_b
            .register_peer("10.0.0.1:51234".to_string(), sender_b_to_a, join_b)
            .await;
        state_b
            .attach_peer_device(
                "10.0.0.1:51234",
                "device-a".to_string(),
                Some("ws://10.0.0.1:8765/".to_string()),
            )
            .await;

        assert!(!state_a.has_trusted_peers(&config_a).await);
        assert!(!state_b.has_trusted_peers(&config_b).await);

        let from_a = state_a
            .observe_local_text("endpoint trust from A".to_string())
            .await
            .expect("device A should observe its local clipboard");
        state_a.broadcast_trusted(&config_a, from_a.into()).await;
        assert!(outbound_a_to_b.try_recv().is_err());

        let from_b = state_b
            .observe_local_text("endpoint trust from B".to_string())
            .await
            .expect("device B should observe its local clipboard");
        state_b.broadcast_trusted(&config_b, from_b.into()).await;
        assert!(outbound_b_to_a.try_recv().is_err());
    }
}
