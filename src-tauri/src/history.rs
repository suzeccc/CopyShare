use std::{fs, path::PathBuf};

use chrono::Utc;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::{
    error::AppResult,
    models::{ClipboardContentType, ClipboardMessage, HistoryDirection, HistoryItem},
};

const HISTORY_FILE: &str = "history.json";
const HISTORY_IMAGE_DIR: &str = "history-images";
const HISTORY_IMAGE_EXTENSION: &str = "b64";
const SUMMARY_LIMIT: usize = 80;
const HISTORY_LIMIT: usize = 100;

pub fn load_history(app: &AppHandle) -> AppResult<Vec<HistoryItem>> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let text = fs::read_to_string(path)?;
    let mut items = load_history_items_from_text(&text)?;
    restore_history_images(&mut items, &history_images_dir(app)?);
    Ok(items)
}

pub fn save_history(app: &AppHandle, items: &[HistoryItem]) -> AppResult<()> {
    let path = history_path(app)?;
    let image_dir = history_images_dir(app)?;
    save_history_images(&image_dir, items)?;
    prune_history_images(&image_dir, items)?;
    let text = serde_json::to_string_pretty(&history_items_for_disk(items))?;
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
        summary: summarize_message(message),
        content: history_content(message),
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

fn summarize_message(message: &ClipboardMessage) -> String {
    match message.content_type {
        ClipboardContentType::Text => summarize(&message.content),
        ClipboardContentType::Image => {
            format!("图片 {}", format_size(base64_payload_size(&message.content)))
        }
        ClipboardContentType::FileList => "文件列表".to_string(),
    }
}

fn history_content(message: &ClipboardMessage) -> String {
    match message.content_type {
        ClipboardContentType::Text | ClipboardContentType::Image => message.content.clone(),
        ClipboardContentType::FileList => String::new(),
    }
}

fn history_items_for_disk(items: &[HistoryItem]) -> Vec<HistoryItem> {
    items
        .iter()
        .map(|item| {
            let mut item = item.clone();
            if item.content_type != ClipboardContentType::Text {
                item.content.clear();
            }
            item
        })
        .collect()
}

fn save_history_images(image_dir: &PathBuf, items: &[HistoryItem]) -> AppResult<()> {
    fs::create_dir_all(image_dir)?;
    for item in items {
        if item.content_type == ClipboardContentType::Image && !item.content.trim().is_empty() {
            fs::write(history_image_path(image_dir, &item.id), &item.content)?;
        }
    }
    Ok(())
}

fn restore_history_images(items: &mut [HistoryItem], image_dir: &PathBuf) {
    for item in items {
        if item.content_type == ClipboardContentType::Image && item.content.trim().is_empty() {
            if let Ok(content) = fs::read_to_string(history_image_path(image_dir, &item.id)) {
                item.content = content;
            }
        }
    }
}

fn prune_history_images(image_dir: &PathBuf, items: &[HistoryItem]) -> AppResult<()> {
    if !image_dir.exists() {
        return Ok(());
    }

    let keep_files: std::collections::HashSet<String> = items
        .iter()
        .filter(|item| item.content_type == ClipboardContentType::Image)
        .map(|item| history_image_file_name(&item.id))
        .collect();

    for entry in fs::read_dir(image_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !keep_files.contains(&file_name) {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

fn history_image_path(image_dir: &PathBuf, id: &str) -> PathBuf {
    image_dir.join(history_image_file_name(id))
}

fn history_image_file_name(id: &str) -> String {
    let safe_id: String = id
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect();
    format!("{safe_id}.{HISTORY_IMAGE_EXTENSION}")
}

fn base64_payload_size(content: &str) -> usize {
    let trimmed = content.trim_end_matches('=');
    (trimmed.len() * 3) / 4
}

fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else {
        format!("{} KB", (bytes + 1023) / 1024)
    }
}

fn history_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join(HISTORY_FILE))
}

fn history_images_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?.join(HISTORY_IMAGE_DIR);
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn load_history_items_from_text(text: &str) -> AppResult<Vec<HistoryItem>> {
    match serde_json::from_str(text) {
        Ok(items) => Ok(items),
        Err(_) => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};

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

    #[test]
    fn history_item_preserves_full_content_for_copying() {
        let message = ClipboardMessage {
            message_id: "m".to_string(),
            source_device_id: "d".to_string(),
            source_device_name: "Device".to_string(),
            content_type: crate::models::ClipboardContentType::Text,
            content: "a".repeat(120),
            content_hash: "hash".to_string(),
            timestamp: 1,
        };

        let item = make_history_item(HistoryDirection::Local, "Device", &message);

        assert_eq!(item.summary.len(), 83);
        assert_eq!(item.content.len(), 120);
    }

    #[test]
    fn image_history_keeps_content_in_memory_for_copying() {
        let content = STANDARD.encode(vec![0; 4096]);
        let message = ClipboardMessage {
            message_id: "m".to_string(),
            source_device_id: "d".to_string(),
            source_device_name: "Device".to_string(),
            content_type: crate::models::ClipboardContentType::Image,
            content: content.clone(),
            content_hash: "hash".to_string(),
            timestamp: 1,
        };

        let item = make_history_item(HistoryDirection::Local, "Device", &message);

        assert_eq!(item.summary, "图片 4 KB");
        assert_eq!(item.content, content);
    }

    #[test]
    fn image_history_content_is_stripped_before_persisting_to_disk() {
        let content = STANDARD.encode(vec![0; 4096]);
        let message = ClipboardMessage {
            message_id: "m".to_string(),
            source_device_id: "d".to_string(),
            source_device_name: "Device".to_string(),
            content_type: crate::models::ClipboardContentType::Image,
            content,
            content_hash: "hash".to_string(),
            timestamp: 1,
        };

        let item = make_history_item(HistoryDirection::Local, "Device", &message);
        let disk_items = history_items_for_disk(&[item]);

        assert!(disk_items[0].summary.ends_with("4 KB"));
        assert!(disk_items[0].content.is_empty());
    }

    #[test]
    fn image_history_content_round_trips_through_cache_files() -> AppResult<()> {
        let image_dir = std::env::temp_dir().join(format!(
            "copyshare-history-images-{}",
            Uuid::new_v4()
        ));
        let content = STANDARD.encode(vec![7; 2048]);
        let mut item = HistoryItem {
            id: "image:one".to_string(),
            direction: HistoryDirection::Local,
            source_device: "Device".to_string(),
            summary: "image".to_string(),
            content: content.clone(),
            content_type: ClipboardContentType::Image,
            success: true,
            created_at: Utc::now(),
        };

        save_history_images(&image_dir, &[item.clone()])?;
        item.content.clear();
        restore_history_images(std::slice::from_mut(&mut item), &image_dir);

        assert_eq!(item.content, content);
        fs::remove_dir_all(image_dir)?;
        Ok(())
    }

    #[test]
    fn stale_history_image_cache_files_are_pruned() -> AppResult<()> {
        let image_dir = std::env::temp_dir().join(format!(
            "copyshare-history-images-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&image_dir)?;
        fs::write(image_dir.join("stale.b64"), "old")?;
        let item = HistoryItem {
            id: "keep".to_string(),
            direction: HistoryDirection::Local,
            source_device: "Device".to_string(),
            summary: "image".to_string(),
            content: STANDARD.encode(vec![1; 16]),
            content_type: ClipboardContentType::Image,
            success: true,
            created_at: Utc::now(),
        };

        save_history_images(&image_dir, &[item.clone()])?;
        prune_history_images(&image_dir, &[item])?;

        assert!(!image_dir.join("stale.b64").exists());
        assert!(image_dir.join("keep.b64").exists());
        fs::remove_dir_all(image_dir)?;
        Ok(())
    }

    #[test]
    fn corrupted_history_json_does_not_block_startup() {
        let items = load_history_items_from_text(r#"[{"id": "broken", "summary": "unterminated}"#)
            .expect("corrupted history should fall back to empty history");

        assert!(items.is_empty());
    }
}
