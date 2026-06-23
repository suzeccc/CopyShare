use crate::models::AppConfig;

pub fn is_device_id_trusted(config: &AppConfig, device_id: &str) -> bool {
    config
        .trusted_devices
        .iter()
        .any(|trusted| trusted == device_id)
}

pub fn trust_device(config: &mut AppConfig, device_id: impl Into<String>) {
    let device_id = device_id.into();
    if !config.trusted_devices.iter().any(|trusted| trusted == &device_id) {
        config.trusted_devices.push(device_id);
    }
}

pub fn untrust_device(config: &mut AppConfig, device_id: &str) {
    config.trusted_devices.retain(|trusted| trusted != device_id);
}

pub fn normalize_device_name(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_device_is_idempotent() {
        let mut config = AppConfig::default();

        trust_device(&mut config, "device-a");
        trust_device(&mut config, "device-a");

        assert_eq!(config.trusted_devices, vec!["device-a"]);
        assert!(is_device_id_trusted(&config, "device-a"));
    }

    #[test]
    fn endpoint_trust_does_not_mark_device_id_trusted() {
        let mut config = AppConfig::default();
        config.trusted_devices.push("10.194.33.156:8765".to_string());

        assert!(!is_device_id_trusted(&config, "device-remote"));
    }

    #[test]
    fn untrust_device_removes_existing_trust() {
        let mut config = AppConfig::default();

        trust_device(&mut config, "device-a");
        untrust_device(&mut config, "device-a");

        assert!(config.trusted_devices.is_empty());
        assert!(!is_device_id_trusted(&config, "device-a"));
    }

    #[test]
    fn blank_device_name_is_rejected() {
        assert_eq!(normalize_device_name("  "), None);
        assert_eq!(normalize_device_name(" Laptop "), Some("Laptop".to_string()));
    }
}
