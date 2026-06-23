use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use crate::{
    error::{AppError, AppResult},
    models::WireMessage,
};

pub fn normalize_peer_url(peer: &str, default_port: u16) -> AppResult<String> {
    let candidate = peer.trim();
    if candidate.is_empty() {
        return Err(AppError::InvalidInput("device address is required".to_string()));
    }

    let with_scheme = if candidate.contains("://") {
        candidate.to_string()
    } else {
        format!("ws://{candidate}")
    };

    let mut url = url::Url::parse(&with_scheme)?;
    if url.scheme() != "ws" {
        return Err(AppError::InvalidInput(
            "MVP only supports ws:// LAN device addresses".to_string(),
        ));
    }
    if url.host_str().is_none() {
        return Err(AppError::InvalidInput(
            "device address must include a host".to_string(),
        ));
    }
    if url.port().is_none() {
        url.set_port(Some(default_port))
            .map_err(|_| AppError::InvalidInput("invalid device port".to_string()))?;
    }
    if url.path().is_empty() {
        url.set_path("/");
    }

    Ok(url.to_string())
}

pub fn normalize_peer_endpoint(peer: &str, default_port: u16) -> AppResult<String> {
    let candidate = peer.trim();
    if candidate.is_empty() {
        return Err(AppError::InvalidInput("device address is required".to_string()));
    }

    if candidate.contains("://") || has_explicit_port(candidate) {
        return normalize_peer_url(candidate, default_port);
    }

    normalize_peer_url(&format!("{candidate}:{default_port}"), default_port)
}

pub fn display_host_from_connection_id(connection_id: &str) -> String {
    if let Ok(url) = url::Url::parse(connection_id) {
        if let Some(host) = url.host_str() {
            return host.to_string();
        }
    }

    if let Ok(address) = connection_id.parse::<SocketAddr>() {
        return address.ip().to_string();
    }

    if let Some((host, port)) = connection_id.rsplit_once(':') {
        if port.parse::<u16>().is_ok() && !host.is_empty() {
            return host.to_string();
        }
    }

    connection_id.to_string()
}

pub fn endpoint_from_connection_id(connection_id: &str, port: u16) -> AppResult<String> {
    normalize_peer_endpoint(&display_host_from_connection_id(connection_id), port)
}

pub fn peer_ip_hint(peer: &str, default_port: u16) -> Option<IpAddr> {
    let endpoint = normalize_peer_endpoint(peer, default_port).ok()?;
    let url = url::Url::parse(&endpoint).ok()?;
    url.host_str()?.parse().ok()
}

pub fn preferred_local_ip(trusted_devices: &[String], default_port: u16) -> Option<IpAddr> {
    let peer_hint = trusted_devices
        .iter()
        .find_map(|peer| peer_ip_hint(peer, default_port));

    preferred_local_ip_for_peer(peer_hint)
}

pub fn preferred_local_ip_for_peer(peer_hint: Option<IpAddr>) -> Option<IpAddr> {
    let selected = local_ip_address::list_afinet_netifas()
        .ok()
        .and_then(|candidates| select_preferred_local_ip(&candidates, peer_hint));

    selected.or_else(|| local_ip_address::local_ip().ok())
}

pub fn select_preferred_local_ip(
    candidates: &[(String, IpAddr)],
    peer_hint: Option<IpAddr>,
) -> Option<IpAddr> {
    candidates
        .iter()
        .filter_map(|(adapter_name, ip)| {
            let IpAddr::V4(ipv4) = ip else {
                return None;
            };
            if !is_usable_ipv4(*ipv4) {
                return None;
            }
            Some((score_local_ip(adapter_name, *ipv4, peer_hint), IpAddr::V4(*ipv4)))
        })
        .min_by_key(|(score, _)| *score)
        .map(|(_, ip)| ip)
}

fn has_explicit_port(candidate: &str) -> bool {
    let Some((host, port)) = candidate.rsplit_once(':') else {
        return false;
    };

    !host.is_empty() && port.parse::<u16>().is_ok()
}

fn score_local_ip(
    adapter_name: &str,
    local_ip: Ipv4Addr,
    peer_hint: Option<IpAddr>,
) -> (u8, u8, u8, u32) {
    let virtual_adapter_penalty = if is_virtual_adapter(adapter_name) { 1 } else { 0 };
    let peer_score = match peer_hint {
        Some(IpAddr::V4(peer_ip)) => peer_match_score(local_ip, peer_ip),
        _ => 4,
    };
    let private_score = private_preference_score(local_ip);

    (
        virtual_adapter_penalty,
        peer_score,
        private_score,
        u32::from(local_ip),
    )
}

fn peer_match_score(local_ip: Ipv4Addr, peer_ip: Ipv4Addr) -> u8 {
    let local = local_ip.octets();
    let peer = peer_ip.octets();

    if local[..3] == peer[..3] {
        return 0;
    }
    if local[..2] == peer[..2] {
        return 1;
    }
    if same_private_range(local_ip, peer_ip) {
        return 2;
    }
    if local_ip.is_private() && peer_ip.is_private() {
        return 3;
    }

    4
}

fn private_preference_score(ip: Ipv4Addr) -> u8 {
    let [first, second, _, _] = ip.octets();

    match (first, second) {
        (10, _) => 0,
        (192, 168) => 1,
        (172, 16..=31) => 2,
        _ if ip.is_private() => 3,
        _ => 4,
    }
}

