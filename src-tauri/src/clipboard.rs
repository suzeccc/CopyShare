use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{ColorType, ImageEncoder};
use tauri::{image::Image, AppHandle};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::error::{AppError, AppResult};
use crate::models::ClipboardTextItem;

pub fn read_clipboard_text(app: &AppHandle) -> AppResult<String> {
    app.clipboard()
        .read_text()
        .map_err(|error| AppError::Clipboard(error.to_string()))
}

pub fn write_clipboard_text(app: &AppHandle, text: &str) -> AppResult<()> {
    app.clipboard()
        .write_text(text.to_string())
        .map_err(|error| AppError::Clipboard(error.to_string()))
}

pub fn read_clipboard_image_base64(app: &AppHandle) -> AppResult<Option<String>> {
    if let Ok(image) = app.clipboard().read_image() {
        return image_to_png_base64(&image).map(Some);
    }

    match read_clipboard_image_file_base64() {
        Ok(image) => Ok(image),
        Err(_) => Ok(None),
    }
}

pub fn write_clipboard_image_base64(app: &AppHandle, content: &str) -> AppResult<()> {
    let image = png_base64_to_image(content)?;
    app.clipboard()
        .write_image(&image)
        .map_err(|error| AppError::Clipboard(error.to_string()))
}

pub fn image_to_png_base64(image: &Image<'_>) -> AppResult<String> {
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    encoder
        .write_image(
            image.rgba(),
            image.width(),
            image.height(),
            ColorType::Rgba8.into(),
        )
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    Ok(STANDARD.encode(png))
}

pub fn png_base64_to_image(content: &str) -> AppResult<Image<'static>> {
    let bytes = STANDARD
        .decode(content)
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| AppError::Clipboard(error.to_string()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    Ok(Image::new_owned(image.into_raw(), width, height))
}

pub fn image_file_to_png_base64(path: &Path) -> AppResult<String> {
    let image = image::open(path)
        .map_err(|error| AppError::Clipboard(error.to_string()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    image_to_png_base64(&Image::new_owned(image.into_raw(), width, height))
}

fn image_file_path_to_png_base64(path: &Path) -> AppResult<Option<String>> {
    if !is_supported_image_file(path) {
        return Ok(None);
    }

    image_file_to_png_base64(path).map(Some)
}

fn is_supported_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "bmp" | "gif" | "jpeg" | "jpg" | "png" | "webp"
            )
        })
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn read_clipboard_image_file_base64() -> AppResult<Option<String>> {
    for path in read_clipboard_file_paths()? {
        if let Some(image) = image_file_path_to_png_base64(&path)? {
            return Ok(Some(image));
        }
    }

    Ok(None)
}

#[cfg(not(target_os = "windows"))]
fn read_clipboard_image_file_base64() -> AppResult<Option<String>> {
    Ok(None)
}

#[cfg(target_os = "windows")]
fn read_clipboard_file_paths() -> AppResult<Vec<PathBuf>> {
    use std::os::windows::ffi::OsStringExt;

    use windows::Win32::{
        System::{
            DataExchange::{
                CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
            },
            Ole::CF_HDROP,
        },
        UI::Shell::{DragQueryFileW, HDROP},
    };

    struct ClipboardGuard;

    impl Drop for ClipboardGuard {
        fn drop(&mut self) {
            let _ = unsafe { CloseClipboard() };
        }
    }

    if unsafe { IsClipboardFormatAvailable(CF_HDROP.0 as u32) }.is_err() {
        return Ok(Vec::new());
    }
    unsafe { OpenClipboard(None) }.map_err(|error| AppError::Clipboard(error.to_string()))?;
    let _guard = ClipboardGuard;
    let handle = unsafe { GetClipboardData(CF_HDROP.0 as u32) }
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let drop_handle = HDROP(handle.0);
    let count = unsafe { DragQueryFileW(drop_handle, u32::MAX, None) };
    let mut paths = Vec::new();

    for index in 0..count {
        let len = unsafe { DragQueryFileW(drop_handle, index, None) };
        if len == 0 {
            continue;
        }
        let mut buffer = vec![0u16; len as usize + 1];
        let written = unsafe { DragQueryFileW(drop_handle, index, Some(&mut buffer)) };
        if written == 0 {
            continue;
        }
        let path = std::ffi::OsString::from_wide(&buffer[..written as usize]);
        paths.push(PathBuf::from(path));
    }

    Ok(paths)
}

