use std::{
    ffi::OsString,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};

use crate::error::AppResult;

pub fn load<T: DeserializeOwned>(path: &Path) -> AppResult<Option<T>> {
    let temporary = artifact_path(path, "tmp");
    let backup = artifact_path(path, "bak");

    if !path.exists() {
        if let Some(value) = promote_valid_candidate::<T>(&temporary, path)? {
            remove_if_exists(&backup)?;
            return Ok(Some(value));
        }
        if let Some(value) = promote_valid_candidate::<T>(&backup, path)? {
            remove_if_exists(&temporary)?;
            return Ok(Some(value));
        }
        return Ok(None);
    }

    match read_candidate(path)? {
        Ok(value) => {
            remove_if_exists(&temporary)?;
            remove_if_exists(&backup)?;
            Ok(Some(value))
        }
        Err(_) => {
            if let Some(value) = replace_corrupt_with_candidate::<T>(path, &backup)? {
                remove_if_exists(&temporary)?;
                return Ok(Some(value));
            }
            if let Some(value) = replace_corrupt_with_candidate::<T>(path, &temporary)? {
                remove_if_exists(&backup)?;
                return Ok(Some(value));
            }

            isolate_corrupt(path)?;
            remove_if_exists(&temporary)?;
            if backup.exists() {
                isolate_corrupt(&backup)?;
            }
            Ok(None)
        }
    }
}

pub fn save<T: Serialize + ?Sized>(path: &Path, value: &T) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary = artifact_path(path, "tmp");
    let backup = artifact_path(path, "bak");
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = File::create(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);

    remove_if_exists(&backup)?;
    if path.exists() {
        fs::rename(path, &backup)?;
    }

    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(error.into());
    }

    remove_if_exists(&backup)?;
    Ok(())
}

fn promote_valid_candidate<T: DeserializeOwned>(
    candidate: &Path,
    target: &Path,
) -> AppResult<Option<T>> {
    if !candidate.exists() {
        return Ok(None);
    }
    let value = match read_candidate(candidate)? {
        Ok(value) => value,
        Err(_) => {
            remove_if_exists(candidate)?;
            return Ok(None);
        }
    };
    fs::rename(candidate, target)?;
    Ok(Some(value))
}

fn replace_corrupt_with_candidate<T: DeserializeOwned>(
    corrupt: &Path,
    candidate: &Path,
) -> AppResult<Option<T>> {
    if !candidate.exists() {
        return Ok(None);
    }
    let value = match read_candidate(candidate)? {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    isolate_corrupt(corrupt)?;
    fs::rename(candidate, corrupt)?;
    Ok(Some(value))
}

fn read_candidate<T: DeserializeOwned>(path: &Path) -> AppResult<Result<T, serde_json::Error>> {
    let bytes = fs::read(path)?;
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    Ok(serde_json::from_slice(bytes))
}

fn artifact_path(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("data.json"));
    name.push(format!(".{suffix}"));
    path.with_file_name(name)
}

fn isolate_corrupt(path: &Path) -> AppResult<PathBuf> {
    let mut target = artifact_path(path, "corrupt");
    let mut index = 1usize;
    while target.exists() {
        target = artifact_path(path, &format!("corrupt.{index}"));
        index += 1;
    }
    fs::rename(path, &target)?;
    Ok(target)
}

fn remove_if_exists(path: &Path) -> AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Record {
        value: String,
    }

    fn test_path(label: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("copyshare-safe-json-{label}-{}", Uuid::new_v4()))
            .join("state.json")
    }

    #[test]
    fn round_trip_replaces_existing_value_without_artifacts() {
        let path = test_path("round-trip");
        save(
            &path,
            &Record {
                value: "first".into(),
            },
        )
        .unwrap();
        save(
            &path,
            &Record {
                value: "second".into(),
            },
        )
        .unwrap();

        let loaded = load::<Record>(&path).unwrap().unwrap();

        assert_eq!(loaded.value, "second");
        assert!(!artifact_path(&path, "tmp").exists());
        assert!(!artifact_path(&path, "bak").exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn missing_target_promotes_valid_temporary_file() {
        let path = test_path("promote-temp");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            artifact_path(&path, "tmp"),
            serde_json::to_vec(&Record {
                value: "latest".into(),
            })
            .unwrap(),
        )
        .unwrap();

        let loaded = load::<Record>(&path).unwrap().unwrap();

        assert_eq!(loaded.value, "latest");
        assert!(path.exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn corrupt_target_recovers_backup_and_preserves_bad_file() {
        let path = test_path("recover-backup");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{broken").unwrap();
        fs::write(
            artifact_path(&path, "bak"),
            serde_json::to_vec(&Record {
                value: "backup".into(),
            })
            .unwrap(),
        )
        .unwrap();

        let loaded = load::<Record>(&path).unwrap().unwrap();

        assert_eq!(loaded.value, "backup");
        assert!(artifact_path(&path, "corrupt").exists());
        assert_eq!(load::<Record>(&path).unwrap().unwrap().value, "backup");
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn unrecoverable_json_is_isolated_and_returns_none() {
        let path = test_path("isolate");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{broken").unwrap();

        assert!(load::<Record>(&path).unwrap().is_none());
        assert!(!path.exists());
        assert!(artifact_path(&path, "corrupt").exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn utf8_bom_is_accepted() {
        let path = test_path("bom");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"\xef\xbb\xbf{\"value\":\"ok\"}").unwrap();

        assert_eq!(load::<Record>(&path).unwrap().unwrap().value, "ok");
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
