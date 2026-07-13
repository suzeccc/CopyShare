use std::{
    cmp::Reverse,
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

use chrono::Utc;
use base64::Engine;
use image::ImageEncoder;
use sha2::{Digest, Sha256};
use tauri::Manager;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{
        ClipboardContentType, HistoryItem, LibraryAssetKind, LibraryAssetRef, LibraryItem,
        LibraryItemUpdate, LibraryRole, LibrarySnapshot,
    },
};

const LIBRARY_FILE: &str = "library.json";
const LIBRARY_ASSET_DIR: &str = "library-assets";
const LIBRARY_THUMBNAIL_DIR: &str = "library-thumbnails";
const LIBRARY_COPY_CACHE_DIR: &str = "library-copy-cache";
const TITLE_LIMIT: usize = 120;
const NOTE_LIMIT: usize = 2_000;
const TAG_LIMIT: usize = 20;
const TAG_LENGTH_LIMIT: usize = 32;
const LIBRARY_ITEM_LIMIT: usize = 5_000;
const FILE_COUNT_LIMIT: usize = 100;
const FILE_SIZE_LIMIT: u64 = 500 * 1024 * 1024;
const FILE_TOTAL_SIZE_LIMIT: u64 = 1024 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum LibraryCopyPayload {
    Text(String),
    Image(String),
    Files(Vec<PathBuf>),
}

struct NormalizedMetadata {
    title: String,
    tags: Vec<String>,
    note: String,
}

fn normalize_metadata(
    title: &str,
    tags: Vec<String>,
    note: &str,
) -> AppResult<NormalizedMetadata> {
    let title = title.trim().to_string();
    let note = note.trim().to_string();
    if title.is_empty() || title.chars().count() > TITLE_LIMIT {
        return Err(AppError::InvalidInput(
            "收藏标题不能为空且不能超过 120 个字符".into(),
        ));
    }
    if note.chars().count() > NOTE_LIMIT {
        return Err(AppError::InvalidInput(
            "收藏备注不能超过 2000 个字符".into(),
        ));
    }
    if tags.len() > TAG_LIMIT {
        return Err(AppError::InvalidInput("每个收藏最多 20 个标签".into()));
    }

    let mut normalized = Vec::new();
    for tag in tags {
        let tag = tag.trim().to_string();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > TAG_LENGTH_LIMIT {
            return Err(AppError::InvalidInput("标签不能超过 32 个字符".into()));
        }
        if !normalized
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(&tag))
        {
            normalized.push(tag);
        }
    }

    Ok(NormalizedMetadata {
        title,
        tags: normalized,
        note,
    })
}

