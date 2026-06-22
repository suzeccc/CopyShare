use tauri::AppHandle;
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
