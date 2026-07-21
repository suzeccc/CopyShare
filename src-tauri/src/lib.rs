mod autostart;
mod clipboard;
mod commands;
mod config;
mod device_store;
mod discovery;
mod error;
mod file_transfer;
mod file_transfer_http;
mod file_transfer_store;
mod history;
mod library;
mod mobile;
mod models;
mod network;
mod network_diagnostics;
mod notifications;
mod ocr;
mod security;
mod state;
mod sync;
mod tray;
mod translator;
mod window_position;

use state::AppState;
use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();
    let state_for_setup = state.clone();

    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .manage(state.clone())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .setup(move |app| {
            notifications::configure_process_app_id(app.handle());

            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
                app.handle().plugin(tauri_plugin_autostart::init(
                    MacosLauncher::LaunchAgent,
                    None,
                ))?;
            }

            tauri::async_runtime::block_on(async {
                state_for_setup.load_from_disk(app.handle()).await?;
                file_transfer::initialize(app.handle(), &state_for_setup).await
            })?;
            tray::setup_tray(app, state_for_setup.clone())?;
            sync::start_clipboard_monitor(app.handle().clone(), state_for_setup.clone());
            discovery::start_discovery_runtime(app.handle().clone(), state_for_setup.clone());

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
                    let _ = sync::start_sync_runtime(app_handle.clone(), state_for_auto_sync.clone()).await;
                    tray::update_tray_status(&app_handle, &state_for_auto_sync).await;
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
            commands::get_network_diagnostics,
            commands::repair_windows_firewall,
            commands::get_history,
            commands::set_history_item_pinned,
            commands::get_library,
            commands::collect_history_item,
            commands::create_text_snippet,
            commands::update_library_item,
            commands::convert_library_item_to_snippet,
            commands::set_library_item_pinned,
            commands::reorder_pinned_library_items,
            commands::remove_library_item,
            commands::copy_library_item,
            commands::get_library_storage_size,
            commands::get_library_image_thumbnail,
            commands::get_clipboard_history,
            commands::read_clipboard_text,
            commands::recognize_clipboard_image,
            commands::translate_text,
            commands::select_file_for_transfer,
            commands::select_files_for_transfer,
            commands::send_file_to_device,
            commands::send_files_to_device,
            commands::accept_file_transfer,
            commands::reject_file_transfer,
            commands::cancel_file_transfer,
            commands::resume_file_transfer,
            commands::get_file_transfers,
            commands::get_transfer_save_dir,
            commands::select_transfer_save_dir,
            commands::reset_transfer_save_dir,
            commands::open_transfer_folder,
            commands::open_history_file_location,
            commands::create_mobile_session,
            commands::get_mobile_session_status,
            commands::close_mobile_session,
            commands::confirm_mobile_clipboard_write,
            commands::clear_history,
            commands::get_cache_size,
            commands::clear_cache,
            commands::copy_history_item,
            commands::get_history_image_thumbnail,
            commands::get_history_file_thumbnail,
            commands::get_history_file_preview_path,
            commands::open_external_url,
            commands::show_main_window,
            commands::hide_main_window,
            commands::exit_app,
            commands::send_test_notification,
            commands::move_floating_window_to_cursor,
            commands::move_main_window_to_center
        ])
        .run(tauri::generate_context!())
        .expect("error while running CopyShare");
}

