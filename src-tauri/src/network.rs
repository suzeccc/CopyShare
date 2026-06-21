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
