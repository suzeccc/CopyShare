use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::{
    error::AppResult,
    models::AppConfig,
};

const CONFIG_FILE: &str = "config.json";

pub fn load_config(app: &AppHandle) -> AppResult<AppConfig> {
    let path = config_path(app)?;
    if !path.exists() {
        let config = AppConfig::default();
        save_config(app, &config)?;
        return Ok(config);
    }

    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
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

#[cfg(test)]
mod tests {
    use crate::models::AppConfig;

    #[test]
    fn default_config_matches_mvp_scope() {
        let config = AppConfig::default();

        assert_eq!(config.port, 8765);
        assert!(config.auto_sync);
        assert!(config.save_history);
        assert!(config.sync_text);
        assert!(!config.sync_image);
        assert!(!config.sync_files);
        assert!(config.trusted_devices.is_empty());
    }
}
