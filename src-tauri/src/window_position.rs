use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSize {
    pub width: i32,
    pub height: i32,
}

pub fn move_floating_window_to_cursor(app: &AppHandle) -> AppResult<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    move_window_to_cursor(window, 16)
}

pub fn move_main_window_to_center(app: &AppHandle) -> AppResult<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };

    center_window_on_current_monitor(window)
}

pub fn floating_position_near_cursor(
    work_area: NativeRect,
    window_size: NativeSize,
    cursor: NativePoint,
    margin: i32,
) -> NativePoint {
    let min_x = work_area.left + margin;
    let min_y = work_area.top + margin;
    let max_x = work_area.right - window_size.width - margin;
    let max_y = work_area.bottom - window_size.height - margin;

    NativePoint {
        x: clamp(cursor.x - window_size.width / 2, min_x, max_x),
        y: clamp(cursor.y - window_size.height / 2, min_y, max_y),
    }
}

pub fn centered_position_in_work_area(
    work_area: NativeRect,
    window_size: NativeSize,
) -> NativePoint {
    NativePoint {
        x: work_area.left + (work_area.right - work_area.left - window_size.width) / 2,
        y: work_area.top + (work_area.bottom - work_area.top - window_size.height) / 2,
    }
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max)
}

#[cfg(target_os = "windows")]
fn move_window_to_cursor(window: tauri::WebviewWindow, margin: i32) -> AppResult<()> {
    use std::mem::size_of;
    use windows::Win32::{
        Foundation::{POINT, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
        UI::WindowsAndMessaging::{
            GetCursorPos, GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE,
            SWP_NOZORDER,
        },
    };

    unsafe {
        let hwnd = window.hwnd().map_err(AppError::from)?;

        let mut window_rect = RECT::default();
        GetWindowRect(hwnd, &mut window_rect)
            .map_err(|error| AppError::Tauri(error.to_string()))?;

        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor).map_err(|error| AppError::Tauri(error.to_string()))?;

        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            return Err(AppError::Tauri(
                "failed to get monitor work area for cursor".to_string(),
            ));
        }

        let position = floating_position_near_cursor(
            NativeRect {
                left: monitor_info.rcWork.left,
                top: monitor_info.rcWork.top,
                right: monitor_info.rcWork.right,
                bottom: monitor_info.rcWork.bottom,
            },
            NativeSize {
                width: window_rect.right - window_rect.left,
                height: window_rect.bottom - window_rect.top,
            },
            NativePoint {
                x: cursor.x,
                y: cursor.y,
            },
            margin,
        );

        SetWindowPos(
            hwnd,
            None,
            position.x,
            position.y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .map_err(|error| AppError::Tauri(error.to_string()))?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn center_window_on_current_monitor(window: tauri::WebviewWindow) -> AppResult<()> {
    use std::mem::size_of;
    use windows::Win32::{
        Foundation::RECT,
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
        UI::WindowsAndMessaging::{
            GetWindowRect, SetWindowPos, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER,
        },
    };

    unsafe {
        let hwnd = window.hwnd().map_err(AppError::from)?;

        let mut window_rect = RECT::default();
        GetWindowRect(hwnd, &mut window_rect)
            .map_err(|error| AppError::Tauri(error.to_string()))?;

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            return Err(AppError::Tauri(
                "failed to get monitor work area for window".to_string(),
            ));
        }

        let position = centered_position_in_work_area(
            NativeRect {
                left: monitor_info.rcWork.left,
                top: monitor_info.rcWork.top,
                right: monitor_info.rcWork.right,
                bottom: monitor_info.rcWork.bottom,
            },
            NativeSize {
                width: window_rect.right - window_rect.left,
                height: window_rect.bottom - window_rect.top,
            },
        );

        SetWindowPos(
            hwnd,
            None,
            position.x,
            position.y,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .map_err(|error| AppError::Tauri(error.to_string()))?;
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn move_window_to_cursor(_window: tauri::WebviewWindow, _margin: i32) -> AppResult<()> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn center_window_on_current_monitor(_window: tauri::WebviewWindow) -> AppResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        centered_position_in_work_area, floating_position_near_cursor, NativePoint, NativeRect,
        NativeSize,
    };

    #[test]
    fn floating_position_centers_window_near_cursor() {
        assert_eq!(
            floating_position_near_cursor(
                NativeRect {
                    left: 0,
                    top: 0,
                    right: 1920,
                    bottom: 1040,
                },
                NativeSize {
                    width: 340,
                    height: 320,
                },
                NativePoint { x: 960, y: 540 },
                16,
            ),
            NativePoint { x: 790, y: 380 }
        );
    }

    #[test]
    fn floating_position_clamps_to_work_area() {
        assert_eq!(
            floating_position_near_cursor(
                NativeRect {
                    left: 0,
                    top: 0,
                    right: 1920,
                    bottom: 1040,
                },
                NativeSize {
                    width: 340,
                    height: 320,
                },
                NativePoint { x: 4, y: 3 },
                16,
            ),
            NativePoint { x: 16, y: 16 }
        );
    }

    #[test]
    fn floating_position_uses_monitor_origin_for_secondary_screen() {
        assert_eq!(
            floating_position_near_cursor(
                NativeRect {
                    left: 1920,
                    top: 0,
                    right: 4480,
                    bottom: 1400,
                },
                NativeSize {
                    width: 340,
                    height: 320,
                },
                NativePoint { x: 4300, y: 1390 },
                16,
            ),
            NativePoint { x: 4124, y: 1064 }
        );
    }

    #[test]
    fn main_window_position_centers_in_work_area() {
        assert_eq!(
            centered_position_in_work_area(
                NativeRect {
                    left: 0,
                    top: 0,
                    right: 1920,
                    bottom: 1040,
                },
                NativeSize {
                    width: 1120,
                    height: 720,
                },
            ),
            NativePoint { x: 400, y: 160 }
        );
    }

    #[test]
    fn main_window_position_centers_on_secondary_screen() {
        assert_eq!(
            centered_position_in_work_area(
                NativeRect {
                    left: 1920,
                    top: 0,
                    right: 4480,
                    bottom: 1400,
                },
                NativeSize {
                    width: 1120,
                    height: 720,
                },
            ),
            NativePoint { x: 2640, y: 340 }
        );
    }
}
