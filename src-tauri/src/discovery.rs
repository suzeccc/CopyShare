use std::{
    collections::{HashMap, HashSet},
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket as StdUdpSocket},
    sync::OnceLock,
    time::Duration,
};

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use get_if_addrs::IfAddr;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::{net::UdpSocket, sync::Mutex};

use crate::{
    config as app_config,
    error::{AppError, AppResult},
    models::{AppConfig, DeviceInfo, DeviceStatus, DiscoveryScanProgress, DiscoveryScanStatus},
    network,
    notifications,
    security,
    state::AppState,
};

const DISCOVERY_PORT: u16 = 8764;
const DISCOVERY_TYPE: &str = "copyshare-discovery";
const DISCOVERY_VERSION: u8 = 1;
const DISCOVERY_ANNOUNCE_INTERVAL: Duration = Duration::from_secs(8);
const ACTIVE_DISCOVERY_WAIT: Duration = Duration::from_secs(2);
const DISCOVERY_OFFLINE_AFTER: ChronoDuration = ChronoDuration::seconds(30);
const DISCOVERY_SWEEP_INTERVAL: Duration = Duration::from_secs(5);
const SUBNET_SCAN_DELAY: Duration = Duration::from_millis(6);

static DISCOVERED_DEVICES: OnceLock<Mutex<HashMap<String, DeviceInfo>>> = OnceLock::new();
static NOTIFIED_ONLINE_DEVICES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn discovered_devices() -> &'static Mutex<HashMap<String, DeviceInfo>> {
    DISCOVERED_DEVICES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn notified_online_devices() -> &'static Mutex<HashSet<String>> {
    NOTIFIED_ONLINE_DEVICES.get_or_init(|| Mutex::new(HashSet::new()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryScanPlan {
    pub hosts: Vec<Ipv4Addr>,
    pub range_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryPayload {
    #[serde(rename = "type")]
    pub payload_type: String,
    pub version: u8,
    pub device_id: String,
    pub device_name: String,
    pub host: String,
    pub port: u16,
    pub app_version: String,
    pub timestamp: i64,
    #[serde(default = "default_discovery_action")]
    pub action: String,
    #[serde(default)]
    pub scan_ranges: Vec<String>,
}

fn default_discovery_action() -> String {
    "announce".to_string()
}

pub fn build_discovery_payload(
    device_id: String,
    device_name: String,
    host: String,
    port: u16,
    app_version: String,
    action: String,
) -> AppResult<DiscoveryPayload> {
    validate_lan_ipv4(&host)?;
    validate_port(port)?;

    let action = match action.trim() {
        "request" => "request",
        _ => "announce",
    };

    Ok(DiscoveryPayload {
        payload_type: DISCOVERY_TYPE.to_string(),
        version: DISCOVERY_VERSION,
        device_id,
        device_name,
        host,
        port,
        app_version,
        timestamp: Utc::now().timestamp(),
        action: action.to_string(),
        scan_ranges: Vec::new(),
    })
}

pub fn parse_discovery_payload(raw: &str) -> AppResult<DiscoveryPayload> {
    let payload: DiscoveryPayload = serde_json::from_str(raw).map_err(|_| {
        AppError::InvalidInput("不是有效的 CopyShare 局域网发现消息".to_string())
    })?;
    validate_discovery_payload(payload)
}

pub fn discovered_device_from_payload(
    payload: &DiscoveryPayload,
    config: &AppConfig,
) -> Option<DeviceInfo> {
    if payload.device_id == config.device_id {
        return None;
    }

    let trusted = security::is_device_id_trusted(config, &payload.device_id);
    Some(DeviceInfo {
        id: payload.device_id.clone(),
        name: payload.device_name.trim().to_string(),
        ip: payload.host.clone(),
        port: payload.port,
        connected: false,
        trusted,
        remote_trusted: false,
        has_connected_before: false,
        last_seen_at: Some(Utc::now()),
        status: DeviceStatus::Online,
    })
}

pub fn merge_discovered_device(
    devices: Vec<DeviceInfo>,
    incoming: DeviceInfo,
) -> Vec<DeviceInfo> {
    let mut next = Vec::with_capacity(devices.len() + 1);
    let mut inserted = false;

    for device in devices {
        if device.id == incoming.id || same_endpoint(&device, &incoming) {
            if !inserted {
                next.push(merge_discovered_existing(device, incoming.clone()));
                inserted = true;
            }
        } else {
            next.push(device);
        }
    }

    if !inserted {
        next.push(incoming);
    }

    next
}

pub async fn discover_devices(app: AppHandle, config: &AppConfig) -> Vec<DeviceInfo> {
    let started_at = tokio::time::Instant::now();
    let started_at_unix = Utc::now().timestamp();
    let scan_id = Utc::now().timestamp_millis().max(0) as u64;
    let scan_config = config.clone();
    let adapters = scan_adapters();
    let plan = discovery_scan_hosts_for_adapters(config, &adapters);
    emit_scan_progress(
        &app,
        DiscoveryScanProgress {
            scan_id,
            status: if plan.hosts.is_empty() {
                DiscoveryScanStatus::Empty
            } else {
                DiscoveryScanStatus::Running
            },
            running: !plan.hosts.is_empty(),
            done: 0,
            total: plan.hosts.len(),
            range_count: plan.range_count,
            started_at: started_at_unix,
            finished_at: if plan.hosts.is_empty() {
                Some(Utc::now().timestamp())
            } else {
                None
            },
        },
    );
    let app_for_scan = app.clone();
    tauri::async_runtime::spawn(async move {
        let _ = scan_local_subnet(&app_for_scan, &scan_config, scan_id, started_at_unix).await;
    });

    for delay in active_discovery_round_delays() {
        let deadline = started_at + delay;
        tokio::time::sleep_until(deadline).await;
        let _ = broadcast_discovery(config, "request").await;
    }

    let finished_at = started_at + ACTIVE_DISCOVERY_WAIT;
    tokio::time::sleep_until(finished_at).await;
    cached_devices().await
}

pub fn start_discovery_runtime(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = run_discovery_runtime(app.clone(), state).await {
            let _ = app.emit("sync-error", format!("局域网设备发现启动失败：{error}"));
        }
    });
}

async fn run_discovery_runtime(app: AppHandle, state: AppState) -> AppResult<()> {
    let std_socket = StdUdpSocket::bind(("0.0.0.0", DISCOVERY_PORT))?;
    std_socket.set_broadcast(true)?;
    std_socket.set_nonblocking(true)?;
    let socket = UdpSocket::from_std(std_socket)?;
    let mut buffer = [0_u8; 4096];
    let mut announce = tokio::time::interval(DISCOVERY_ANNOUNCE_INTERVAL);
    let mut sweep = tokio::time::interval(DISCOVERY_SWEEP_INTERVAL);
    announce.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    sweep.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let config = state.config().await;
    let _ = send_discovery_message(&socket, &config, "announce", None).await;

    loop {
        tokio::select! {
            _ = announce.tick() => {
                let config = state.config().await;
                let _ = send_discovery_message(&socket, &config, "announce", None).await;
            }
            _ = sweep.tick() => {
                expire_cached_discovered_devices(&app, &state).await;
            }
            received = socket.recv_from(&mut buffer) => {
                let Ok((size, source)) = received else {
                    continue;
                };
                let Ok(text) = std::str::from_utf8(&buffer[..size]) else {
                    continue;
                };
                let Ok(payload) = parse_discovery_payload(text) else {
                    continue;
                };
                let config = state.config().await;
                merge_shared_scan_ranges(&app, &state, &payload.scan_ranges).await;
                if let Some(device) = discovered_device_from_payload(&payload, &config) {
                    let device = state.upsert_device(device.clone()).await;
                    cache_device(device.clone()).await;
                    notify_device_online_once(&app, &device).await;
                    let _ = app.emit("device-discovered", device);
                }
                if payload.action == "request" {
                    let target = SocketAddr::new(source.ip(), DISCOVERY_PORT);
                    let _ = send_discovery_message(&socket, &config, "announce", Some(target)).await;
                }
            }
        }
    }
}

async fn broadcast_discovery(config: &AppConfig, action: &str) -> AppResult<()> {
    let std_socket = StdUdpSocket::bind(("0.0.0.0", 0))?;
    std_socket.set_broadcast(true)?;
    std_socket.set_nonblocking(true)?;
    let socket = UdpSocket::from_std(std_socket)?;
    send_discovery_message(&socket, config, action, None).await
}

async fn send_discovery_message(
    socket: &UdpSocket,
    config: &AppConfig,
    action: &str,
    target: Option<SocketAddr>,
) -> AppResult<()> {
    let Some(host) = local_discovery_host(config) else {
        return Ok(());
    };
    let mut payload = build_discovery_payload(
        config.device_id.clone(),
        config.device_name.clone(),
        host,
        config.port,
        env!("CARGO_PKG_VERSION").to_string(),
        action.to_string(),
    )?;
    payload.scan_ranges = config.discovery_scan_ranges.clone();
    let message = serde_json::to_vec(&payload)?;
    if let Some(target) = target {
        let _ = socket.send_to(&message, target).await?;
        return Ok(());
    }

    for target in broadcast_targets()
        .into_iter()
        .map(|ip| SocketAddr::V4(SocketAddrV4::new(ip, DISCOVERY_PORT)))
    {
        let _ = socket.send_to(&message, target).await;
    }
    Ok(())
}

async fn cache_device(device: DeviceInfo) {
    let devices = cached_devices().await;
    let merged = merge_discovered_device(devices, device);
    let mut cache = discovered_devices().lock().await;
    cache.clear();
    for device in merged {
        cache.insert(device.id.clone(), device);
    }
}

async fn cached_devices() -> Vec<DeviceInfo> {
    discovered_devices()
        .lock()
        .await
        .values()
        .cloned()
        .collect()
}

async fn expire_cached_discovered_devices(app: &AppHandle, state: &AppState) {
    let now = Utc::now();
    let current_cache = cached_devices().await;
    let expired_cache =
        expire_stale_discovered_devices(current_cache, now, DISCOVERY_OFFLINE_AFTER);
    {
        let mut cache = discovered_devices().lock().await;
        cache.clear();
        for device in expired_cache {
            cache.insert(device.id.clone(), device);
        }
    }

    let current_devices = state.devices().await;
    let expired_devices =
        expire_stale_discovered_devices(current_devices, now, DISCOVERY_OFFLINE_AFTER);
    let mut notified = notified_online_devices().lock().await;
    let offline_devices = expired_devices
        .iter()
        .filter(|device| {
            !device.connected
                && device.status == DeviceStatus::Offline
                && notified.contains(&device.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    for device in offline_devices {
        notified.remove(&device.id);
        notifications::notify_device_offline(app, &device);
        let _ = app.emit("device-discovered", device);
    }
    state.replace_devices(expired_devices).await;
}

async fn notify_device_online_once(app: &AppHandle, device: &DeviceInfo) {
    if device.connected || device.status != DeviceStatus::Online {
        return;
    }
    let mut notified = notified_online_devices().lock().await;
    if notified.insert(device.id.clone()) {
        notifications::notify_device_discovered(app, device);
    }
}

async fn merge_shared_scan_ranges(app: &AppHandle, state: &AppState, ranges: &[String]) {
    let mut config = state.config().await;
    let original = config.discovery_scan_ranges.clone();
    for range in ranges {
        let Some(range) = normalize_scan_cidr(range) else {
            continue;
        };
        if !config.discovery_scan_ranges.contains(&range) {
            config.discovery_scan_ranges.push(range);
        }
    }
    app_config::normalize_config(&mut config);
    if config.discovery_scan_ranges != original {
        let _ = app_config::save_config(app, &config);
        state.set_config(config.clone()).await;
        let _ = app.emit("config-updated", config);
    }
}

pub fn merge_scan_ranges(existing: &[String], incoming: &[String]) -> Vec<String> {
    let mut merged = existing
        .iter()
        .filter_map(|range| normalize_scan_cidr(range))
        .fold(Vec::<String>::new(), |mut ranges, range| {
            if !ranges.contains(&range) {
                ranges.push(range);
            }
            ranges
        });
    for range in incoming.iter().filter_map(|range| normalize_scan_cidr(range)) {
        if !merged.contains(&range) {
            merged.push(range);
        }
    }
    merged
}

pub fn expire_stale_discovered_devices(
    devices: Vec<DeviceInfo>,
    now: DateTime<Utc>,
    ttl: ChronoDuration,
) -> Vec<DeviceInfo> {
    devices
        .into_iter()
        .map(|mut device| {
            let is_stale = device
                .last_seen_at
                .map(|last_seen| now.signed_duration_since(last_seen) > ttl)
                .unwrap_or(false);
            if !device.connected && device.status == DeviceStatus::Online && is_stale {
                device.status = DeviceStatus::Offline;
            }
            device
        })
        .collect()
}

pub fn active_discovery_round_delays() -> Vec<Duration> {
    vec![
        Duration::from_millis(0),
        Duration::from_millis(500),
        Duration::from_millis(1500),
    ]
}

async fn scan_local_subnet(
    app: &AppHandle,
    config: &AppConfig,
    scan_id: u64,
    started_at: i64,
) -> AppResult<()> {
    let Some(host) = local_discovery_host(config) else {
        return Ok(());
    };
    let Ok(local_ip) = host.parse::<Ipv4Addr>() else {
        return Ok(());
    };
    let adapters = scan_adapters();
    let plan = discovery_scan_hosts_for_adapters(config, &adapters);
    let total = plan.hosts.len();
    if total == 0 {
        return Ok(());
    }

    let std_socket = StdUdpSocket::bind(("0.0.0.0", 0))?;
    std_socket.set_broadcast(true)?;
    std_socket.set_nonblocking(true)?;
    let socket = UdpSocket::from_std(std_socket)?;
    let payload = build_discovery_payload(
        config.device_id.clone(),
        config.device_name.clone(),
        host,
        config.port,
        env!("CARGO_PKG_VERSION").to_string(),
        "request".to_string(),
    )?;
    let message = serde_json::to_vec(&payload)?;

    for (index, ip) in plan.hosts.into_iter().enumerate() {
        if ip == local_ip {
            continue;
        }
        let _ = socket
            .send_to(&message, SocketAddr::V4(SocketAddrV4::new(ip, DISCOVERY_PORT)))
            .await;
        emit_scan_progress(
            app,
            DiscoveryScanProgress {
                scan_id,
                status: DiscoveryScanStatus::Running,
                running: true,
                done: index + 1,
                total,
                range_count: plan.range_count,
                started_at,
                finished_at: None,
            },
        );
        tokio::time::sleep(SUBNET_SCAN_DELAY).await;
    }
    emit_scan_progress(
        app,
        DiscoveryScanProgress {
            scan_id,
            status: DiscoveryScanStatus::Done,
            running: false,
            done: total,
            total,
            range_count: plan.range_count,
            started_at,
            finished_at: Some(Utc::now().timestamp()),
        },
    );

    Ok(())
}

fn emit_scan_progress(app: &AppHandle, progress: DiscoveryScanProgress) {
    let _ = app.emit("lan-discovery-progress", progress);
}

#[cfg(test)]
pub fn discovery_scan_hosts_for_config(
    config: &AppConfig,
    local_ip: Option<Ipv4Addr>,
) -> DiscoveryScanPlan {
    let adapters = local_ip
        .map(|ip| vec![("local".to_string(), ip, Ipv4Addr::new(255, 255, 255, 0))])
        .unwrap_or_default();
    discovery_scan_hosts_for_adapters(config, &adapters)
}

pub fn discovery_scan_hosts_for_adapters(
    config: &AppConfig,
    adapters: &[(String, Ipv4Addr, Ipv4Addr)],
) -> DiscoveryScanPlan {
    let mut seen_hosts = HashSet::new();
    let mut seen_ranges = HashSet::new();
    let mut hosts = Vec::new();

    for (name, ip, netmask) in adapters {
        if let Some(cidr) = adapter_scan_cidr(name, *ip, *netmask) {
            push_cidr_hosts(&cidr, &mut seen_ranges, &mut seen_hosts, &mut hosts);
        }
    }

    for range in &config.discovery_scan_ranges {
        let Some(cidr) = normalize_scan_cidr(range) else {
            continue;
        };
        push_cidr_hosts(&cidr, &mut seen_ranges, &mut seen_hosts, &mut hosts);
    }

    DiscoveryScanPlan {
        hosts,
        range_count: seen_ranges.len(),
    }
}

fn push_cidr_hosts(
    cidr: &str,
    seen_ranges: &mut HashSet<String>,
    seen_hosts: &mut HashSet<Ipv4Addr>,
    hosts: &mut Vec<Ipv4Addr>,
) {
    if !seen_ranges.insert(cidr.to_string()) {
        return;
    }
    for host in cidr_hosts(cidr) {
        if seen_hosts.insert(host) {
            hosts.push(host);
        }
    }
}

fn adapter_scan_cidr(name: &str, ip: Ipv4Addr, netmask: Ipv4Addr) -> Option<String> {
    if is_virtual_adapter(name) || !is_usable_lan_ipv4(ip) {
        return None;
    }
    let prefix = netmask_prefix(netmask)?;
    let cidr = cidr_for_ip_prefix(ip, prefix);
    normalize_scan_cidr(&cidr).or_else(|| normalize_scan_cidr(&local_24_cidr(ip)))
}

fn local_24_cidr(local_ip: Ipv4Addr) -> String {
    let [a, b, c, _] = local_ip.octets();
    format!("{a}.{b}.{c}.0/24")
}

fn cidr_for_ip_prefix(ip: Ipv4Addr, prefix: u8) -> String {
    let host_bits = 32 - prefix;
    let mask = if host_bits == 32 {
        0
    } else {
        u32::MAX << host_bits
    };
    let network = Ipv4Addr::from(u32::from(ip) & mask);
    format!("{network}/{prefix}")
}

fn netmask_prefix(netmask: Ipv4Addr) -> Option<u8> {
    let mask = u32::from(netmask);
    let prefix = mask.count_ones() as u8;
    let expected = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix)
    };
    (mask == expected).then_some(prefix)
}

pub fn normalize_scan_cidr(input: &str) -> Option<String> {
    let (network, prefix) = parse_scan_cidr(input)?;
    if !network.is_private()
        || network.is_loopback()
        || network.is_unspecified()
        || network.is_multicast()
        || network.is_link_local()
    {
        return None;
    }
    Some(format!("{network}/{prefix}"))
}

fn cidr_hosts(cidr: &str) -> Vec<Ipv4Addr> {
    let Some((network, prefix)) = parse_scan_cidr(cidr) else {
        return Vec::new();
    };
    let host_count = (1_u32 << (32 - prefix)) - 2;
    let base = u32::from(network);
    (1..=host_count)
        .map(|offset| Ipv4Addr::from(base + offset))
        .collect()
}

fn parse_scan_cidr(input: &str) -> Option<(Ipv4Addr, u8)> {
    let (ip, prefix) = input.trim().split_once('/')?;
    let ip = ip.parse::<Ipv4Addr>().ok()?;
    let prefix = prefix.parse::<u8>().ok()?;
    if !(16..=30).contains(&prefix) {
        return None;
    }
    let host_bits = 32 - prefix;
    let host_count = (1_u32 << host_bits) - 2;
    if host_count == 0 || host_count > 1024 {
        return None;
    }
    let mask = u32::MAX << host_bits;
    let network = Ipv4Addr::from(u32::from(ip) & mask);
    Some((network, prefix))
}

fn scan_adapters() -> Vec<(String, Ipv4Addr, Ipv4Addr)> {
    get_if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| match iface.addr {
            IfAddr::V4(addr) => Some((iface.name, addr.ip, addr.netmask)),
            _ => None,
        })
        .collect()
}

fn broadcast_targets() -> Vec<Ipv4Addr> {
    let adapters = scan_adapters();
    compute_directed_broadcast_targets(&adapters)
}

pub fn compute_directed_broadcast_targets(
    adapters: &[(String, Ipv4Addr, Ipv4Addr)],
) -> Vec<Ipv4Addr> {
    let mut seen = HashSet::new();
    let mut targets = Vec::new();
    push_unique_target(&mut targets, &mut seen, Ipv4Addr::BROADCAST);

    for (name, ip, netmask) in adapters {
        if is_virtual_adapter(name) || !is_usable_lan_ipv4(*ip) {
            continue;
        }
        let broadcast = Ipv4Addr::from(u32::from(*ip) | !u32::from(*netmask));
        if broadcast == *ip || !broadcast.is_broadcast() && !is_usable_broadcast(broadcast) {
            continue;
        }
        push_unique_target(&mut targets, &mut seen, broadcast);
    }

    targets
}

fn push_unique_target(
    targets: &mut Vec<Ipv4Addr>,
    seen: &mut HashSet<Ipv4Addr>,
    target: Ipv4Addr,
) {
    if seen.insert(target) {
        targets.push(target);
    }
}

fn is_usable_lan_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_private()
        && !ip.is_loopback()
        && !ip.is_unspecified()
        && !ip.is_multicast()
        && !ip.is_broadcast()
        && !ip.is_link_local()
}

