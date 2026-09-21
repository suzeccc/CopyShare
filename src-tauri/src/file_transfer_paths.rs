use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::{
    error::{AppError, AppResult},
    models::AppConfig,
};

pub(crate) fn sanitize_file_name(value: &str) -> String {
    let normalized = value.replace('\\', "/");
    let base = normalized.rsplit('/').next().unwrap_or_default().trim();
    let mut cleaned = base
        .chars()
        .filter(|character| {
            !character.is_control()
                && !matches!(
                    character,
                    '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'
                )
        })
        .collect::<String>();
    while cleaned.contains("..") {
        cleaned = cleaned.replace("..", ".");
    }
    let cleaned = cleaned.trim_matches(|character| character == '.' || character == ' ');
    if cleaned.is_empty() {
        "download".to_string()
    } else {
        cleaned.to_string()
    }
}

fn default_save_dir() -> AppResult<PathBuf> {
    let downloads = dirs::download_dir()
        .ok_or_else(|| AppError::InvalidInput("cannot locate downloads folder".to_string()))?;
    Ok(downloads.join("Copy-Sharer"))
}

pub(crate) fn transfer_save_dir(config: &AppConfig) -> AppResult<PathBuf> {
    config
        .file_save_dir
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(default_save_dir)
}

#[cfg(test)]
pub(crate) fn unique_save_path(save_dir: &Path, file_name: &str) -> PathBuf {
    unique_save_path_with_reserved(save_dir, file_name, &HashSet::new())
}

pub(crate) fn unique_save_path_with_reserved(
    save_dir: &Path,
    file_name: &str,
    reserved_paths: &HashSet<PathBuf>,
) -> PathBuf {
    let sanitized = sanitize_file_name(file_name);
    let candidate = save_dir.join(&sanitized);
    if !candidate.exists()
        && !part_path_for(&candidate).exists()
        && !reserved_paths.contains(&candidate)
    {
        return candidate;
    }

    let path = Path::new(&sanitized);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1.. {
        let name = match extension {
            Some(extension) if !extension.is_empty() => {
                format!("{stem} ({index}).{extension}")
            }
            _ => format!("{stem} ({index})"),
        };
        let candidate = save_dir.join(name);
        if !candidate.exists()
            && !part_path_for(&candidate).exists()
            && !reserved_paths.contains(&candidate)
        {
            return candidate;
        }
    }
    unreachable!()
}

pub(crate) fn part_path_for(final_path: &Path) -> PathBuf {
    final_path.with_extension(format!(
        "{}part",
        final_path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!("{value}."))
            .unwrap_or_default()
    ))
}