#[cfg(target_os = "windows")]
pub async fn read_clipboard_history_text(limit: usize) -> AppResult<Vec<ClipboardTextItem>> {
    use windows::ApplicationModel::DataTransfer::{
        Clipboard, ClipboardHistoryItemsResultStatus, StandardDataFormats,
    };

    if !Clipboard::IsHistoryEnabled().map_err(|error| AppError::Clipboard(error.to_string()))? {
        return Ok(Vec::new());
    }

    let result = Clipboard::GetHistoryItemsAsync()
        .map_err(|error| AppError::Clipboard(error.to_string()))?
        .get()
        .map_err(|error| AppError::Clipboard(error.to_string()))?;

    if result
        .Status()
        .map_err(|error| AppError::Clipboard(error.to_string()))?
        != ClipboardHistoryItemsResultStatus::Success
    {
        return Ok(Vec::new());
    }

    let text_format = StandardDataFormats::Text()
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let history_items = result
        .Items()
        .map_err(|error| AppError::Clipboard(error.to_string()))?;
    let mut items = Vec::new();

    for item in history_items {
        if items.len() >= limit {
            break;
        }

        let content = item
            .Content()
            .map_err(|error| AppError::Clipboard(error.to_string()))?;
        if !content
            .Contains(&text_format)
            .map_err(|error| AppError::Clipboard(error.to_string()))?
        {
            continue;
        }

        let text = content
            .GetTextAsync()
            .map_err(|error| AppError::Clipboard(error.to_string()))?
            .get()
            .map_err(|error| AppError::Clipboard(error.to_string()))?
            .to_string()
            .trim()
            .to_string();

        if text.is_empty() {
            continue;
        }

        items.push(ClipboardTextItem {
            id: item
                .Id()
                .map_err(|error| AppError::Clipboard(error.to_string()))?
                .to_string(),
            text,
        });
    }

    Ok(items)
}

#[cfg(not(target_os = "windows"))]
pub async fn read_clipboard_history_text(_limit: usize) -> AppResult<Vec<ClipboardTextItem>> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_payload_round_trips_as_png_base64() {
        let image = tauri::image::Image::new_owned(
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
            2,
            2,
        );

        let encoded = image_to_png_base64(&image).expect("image should encode to PNG base64");
        let decoded = png_base64_to_image(&encoded).expect("PNG base64 should decode to image");

        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        assert_eq!(decoded.rgba(), image.rgba());
    }

    #[test]
    fn image_file_payload_encodes_as_png_base64() {
        let path = std::env::temp_dir().join("copyshare-image-file-test.png");
        let image = tauri::image::Image::new_owned(
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
            2,
            2,
        );
        let png_base64 = image_to_png_base64(&image).expect("image should encode");
        std::fs::write(
            &path,
            STANDARD
                .decode(png_base64)
                .expect("test PNG base64 should decode"),
        )
        .expect("test image should be written");

        let encoded =
            image_file_to_png_base64(&path).expect("image file should be encoded as PNG base64");
        let decoded = png_base64_to_image(&encoded).expect("encoded image should decode");

        let _ = std::fs::remove_file(path);
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
        assert_eq!(decoded.rgba(), image.rgba());
    }

    #[test]
    fn non_image_file_path_is_ignored() {
        let path = std::env::temp_dir().join("copyshare-not-image.txt");

        assert!(
            image_file_path_to_png_base64(&path)
                .expect("unsupported extension should not be an error")
                .is_none()
        );
    }
}