fn text_hash(role: &LibraryRole, content: &str) -> String {
    let mut hasher = Sha256::new();
    let role_name = match role {
        LibraryRole::Saved => "saved",
        LibraryRole::Snippet => "snippet",
    };
    hasher.update(role_name.as_bytes());
    hasher.update([0]);
    hasher.update(content.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn new_snippet(
    title: &str,
    content: &str,
    tags: Vec<String>,
    note: &str,
) -> AppResult<LibraryItem> {
    let metadata = normalize_metadata(title, tags, note)?;
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err(AppError::InvalidInput("常用片段正文不能为空".into()));
    }

    let now = Utc::now();
    Ok(LibraryItem {
        id: Uuid::new_v4().to_string(),
        role: LibraryRole::Snippet,
        content_type: ClipboardContentType::Text,
        title: metadata.title,
        summary: crate::history::summarize(&content),
        content_hash: text_hash(&LibraryRole::Snippet, &content),
        content,
        assets: vec![],
        source_history_id: None,
        source_content_hash: None,
        source_device: String::new(),
        tags: metadata.tags,
        note: metadata.note,
        is_pinned: false,
        pin_order: None,
        created_at: now,
        updated_at: now,
    })
}

fn sort_library_items(mut items: Vec<LibraryItem>) -> Vec<LibraryItem> {
    items.sort_by_key(|item| {
        (
            !item.is_pinned,
            item.pin_order.unwrap_or(u64::MAX),
            Reverse(item.updated_at.timestamp_millis()),
        )
    });
    items
}

pub fn load_library(app: &tauri::AppHandle) -> LibrarySnapshot {
    match app.path().app_data_dir() {
        Ok(root) => load_library_from_dir(&root),
        Err(error) => LibrarySnapshot {
            items: vec![],
            warning: Some(format!("无法定位收藏库：{error}")),
        },
    }
}

fn load_library_from_dir(root: &Path) -> LibrarySnapshot {
    if let Err(error) = clear_file_copy_cache(root) {
        eprintln!("failed to clear stale library copy cache: {error}");
    }
    let path = root.join(LIBRARY_FILE);
    if !path.exists() {
        let backup = root.join("library.json.bak");
        if !backup.exists() {
            return LibrarySnapshot::default();
        }
        if let Err(error) = fs::rename(&backup, &path) {
            eprintln!("failed to restore interrupted library replacement: {error}");
            return LibrarySnapshot {
                items: vec![],
                warning: Some(format!("收藏库备份恢复失败：{error}")),
            };
        }
    }

    match fs::read_to_string(&path)
        .map_err(AppError::from)
        .and_then(|text| serde_json::from_str::<Vec<LibraryItem>>(&text).map_err(AppError::from))
    {
        Ok(items) => LibrarySnapshot {
            items: sort_library_items(items),
            warning: None,
        },
        Err(error) => {
            let preserved = root.join("library.json.corrupt");
            if !preserved.exists() {
                let _ = fs::copy(&path, &preserved);
            }
            eprintln!("failed to load library metadata: {error}");
            LibrarySnapshot {
                items: vec![],
                warning: Some(format!(
                    "收藏库数据损坏，原文件已备份为 library.json.corrupt：{error}"
                )),
            }
        }
    }
}

pub(crate) fn save_library_to_dir(root: &Path, items: &[LibraryItem]) -> AppResult<()> {
    fs::create_dir_all(root)?;
    let target = root.join(LIBRARY_FILE);
    let temp = root.join("library.json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(items)?)?;
    replace_with_backup(&target, &temp, |from, to| fs::rename(from, to))
}

fn replace_with_backup<F>(target: &Path, temp: &Path, mut rename: F) -> AppResult<()>
where
    F: FnMut(&Path, &Path) -> std::io::Result<()>,
{
    let backup = target.with_file_name("library.json.bak");
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if target.exists() {
        rename(target, &backup)?;
    }
    if let Err(error) = rename(temp, target) {
        if backup.exists() {
            rename(&backup, target)?;
        }
        return Err(error.into());
    }
    if backup.exists() {
        if let Err(error) = fs::remove_file(backup) {
            eprintln!("failed to remove committed library backup: {error}");
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn safe_extension(file_name: &str) -> String {
    Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| {
            value
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .take(10)
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "bin".to_string())
}

fn store_asset(
    root: &Path,
    kind: LibraryAssetKind,
    file_name: &str,
    bytes: &[u8],
) -> AppResult<LibraryAssetRef> {
    let sha256 = sha256_hex(bytes);
    let directory = root.join(LIBRARY_ASSET_DIR);
    fs::create_dir_all(&directory)?;
    let hash_prefix = format!("{sha256}.");
    let stored_name = fs::read_dir(&directory)?
        .filter_map(Result::ok)
        .find(|entry| {
            entry.file_type().map(|kind| kind.is_file()).unwrap_or(false)
                && entry.file_name().to_string_lossy().starts_with(&hash_prefix)
        })
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .unwrap_or_else(|| format!("{sha256}.{}", safe_extension(file_name)));
    let relative_path = format!("{LIBRARY_ASSET_DIR}/{stored_name}");
    let target = directory.join(&stored_name);

    if target.exists() {
        if sha256_hex(&fs::read(&target)?) != sha256 {
            return Err(AppError::InvalidInput("收藏资源校验失败".into()));
        }
    } else {
        let temp = directory.join(format!(".{stored_name}.{}.tmp", Uuid::new_v4()));
        fs::write(&temp, bytes)?;
        if sha256_hex(&fs::read(&temp)?) != sha256 {
            let _ = fs::remove_file(&temp);
            return Err(AppError::InvalidInput("收藏资源写入校验失败".into()));
        }
        if let Err(error) = fs::rename(&temp, &target) {
            let _ = fs::remove_file(&temp);
            if !target.exists() {
                return Err(error.into());
            }
        }
    }

    Ok(LibraryAssetRef {
        asset_id: sha256.clone(),
        kind,
        file_name: file_name.to_string(),
        relative_path,
        sha256,
        size: bytes.len() as u64,
    })
}

fn resolve_asset_path(root: &Path, asset: &LibraryAssetRef) -> AppResult<PathBuf> {
    let path = Path::new(&asset.relative_path);
    let components = path.components().collect::<Vec<_>>();
    let valid = matches!(components.as_slice(), [Component::Normal(directory), Component::Normal(_)] if *directory == std::ffi::OsStr::new(LIBRARY_ASSET_DIR));
    if !valid {
        return Err(AppError::InvalidInput("收藏资源路径无效".into()));
    }
    Ok(root.join(path))
}

fn prune_assets(root: &Path, references: &[LibraryAssetRef]) -> AppResult<()> {
    let keep = references
        .iter()
        .map(|asset| asset.relative_path.as_str())
        .collect::<HashSet<_>>();
    let directory = root.join(LIBRARY_ASSET_DIR);
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let relative = format!(
            "{LIBRARY_ASSET_DIR}/{}",
            entry.file_name().to_string_lossy()
        );
        if !keep.contains(relative.as_str()) {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn directory_size(directory: &Path) -> AppResult<u64> {
    if !directory.exists() {
        return Ok(0);
    }
    fs::read_dir(directory)?.try_fold(0_u64, |total, entry| {
        let entry = entry?;
        let metadata = entry.metadata()?;
        Ok(total
            + if metadata.is_dir() {
                directory_size(&entry.path())?
            } else {
                metadata.len()
            })
    })
}

pub fn library_storage_size(root: &Path) -> AppResult<u64> {
    Ok(directory_size(&root.join(LIBRARY_ASSET_DIR))?
        + directory_size(&root.join(LIBRARY_THUMBNAIL_DIR))?
        + directory_size(&root.join(LIBRARY_COPY_CACHE_DIR))?)
}

fn default_text_title(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or_default().trim();
    let title = first_line.chars().take(40).collect::<String>();
    if title.is_empty() {
        "文本".to_string()
    } else {
        title
    }
}

fn asset_collection_hash(
    content_type: &ClipboardContentType,
    assets: &[LibraryAssetRef],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(match content_type {
        ClipboardContentType::Text => b"text".as_slice(),
        ClipboardContentType::Image => b"image".as_slice(),
        ClipboardContentType::FileList => b"fileList".as_slice(),
    });
    for asset in assets {
        hasher.update([0]);
        hasher.update(asset.sha256.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn source_hash(history: &HistoryItem) -> String {
    if history.content_hash.trim().is_empty() {
        crate::sync::content_hash(&history.content_type, &history.content)
    } else {
        history.content_hash.clone()
    }
}

fn next_pin_order(items: &[LibraryItem]) -> u64 {
    items
        .iter()
        .filter_map(|item| item.pin_order)
        .max()
        .map(|order| order.saturating_add(1))
        .unwrap_or(0)
}

fn saved_item_from_history(root: &Path, history: &HistoryItem) -> AppResult<LibraryItem> {
    if !history.success {
        return Err(AppError::InvalidInput("历史项不可用".into()));
    }

    let now = Utc::now();
    let mut item = LibraryItem {
        id: Uuid::new_v4().to_string(),
        role: LibraryRole::Saved,
        content_type: history.content_type.clone(),
        title: history.summary.trim().to_string(),
        content: String::new(),
        summary: history.summary.trim().to_string(),
        assets: vec![],
        source_history_id: Some(history.id.clone()),
        source_content_hash: Some(source_hash(history)),
        source_device: history.source_device.clone(),
        content_hash: String::new(),
        tags: vec![],
        note: String::new(),
        is_pinned: false,
        pin_order: None,
        created_at: now,
        updated_at: now,
    };

    match history.content_type {
        ClipboardContentType::Text => {
            let content = history.content.trim();
            if content.is_empty() {
                return Err(AppError::InvalidInput("文本历史为空".into()));
            }
            item.title = default_text_title(content);
            item.summary = crate::history::summarize(content);
            item.content = content.to_string();
            item.content_hash = text_hash(&LibraryRole::Saved, content);
        }
        ClipboardContentType::Image => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(history.content.trim())
                .map_err(|error| AppError::InvalidInput(format!("图片数据无效：{error}")))?;
            image::load_from_memory(&bytes)
                .map_err(|error| AppError::InvalidInput(format!("图片数据损坏：{error}")))?;
            let asset = store_asset(root, LibraryAssetKind::Image, "image.png", &bytes)?;
            item.title = if item.title.is_empty() {
                "图片".into()
            } else {
                item.title
            };
            item.summary = "图片".into();
            item.assets = vec![asset];
            item.content_hash = asset_collection_hash(&item.content_type, &item.assets);
        }
        ClipboardContentType::FileList => {
            let entries = crate::clipboard::clipboard_content_to_file_entries(&history.content)?;
            if entries.is_empty() || entries.len() > FILE_COUNT_LIMIT {
                return Err(AppError::InvalidInput("文件数量必须在 1 到 100 之间".into()));
            }

            let mut total_size = 0_u64;
            let mut sources = Vec::with_capacity(entries.len());
            for entry in entries {
                let path = PathBuf::from(&entry.path);
                let metadata = fs::metadata(&path).map_err(|_| {
                    AppError::InvalidInput(format!("文件不存在或无法读取：{}", path.display()))
                })?;
                if !metadata.is_file() {
                    return Err(AppError::InvalidInput(format!(
                        "收藏项不是文件：{}",
                        path.display()
                    )));
                }
                if metadata.len() > FILE_SIZE_LIMIT {
                    return Err(AppError::InvalidInput(format!(
                        "单个文件不能超过 500 MiB：{}",
                        entry.name
                    )));
                }
                total_size = total_size.saturating_add(metadata.len());
                if total_size > FILE_TOTAL_SIZE_LIMIT {
                    return Err(AppError::InvalidInput("单次收藏文件总量不能超过 1 GiB".into()));
                }
                sources.push((entry.name, path));
            }

            for (file_name, path) in sources {
                let bytes = fs::read(&path)?;
                item.assets.push(store_asset(
                    root,
                    LibraryAssetKind::File,
                    &file_name,
                    &bytes,
                )?);
            }
            item.title = if item.title.is_empty() {
                "文件".into()
            } else {
                item.title
            };
            item.content_hash = asset_collection_hash(&item.content_type, &item.assets);
        }
    }

    Ok(item)
}

pub fn collect_history_item(
    root: &Path,
    items: &[LibraryItem],
    history: &HistoryItem,
    pin: bool,
) -> AppResult<Vec<LibraryItem>> {
    let candidate = match saved_item_from_history(root, history) {
        Ok(item) => item,
        Err(error) => {
            let _ = prune_library_resources(root, items);
            return Err(error);
        }
    };
    let mut next = items.to_vec();

    if let Some(existing) = next.iter_mut().find(|item| {
        item.role == LibraryRole::Saved && item.content_hash == candidate.content_hash
    }) {
        existing.source_history_id = candidate.source_history_id;
        existing.source_content_hash = candidate.source_content_hash;
        existing.source_device = candidate.source_device;
        existing.updated_at = Utc::now();
        if pin && !existing.is_pinned {
            existing.is_pinned = true;
            existing.pin_order = Some(next_pin_order(items));
        }
        prune_library_resources(root, &next)?;
        return Ok(sort_library_items(next));
    }

    if next.len() >= LIBRARY_ITEM_LIMIT {
        prune_library_resources(root, items)?;
        return Err(AppError::InvalidInput("收藏库最多保存 5000 项".into()));
    }

    let mut candidate = candidate;
    if pin {
        candidate.is_pinned = true;
        candidate.pin_order = Some(next_pin_order(items));
    }
    next.push(candidate);
    Ok(sort_library_items(next))
}

pub fn create_snippet(
    items: &[LibraryItem],
    title: &str,
    content: &str,
    tags: Vec<String>,
    note: &str,
) -> AppResult<Vec<LibraryItem>> {
    if items.len() >= LIBRARY_ITEM_LIMIT {
        return Err(AppError::InvalidInput("收藏库最多保存 5000 项".into()));
    }
    let mut next = items.to_vec();
    next.push(new_snippet(title, content, tags, note)?);
    Ok(sort_library_items(next))
}

pub fn update_item(
    items: &[LibraryItem],
    id: &str,
    update: LibraryItemUpdate,
) -> AppResult<Vec<LibraryItem>> {
    let metadata = normalize_metadata(&update.title, update.tags, &update.note)?;
    let mut next = items.to_vec();
    let item = next
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| AppError::InvalidInput("收藏项不存在".into()))?;
    if item.role == LibraryRole::Saved && update.content.is_some() {
        return Err(AppError::InvalidInput("普通收藏正文不可编辑".into()));
    }
    if item.role == LibraryRole::Snippet {
        let content = update.content.unwrap_or_else(|| item.content.clone());
        let content = content.trim().to_string();
        if content.is_empty() {
            return Err(AppError::InvalidInput("常用片段正文不能为空".into()));
        }
        item.summary = crate::history::summarize(&content);
        item.content_hash = text_hash(&LibraryRole::Snippet, &content);
        item.content = content;
    }
    item.title = metadata.title;
    item.tags = metadata.tags;
    item.note = metadata.note;
    item.updated_at = Utc::now();
    Ok(sort_library_items(next))
}

pub fn convert_to_snippet(items: &[LibraryItem], id: &str) -> AppResult<Vec<LibraryItem>> {
    let mut next = items.to_vec();
    let item = next
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| AppError::InvalidInput("收藏项不存在".into()))?;
    if item.role != LibraryRole::Saved
        || item.content_type != ClipboardContentType::Text
        || item.content.trim().is_empty()
    {
        return Err(AppError::InvalidInput("仅文本收藏可转换为常用片段".into()));
    }
    item.role = LibraryRole::Snippet;
    item.source_history_id = None;
    item.source_content_hash = None;
    item.content_hash = text_hash(&LibraryRole::Snippet, &item.content);
    item.updated_at = Utc::now();
    Ok(sort_library_items(next))
}

pub fn set_pinned(items: &[LibraryItem], id: &str, pinned: bool) -> AppResult<Vec<LibraryItem>> {
    let order = next_pin_order(items);
    let mut next = items.to_vec();
    let item = next
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| AppError::InvalidInput("收藏项不存在".into()))?;
    if pinned {
        if !item.is_pinned {
            item.pin_order = Some(order);
        }
        item.is_pinned = true;
    } else {
        item.is_pinned = false;
        item.pin_order = None;
    }
    item.updated_at = Utc::now();
    Ok(sort_library_items(next))
}

pub fn reorder_pinned(
    items: &[LibraryItem],
    ordered_ids: &[String],
) -> AppResult<Vec<LibraryItem>> {
    let current = items
        .iter()
        .filter(|item| item.is_pinned)
        .map(|item| item.id.as_str())
        .collect::<HashSet<_>>();
    let requested = ordered_ids.iter().map(String::as_str).collect::<HashSet<_>>();
    if current.len() != ordered_ids.len() || requested.len() != ordered_ids.len() || current != requested {
        return Err(AppError::InvalidInput("置顶排序项目不完整".into()));
    }

    let mut next = items.to_vec();
    for (order, id) in ordered_ids.iter().enumerate() {
        let item = next
            .iter_mut()
            .find(|item| item.id == *id)
            .ok_or_else(|| AppError::InvalidInput("收藏项不存在".into()))?;
        item.pin_order = Some(order as u64);
        item.updated_at = Utc::now();
    }
    Ok(sort_library_items(next))
}

pub fn remove_item(items: &[LibraryItem], id: &str) -> AppResult<Vec<LibraryItem>> {
    if !items.iter().any(|item| item.id == id) {
        return Err(AppError::InvalidInput("收藏项不存在".into()));
    }
    Ok(sort_library_items(
        items
            .iter()
            .filter(|item| item.id != id)
            .cloned()
            .collect(),
    ))
}

fn verify_asset_bytes(asset: &LibraryAssetRef, bytes: &[u8]) -> AppResult<()> {
    if bytes.len() as u64 != asset.size || sha256_hex(bytes) != asset.sha256 {
        return Err(AppError::InvalidInput(format!(
            "收藏资源损坏：{}",
            asset.file_name
        )));
    }
    Ok(())
}

fn copy_file_name(asset: &LibraryAssetRef) -> AppResult<&str> {
    let file_name = asset.file_name.as_str();
    let components = Path::new(file_name).components().collect::<Vec<_>>();
    let is_single_component = matches!(
        components.as_slice(),
        [Component::Normal(name)] if *name == std::ffi::OsStr::new(file_name)
    );
    let reserved_base = file_name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let is_reserved = matches!(reserved_base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || reserved_base
            .strip_prefix("COM")
            .or_else(|| reserved_base.strip_prefix("LPT"))
            .is_some_and(|number| matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"));
    if !is_single_component
        || file_name.ends_with(['.', ' '])
        || file_name.chars().any(|character| {
            character.is_control()
                || matches!(character, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
        })
        || is_reserved
    {
        return Err(AppError::InvalidInput("收藏文件名无效".into()));
    }
    Ok(file_name)
}

fn materialize_file_copy(root: &Path, item: &LibraryItem) -> AppResult<Vec<PathBuf>> {
    let cache_root = root.join(LIBRARY_COPY_CACHE_DIR);
    let session = cache_root.join(Uuid::new_v4().to_string());
    fs::create_dir_all(&session)?;

    let result = (|| {
        let mut paths = Vec::with_capacity(item.assets.len());
        for (index, asset) in item.assets.iter().enumerate() {
            let source = resolve_asset_path(root, asset)?;
            let bytes = fs::read(source)?;
            verify_asset_bytes(asset, &bytes)?;
            let directory = session.join(index.to_string());
            fs::create_dir(&directory)?;
            let destination = directory.join(copy_file_name(asset)?);
            fs::write(&destination, bytes)?;
            paths.push(destination);
        }
        Ok(paths)
    })();

    let paths = match result {
        Ok(paths) => paths,
        Err(error) => {
            let _ = fs::remove_dir_all(&session);
            return Err(error);
        }
    };
    Ok(paths)
}

fn file_copy_cache_session(paths: &[PathBuf]) -> AppResult<PathBuf> {
    let session = paths
        .first()
        .and_then(|path| path.parent())
        .and_then(Path::parent)
        .ok_or_else(|| AppError::InvalidInput("收藏复制缓存路径无效".into()))?
        .to_path_buf();
    if !paths.iter().all(|path| path.starts_with(&session)) {
        return Err(AppError::InvalidInput("收藏复制缓存路径不一致".into()));
    }
    Ok(session)
}

pub fn discard_file_copy_cache(paths: &[PathBuf]) -> AppResult<()> {
    let session = file_copy_cache_session(paths)?;
    if session.exists() {
        fs::remove_dir_all(session)?;
    }
    Ok(())
}

pub fn commit_file_copy_cache(root: &Path, paths: &[PathBuf]) -> AppResult<()> {
    let cache_root = root.join(LIBRARY_COPY_CACHE_DIR);
    let session = file_copy_cache_session(paths)?;
    if session.parent() != Some(cache_root.as_path()) {
        return Err(AppError::InvalidInput("收藏复制缓存目录无效".into()));
    }
    for entry in fs::read_dir(cache_root)? {
        let entry = entry?;
        if entry.path() != session {
            fs::remove_dir_all(entry.path())?;
        }
    }
    Ok(())
}

pub fn clear_file_copy_cache(root: &Path) -> AppResult<()> {
    let cache_root = root.join(LIBRARY_COPY_CACHE_DIR);
    if cache_root.exists() {
        fs::remove_dir_all(cache_root)?;
    }
    Ok(())
}

pub fn copy_payload(root: &Path, item: &LibraryItem) -> AppResult<LibraryCopyPayload> {
    match item.content_type {
        ClipboardContentType::Text => {
            if item.content.is_empty() {
                return Err(AppError::InvalidInput("收藏文本为空".into()));
            }
            Ok(LibraryCopyPayload::Text(item.content.clone()))
        }
        ClipboardContentType::Image => {
            let asset = item
                .assets
                .iter()
                .find(|asset| asset.kind == LibraryAssetKind::Image)
                .ok_or_else(|| AppError::InvalidInput("收藏图片资源缺失".into()))?;
            let bytes = fs::read(resolve_asset_path(root, asset)?)?;
            verify_asset_bytes(asset, &bytes)?;
            Ok(LibraryCopyPayload::Image(
                base64::engine::general_purpose::STANDARD.encode(bytes),
            ))
        }
        ClipboardContentType::FileList => {
            if item.assets.is_empty() {
                return Err(AppError::InvalidInput("收藏文件资源缺失".into()));
            }
            Ok(LibraryCopyPayload::Files(materialize_file_copy(root, item)?))
        }
    }
}

fn prune_thumbnails(root: &Path, references: &[LibraryAssetRef]) -> AppResult<()> {
    let image_hashes = references
        .iter()
        .filter(|asset| asset.kind == LibraryAssetKind::Image)
        .map(|asset| asset.sha256.as_str())
        .collect::<HashSet<_>>();
    let directory = root.join(LIBRARY_THUMBNAIL_DIR);
    if !directory.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !image_hashes
            .iter()
            .any(|hash| name.starts_with(&format!("{hash}-")))
        {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

pub fn prune_library_resources(root: &Path, items: &[LibraryItem]) -> AppResult<()> {
    let references = items
        .iter()
        .flat_map(|item| item.assets.iter().cloned())
        .collect::<Vec<_>>();
    prune_assets(root, &references)?;
    prune_thumbnails(root, &references)
}

pub fn image_thumbnail(root: &Path, item: &LibraryItem, max_size: u32) -> AppResult<String> {
    let asset = item
        .assets
        .iter()
        .find(|asset| asset.kind == LibraryAssetKind::Image)
        .ok_or_else(|| AppError::InvalidInput("收藏图片资源缺失".into()))?;
    let size = max_size.clamp(32, 800);
    let directory = root.join(LIBRARY_THUMBNAIL_DIR);
    let target = directory.join(format!("{}-{size}.png", asset.sha256));
    let bytes = if target.exists() {
        fs::read(&target)?
    } else {
        let source = fs::read(resolve_asset_path(root, asset)?)?;
        verify_asset_bytes(asset, &source)?;
        let thumbnail = image::load_from_memory(&source)
            .map_err(|error| AppError::InvalidInput(format!("收藏图片损坏：{error}")))?
            .thumbnail(size, size);
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(
                thumbnail.as_bytes(),
                thumbnail.width(),
                thumbnail.height(),
                thumbnail.color().into(),
            )
            .map_err(|error| AppError::InvalidInput(format!("缩略图编码失败：{error}")))?;
        fs::create_dir_all(&directory)?;
        let temp = directory.join(format!(".{}-{}.tmp", asset.sha256, Uuid::new_v4()));
        fs::write(&temp, &bytes)?;
        fs::rename(temp, &target)?;
        bytes
    };
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use base64::Engine;
    use image::ImageEncoder;

    use crate::models::{
        HistoryDirection, HistoryItem, LibraryAssetKind, LibraryItemUpdate, SyncStatus,
    };

    fn temp_dir(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "copyshare-library-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn history_fixture(
        id: &str,
        content_type: ClipboardContentType,
        content: String,
    ) -> HistoryItem {
        let content_hash = crate::sync::content_hash(&content_type, &content);
        HistoryItem {
            id: id.to_string(),
            direction: HistoryDirection::Local,
            source_device: "This device".into(),
            summary: crate::history::summarize(&content),
            content,
            content_hash,
            content_type,
            sync_status: SyncStatus::Synced,
            file_transfer_id: None,
            file_transfer_status: None,
            success: true,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn collecting_text_deduplicates_and_pin_only_changes_sorting() {
        let root = temp_dir("collect-text");
        let history = history_fixture(
            "history-1",
            ClipboardContentType::Text,
            "VPN URL".into(),
        );
        let items = collect_history_item(&root, &[], &history, false).unwrap();
        let items = collect_history_item(&root, &items, &history, true).unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].is_pinned);
        assert_eq!(items[0].pin_order, Some(0));
        assert_eq!(items[0].source_history_id.as_deref(), Some("history-1"));
        assert_eq!(
            items[0].source_content_hash.as_deref(),
            Some(history.content_hash.as_str())
        );
        assert_eq!(items[0].content, "VPN URL");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn collecting_image_copies_png_and_builds_thumbnail_and_copy_payload() {
        let root = temp_dir("collect-image");
        let image = image::RgbaImage::from_pixel(40, 20, image::Rgba([20, 40, 60, 255]));
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&image, 40, 20, image::ColorType::Rgba8.into())
            .unwrap();
        let history = history_fixture(
            "history-image",
            ClipboardContentType::Image,
            base64::engine::general_purpose::STANDARD.encode(&png),
        );

        let items = collect_history_item(&root, &[], &history, false).unwrap();
        let managed = resolve_asset_path(&root, &items[0].assets[0]).unwrap();
        assert_eq!(std::fs::read(&managed).unwrap(), png);
        assert!(matches!(
            copy_payload(&root, &items[0]).unwrap(),
            LibraryCopyPayload::Image(_)
        ));
        let thumbnail = image_thumbnail(&root, &items[0], 16).unwrap();
        let thumbnail = base64::engine::general_purpose::STANDARD
            .decode(thumbnail)
            .unwrap();
        let decoded = image::load_from_memory(&thumbnail).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (32, 16));
        assert!(root.join(LIBRARY_THUMBNAIL_DIR).exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn collecting_files_copies_all_files_transactionally() {
        let root = temp_dir("collect-files");
        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        let first = source.join("a.txt");
        let second = source.join("b.txt");
        std::fs::write(&first, b"alpha").unwrap();
        std::fs::write(&second, b"beta").unwrap();
        let content =
            crate::clipboard::file_paths_to_clipboard_content(&[first.clone(), second.clone()])
                .unwrap();
        let history = history_fixture("history-files", ClipboardContentType::FileList, content);
        let items = collect_history_item(&root, &[], &history, false).unwrap();
        assert_eq!(items[0].assets.len(), 2);
        assert!(items[0]
            .assets
            .iter()
            .all(|asset| resolve_asset_path(&root, asset).unwrap().exists()));

        std::fs::remove_file(second).unwrap();
        let broken_content = crate::clipboard::file_paths_to_clipboard_content(&[
            first,
            source.join("b.txt"),
        ])
        .unwrap();
        let broken = history_fixture(
            "history-broken",
            ClipboardContentType::FileList,
            broken_content,
        );
        let before = library_storage_size(&root).unwrap();
        assert!(collect_history_item(&root, &items, &broken, false).is_err());
        assert_eq!(library_storage_size(&root).unwrap(), before);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn snippet_updates_convert_text_and_validate_reorder() {
        let snippet = new_snippet("Greeting", "Hello", vec![], "").unwrap();
        let old_hash = snippet.content_hash.clone();
        let updated = update_item(
            std::slice::from_ref(&snippet),
            &snippet.id,
            LibraryItemUpdate {
                title: "Greeting".into(),
                content: Some("Hello team".into()),
                tags: vec!["reply".into()],
                note: "Daily".into(),
            },
        )
        .unwrap();
        assert_ne!(updated[0].content_hash, old_hash);
        assert_eq!(updated[0].summary, "Hello team");

        let root = temp_dir("convert-text");
        let history = history_fixture("history-text", ClipboardContentType::Text, "Body".into());
        let saved = collect_history_item(&root, &[], &history, false).unwrap();
        let converted = convert_to_snippet(&saved, &saved[0].id).unwrap();
        assert_eq!(converted[0].role, LibraryRole::Snippet);
        assert_eq!(converted[0].source_history_id, None);

        let first_id = updated[0].id.clone();
        let second = new_snippet("Second", "Two", vec![], "").unwrap();
        let mut pinned = vec![updated[0].clone(), second];
        pinned = set_pinned(&pinned, &first_id, true).unwrap();
        let second_id = pinned[1].id.clone();
        pinned = set_pinned(&pinned, &second_id, true).unwrap();
        let reordered = reorder_pinned(&pinned, &[second_id.clone(), first_id.clone()]).unwrap();
        assert_eq!(reordered[0].id, second_id);
        assert!(reorder_pinned(&pinned, &[first_id]).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn removing_last_reference_prunes_asset_but_unpinning_does_not() {
        let root = temp_dir("remove-shared");
        let asset = store_asset(&root, LibraryAssetKind::File, "shared.txt", b"shared").unwrap();
        let mut first = new_snippet("first", "one", vec![], "").unwrap();
        first.content_type = ClipboardContentType::FileList;
        first.role = LibraryRole::Saved;
        first.content.clear();
        first.assets = vec![asset.clone()];
        first.is_pinned = true;
        first.pin_order = Some(0);
        let mut second = first.clone();
        second.id = Uuid::new_v4().to_string();
        let path = resolve_asset_path(&root, &asset).unwrap();

        let items = set_pinned(&[first.clone(), second.clone()], &first.id, false).unwrap();
        prune_library_resources(&root, &items).unwrap();
        assert!(path.exists());
        let items = remove_item(&items, &first.id).unwrap();
        prune_library_resources(&root, &items).unwrap();
        assert!(path.exists());
        let items = remove_item(&items, &second.id).unwrap();
        prune_library_resources(&root, &items).unwrap();
        assert!(items.is_empty());
        assert!(!path.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn copy_payload_uses_managed_text_and_file_assets() {
        let root = temp_dir("copy-payload");
        let snippet = new_snippet("reply", "received", vec![], "").unwrap();
        assert_eq!(
            copy_payload(&root, &snippet).unwrap(),
            LibraryCopyPayload::Text("received".into())
        );

        let asset = store_asset(&root, LibraryAssetKind::File, "report.txt", b"report").unwrap();
        let duplicate =
            store_asset(&root, LibraryAssetKind::File, "summary.txt", b"report").unwrap();
        for invalid_name in ["C:escape.txt", "../escape.txt", "CON.txt", "trailing."] {
            let mut invalid = asset.clone();
            invalid.file_name = invalid_name.into();
            assert!(copy_file_name(&invalid).is_err(), "accepted {invalid_name}");
        }
        let mut file_item = snippet;
        file_item.role = LibraryRole::Saved;
        file_item.content_type = ClipboardContentType::FileList;
        file_item.content.clear();
        file_item.assets = vec![asset, duplicate];
        let old_cache = root.join(LIBRARY_COPY_CACHE_DIR).join("old").join("0");
        std::fs::create_dir_all(&old_cache).unwrap();
        std::fs::write(old_cache.join("old.txt"), b"old").unwrap();
        let LibraryCopyPayload::Files(paths) = copy_payload(&root, &file_item).unwrap() else {
            panic!("expected file copy payload");
        };
        assert!(old_cache.join("old.txt").exists());
        assert_eq!(library_storage_size(&root).unwrap(), 21);
        assert_eq!(
            paths
                .iter()
                .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
                .collect::<Vec<_>>(),
            vec!["report.txt", "summary.txt"]
        );
        assert_ne!(paths[0], paths[1]);
        assert_eq!(std::fs::read(&paths[0]).unwrap(), b"report");
        assert_eq!(std::fs::read(&paths[1]).unwrap(), b"report");
        commit_file_copy_cache(&root, &paths).unwrap();
        assert!(!old_cache.exists());
        assert_eq!(library_storage_size(&root).unwrap(), 18);
        discard_file_copy_cache(&paths).unwrap();
        assert!(!paths[0].exists());
        assert_eq!(library_storage_size(&root).unwrap(), 6);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_round_trips_and_corruption_returns_warning() {
        let root = temp_dir("round-trip");
        let item = new_snippet("reply", "received", vec![], "").unwrap();
        save_library_to_dir(&root, std::slice::from_ref(&item)).unwrap();
        let snapshot = load_library_from_dir(&root);
        assert_eq!(snapshot.items, vec![item]);
        assert_eq!(snapshot.warning, None);

        std::fs::write(root.join("library.json"), "{broken").unwrap();
        let corrupted = load_library_from_dir(&root);
        assert!(corrupted.items.is_empty());
        assert!(corrupted
            .warning
            .unwrap()
            .contains("收藏库数据损坏"));
        assert_eq!(
            std::fs::read_to_string(root.join("library.json.corrupt")).unwrap(),
            "{broken"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_recovers_backup_when_atomic_replace_was_interrupted() {
        let root = temp_dir("recover-backup");
        let item = new_snippet("reply", "received", vec![], "").unwrap();
        save_library_to_dir(&root, std::slice::from_ref(&item)).unwrap();
        std::fs::rename(
            root.join("library.json"),
            root.join("library.json.bak"),
        )
        .unwrap();

        let snapshot = load_library_from_dir(&root);

        assert_eq!(snapshot.items, vec![item]);
        assert_eq!(snapshot.warning, None);
        assert!(root.join("library.json").exists());
        assert!(!root.join("library.json.bak").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn library_startup_clears_stale_file_copy_cache() {
        let root = temp_dir("stale-copy-cache");
        let stale = root.join(LIBRARY_COPY_CACHE_DIR).join("old").join("0");
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("report.txt"), b"stale").unwrap();

        let snapshot = load_library_from_dir(&root);

        assert!(snapshot.items.is_empty());
        assert!(!root.join(LIBRARY_COPY_CACHE_DIR).exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_atomic_replace_restores_original_library() {
        let root = temp_dir("rollback");
        let target = root.join("library.json");
        let temp = root.join("library.json.tmp");
        std::fs::write(&target, b"old").unwrap();
        std::fs::write(&temp, b"new").unwrap();

        let mut call_count = 0;
        let result = replace_with_backup(&target, &temp, |from, to| {
            call_count += 1;
            if call_count == 2 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected replacement failure",
                ));
            }
            std::fs::rename(from, to)
        });

        assert!(result.is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"old");
        assert!(!root.join("library.json.bak").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn committed_library_save_ignores_backup_cleanup_failure() {
        use std::os::windows::fs::OpenOptionsExt;

        let root = temp_dir("committed-backup-cleanup");
        let target = root.join("library.json");
        let temp = root.join("library.json.tmp");
        std::fs::write(&target, b"old").unwrap();
        std::fs::write(&temp, b"new").unwrap();
        let backup = root.join("library.json.bak");
        let mut backup_lock = None;

        let result = replace_with_backup(&target, &temp, |from, to| {
            std::fs::rename(from, to)?;
            if to == target && backup.exists() {
                backup_lock = Some(
                    std::fs::OpenOptions::new()
                        .read(true)
                        .share_mode(1)
                        .open(&backup)?,
                );
            }
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        drop(backup_lock);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn content_addressed_assets_deduplicate_and_prune() {
        let root = temp_dir("assets");
        let first = store_asset(&root, LibraryAssetKind::File, "a.txt", b"same").unwrap();
        let second = store_asset(&root, LibraryAssetKind::File, "b.log", b"same").unwrap();
        assert_eq!(first.sha256, second.sha256);
        assert_eq!(first.relative_path, second.relative_path);

        prune_assets(&root, std::slice::from_ref(&first)).unwrap();
        assert!(resolve_asset_path(&root, &first).unwrap().exists());
        assert_eq!(library_storage_size(&root).unwrap(), 4);
        prune_assets(&root, &[]).unwrap();
        assert!(!resolve_asset_path(&root, &first).unwrap().exists());
        assert_eq!(library_storage_size(&root).unwrap(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn metadata_validation_normalizes_tags_and_limits_fields() {
        let metadata = normalize_metadata(
            "  VPN 登录信息  ",
            vec![" Work ".into(), "work".into(), "运维".into()],
            "  仅限内网  ",
        )
        .unwrap();

        assert_eq!(metadata.title, "VPN 登录信息");
        assert_eq!(metadata.tags, vec!["Work", "运维"]);
        assert_eq!(metadata.note, "仅限内网");
        assert!(normalize_metadata(&"x".repeat(121), vec![], "").is_err());
        assert!(normalize_metadata("title", vec!["x".repeat(33)], "").is_err());
        assert!(normalize_metadata("title", vec![], &"x".repeat(2001)).is_err());
    }

    #[test]
    fn snippet_requires_title_and_body() {
        assert!(new_snippet("", "body", vec![], "").is_err());
        assert!(new_snippet("title", "  ", vec![], "").is_err());

        let item = new_snippet("问候语", "您好", vec!["回复".into()], "").unwrap();
        assert_eq!(item.role, LibraryRole::Snippet);
        assert_eq!(item.content_type, ClipboardContentType::Text);
        assert_eq!(item.title, "问候语");
        assert_eq!(item.content, "您好");
        assert!(item.assets.is_empty());
    }

    #[test]
    fn pinned_items_sort_before_recent_items() {
        let mut first = new_snippet("first", "1", vec![], "").unwrap();
        let mut second = new_snippet("second", "2", vec![], "").unwrap();
        let normal = new_snippet("normal", "3", vec![], "").unwrap();
        first.is_pinned = true;
        first.pin_order = Some(1);
        second.is_pinned = true;
        second.pin_order = Some(0);

        let sorted = sort_library_items(vec![normal, first, second]);
        assert_eq!(sorted[0].title, "second");
        assert_eq!(sorted[1].title, "first");
        assert_eq!(sorted[2].title, "normal");
    }
}
