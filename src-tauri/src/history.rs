use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use base64::Engine;
use chrono::Utc;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::{
    error::AppResult,
    models::{ClipboardContentType, ClipboardMessage, HistoryDirection, HistoryItem, SyncStatus},
};

const HISTORY_FILE: &str = "history.json";
const HISTORY_IMAGE_DIR: &str = "history-images";
const HISTORY_IMAGE_EXTENSION: &str = "b64";
const HISTORY_THUMBNAIL_DIR: &str = "history-thumbnails";
const HISTORY_THUMBNAIL_EXTENSION: &str = "png";
const SUMMARY_LIMIT: usize = 80;
const HISTORY_LIMIT: usize = 100;
const DEFAULT_THUMBNAIL_SIZE: u32 = 200;

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
    let thumbnail_dir = history_thumbnails_dir(app)?;
    save_history_images(&image_dir, items)?;
    prune_history_images(&image_dir, items)?;
    prune_history_thumbnails(&thumbnail_dir, items)?;
    let text = serde_json::to_string_pretty(&history_items_for_disk(items))?;
    fs::write(path, text)?;
    Ok(())
}

pub fn clear_history(app: &AppHandle) -> AppResult<()> {
    save_history(app, &[])
}

pub fn cache_size(app: &AppHandle) -> AppResult<u64> {
    cache_size_from_app_data_dir(&app.path().app_data_dir()?)
}

pub fn clear_cache(app: &AppHandle) -> AppResult<u64> {
    clear_cache_from_app_data_dir(&app.path().app_data_dir()?)
}

pub fn make_history_item(
    direction: HistoryDirection,
    source_device: impl Into<String>,
    message: &ClipboardMessage,
) -> HistoryItem {
    make_history_item_with_status(direction, source_device, message, SyncStatus::Synced)
}

pub fn make_history_item_with_status(
    direction: HistoryDirection,
    source_device: impl Into<String>,
    message: &ClipboardMessage,
    sync_status: SyncStatus,
) -> HistoryItem {
    HistoryItem {
        id: Uuid::new_v4().to_string(),
        direction,
        source_device: source_device.into(),
        summary: summarize_message(message),
        content: history_content(message),
        content_hash: message.content_hash.clone(),
        content_type: message.content_type.clone(),
        sync_status,
        file_transfer_id: None,
        file_transfer_status: None,
        success: true,
        created_at: Utc::now(),
    }
}

pub fn push_history(items: &mut Vec<HistoryItem>, item: HistoryItem) {
    items.insert(0, item);
    items.truncate(HISTORY_LIMIT);
}

