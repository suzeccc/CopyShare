use std::cmp::Reverse;

use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{ClipboardContentType, LibraryItem, LibraryRole},
};

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

#[cfg(test)]
mod tests {
    use super::*;

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
