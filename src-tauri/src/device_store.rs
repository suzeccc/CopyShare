use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};

use crate::{
    error::AppResult,
    models::{DeviceInfo, DeviceStatus},
};

const DEVICES_FILE: &str = "devices.json";
const DEVICES_LIMIT: usize = 100;

pub fn load_devices(app: &AppHandle) -> AppResult<Vec<DeviceInfo>> {
    let path = devices_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(path)?;
    load_device_items_from_text(&text)
}

pub fn save_devices(app: &AppHandle, devices: &[DeviceInfo]) -> AppResult<()> {
    let path = devices_path(app)?;
    let text = serde_json::to_string_pretty(&device_history_snapshot(devices))?;
    fs::write(path, text)?;
    Ok(())
}

fn devices_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join(DEVICES_FILE))
}

fn load_device_items_from_text(text: &str) -> AppResult<Vec<DeviceInfo>> {
    match serde_json::from_str::<Vec<DeviceInfo>>(text) {
        Ok(items) => Ok(device_history_snapshot(&items)),
        Err(_) => Ok(Vec::new()),
    }
}

fn device_history_snapshot(devices: &[DeviceInfo]) -> Vec<DeviceInfo> {
    let mut snapshot = devices.to_vec();
    snapshot.sort_by(|left, right| right.last_seen_at.cmp(&left.last_seen_at));
    snapshot.truncate(DEVICES_LIMIT);

    for device in &mut snapshot {
        device.connected = false;
        device.status = DeviceStatus::Offline;
    }

    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn device(id: &str, connected: bool) -> DeviceInfo {
        DeviceInfo {
            id: id.to_string(),
            name: "Laptop".to_string(),
            ip: "10.194.33.156".to_string(),
            port: 8765,
            connected,
            trusted: true,
            last_seen_at: Some(Utc::now()),
            status: if connected {
                DeviceStatus::Online
            } else {
                DeviceStatus::Offline
            },
        }
    }

    #[test]
    fn saved_devices_reload_as_offline_history() {
        let devices = vec![device("device-remote", true)];
        let text = serde_json::to_string(&device_history_snapshot(&devices)).unwrap();
        let loaded = load_device_items_from_text(&text).unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "device-remote");
        assert!(!loaded[0].connected);
        assert_eq!(loaded[0].status, DeviceStatus::Offline);
        assert!(loaded[0].trusted);
    }

    #[test]
    fn corrupted_devices_json_does_not_block_startup() {
        let loaded = load_device_items_from_text(r#"[{"id":"broken""#).unwrap();

        assert!(loaded.is_empty());
    }
}