use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{FileTransferDirection, FileTransferFileStatus, FileTransferTask},
};

pub const TRANSFER_SNAPSHOT_VERSION: u32 = 1;
const TRANSFER_EXPIRY_DAYS: i64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedTransferFile {
    pub file_id: String,
    pub source_path: Option<PathBuf>,
    pub source_size: Option<u64>,
    pub source_modified_ms: Option<i64>,
    pub temp_path: Option<PathBuf>,
    pub final_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferSnapshot {
    pub version: u32,
    pub task: FileTransferTask,
    pub files: Vec<PersistedTransferFile>,
    pub download_host: Option<String>,
    pub download_port: Option<u16>,
    pub retry_count: u8,
    pub last_activity_at: DateTime<Utc>,
}

pub fn save(root: &Path, snapshot: &TransferSnapshot) -> AppResult<()> {
    let directory = snapshot_directory(root, &snapshot.task.direction);
    fs::create_dir_all(&directory)?;
    let target = snapshot_path(root, &snapshot.task.direction, &snapshot.task.transfer_id);
    let temp = target.with_extension("json.tmp");
    let backup = target.with_extension("json.bak");
    let bytes = serde_json::to_vec_pretty(snapshot)?;
    let mut file = File::create(&temp)?;
    file.write_all(&bytes)?;
    file.sync_all()?;

    if target.exists() {
        let _ = fs::remove_file(&backup);
        fs::rename(&target, &backup)?;
        if let Err(error) = fs::rename(&temp, &target) {
            let _ = fs::rename(&backup, &target);
            return Err(error.into());
        }
        let _ = fs::remove_file(backup);
    } else {
        fs::rename(temp, target)?;
    }
    Ok(())
}

pub fn load_all(root: &Path) -> AppResult<Vec<TransferSnapshot>> {
    let mut snapshots = Vec::new();
    for direction in [FileTransferDirection::Send, FileTransferDirection::Receive] {
        let directory = snapshot_directory(root, &direction);
        if !directory.exists() {
            continue;
        }
        recover_backups(&directory)?;
        cleanup_temporary_snapshots(&directory)?;
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let parsed = fs::read(&path)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<TransferSnapshot>(&bytes).ok());
            match parsed {
                Some(snapshot) if snapshot.version == TRANSFER_SNAPSHOT_VERSION => {
                    snapshots.push(snapshot)
                }
                _ => isolate_invalid_snapshot(&path)?,
            }
        }
    }
    snapshots.sort_by(|left, right| left.task.transfer_id.cmp(&right.task.transfer_id));
    Ok(snapshots)
}

pub fn remove(
    root: &Path,
    direction: &FileTransferDirection,
    transfer_id: &str,
) -> AppResult<()> {
    let path = snapshot_path(root, direction, transfer_id);
    for artifact in [
        path.clone(),
        path.with_extension("json.bak"),
        path.with_extension("json.tmp"),
    ] {
        if artifact.exists() {
            fs::remove_file(artifact)?;
        }
    }
    Ok(())
}

pub fn prune_expired(root: &Path, now: DateTime<Utc>) -> AppResult<Vec<TransferSnapshot>> {
    let snapshots = load_all(root)?;
    let mut expired = Vec::new();
    for snapshot in snapshots {
        if now.signed_duration_since(snapshot.last_activity_at)
            > Duration::days(TRANSFER_EXPIRY_DAYS)
        {
            remove(root, &snapshot.task.direction, &snapshot.task.transfer_id)?;
            expired.push(snapshot);
        }
    }
    Ok(expired)
}

