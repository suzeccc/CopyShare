use std::{process::Command, time::Duration};

use tauri::{AppHandle, Emitter, Manager, State};
use url::Url;
use crate::{
    autostart, clipboard,
    config,
    discovery,
    device_store,
    error::{AppError, AppResult},
    history,
    models::{AppConfig, AppStatus, ClipboardContentType, ClipboardTextItem, DeviceInfo, HistoryItem},
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
    let app_state = state.inner().clone();
    sync::start_sync_runtime(app, app_state.clone()).await?;
    sync::wait_for_sync_ready(&app_state, Duration::from_secs(2)).await?;
    Ok(state.status().await)
}

#[tauri::command]
pub async fn stop_sync(app: AppHandle, state: State<'_, AppState>) -> AppResult<AppStatus> {
    state.stop_runtime().await?;
    device_store::save_devices(&app, &state.devices().await)?;
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
    if let Some(device) = state.connected_device_for_endpoint(&ip, port).await? {
        return Ok(device);
    }

    let app_state = state.inner().clone();
    if sync::should_start_sync_for_manual_connect(state.status().await.running) {
        match sync::start_sync_runtime(app.clone(), app_state.clone()).await {
            Ok(()) | Err(AppError::AlreadyRunning) => {}
            Err(error) => return Err(error),
        }
        sync::wait_for_sync_ready(&app_state, Duration::from_secs(2)).await?;
    }

    sync::connect_to_peer(app, app_state, ip, port).await
}

#[tauri::command]
pub async fn disconnect_device(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
) -> AppResult<()> {
    state.remove_peer(&device_id).await;
    state
        .mark_device_disconnected(&device_id)
        .await
        .ok_or(AppError::UnknownDevice(device_id))?;
    device_store::save_devices(&app, &state.devices().await)?;
    Ok(())
}

#[tauri::command]
pub async fn trust_device(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
) -> AppResult<()> {
    let mut next_config = state.config().await;
    for key in state.trust_keys_for_device(&device_id).await {
        security::trust_device(&mut next_config, key);
    }
    config::save_config(&app, &next_config)?;
    state.set_config(next_config.clone()).await;
    state.mark_device_trusted(&device_id).await;
    state.clear_manual_trust_required(&device_id).await;
    state.reset_local_clipboard_observation().await;
    device_store::save_devices(&app, &state.devices().await)?;
    sync::notify_peer_trusted(state.inner(), &next_config, &device_id).await;
    app.emit("config-updated", next_config)?;
    Ok(())
}

#[tauri::command]
pub async fn reject_device(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
) -> AppResult<()> {
    let trust_keys = state.trust_keys_for_device(&device_id).await;
    state.remove_peer(&device_id).await;
    state.remove_device(&device_id).await;

    let mut next_config = state.config().await;
    for key in trust_keys {
        security::untrust_device(&mut next_config, &key);
    }
    state.clear_manual_trust_required(&device_id).await;
    config::save_config(&app, &next_config)?;
    state.set_config(next_config.clone()).await;
    device_store::save_devices(&app, &state.devices().await)?;
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
    next_config.device_id = if next_config.device_id.trim().is_empty() {
        current.device_id.clone()
    } else {
        next_config.device_id.trim().to_string()
    };
    next_config.sync_text = true;
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
pub async fn copy_history_item(
    app: AppHandle,
    state: State<'_, AppState>,
    history_id: String,
) -> AppResult<()> {
    let item = state
        .history()
        .await
        .into_iter()
        .find(|item| item.id == history_id)
        .ok_or(AppError::InvalidInput("历史记录不存在".to_string()))?;

    match item.content_type {
        ClipboardContentType::Text => {
            let text = if item.content.trim().is_empty() {
                item.summary
            } else {
                item.content
            };
            clipboard::write_clipboard_text(&app, &text)
        }
        ClipboardContentType::Image => {
            if item.content.trim().is_empty() {
                return Err(AppError::InvalidInput(
                    "这条图片历史没有可复制的图片内容，请重新复制或同步一次图片".to_string(),
                ));
            }
            clipboard::write_clipboard_image_base64(&app, &item.content)
        }
        ClipboardContentType::FileList => Err(AppError::InvalidInput(
            "暂不支持复制文件列表历史".to_string(),
        )),
    }
}

#[tauri::command]
pub async fn open_external_url(url: String) -> AppResult<()> {
    let parsed = Url::parse(&url).map_err(|_| {
        AppError::InvalidInput("只能打开有效的 http 或 https 链接".to_string())
    })?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(AppError::InvalidInput(
                "只能打开 http 或 https 链接".to_string(),
            ));
        }
    }

    open_url_with_system_browser(parsed.as_str())?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn open_url_with_system_browser(url: &str) -> AppResult<()> {
    Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_url_with_system_browser(url: &str) -> AppResult<()> {
    Command::new("open").arg(url).spawn()?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_url_with_system_browser(url: &str) -> AppResult<()> {
    Command::new("xdg-open").arg(url).spawn()?;
    Ok(())
}

#[tauri::command]
pub async fn show_main_window(app: AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.unminimize()?;
        window.center()?;
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