pub fn upsert_history_by_content(items: &mut Vec<HistoryItem>, item: HistoryItem) -> HistoryItem {
    if let Some(index) = items
        .iter()
        .position(|existing| should_update_existing_history(existing, &item))
    {
        let mut existing = items.remove(index);
        existing.direction = item.direction;
        existing.source_device = item.source_device;
        existing.summary = item.summary;
        if !item.content.trim().is_empty() {
            existing.content = item.content;
        }
        existing.content_type = item.content_type;
        existing.file_transfer_id = item.file_transfer_id;
        existing.file_transfer_status = item.file_transfer_status;
        existing.success = item.success;
        existing.created_at = item.created_at;
        if item.sync_status == SyncStatus::Synced {
            existing.sync_status = SyncStatus::Synced;
        }
        items.insert(0, existing.clone());
        items.truncate(HISTORY_LIMIT);
        return existing;
    }

    push_history(items, item.clone());
    item
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

fn should_update_existing_history(existing: &HistoryItem, item: &HistoryItem) -> bool {
    existing.sync_status == SyncStatus::Unsynced
        && existing.content_type == item.content_type
        && history_identity(existing) == history_identity(item)
}

fn history_identity(item: &HistoryItem) -> &str {
    let content = item.content.trim();
    if content.is_empty() {
        item.summary.trim()
    } else {
        content
    }
}

fn summarize_message(message: &ClipboardMessage) -> String {
    match message.content_type {
        ClipboardContentType::Text => summarize(&message.content),
        ClipboardContentType::Image => "图片".to_string(),
        ClipboardContentType::FileList => crate::clipboard::summarize_file_content(&message.content),
    }
}

pub fn update_file_transfer_history(
    items: &mut Vec<HistoryItem>,
    transfer_id: &str,
    status: crate::models::FileTransferStatus,
    content: Option<String>,
) -> Option<HistoryItem> {
    let item = items
        .iter_mut()
        .find(|item| item.file_transfer_id.as_deref() == Some(transfer_id))?;
    item.file_transfer_status = Some(status);
    if let Some(content) = content {
        if !content.trim().is_empty() {
            item.content = content;
        }
    }
    Some(item.clone())
}

fn history_content(message: &ClipboardMessage) -> String {
    match message.content_type {
        ClipboardContentType::Text | ClipboardContentType::Image | ClipboardContentType::FileList => {
            message.content.clone()
        }
    }
}

fn history_items_for_disk(items: &[HistoryItem]) -> Vec<HistoryItem> {
    items
        .iter()
        .map(strip_frontend_heavy_content)
        .collect()
}

pub fn history_items_for_frontend(items: &[HistoryItem]) -> Vec<HistoryItem> {
    items.iter().map(strip_frontend_heavy_content).collect()
}

pub fn history_item_for_frontend(item: &HistoryItem) -> HistoryItem {
    strip_frontend_heavy_content(item)
}

fn strip_frontend_heavy_content(item: &HistoryItem) -> HistoryItem {
    let mut item = item.clone();
    if item.content_type == ClipboardContentType::Image {
        item.content.clear();
    }
    item
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

fn cache_size_from_app_data_dir(app_data_dir: &Path) -> AppResult<u64> {
    let mut total = 0;
    for dir_name in [HISTORY_IMAGE_DIR, HISTORY_THUMBNAIL_DIR] {
        total += directory_size(&app_data_dir.join(dir_name))?;
    }
    Ok(total)
}

fn clear_cache_from_app_data_dir(app_data_dir: &Path) -> AppResult<u64> {
    for dir_name in [HISTORY_IMAGE_DIR, HISTORY_THUMBNAIL_DIR] {
        let dir = app_data_dir.join(dir_name);
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
        fs::create_dir_all(&dir)?;
    }
    cache_size_from_app_data_dir(app_data_dir)
}

fn directory_size(path: &Path) -> AppResult<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            total += metadata.len();
        } else if metadata.is_dir() {
            total += directory_size(&entry.path())?;
        }
    }
    Ok(total)
}

pub fn get_history_image_thumbnail(
    app: &AppHandle,
    item: &HistoryItem,
    max_size: Option<u32>,
) -> AppResult<String> {
    get_history_image_thumbnail_from_dirs(
        &history_images_dir(app)?,
        &history_thumbnails_dir(app)?,
        item,
        max_size.unwrap_or(DEFAULT_THUMBNAIL_SIZE),
    )
}