pub fn reconcile_receiver_progress(snapshot: &mut TransferSnapshot) -> AppResult<()> {
    if snapshot.task.direction != FileTransferDirection::Receive {
        return Ok(());
    }
    for persisted in &snapshot.files {
        let Some(file) = snapshot
            .task
            .files
            .iter_mut()
            .find(|file| file.id == persisted.file_id)
        else {
            continue;
        };
        let final_is_valid = persisted
            .final_path
            .as_deref()
            .filter(|path| path.exists())
            .map(fs::metadata)
            .transpose()?
            .is_some_and(|metadata| metadata.is_file() && metadata.len() == file.size);
        if final_is_valid {
            file.status = FileTransferFileStatus::Completed;
            file.transferred_bytes = file.size;
            file.saved_path = persisted
                .final_path
                .as_deref()
                .map(|path| path.to_string_lossy().to_string());
            continue;
        }
        if file.status == FileTransferFileStatus::Completed {
            file.status = FileTransferFileStatus::Pending;
            file.saved_path = None;
        }
        let actual = persisted
            .temp_path
            .as_deref()
            .filter(|path| path.exists())
            .map(fs::metadata)
            .transpose()?
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        if actual > file.size {
            return Err(AppError::InvalidInput(format!(
                "partial file is larger than expected: {}",
                file.name
            )));
        }
        file.transferred_bytes = actual;
        if actual > 0 {
            file.status = FileTransferFileStatus::Transferring;
        }
    }
    snapshot.task.transferred_bytes = snapshot
        .task
        .files
        .iter()
        .map(|file| file.transferred_bytes.min(file.size))
        .sum::<u64>()
        .min(snapshot.task.total_size);
    Ok(())
}

fn snapshot_directory(root: &Path, direction: &FileTransferDirection) -> PathBuf {
    root.join(match direction {
        FileTransferDirection::Send => "sender",
        FileTransferDirection::Receive => "receiver",
    })
}

fn snapshot_path(root: &Path, direction: &FileTransferDirection, transfer_id: &str) -> PathBuf {
    let digest = Sha256::digest(transfer_id.as_bytes());
    snapshot_directory(root, direction).join(format!("{digest:x}.json"))
}

fn isolate_invalid_snapshot(path: &Path) -> AppResult<()> {
    let mut target = path.with_extension("invalid");
    if target.exists() {
        target = path.with_extension(format!("{}.invalid", Uuid::new_v4()));
    }
    fs::rename(path, target)?;
    Ok(())
}

fn recover_backups(directory: &Path) -> AppResult<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("bak") {
            continue;
        }
        let target = path.with_extension("json");
        if target.exists() {
            fs::remove_file(path)?;
        } else {
            fs::rename(path, target)?;
        }
    }
    Ok(())
}

