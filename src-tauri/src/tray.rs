use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    App, AppHandle,
};
use crate::{
    i18n,
    models::AppConfig,
    notifications,
    state::AppState,
    sync,
};

const TRAY_ID: &str = "copyshare-main";
const TRAY_ICON: tauri::image::Image<'static> = tauri::include_image!("icons/16x16.png");

pub fn setup_tray(app: &mut App, state: AppState) -> tauri::Result<()> {
    let config = tauri::async_runtime::block_on(state.config());
    let show = MenuItem::with_id(
        app,
        "show",
        i18n::translate(&config, "显示窗口"),
        true,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(
        app,
        "pause",
        i18n::translate(&config, "暂停同步"),
        true,
        None::<&str>,
    )?;
    let resume = MenuItem::with_id(
        app,
        "resume",
        i18n::translate(&config, "继续同步"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        i18n::translate(&config, "退出"),
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&show, &pause, &resume, &quit])?;

    let state_for_menu = state.clone();

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(TRAY_ICON)
        .tooltip(i18n::translate(&config, "CopyShare - 待启动同步"))
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

pub fn update_tray_locale(app: &AppHandle, config: &AppConfig) -> tauri::Result<()> {
    let show = MenuItem::with_id(
        app,
        "show",
        i18n::translate(config, "显示窗口"),
        true,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(
        app,
        "pause",
        i18n::translate(config, "暂停同步"),
        true,
        None::<&str>,
    )?;
    let resume = MenuItem::with_id(
        app,
        "resume",
        i18n::translate(config, "继续同步"),
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        i18n::translate(config, "退出"),
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&show, &pause, &resume, &quit])?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

pub async fn update_tray_status(app: &AppHandle, state: &AppState) {
    let status = state.status().await;
    let config = state.config().await;
    let message = if status.running {
        format!("CopyShare - 运行中，已连接 {} 台设备", status.connected_count)
    } else {
        "CopyShare - 同步已暂停".to_string()
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(i18n::translate(&config, &message)));
    }
}
