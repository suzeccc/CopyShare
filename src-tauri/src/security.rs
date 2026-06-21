use crate::models::AppConfig;

pub fn is_trusted(config: &AppConfig, device_id: &str) -> bool {
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
        assert!(is_trusted(&config, "device-a"));
    }

    #[test]
    fn blank_device_name_is_rejected() {
        assert_eq!(normalize_device_name("  "), None);
        assert_eq!(normalize_device_name(" Laptop "), Some("Laptop".to_string()));
    }
}