pub fn get_history_file_thumbnail(
    app: &AppHandle,
    item: &HistoryItem,
    max_size: Option<u32>,
) -> AppResult<String> {
    if item.content_type != ClipboardContentType::FileList {
        return Err(crate::error::AppError::InvalidInput(
            "历史记录不是文件".to_string(),
        ));
    }

    let max_size = max_size.unwrap_or(DEFAULT_THUMBNAIL_SIZE).max(1);
    let thumbnail_dir = history_thumbnails_dir(app)?;
    fs::create_dir_all(&thumbnail_dir)?;
    let thumbnail_path = video_thumbnail_path(&thumbnail_dir, &item.id, max_size);
    if thumbnail_path.exists() {
        return Ok(base64::engine::general_purpose::STANDARD.encode(fs::read(thumbnail_path)?));
    }

    if let Some(thumbnail) = embedded_video_thumbnail(item)? {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(thumbnail.trim())
            .map_err(|error| crate::error::AppError::Clipboard(error.to_string()))?;
        fs::write(&thumbnail_path, &bytes)?;
        return Ok(base64::engine::general_purpose::STANDARD.encode(bytes));
    }

    let path = crate::file_transfer::file_path_from_history_item(item)?;
    let thumbnail = video_thumbnail_base64_for_path(&path, max_size)?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(thumbnail.trim())
        .map_err(|error| crate::error::AppError::Clipboard(error.to_string()))?;
    fs::write(&thumbnail_path, &bytes)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

pub fn get_history_file_preview_path(item: &HistoryItem) -> AppResult<String> {
    if item.content_type != ClipboardContentType::FileList {
        return Err(crate::error::AppError::InvalidInput(
            "历史记录不是文件".to_string(),
        ));
    }

    for entry in crate::clipboard::clipboard_content_to_file_entries(&item.content)? {
        let path = PathBuf::from(entry.path);
        if (is_video_file_name(&entry.name) || is_video_file_path(&path)) && path.exists() {
            return Ok(path.to_string_lossy().to_string());
        }
    }

    Err(crate::error::AppError::InvalidInput(
        "视频文件不存在或尚未下载".to_string(),
    ))
}

pub fn video_thumbnail_base64_for_path(path: &Path, max_size: u32) -> AppResult<String> {
    if !is_video_file_path(path) {
        return Err(crate::error::AppError::InvalidInput(
            "文件不是视频".to_string(),
        ));
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(
        thumbnail_from_windows_shell(path, max_size.max(1))?,
    ))
}

fn embedded_video_thumbnail(item: &HistoryItem) -> AppResult<Option<String>> {
    let entries = crate::clipboard::clipboard_content_to_file_entries(&item.content)?;
    Ok(entries
        .into_iter()
        .find(|entry| {
            is_video_file_name(&entry.name) || is_video_file_path(Path::new(&entry.path))
        })
        .and_then(|entry| entry.thumbnail)
        .filter(|thumbnail| !thumbnail.trim().is_empty()))
}

fn is_video_file_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_video_file_name)
}

fn is_video_file_name(name: &str) -> bool {
    name.rsplit_once('.')
        .map(|(_, extension)| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "mp4" | "mov" | "mkv" | "avi" | "webm" | "m4v" | "wmv"
            )
        })
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn thumbnail_from_windows_shell(path: &Path, max_size: u32) -> AppResult<Vec<u8>> {
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join(format!("copyshare-video-thumb-{}.ps1", Uuid::new_v4()));
    let output_path = temp_dir.join(format!("copyshare-video-thumb-{}.png", Uuid::new_v4()));
    fs::write(&script_path, WINDOWS_SHELL_THUMBNAIL_SCRIPT)?;

    let mut command = Command::new("powershell.exe");
    let status = command
        .creation_flags(0x08000000)
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(&script_path)
        .arg(path)
        .arg(&output_path)
        .arg(max_size.to_string())
        .status()
        .map_err(|error| crate::error::AppError::Clipboard(error.to_string()))?;

    let _ = fs::remove_file(&script_path);
    if !status.success() || !output_path.exists() {
        let _ = fs::remove_file(&output_path);
        return Err(crate::error::AppError::Clipboard(
            "视频缩略图生成失败".to_string(),
        ));
    }

    let bytes = fs::read(&output_path)?;
    let _ = fs::remove_file(&output_path);
    Ok(bytes)
}

#[cfg(not(target_os = "windows"))]
fn thumbnail_from_windows_shell(_path: &Path, _max_size: u32) -> AppResult<Vec<u8>> {
    Err(crate::error::AppError::Clipboard(
        "当前系统不支持视频缩略图".to_string(),
    ))
}

