use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::{
    error::AppResult,
    models::{new_device_id, AppConfig},
};

const CONFIG_FILE: &str = "config.json";
const CURRENT_CONFIG_VERSION: u16 = 1;

pub fn load_config(app: &AppHandle) -> AppResult<AppConfig> {
    let path = config_path(app)?;
    if !path.exists() {
        let config = AppConfig::default();
        save_config(app, &config)?;
        return Ok(config);
    }

    let text = fs::read_to_string(path)?;
    let mut config: AppConfig = serde_json::from_str(&text)?;
    let mut changed = ensure_config_device_id(&mut config);
    changed |= migrate_config(&mut config);
    if changed {
        save_config(app, &config)?;
    }
    Ok(config)
}

pub fn save_config(app: &AppHandle, config: &AppConfig) -> AppResult<()> {
    let path = config_path(app)?;
    let text = serde_json::to_string_pretty(config)?;
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
    config.config_version = CURRENT_CONFIG_VERSION;
    true
}

#[cfg(test)]
mod tests {
    use super::ensure_config_device_id;
    use crate::models::AppConfig;

    #[test]
    fn default_config_matches_mvp_scope() {
        let config = AppConfig::default();

        assert_eq!(config.port, 8765);
        assert_eq!(config.theme, crate::models::AppTheme::Win11Dark);
        assert_eq!(config.close_action, crate::models::CloseAction::Ask);
        assert!(config.auto_sync);
        assert!(config.save_history);
        assert!(config.sync_text);
        assert!(config.sync_image);
        assert!(!config.sync_files);
        assert!(config.trusted_devices.is_empty());
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
    }

    #[test]
    fn legacy_config_enables_image_sync_once() {
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
        let mut config: AppConfig = serde_json::from_value(json).unwrap();

        assert!(super::migrate_config(&mut config));
        assert_eq!(config.config_version, 1);
        assert!(config.sync_image);

        config.sync_image = false;
        assert!(!super::migrate_config(&mut config));
        assert_eq!(config.config_version, 1);
        assert!(!config.sync_image);
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