fn is_usable_broadcast(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[3] == 255 && !ip.is_loopback() && !ip.is_unspecified() && !ip.is_multicast()
}

fn is_virtual_adapter(adapter_name: &str) -> bool {
    let normalized = adapter_name.to_ascii_lowercase();
    [
        "vethernet",
        "wsl",
        "docker",
        "hyper-v",
        "hyperv",
        "virtual",
        "vmware",
        "virtualbox",
        "loopback",
        "npcap",
        "tap",
        "tun",
        "tunnel",
        "vpn",
        "openvpn",
        "wireguard",
        "xray",
        "tailscale",
        "zerotier",
        "bluetooth",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}

fn validate_discovery_payload(payload: DiscoveryPayload) -> AppResult<DiscoveryPayload> {
    if payload.payload_type != DISCOVERY_TYPE {
        return Err(AppError::InvalidInput(
            "不是 CopyShare 局域网发现消息".to_string(),
        ));
    }
    if payload.version != DISCOVERY_VERSION {
        return Err(AppError::InvalidInput(
            "CopyShare 发现协议版本不兼容".to_string(),
        ));
    }
    if payload.device_id.trim().is_empty() {
        return Err(AppError::InvalidInput("发现消息缺少设备 ID".to_string()));
    }
    validate_lan_ipv4(&payload.host)?;
    validate_port(payload.port)?;
    Ok(payload)
}

fn validate_lan_ipv4(host: &str) -> AppResult<Ipv4Addr> {
    let host = host.trim();
    if host.is_empty() {
        return Err(AppError::InvalidInput("发现消息缺少局域网 IP".to_string()));
    }

    let ip: Ipv4Addr = host
        .parse()
        .map_err(|_| AppError::InvalidInput("发现消息地址不是 IPv4".to_string()))?;

    if !ip.is_private()
        || ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_link_local()
    {
        return Err(AppError::InvalidInput(
            "发现消息地址不是局域网 IPv4".to_string(),
        ));
    }

    Ok(ip)
}

fn validate_port(port: u16) -> AppResult<()> {
    if port == 0 {
        return Err(AppError::InvalidInput("发现消息端口无效".to_string()));
    }
    Ok(())
}

fn local_discovery_host(config: &AppConfig) -> Option<String> {
    network::preferred_local_ip(&config.trusted_devices, config.port).and_then(|ip| match ip {
        IpAddr::V4(ip) if validate_lan_ipv4(&ip.to_string()).is_ok() => Some(ip.to_string()),
        _ => None,
    })
}

fn same_endpoint(left: &DeviceInfo, right: &DeviceInfo) -> bool {
    left.ip == right.ip && left.port == right.port
}

fn merge_discovered_existing(existing: DeviceInfo, incoming: DeviceInfo) -> DeviceInfo {
    if existing.connected {
        return DeviceInfo {
            trusted: existing.trusted || incoming.trusted,
            remote_trusted: existing.remote_trusted || incoming.remote_trusted,
            has_connected_before: existing.has_connected_before || incoming.has_connected_before,
            last_seen_at: incoming.last_seen_at.or(existing.last_seen_at),
            ..existing
        };
    }

    DeviceInfo {
        trusted: existing.trusted || incoming.trusted,
        remote_trusted: existing.remote_trusted || incoming.remote_trusted,
        has_connected_before: existing.has_connected_before || incoming.has_connected_before,
        ..incoming
    }
}

#[cfg(test)]
mod tests {
    use std::{net::Ipv4Addr, time::Duration};

    use chrono::{Duration as ChronoDuration, Utc};

    use crate::models::{AppConfig, DeviceInfo, DeviceStatus};

    use super::{
        active_discovery_round_delays, build_discovery_payload, compute_directed_broadcast_targets,
        discovery_scan_hosts_for_adapters, discovery_scan_hosts_for_config,
        discovered_device_from_payload,
        expire_stale_discovered_devices, merge_discovered_device, merge_scan_ranges,
        normalize_scan_cidr, parse_discovery_payload,
    };

    #[test]
    fn discovery_payload_round_trips_as_json() {
        let payload = build_discovery_payload(
            "device-a".to_string(),
            "Laptop A".to_string(),
            "192.168.1.23".to_string(),
            8765,
            "3.0.0".to_string(),
            "announce".to_string(),
        )
        .unwrap();

        let json = serde_json::to_string(&payload).unwrap();
        let decoded = parse_discovery_payload(&json).unwrap();

        assert_eq!(decoded, payload);
        assert!(json.contains("\"type\":\"copyshare-discovery\""));
        assert!(json.contains("\"deviceId\":\"device-a\""));
    }

    #[test]
    fn discovery_payload_rejects_non_copyshare_public_loopback_and_empty_host() {
        assert!(parse_discovery_payload(r#"{"type":"other","version":1}"#).is_err());
        assert!(parse_discovery_payload(
            r#"{"type":"copyshare-discovery","version":1,"deviceId":"a","deviceName":"A","host":"8.8.8.8","port":8765,"appVersion":"3.0.0","timestamp":1,"action":"announce"}"#,
        )
        .is_err());
        assert!(parse_discovery_payload(
            r#"{"type":"copyshare-discovery","version":1,"deviceId":"a","deviceName":"A","host":"127.0.0.1","port":8765,"appVersion":"3.0.0","timestamp":1,"action":"announce"}"#,
        )
        .is_err());
        assert!(parse_discovery_payload(
            r#"{"type":"copyshare-discovery","version":1,"deviceId":"a","deviceName":"A","host":"","port":8765,"appVersion":"3.0.0","timestamp":1,"action":"announce"}"#,
        )
        .is_err());
    }

    #[test]
    fn local_device_id_is_ignored() {
        let config = AppConfig {
            device_id: "device-a".to_string(),
            ..AppConfig::default()
        };
        let payload = build_discovery_payload(
            "device-a".to_string(),
            "Laptop A".to_string(),
            "192.168.1.23".to_string(),
            8765,
            "3.0.0".to_string(),
            "announce".to_string(),
        )
        .unwrap();

        assert!(discovered_device_from_payload(&payload, &config).is_none());
    }

    #[test]
    fn valid_lan_payload_becomes_unconnected_device_info() {
        let config = AppConfig {
            device_id: "device-local".to_string(),
            trusted_devices: vec!["device-remote".to_string()],
            ..AppConfig::default()
        };
        let payload = build_discovery_payload(
            "device-remote".to_string(),
            "Laptop B".to_string(),
            "10.0.0.23".to_string(),
            8765,
            "3.0.0".to_string(),
            "announce".to_string(),
        )
        .unwrap();

        let device = discovered_device_from_payload(&payload, &config).unwrap();

        assert_eq!(device.id, "device-remote");
        assert_eq!(device.name, "Laptop B");
        assert_eq!(device.ip, "10.0.0.23");
        assert_eq!(device.port, 8765);
        assert!(!device.connected);
        assert!(device.trusted);
        assert!(!device.remote_trusted);
        assert_eq!(device.status, DeviceStatus::Online);
    }

    #[test]
    fn duplicate_discovery_updates_existing_device_without_duplicate() {
        let first = discovered_device_from_payload(
            &build_discovery_payload(
                "device-remote".to_string(),
                "Laptop B".to_string(),
                "10.0.0.23".to_string(),
                8765,
                "3.0.0".to_string(),
                "announce".to_string(),
            )
            .unwrap(),
            &AppConfig::default(),
        )
        .unwrap();
        let second = DeviceInfo {
            name: "Laptop B New".to_string(),
            ..first.clone()
        };

        let merged = merge_discovered_device(vec![first], second);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "Laptop B New");
    }

    #[test]
    fn directed_broadcast_targets_include_limited_and_adapter_broadcasts() {
        let targets = compute_directed_broadcast_targets(&[
            (
                "Wi-Fi".to_string(),
                Ipv4Addr::new(192, 168, 1, 23),
                Ipv4Addr::new(255, 255, 255, 0),
            ),
            (
                "Ethernet".to_string(),
                Ipv4Addr::new(10, 0, 2, 15),
                Ipv4Addr::new(255, 255, 0, 0),
            ),
        ]);

        assert_eq!(
            targets,
            vec![
                Ipv4Addr::new(255, 255, 255, 255),
                Ipv4Addr::new(192, 168, 1, 255),
                Ipv4Addr::new(10, 0, 255, 255),
            ]
        );
    }

    #[test]
    fn directed_broadcast_targets_skip_virtual_and_unusable_adapters() {
        let targets = compute_directed_broadcast_targets(&[
            (
                "vEthernet (WSL)".to_string(),
                Ipv4Addr::new(172, 18, 0, 1),
                Ipv4Addr::new(255, 255, 0, 0),
            ),
            (
                "Loopback".to_string(),
                Ipv4Addr::new(127, 0, 0, 1),
                Ipv4Addr::new(255, 0, 0, 0),
            ),
            (
                "Wi-Fi".to_string(),
                Ipv4Addr::new(192, 168, 31, 88),
                Ipv4Addr::new(255, 255, 255, 0),
            ),
        ]);

        assert_eq!(
            targets,
            vec![
                Ipv4Addr::new(255, 255, 255, 255),
                Ipv4Addr::new(192, 168, 31, 255),
            ]
        );
    }

    #[test]
    fn stale_discovered_devices_are_marked_offline_after_ttl() {
        let now = Utc::now();
        let fresh = DeviceInfo {
            id: "fresh".to_string(),
            name: "Fresh".to_string(),
            ip: "192.168.1.20".to_string(),
            port: 8765,
            connected: false,
            trusted: false,
            remote_trusted: false,
            has_connected_before: false,
            last_seen_at: Some(now - ChronoDuration::seconds(10)),
            status: DeviceStatus::Online,
        };
        let stale = DeviceInfo {
            id: "stale".to_string(),
            name: "Stale".to_string(),
            ip: "192.168.1.21".to_string(),
            port: 8765,
            connected: false,
            trusted: false,
            remote_trusted: false,
            has_connected_before: false,
            last_seen_at: Some(now - ChronoDuration::seconds(31)),
            status: DeviceStatus::Online,
        };

        let expired = expire_stale_discovered_devices(
            vec![fresh, stale],
            now,
            ChronoDuration::seconds(30),
        );

        assert_eq!(expired[0].status, DeviceStatus::Online);
        assert_eq!(expired[1].status, DeviceStatus::Offline);
    }

    #[test]
    fn active_discovery_uses_three_request_rounds() {
        assert_eq!(
            active_discovery_round_delays(),
            vec![
                Duration::from_millis(0),
                Duration::from_millis(500),
                Duration::from_millis(1500),
            ]
        );
    }

    #[test]
    fn scan_cidr_normalizes_private_ranges_and_rejects_invalid_or_public_ranges() {
        assert_eq!(normalize_scan_cidr("192.168.1.23/24"), Some("192.168.1.0/24".to_string()));
        assert_eq!(normalize_scan_cidr("10.2.3.4/24"), Some("10.2.3.0/24".to_string()));
        assert_eq!(normalize_scan_cidr("172.20.9.1/24"), Some("172.20.9.0/24".to_string()));
        assert_eq!(normalize_scan_cidr("8.8.8.0/24"), None);
        assert_eq!(normalize_scan_cidr("192.168.1.0/31"), None);
        assert_eq!(normalize_scan_cidr("bad"), None);
    }

    #[test]
    fn discovery_scan_hosts_include_saved_ranges_without_duplicates() {
        let config = AppConfig {
            discovery_scan_ranges: vec![
                "192.168.55.0/30".to_string(),
                "192.168.55.1/30".to_string(),
                "10.0.0.0/30".to_string(),
            ],
            ..AppConfig::default()
        };

        let plan = discovery_scan_hosts_for_config(&config, Some(Ipv4Addr::new(192, 168, 55, 1)));

        assert!(plan.hosts.contains(&Ipv4Addr::new(192, 168, 55, 2)));
        assert!(plan.hosts.contains(&Ipv4Addr::new(10, 0, 0, 1)));
        assert!(plan.hosts.contains(&Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(
            plan.hosts.iter().filter(|ip| **ip == Ipv4Addr::new(192, 168, 55, 2)).count(),
            1
        );
        assert_eq!(plan.range_count, 3);
    }

    #[test]
    fn default_scan_uses_real_adapter_prefix_not_fixed_24() {
        let config = AppConfig::default();
        let plan = discovery_scan_hosts_for_adapters(
            &config,
            &[(
                "WLAN".to_string(),
                Ipv4Addr::new(10, 194, 34, 119),
                Ipv4Addr::new(255, 255, 252, 0),
            )],
        );

        assert_eq!(plan.range_count, 1);
        assert!(plan.hosts.contains(&Ipv4Addr::new(10, 194, 33, 156)));
        assert!(plan.hosts.contains(&Ipv4Addr::new(10, 194, 35, 254)));
        assert!(!plan.hosts.contains(&Ipv4Addr::new(10, 194, 36, 1)));
    }

    #[test]
    fn shared_scan_ranges_are_normalized_and_merged_without_duplicates() {
        let merged = merge_scan_ranges(
            &["192.168.1.0/24".to_string()],
            &[
                "192.168.1.23/24".to_string(),
                "10.0.0.4/24".to_string(),
                "8.8.8.0/24".to_string(),
                "bad".to_string(),
            ],
        );

        assert_eq!(merged, vec!["192.168.1.0/24", "10.0.0.0/24"]);
    }

    #[test]
    fn discovery_payload_accepts_missing_legacy_scan_ranges() {
        let decoded = parse_discovery_payload(
            r#"{"type":"copyshare-discovery","version":1,"deviceId":"a","deviceName":"A","host":"192.168.1.10","port":8765,"appVersion":"2.7.0","timestamp":1,"action":"announce"}"#,
        )
        .unwrap();

        assert!(decoded.scan_ranges.is_empty());
    }
}