#[cfg(target_os = "windows")]
const WINDOWS_SHELL_THUMBNAIL_SCRIPT: &str = r#"
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @"
using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.Runtime.InteropServices;

[ComImport]
[Guid("bcc18b79-ba16-442f-80c4-8a59c30c463b")]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IShellItemImageFactory
{
    void GetImage([In] SIZE size, [In] SIIGBF flags, out IntPtr phbm);
}

[StructLayout(LayoutKind.Sequential)]
public struct SIZE
{
    public int cx;
    public int cy;
    public SIZE(int x, int y) { cx = x; cy = y; }
}

[Flags]
public enum SIIGBF
{
    ResizeToFit = 0,
    BiggerSizeOk = 1,
    MemoryOnly = 2,
    IconOnly = 4,
    ThumbnailOnly = 8,
    InCacheOnly = 16
}

public static class CopyShareShellThumbnail
{
    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
    static extern void SHCreateItemFromParsingName(
        string pszPath,
        IntPtr pbc,
        [MarshalAs(UnmanagedType.LPStruct)] Guid riid,
        out IShellItemImageFactory ppv
    );

    [DllImport("gdi32.dll")]
    static extern bool DeleteObject(IntPtr hObject);

    public static void Save(string input, string output, int size)
    {
        IShellItemImageFactory factory;
        SHCreateItemFromParsingName(input, IntPtr.Zero, typeof(IShellItemImageFactory).GUID, out factory);
        IntPtr bitmapHandle;
        factory.GetImage(new SIZE(size, size), SIIGBF.BiggerSizeOk | SIIGBF.ThumbnailOnly, out bitmapHandle);
        try
        {
            using (var bitmap = Image.FromHbitmap(bitmapHandle))
            {
                bitmap.Save(output, ImageFormat.Png);
            }
        }
        finally
        {
            DeleteObject(bitmapHandle);
        }
    }
}
"@

[CopyShareShellThumbnail]::Save($args[0], $args[1], [int]$args[2])
"#;

fn get_history_image_thumbnail_from_dirs(
    image_dir: &PathBuf,
    thumbnail_dir: &PathBuf,
    item: &HistoryItem,
    max_size: u32,
) -> AppResult<String> {
    if item.content_type != ClipboardContentType::Image {
        return Err(crate::error::AppError::InvalidInput(
            "历史记录不是图片".to_string(),
        ));
    }

    fs::create_dir_all(thumbnail_dir)?;
    let thumbnail_path = history_thumbnail_path(thumbnail_dir, &item.id, max_size);
    if thumbnail_path.exists() {
        return Ok(base64::engine::general_purpose::STANDARD.encode(fs::read(thumbnail_path)?));
    }

    let content = if item.content.trim().is_empty() {
        fs::read_to_string(history_image_path(image_dir, &item.id))?
    } else {
        item.content.clone()
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(content.trim())
        .map_err(|error| crate::error::AppError::Clipboard(error.to_string()))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| crate::error::AppError::Clipboard(error.to_string()))?;
    let (width, height) = (image.width(), image.height());
    let max_size = max_size.max(1);
    let scale = if width > max_size || height > max_size {
        max_size as f32 / width.max(height) as f32
    } else {
        1.0
    };
    let thumbnail = if scale < 1.0 {
        image.resize(
            (width as f32 * scale).round().max(1.0) as u32,
            (height as f32 * scale).round().max(1.0) as u32,
            image::imageops::FilterType::Triangle,
        )
    } else {
        image
    };
    let mut output = std::io::Cursor::new(Vec::new());
    thumbnail
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|error| crate::error::AppError::Clipboard(error.to_string()))?;
    let data = output.into_inner();
    fs::write(&thumbnail_path, &data)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(data))
}

