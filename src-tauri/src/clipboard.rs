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
    let image = match app.clipboard().read_image() {
        Ok(image) => image,
        Err(_) => return Ok(None),
    };
    image_to_png_base64(&image).map(Some)
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
}
