use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle,
};
use crate::{notifications, state::AppState, sync};

const TRAY_ID: &str = "copyshare-main";

pub fn setup_tray(app: &mut App, state: AppState) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "暂停同步", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "继续同步", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &pause, &resume, &quit])?;

    let state_for_menu = state.clone();

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("CopyShare - 待启动同步")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                notifications::show_main_window(app);
            }
            "pause" => {
                let state = state_for_menu.clone();
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = state.stop_runtime().await;
                    update_tray_status(&app_handle, &state).await;
                });
            }
            "resume" => {
                let state = state_for_menu.clone();
                let app_handle = app.clone();
                let app_for_status = app.clone();
                let state_for_status = state.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = sync::start_sync_runtime(app_handle, state).await;
                    update_tray_status(&app_for_status, &state_for_status).await;
                });
                notifications::show_main_window(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                notifications::show_main_window(app);
            }
        })
        .build(app)?;

    Ok(())
}

pub async fn update_tray_status(app: &AppHandle, state: &AppState) {
    let status = state.status().await;
    let message = if status.running {
        format!("CopyShare - 运行中，已连接 {} 台设备", status.connected_count)
    } else {
        "CopyShare - 同步已暂停".to_string()
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(message));
    }
}