fn prune_history_thumbnails(thumbnail_dir: &PathBuf, items: &[HistoryItem]) -> AppResult<()> {
    if !thumbnail_dir.exists() {
        return Ok(());
    }

    let keep_ids: std::collections::HashSet<String> = items
        .iter()
        .filter(|item| {
            matches!(
                item.content_type,
                ClipboardContentType::Image | ClipboardContentType::FileList
            )
        })
        .map(|item| safe_history_id(&item.id))
        .collect();

    for entry in fs::read_dir(thumbnail_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let is_kept_thumbnail = file_name
            .strip_suffix(&format!(".{HISTORY_THUMBNAIL_EXTENSION}"))
            .and_then(|stem| stem.rsplit_once('-'))
            .is_some_and(|(id, size)| {
                size.chars().all(|ch| ch.is_ascii_digit()) && keep_ids.contains(id)
            });
        if !is_kept_thumbnail {
            fs::remove_file(entry.path())?;
        }
    }

    Ok(())
}

fn history_image_path(image_dir: &PathBuf, id: &str) -> PathBuf {
    image_dir.join(history_image_file_name(id))
}

fn history_image_file_name(id: &str) -> String {
    format!("{safe_id}.{HISTORY_IMAGE_EXTENSION}", safe_id = safe_history_id(id))
}

fn history_thumbnail_path(thumbnail_dir: &PathBuf, id: &str, max_size: u32) -> PathBuf {
    thumbnail_dir.join(history_thumbnail_file_name(id, max_size))
}

fn video_thumbnail_path(thumbnail_dir: &PathBuf, id: &str, max_size: u32) -> PathBuf {
    history_thumbnail_path(thumbnail_dir, id, max_size)
}

fn history_thumbnail_file_name(id: &str, max_size: u32) -> String {
    format!(
        "{safe_id}-{max_size}.{HISTORY_THUMBNAIL_EXTENSION}",
        safe_id = safe_history_id(id)
    )
}

fn safe_history_id(id: &str) -> String {
    let safe_id: String = id
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '_',
        })
        .collect();
    safe_id
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