fn cleanup_temporary_snapshots(directory: &Path) -> AppResult<()> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.ends_with(".json.tmp"))
        {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use super::*;
    use crate::models::{
        FileTransferDirection, FileTransferFile, FileTransferFileStatus, FileTransferStatus,
        FileTransferTask,
    };

    fn test_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "copyshare-transfer-store-{name}-{}",
            Uuid::new_v4()
        ))
    }

    fn snapshot(root: &std::path::Path) -> TransferSnapshot {
        let temp_path = root.join("video.mp4.part");
        let final_path = root.join("video.mp4");
        TransferSnapshot {
            version: TRANSFER_SNAPSHOT_VERSION,
            task: FileTransferTask {
                transfer_id: "transfer-1".to_string(),
                direction: FileTransferDirection::Receive,
                peer_device_id: "device-a".to_string(),
                peer_device_name: "Laptop A".to_string(),
                clipboard_sync: true,
                files: vec![FileTransferFile {
                    id: "file-1".to_string(),
                    name: "video.mp4".to_string(),
                    size: 10,
                    sha256: "hash".to_string(),
                    thumbnail: None,
                    saved_path: None,
                    transferred_bytes: 0,
                    status: FileTransferFileStatus::Pending,
                    error: None,
                }],
                total_size: 10,
                transferred_bytes: 0,
                status: FileTransferStatus::WaitingForPeer,
                created_at: Utc::now(),
                completed_at: None,
                error: None,
            },
            files: vec![PersistedTransferFile {
                file_id: "file-1".to_string(),
                source_path: None,
                source_size: None,
                source_modified_ms: None,
                temp_path: Some(temp_path),
                final_path: Some(final_path),
            }],
            download_host: None,
            download_port: None,
            retry_count: 0,
            last_activity_at: Utc::now(),
        }
    }

    #[test]
    fn snapshot_round_trips_and_replaces_existing_state() {
        let root = test_root("round-trip");
        let mut first = snapshot(&root);
        save(&root, &first).unwrap();
        first.retry_count = 2;
        save(&root, &first).unwrap();

        let loaded = load_all(&root).unwrap();

        assert_eq!(loaded, vec![first]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_temporary_snapshot_is_removed_during_load() {
        let root = test_root("stale-temp");
        let directory = snapshot_directory(&root, &FileTransferDirection::Receive);
        fs::create_dir_all(&directory).unwrap();
        let stale = directory.join("stale.json.tmp");
        fs::write(&stale, b"partial json").unwrap();

        assert!(load_all(&root).unwrap().is_empty());
        assert!(!stale.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn remove_clears_snapshot_backup_and_temporary_files() {
        let root = test_root("remove-all-artifacts");
        let value = snapshot(&root);
        save(&root, &value).unwrap();
        let target = snapshot_path(
            &root,
            &FileTransferDirection::Receive,
            &value.task.transfer_id,
        );
        let backup = target.with_extension("json.bak");
        let temporary = target.with_extension("json.tmp");
        fs::write(&backup, b"backup").unwrap();
        fs::write(&temporary, b"temporary").unwrap();

        remove(
            &root,
            &FileTransferDirection::Receive,
            &value.task.transfer_id,
        )
        .unwrap();

        assert!(!target.exists());
        assert!(!backup.exists());
        assert!(!temporary.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unknown_snapshot_versions_are_isolated() {
        let root = test_root("unknown-version");
        let mut value = snapshot(&root);
        value.version = TRANSFER_SNAPSHOT_VERSION + 1;
        save(&root, &value).unwrap();

        assert!(load_all(&root).unwrap().is_empty());
        assert!(snapshot_directory(&root, &FileTransferDirection::Receive)
            .read_dir()
            .unwrap()
            .any(|entry| entry.unwrap().path().extension().is_some_and(|value| value == "invalid")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn receiver_progress_uses_the_actual_part_length() {
        let root = test_root("part-length");
        fs::create_dir_all(&root).unwrap();
        let mut value = snapshot(&root);
        fs::write(value.files[0].temp_path.as_ref().unwrap(), b"data").unwrap();

        reconcile_receiver_progress(&mut value).unwrap();

        assert_eq!(value.task.files[0].transferred_bytes, 4);
        assert_eq!(value.task.transferred_bytes, 4);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_completed_file_falls_back_to_the_actual_partial_length() {
        let root = test_root("missing-completed-file");
        fs::create_dir_all(&root).unwrap();
        let mut value = snapshot(&root);
        value.task.files[0].status = FileTransferFileStatus::Completed;
        value.task.files[0].transferred_bytes = value.task.files[0].size;
        fs::write(value.files[0].temp_path.as_ref().unwrap(), b"data").unwrap();

        reconcile_receiver_progress(&mut value).unwrap();

        assert_eq!(
            value.task.files[0].status,
            FileTransferFileStatus::Transferring
        );
        assert_eq!(value.task.files[0].transferred_bytes, 4);
        assert_eq!(value.task.transferred_bytes, 4);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn renamed_final_file_recovers_as_completed_after_a_crash() {
        let root = test_root("renamed-final-file");
        fs::create_dir_all(&root).unwrap();
        let mut value = snapshot(&root);
        value.task.files[0].status = FileTransferFileStatus::Transferring;
        value.task.files[0].transferred_bytes = 4;
        fs::write(value.files[0].final_path.as_ref().unwrap(), b"0123456789").unwrap();

        reconcile_receiver_progress(&mut value).unwrap();

        assert_eq!(value.task.files[0].status, FileTransferFileStatus::Completed);
        assert_eq!(value.task.files[0].transferred_bytes, 10);
        assert_eq!(
            value.task.files[0].saved_path.as_deref(),
            value.files[0]
                .final_path
                .as_deref()
                .map(|path| path.to_string_lossy())
                .as_deref(),
        );
        assert_eq!(value.task.transferred_bytes, 10);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn snapshots_expire_after_seven_days() {
        let root = test_root("expiry");
        let mut value = snapshot(&root);
        value.last_activity_at = Utc::now() - Duration::days(8);
        save(&root, &value).unwrap();

        let expired = prune_expired(&root, Utc::now()).unwrap();

        assert_eq!(expired, vec![value]);
        assert!(load_all(&root).unwrap().is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
