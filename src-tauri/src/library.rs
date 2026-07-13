use std::{
    cmp::Reverse,
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

use chrono::Utc;
use sha2::{Digest, Sha256};
use tauri::Manager;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{
        ClipboardContentType, LibraryAssetKind, LibraryAssetRef, LibraryItem, LibraryRole,
        LibrarySnapshot,
    },
};

const LIBRARY_FILE: &str = "library.json";
const LIBRARY_ASSET_DIR: &str = "library-assets";
const LIBRARY_THUMBNAIL_DIR: &str = "library-thumbnails";
const TITLE_LIMIT: usize = 120;
const NOTE_LIMIT: usize = 2_000;
const TAG_LIMIT: usize = 20;
const TAG_LENGTH_LIMIT: usize = 32;

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
    let path = root.join(LIBRARY_FILE);
    if !path.exists() {
        return LibrarySnapshot::default();
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

pub fn save_library(app: &tauri::AppHandle, items: &[LibraryItem]) -> AppResult<()> {
    save_library_to_dir(&app.path().app_data_dir()?, items)
}

fn save_library_to_dir(root: &Path, items: &[LibraryItem]) -> AppResult<()> {
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
        fs::remove_file(backup)?;
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
    let stored_name = format!("{sha256}.{}", safe_extension(file_name));
    let relative_path = format!("{LIBRARY_ASSET_DIR}/{stored_name}");
    let directory = root.join(LIBRARY_ASSET_DIR);
    let target = directory.join(&stored_name);
    fs::create_dir_all(&directory)?;

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
        let metadata = entry?.metadata()?;
        Ok(total + if metadata.is_file() { metadata.len() } else { 0 })
    })
}

pub fn library_storage_size(root: &Path) -> AppResult<u64> {
    Ok(directory_size(&root.join(LIBRARY_ASSET_DIR))?
        + directory_size(&root.join(LIBRARY_THUMBNAIL_DIR))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::models::LibraryAssetKind;

    fn temp_dir(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "copyshare-library-{label}-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&root).unwrap();
        root
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

    #[test]
    fn content_addressed_assets_deduplicate_and_prune() {
        let root = temp_dir("assets");
        let first = store_asset(&root, LibraryAssetKind::File, "a.txt", b"same").unwrap();
        let second = store_asset(&root, LibraryAssetKind::File, "b.txt", b"same").unwrap();
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