fn same_private_range(left: Ipv4Addr, right: Ipv4Addr) -> bool {
    let [left_first, left_second, _, _] = left.octets();
    let [right_first, right_second, _, _] = right.octets();

    match (left_first, left_second, right_first, right_second) {
        (10, _, 10, _) => true,
        (192, 168, 192, 168) => true,
        (172, 16..=31, 172, 16..=31) => true,
        _ => false,
    }
}

fn is_usable_ipv4(ip: Ipv4Addr) -> bool {
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_link_local())
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

pub fn encode_wire_message(message: &WireMessage) -> AppResult<String> {
    Ok(serde_json::to_string(message)?)
}

pub fn decode_wire_message(text: &str) -> AppResult<WireMessage> {
    Ok(serde_json::from_str(text)?)
}

#[cfg(test)]
mod tests {
    use crate::models::{ClipboardContentType, ClipboardMessage};

    use super::*;

    #[test]
    fn normalize_peer_url_accepts_ip_and_host_port() {
        assert_eq!(
            normalize_peer_url("192.168.1.20", 8765).unwrap(),
            "ws://192.168.1.20:8765/"
        );
        assert_eq!(
            normalize_peer_url("ws://192.168.1.20:9000", 8765).unwrap(),
            "ws://192.168.1.20:9000/"
        );
    }

    #[test]
    fn normalize_peer_endpoint_does_not_duplicate_ports() {
        assert_eq!(
            normalize_peer_endpoint("10.194.33.156", 8765).unwrap(),
            "ws://10.194.33.156:8765/"
        );
        assert_eq!(
            normalize_peer_endpoint("10.194.33.156:9000", 8765).unwrap(),
            "ws://10.194.33.156:9000/"
        );
        assert_eq!(
            normalize_peer_endpoint("ws://10.194.33.156:9000/", 8765).unwrap(),
            "ws://10.194.33.156:9000/"
        );
    }

    #[test]
    fn display_host_strips_url_and_port() {
        assert_eq!(
            display_host_from_connection_id("ws://10.194.33.156:8765/"),
            "10.194.33.156"
        );
        assert_eq!(
            display_host_from_connection_id("10.194.33.156:51234"),
            "10.194.33.156"
        );
    }

    #[test]
    fn endpoint_from_connection_id_uses_declared_listening_port() {
        assert_eq!(
            endpoint_from_connection_id("10.194.33.156:51234", 8765).unwrap(),
            "ws://10.194.33.156:8765/"
        );
        assert_eq!(
            endpoint_from_connection_id("ws://10.194.33.156:8765/", 8765).unwrap(),
            "ws://10.194.33.156:8765/"
        );
    }

    #[test]
    fn peer_ip_hint_reads_websocket_and_bare_endpoints() {
        assert_eq!(
            peer_ip_hint("ws://10.194.33.156:8765/", 8765),
            Some("10.194.33.156".parse().unwrap())
        );
        assert_eq!(
            peer_ip_hint("10.194.33.156", 8765),
            Some("10.194.33.156".parse().unwrap())
        );
        assert_eq!(peer_ip_hint("device-a", 8765), None);
    }

    #[test]
    fn preferred_local_ip_uses_real_lan_adapter_near_peer() {
        let candidates = vec![
            ("vEthernet (WSL)".to_string(), "172.18.0.1".parse().unwrap()),
            ("DockerNAT".to_string(), "172.17.0.1".parse().unwrap()),
            ("Wi-Fi".to_string(), "10.194.34.119".parse().unwrap()),
        ];

        assert_eq!(
            select_preferred_local_ip(&candidates, Some("10.194.33.156".parse().unwrap())),
            Some("10.194.34.119".parse().unwrap())
        );
    }

    #[test]
    fn preferred_local_ip_excludes_virtual_adapters_without_peer_hint() {
        let candidates = vec![
            ("vEthernet (Default Switch)".to_string(), "172.18.0.1".parse().unwrap()),
            ("VMware Network Adapter".to_string(), "192.168.56.1".parse().unwrap()),
            ("xray_tun".to_string(), "172.18.0.1".parse().unwrap()),
            ("Ethernet".to_string(), "192.168.1.88".parse().unwrap()),
        ];

        assert_eq!(
            select_preferred_local_ip(&candidates, None),
            Some("192.168.1.88".parse().unwrap())
        );
    }

    #[test]
    fn virtual_adapter_names_include_tunnel_and_vm_adapters() {
        assert!(is_virtual_adapter("xray_tun"));
        assert!(is_virtual_adapter("OpenVPN Connect DCO Adapter"));
        assert!(is_virtual_adapter("VMware Network Adapter VMnet8"));
        assert!(is_virtual_adapter("vEthernet (WSL)"));
        assert!(!is_virtual_adapter("WLAN"));
        assert!(!is_virtual_adapter("Wi-Fi"));
    }

    #[test]
    fn normalize_peer_url_rejects_non_ws_urls() {
        assert!(normalize_peer_url("wss://example.test", 8765).is_err());
        assert!(normalize_peer_url("", 8765).is_err());
    }

    #[test]
    fn clipboard_message_round_trips_as_wire_json() {
        let message = ClipboardMessage {
            message_id: "msg-1".to_string(),
            source_device_id: "device-a".to_string(),
            source_device_name: "Laptop A".to_string(),
            content_type: ClipboardContentType::Text,
            content: "hello".to_string(),
            content_hash: "hash".to_string(),
            timestamp: 1,
        };

        let encoded = encode_wire_message(&message.clone().into()).unwrap();
        let decoded = decode_wire_message(&encoded).unwrap();

        assert_eq!(ClipboardMessage::try_from(decoded).unwrap(), message);
    }
}
