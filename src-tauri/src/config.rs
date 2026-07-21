use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::{
    discovery,
    error::AppResult,
    models::{new_device_id, AppConfig},
    security,
};

const CONFIG_FILE: &str = "config.json";
const CURRENT_CONFIG_VERSION: u16 = 6;

pub fn load_config(app: &AppHandle) -> AppResult<AppConfig> {
    let path = config_path(app)?;
    if !path.exists() {
        let config = AppConfig::default();
        save_config(app, &config)?;
        return Ok(config);
    }

    let text = fs::read_to_string(path)?;
    let mut config = parse_config_text(&text)?;
    let mut changed = ensure_config_device_id(&mut config);
    changed |= migrate_config(&mut config);
    changed |= normalize_config(&mut config);
    if changed {
        save_config(app, &config)?;
    }
    Ok(config)
}

pub fn save_config(app: &AppHandle, config: &AppConfig) -> AppResult<()> {
    let path = config_path(app)?;
    let mut normalized = config.clone();
    normalize_config(&mut normalized);
    let text = serde_json::to_string_pretty(&normalized)?;
    fs::write(path, text)?;
    Ok(())
}

fn config_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join(CONFIG_FILE))
}

pub fn ensure_config_device_id(config: &mut AppConfig) -> bool {
    if !config.device_id.trim().is_empty() {
        return false;
    }

    config.device_id = new_device_id();
    true
}

fn migrate_config(config: &mut AppConfig) -> bool {
    if config.config_version >= CURRENT_CONFIG_VERSION {
        return false;
    }

    config.sync_image = true;
    config.sync_files = true;
    config.notification_clipboard_preview = true;
    config.notify_device_status = true;
    config.config_version = CURRENT_CONFIG_VERSION;
    true
}

fn parse_config_text(text: &str) -> AppResult<AppConfig> {
    Ok(serde_json::from_str(text.trim_start_matches('\u{feff}'))?)
}

