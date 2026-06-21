use std::{fs, path::PathBuf};

use chrono::Utc;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::{
    error::AppResult,
    models::{ClipboardMessage, HistoryDirection, HistoryItem},
};

const HISTORY_FILE: &str = "history.json";
const SUMMARY_LIMIT: usize = 80;
const HISTORY_LIMIT: usize = 100;

pub fn load_history(app: &AppHandle) -> AppResult<Vec<HistoryItem>> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

pub fn save_history(app: &AppHandle, items: &[HistoryItem]) -> AppResult<()> {
    let path = history_path(app)?;
    let text = serde_json::to_string_pretty(items)?;
    fs::write(path, text)?;
    Ok(())
}

pub fn clear_history(app: &AppHandle) -> AppResult<()> {
    save_history(app, &[])
}

pub fn make_history_item(
    direction: HistoryDirection,
    source_device: impl Into<String>,
    message: &ClipboardMessage,
) -> HistoryItem {
    HistoryItem {
        id: Uuid::new_v4().to_string(),
        direction,
        source_device: source_device.into(),
        summary: summarize(&message.content),
        content_type: message.content_type.clone(),
        success: true,
        created_at: Utc::now(),
    }
}

pub fn push_history(items: &mut Vec<HistoryItem>, item: HistoryItem) {
    items.insert(0, item);
    items.truncate(HISTORY_LIMIT);
}

pub fn summarize(content: &str) -> String {
    let mut summary = String::new();
    for (index, ch) in content.chars().enumerate() {
        if index >= SUMMARY_LIMIT {
            summary.push_str("...");
            break;
        }
        summary.push(ch);
    }
    summary
}

fn history_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join(HISTORY_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_is_limited_to_eighty_characters() {
        let content = "a".repeat(120);
        let summary = summarize(&content);

        assert_eq!(summary.len(), 83);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn history_is_capped_to_latest_items() {
        let mut items = Vec::new();
        let message = ClipboardMessage {
            message_id: "m".to_string(),
            source_device_id: "d".to_string(),
            source_device_name: "Device".to_string(),
            content_type: crate::models::ClipboardContentType::Text,
            content: "hello".to_string(),
            content_hash: "hash".to_string(),
            timestamp: 1,
        };

        for _ in 0..105 {
            push_history(
                &mut items,
                make_history_item(HistoryDirection::Local, "Device", &message),
            );
        }

        assert_eq!(items.len(), 100);
    }
}
