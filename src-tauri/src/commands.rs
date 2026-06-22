use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::watch;

use crate::{
    autostart, clipboard,
    config,
    discovery,
    error::{AppError, AppResult},
    history,
    models::{AppConfig, AppStatus, ClipboardTextItem, DeviceInfo, HistoryItem},
    security,
    state::AppState,
    sync,
};

#[tauri::command]
pub async fn get_status(state: State<'_, AppState>) -> AppResult<AppStatus> {
    Ok(state.status().await)
}

#[tauri::command]
pub async fn start_sync(app: AppHandle, state: State<'_, AppState>) -> AppResult<AppStatus> {
    let state_clone = state.inner().clone();
    let app_clone = app.clone();
    let (stop, stop_rx) = watch::channel(false);
    let join = tauri::async_runtime::spawn(sync::run_sync_runtime(app_clone, state_clone.clone(), stop_rx));
    state_clone.start_runtime(stop, join).await?;
    Ok(state.status().await)
}

#[tauri::command]
pub async fn stop_sync(state: State<'_, AppState>) -> AppResult<AppStatus> {
    state.stop_runtime().await?;
    Ok(state.status().await)
}

#[tauri::command]
pub async fn get_devices(state: State<'_, AppState>) -> AppResult<Vec<DeviceInfo>> {
    let mut devices = state.devices().await;
    devices.extend(discovery::discover_devices().await);
    Ok(devices)
}

#[tauri::command]
pub async fn connect_device(
    app: AppHandle,
    state: State<'_, AppState>,
    ip: String,
    port: u16,
) -> AppResult<DeviceInfo> {
    sync::connect_to_peer(app, state.inner().clone(), ip, port).await
}

#[tauri::command]
pub async fn disconnect_device(
    state: State<'_, AppState>,
    device_id: String,
) -> AppResult<()> {
    state.remove_peer(&device_id).await;
    state
        .mark_device_disconnected(&device_id)
        .await
        .ok_or(AppError::UnknownDevice(device_id))?;
    Ok(())
}

#[tauri::command]
pub async fn trust_device(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
) -> AppResult<()> {
    let mut next_config = state.config().await;
    security::trust_device(&mut next_config, device_id.clone());
    config::save_config(&app, &next_config)?;
    state.set_config(next_config.clone()).await;
    app.emit("config-updated", next_config)?;
    Ok(())
}

#[tauri::command]
pub async fn get_config(app: AppHandle, state: State<'_, AppState>) -> AppResult<AppConfig> {
    let mut config = state.config().await;
    if let Ok(enabled) = autostart::is_autostart_enabled(&app) {
        config.auto_start = enabled;
    }
    Ok(config)
}

#[tauri::command]
pub async fn update_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: AppConfig,
) -> AppResult<AppConfig> {
    let running = state.status().await.running;
    let current = state.config().await;
    if running && current.port != config.port {
        return Err(AppError::InvalidInput(
            "请先停止同步，再修改监听端口".to_string(),
        ));
    }
    let Some(device_name) = security::normalize_device_name(&config.device_name) else {
        return Err(AppError::InvalidInput("设备名称不能为空".to_string()));
    };

    let mut next_config = config;
    next_config.device_name = device_name;
    next_config.sync_text = true;
    next_config.sync_image = false;
    next_config.sync_files = false;
    let current_auto_start =
        autostart::is_autostart_enabled(&app).unwrap_or(current.auto_start);
    if autostart::should_update_autostart(current_auto_start, next_config.auto_start) {
        autostart::set_autostart(&app, next_config.auto_start)?;
    }
    config::save_config(&app, &next_config)?;
    state.set_config(next_config.clone()).await;
    app.emit("config-updated", next_config.clone())?;
    Ok(next_config)
}

#[tauri::command]
pub async fn get_history(state: State<'_, AppState>) -> AppResult<Vec<HistoryItem>> {
    Ok(state.history().await)
}

#[tauri::command]
pub async fn get_clipboard_history() -> AppResult<Vec<ClipboardTextItem>> {
    clipboard::read_clipboard_history_text(3).await
}

#[tauri::command]
pub async fn clear_history(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    history::clear_history(&app)?;
    state.replace_history(Vec::new()).await;
    Ok(())
}

#[tauri::command]
pub async fn show_main_window(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
    }
    Ok(())
}

#[tauri::command]
pub async fn hide_main_window(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide()?;
    }
    Ok(())
}