pub(crate) fn normalize_config(config: &mut AppConfig) -> bool {
    let mut changed = security::normalize_trusted_devices(config);
    if config.notify_file_transfer {
        config.notify_file_transfer = false;
        changed = true;
    }
    let normalized_file_save_dir = config
        .file_save_dir
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned);
    if config.file_save_dir != normalized_file_save_dir {
        config.file_save_dir = normalized_file_save_dir;
        changed = true;
    }
    let normalized_ranges = discovery::merge_scan_ranges(&config.discovery_scan_ranges, &[]);
    if config.discovery_scan_ranges != normalized_ranges {
        config.discovery_scan_ranges = normalized_ranges;
        changed = true;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::ensure_config_device_id;
    use crate::models::AppConfig;

    #[test]
    fn default_config_matches_mvp_scope() {
        let config = AppConfig::default();

        assert_eq!(config.config_version, 6);
        assert_eq!(config.port, 8765);
        assert_eq!(config.theme, crate::models::AppTheme::Win11Dark);
        assert_eq!(config.close_action, crate::models::CloseAction::Ask);
        assert!(config.auto_sync);
        assert!(config.save_history);
        assert!(config.sync_text);
        assert!(config.sync_image);
        assert!(config.sync_files);
        assert!(config.deduplicate_sync_content);
        assert_eq!(config.file_save_dir, None);
        assert!(!config.auto_open_folder_after_save);
        assert!(config.trusted_devices.is_empty());
        assert!(config.discovery_scan_ranges.is_empty());
        assert!(config.desktop_notifications);
        assert!(config.notify_clipboard);
        assert!(config.notify_trust_required);
        assert!(!config.notify_file_transfer);
        assert!(config.notify_device_status);
        assert!(config.notify_sync_error);
        assert!(config.notification_clipboard_preview);
        assert_eq!(config.translation_engine, crate::models::TranslationEngine::Google);
        assert_eq!(config.translation_api_url, "");
        assert_eq!(config.translation_api_key, "");
        assert_eq!(config.translation_model, "gpt-4o-mini");
        assert_eq!(config.translation_proxy, "");
    }

    #[test]
    fn legacy_config_without_theme_uses_win11_dark() {
        let json = serde_json::json!({
            "deviceName": "CopyShare",
            "deviceId": "device-test",
            "port": 8765,
            "autoStart": false,
            "autoSync": true,
            "saveHistory": true,
            "trustedDevices": [],
            "syncText": true,
            "syncImage": false,
            "syncFiles": false
        });

        let config: AppConfig = serde_json::from_value(json).unwrap();

        assert_eq!(config.theme, crate::models::AppTheme::Win11Dark);
        assert!(config.deduplicate_sync_content);
    }

    #[test]
    fn explicit_disabled_deduplication_is_preserved() {
        let json = serde_json::json!({
            "deviceName": "CopyShare",
            "deviceId": "device-test",
            "port": 8765,
            "autoStart": false,
            "autoSync": true,
            "saveHistory": true,
            "trustedDevices": [],
            "syncText": true,
            "syncImage": true,
            "syncFiles": true,
            "deduplicateSyncContent": false
        });

        let config: AppConfig = serde_json::from_value(json).unwrap();

        assert!(!config.deduplicate_sync_content);
    }

    #[test]
    fn config_json_with_utf8_bom_is_accepted() {
        let json = "\u{feff}{\"deviceName\":\"CopyShare\",\"deviceId\":\"device-test\",\"port\":8765,\"autoStart\":false,\"autoSync\":true,\"saveHistory\":true,\"trustedDevices\":[],\"syncText\":true,\"syncImage\":true,\"syncFiles\":true}";

        let config = super::parse_config_text(json).expect("config with BOM should parse");

        assert_eq!(config.device_name, "CopyShare");
        assert!(config.sync_files);
    }

    #[test]
    fn legacy_config_enables_image_sync_once() {
        let json = serde_json::json!({
            "configVersion": 4,
            "deviceName": "CopyShare",
            "deviceId": "device-test",
            "port": 8765,
            "autoStart": false,
            "autoSync": true,
            "saveHistory": true,
            "trustedDevices": [],
            "syncText": true,
            "syncImage": false,
            "syncFiles": false
        });
        let mut config: AppConfig = serde_json::from_value(json).unwrap();

        assert!(super::migrate_config(&mut config));
        assert_eq!(config.config_version, 6);
        assert!(config.sync_image);
        assert!(config.sync_files);
        assert!(config.notification_clipboard_preview);
        assert!(config.notify_device_status);

        config.sync_image = false;
        config.sync_files = false;
        config.notification_clipboard_preview = false;
        config.notify_device_status = false;
        assert!(!super::migrate_config(&mut config));
        assert_eq!(config.config_version, 6);
        assert!(!config.sync_image);
        assert!(!config.sync_files);
        assert!(!config.notification_clipboard_preview);
        assert!(!config.notify_device_status);
    }

    #[test]
    fn normalize_config_trims_blank_file_save_dir() {
        let mut config = AppConfig {
            file_save_dir: Some("  ".to_string()),
            ..AppConfig::default()
        };

        assert!(super::normalize_config(&mut config));
        assert_eq!(config.file_save_dir, None);

        config.file_save_dir = Some(" C:\\Receive ".to_string());
        assert!(super::normalize_config(&mut config));
        assert_eq!(config.file_save_dir.as_deref(), Some("C:\\Receive"));
    }

    #[test]
    fn normalize_config_removes_endpoint_trust_aliases_and_duplicates() {
        let mut config = AppConfig {
            trusted_devices: vec![
                "device-a".to_string(),
                "10.194.33.156:8765".to_string(),
                "ws://10.194.33.156:8765/".to_string(),
                "device-a".to_string(),
                "device-b".to_string(),
            ],
            discovery_scan_ranges: vec![
                "192.168.1.23/24".to_string(),
                "192.168.1.0/24".to_string(),
                "10.0.0.1/24".to_string(),
                "8.8.8.0/24".to_string(),
                "bad".to_string(),
            ],
            ..AppConfig::default()
        };

        assert!(super::normalize_config(&mut config));
        assert_eq!(config.trusted_devices, vec!["device-a", "device-b"]);
        assert_eq!(
            config.discovery_scan_ranges,
            vec!["192.168.1.0/24", "10.0.0.0/24"]
        );
        assert!(!super::normalize_config(&mut config));
    }

    #[test]
    fn missing_device_id_is_generated_once() {
        let mut config = AppConfig {
            device_id: String::new(),
            ..AppConfig::default()
        };

        assert!(ensure_config_device_id(&mut config));
        let generated = config.device_id.clone();
        assert!(generated.starts_with("device-"));

        assert!(!ensure_config_device_id(&mut config));
        assert_eq!(config.device_id, generated);
    }
}