fn history_thumbnails_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?.join(HISTORY_THUMBNAIL_DIR);
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
    use image::ImageEncoder;

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
    fn image_history_summary_omits_size_suffix() {
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

        assert_eq!(item.summary, "图片");
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

        assert_eq!(disk_items[0].summary, "图片");
        assert!(disk_items[0].content.is_empty());
    }

    #[test]
    fn image_history_content_is_stripped_before_sending_to_frontend() {
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
        let frontend_items = history_items_for_frontend(&[item.clone()]);

        assert!(!item.content.is_empty(), "state keeps content for copy");
        assert_eq!(frontend_items[0].summary, item.summary);
        assert!(frontend_items[0].content.is_empty());
    }

    #[test]
    fn file_list_history_preserves_content_for_copying() {
        let content = r#"[{"path":"C:\\a.txt","name":"a.txt","size":1024}]"#;
        let message = ClipboardMessage {
            message_id: "m".to_string(),
            source_device_id: "d".to_string(),
            source_device_name: "Device".to_string(),
            content_type: ClipboardContentType::FileList,
            content: content.to_string(),
            content_hash: "hash".to_string(),
            timestamp: 1,
        };

        let item = make_history_item(HistoryDirection::Local, "Device", &message);
        let disk_items = history_items_for_disk(&[item.clone()]);

        assert_eq!(item.summary, "a.txt 1.00 KB");
        assert_eq!(item.content, content);
        assert_eq!(item.content_hash, "hash");
        assert_eq!(disk_items[0].content, content);
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
            content_hash: String::new(),
            content_type: ClipboardContentType::Image,
            sync_status: SyncStatus::Synced,
            file_transfer_id: None,
            file_transfer_status: None,
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
            content_hash: String::new(),
            content_type: ClipboardContentType::Image,
            sync_status: SyncStatus::Synced,
            file_transfer_id: None,
            file_transfer_status: None,
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
    fn image_thumbnail_is_generated_within_requested_size() -> AppResult<()> {
        let image_dir = std::env::temp_dir().join(format!(
            "copyshare-history-images-{}",
            Uuid::new_v4()
        ));
        let thumb_dir = std::env::temp_dir().join(format!(
            "copyshare-history-thumbnails-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&image_dir)?;
        fs::create_dir_all(&thumb_dir)?;

        let image = image::RgbaImage::from_pixel(400, 100, image::Rgba([7, 8, 9, 255]));
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&image, 400, 100, image::ColorType::Rgba8.into())
            .expect("test image should encode");
        let item = HistoryItem {
            id: "wide-image".to_string(),
            direction: HistoryDirection::Local,
            source_device: "Device".to_string(),
            summary: "image".to_string(),
            content: STANDARD.encode(png),
            content_hash: String::new(),
            content_type: ClipboardContentType::Image,
            sync_status: SyncStatus::Synced,
            file_transfer_id: None,
            file_transfer_status: None,
            success: true,
            created_at: Utc::now(),
        };
        save_history_images(&image_dir, &[item.clone()])?;

        let thumb = get_history_image_thumbnail_from_dirs(&image_dir, &thumb_dir, &item, 200)?;
        let decoded = image::load_from_memory(
            &STANDARD.decode(thumb).expect("thumbnail should be base64 PNG"),
        )
        .expect("thumbnail should decode");

        assert_eq!(decoded.width(), 200);
        assert_eq!(decoded.height(), 50);
        assert!(thumb_dir.join("wide-image-200.png").exists());

        fs::remove_dir_all(image_dir)?;
        fs::remove_dir_all(thumb_dir)?;
        Ok(())
    }

    #[test]
    fn stale_history_thumbnail_cache_files_are_pruned() -> AppResult<()> {
        let thumb_dir = std::env::temp_dir().join(format!(
            "copyshare-history-thumbnails-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&thumb_dir)?;
        fs::write(thumb_dir.join("stale.png"), "old")?;
        let item = HistoryItem {
            id: "keep".to_string(),
            direction: HistoryDirection::Local,
            source_device: "Device".to_string(),
            summary: "image".to_string(),
            content: STANDARD.encode(vec![1; 16]),
            content_hash: String::new(),
            content_type: ClipboardContentType::Image,
            sync_status: SyncStatus::Synced,
            file_transfer_id: None,
            file_transfer_status: None,
            success: true,
            created_at: Utc::now(),
        };
        fs::write(thumb_dir.join("keep-200.png"), "new")?;

        prune_history_thumbnails(&thumb_dir, &[item])?;

        assert!(!thumb_dir.join("stale.png").exists());
        assert!(thumb_dir.join("keep-200.png").exists());
        fs::remove_dir_all(thumb_dir)?;
        Ok(())
    }

    #[test]
    fn cache_size_counts_history_images_and_thumbnails_only() -> AppResult<()> {
        let app_data_dir = std::env::temp_dir().join(format!(
            "copyshare-cache-size-{}",
            Uuid::new_v4()
        ));
        let image_dir = app_data_dir.join(HISTORY_IMAGE_DIR);
        let thumbnail_dir = app_data_dir.join(HISTORY_THUMBNAIL_DIR);
        fs::create_dir_all(&image_dir)?;
        fs::create_dir_all(&thumbnail_dir)?;
        fs::write(image_dir.join("image.b64"), vec![1; 12])?;
        fs::write(thumbnail_dir.join("thumb.png"), vec![2; 34])?;
        fs::write(app_data_dir.join(HISTORY_FILE), vec![3; 56])?;

        assert_eq!(cache_size_from_app_data_dir(&app_data_dir)?, 46);

        fs::remove_dir_all(app_data_dir)?;
        Ok(())
    }

    #[test]
    fn clear_cache_removes_images_and_thumbnails_but_keeps_history_file() -> AppResult<()> {
        let app_data_dir = std::env::temp_dir().join(format!(
            "copyshare-clear-cache-{}",
            Uuid::new_v4()
        ));
        let image_dir = app_data_dir.join(HISTORY_IMAGE_DIR);
        let thumbnail_dir = app_data_dir.join(HISTORY_THUMBNAIL_DIR);
        fs::create_dir_all(&image_dir)?;
        fs::create_dir_all(&thumbnail_dir)?;
        fs::write(image_dir.join("image.b64"), vec![1; 12])?;
        fs::write(thumbnail_dir.join("thumb.png"), vec![2; 34])?;
        fs::write(app_data_dir.join(HISTORY_FILE), "[]")?;

        assert_eq!(clear_cache_from_app_data_dir(&app_data_dir)?, 0);
        assert!(app_data_dir.join(HISTORY_FILE).exists());
        assert!(!image_dir.join("image.b64").exists());
        assert!(!thumbnail_dir.join("thumb.png").exists());

        fs::remove_dir_all(app_data_dir)?;
        Ok(())
    }

    #[test]
    fn legacy_history_items_default_to_synced_status() {
        let text = r#"[{"id":"old","direction":"local","sourceDevice":"CopyShare","summary":"hello","content":"hello","contentType":"text","success":true,"createdAt":"2026-06-28T00:00:00Z"}]"#;
        let items = load_history_items_from_text(text).expect("legacy history should load");

        assert_eq!(items[0].sync_status, crate::models::SyncStatus::Synced);
    }

    #[test]
    fn file_transfer_history_updates_existing_item() {
        let message = ClipboardMessage {
            message_id: "transfer-1".to_string(),
            source_device_id: "device-a".to_string(),
            source_device_name: "Laptop A".to_string(),
            content_type: ClipboardContentType::FileList,
            content: r#"[{"path":"","name":"a.txt","size":3}]"#.to_string(),
            content_hash: "hash".to_string(),
            timestamp: 1,
        };
        let mut item = make_history_item(HistoryDirection::Remote, "Laptop A", &message);
        item.file_transfer_id = Some("transfer-1".to_string());
        item.file_transfer_status = Some(crate::models::FileTransferStatus::Pending);
        let mut items = vec![item];

        let updated = update_file_transfer_history(
            &mut items,
            "transfer-1",
            crate::models::FileTransferStatus::Completed,
            Some(r#"[{"path":"C:\\a.txt","name":"a.txt","size":3}]"#.to_string()),
        )
        .expect("history item should update");

        assert_eq!(items.len(), 1);
        assert_eq!(
            updated.file_transfer_status,
            Some(crate::models::FileTransferStatus::Completed)
        );
        assert!(updated.content.contains(r#"C:\\a.txt"#));
    }

    #[test]
    fn unsynced_history_item_upgrades_to_synced_without_duplicate() {
        let message = ClipboardMessage {
            message_id: "m".to_string(),
            source_device_id: "d".to_string(),
            source_device_name: "Device".to_string(),
            content_type: crate::models::ClipboardContentType::Text,
            content: "same content".to_string(),
            content_hash: "hash".to_string(),
            timestamp: 1,
        };
        let mut items = Vec::new();

        upsert_history_by_content(
            &mut items,
            make_history_item_with_status(
                HistoryDirection::Local,
                "Device",
                &message,
                crate::models::SyncStatus::Unsynced,
            ),
        );
        upsert_history_by_content(
            &mut items,
            make_history_item_with_status(
                HistoryDirection::Local,
                "Device",
                &message,
                crate::models::SyncStatus::Synced,
            ),
        );

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].sync_status, crate::models::SyncStatus::Synced);
    }

    #[test]
    fn corrupted_history_json_does_not_block_startup() {
        let items = load_history_items_from_text(r#"[{"id": "broken", "summary": "unterminated}"#)
            .expect("corrupted history should fall back to empty history");

        assert!(items.is_empty());
    }
}
