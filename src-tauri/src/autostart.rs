use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

use crate::error::{AppError, AppResult};

pub fn set_autostart(app: &AppHandle, enabled: bool) -> AppResult<()> {
    let manager = app.autolaunch();
    if enabled {
        return manager
            .enable()
            .map_err(|error| AppError::Tauri(error.to_string()));
    }

    match manager.is_enabled() {
        Ok(false) => return Ok(()),
        Ok(true) => {}
        Err(error) if is_missing_autostart_entry(&error.to_string()) => return Ok(()),
        Err(error) => return Err(AppError::Tauri(error.to_string())),
    }

    manager.disable().or_else(|error| {
        if is_missing_autostart_entry(&error.to_string()) {
            Ok(())
        } else {
            Err(AppError::Tauri(error.to_string()))
        }
    })
}

pub fn is_autostart_enabled(app: &AppHandle) -> AppResult<bool> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| AppError::Tauri(error.to_string()))
}

pub fn should_update_autostart(current: bool, next: bool) -> bool {
    current != next
}

fn is_missing_autostart_entry(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    lower.contains("os error 2")
        || lower.contains("no such file")
        || lower.contains("cannot find the file")
        || message.contains("系统找不到指定的文件")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autostart_update_is_needed_only_when_value_changes() {
        assert!(!should_update_autostart(false, false));
        assert!(!should_update_autostart(true, true));
        assert!(should_update_autostart(false, true));
        assert!(should_update_autostart(true, false));
    }

    #[test]
    fn missing_autostart_entry_means_disabled_when_turning_off() {
        assert!(is_missing_autostart_entry("系统找不到指定的文件。(os error 2)"));
        assert!(is_missing_autostart_entry("No such file or directory (os error 2)"));
        assert!(!is_missing_autostart_entry("permission denied"));
    }
}
