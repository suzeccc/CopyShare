mod autostart;
mod clipboard;
mod commands;
mod config;
mod device_store;
mod discovery;
mod error;
mod history;
mod models;
mod network;
mod security;
mod state;
mod sync;
mod tray;

use state::AppState;
use tauri_plugin_autostart::MacosLauncher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();
    let state_for_setup = state.clone();

    tauri::Builder::default()
        .manage(state.clone())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(move |app| {
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_autostart::init(
                MacosLauncher::LaunchAgent,
                None,
            ))?;

            tauri::async_runtime::block_on(state_for_setup.load_from_disk(app.handle()))?;
            tray::setup_tray(app, state_for_setup.clone())?;

            let app_handle = app.handle().clone();
            let state_for_auto_sync = state_for_setup.clone();
            let should_auto_sync = tauri::async_runtime::block_on(async {
                sync::should_auto_start_sync(
                    &state_for_auto_sync.config().await,
                    state_for_auto_sync.status().await.running,
                )
            });
            if should_auto_sync {
                tauri::async_runtime::spawn(async move {
                    let _ = sync::start_sync_runtime(app_handle, state_for_auto_sync).await;
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::start_sync,
            commands::stop_sync,
            commands::get_devices,
            commands::connect_device,
            commands::disconnect_device,
            commands::trust_device,
            commands::reject_device,
            commands::get_config,
            commands::update_config,
            commands::get_history,
            commands::get_clipboard_history,
            commands::clear_history,
            commands::open_external_url,
            commands::show_main_window,
            commands::hide_main_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running Copy-Sharer");
}
