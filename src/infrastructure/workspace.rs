//! Persist workspace metadata and read bounded previews without following external symlinks.

use super::process::Failure;
use crate::domain::workspace::{FileEntry, Workspace};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) const PREVIEW_LIMIT: u64 = 512 * 1024;

fn config_path() -> Result<PathBuf, Failure> {
    if let Some(path) = std::env::var_os("IWE_DATA_DIR") {
        return Ok(PathBuf::from(path).join("workspace.json"));
    }
    directories::ProjectDirs::from("", "", "iwe")
        .map(|dirs| dirs.config_dir().join("workspace.json"))
        .ok_or(Failure::Storage)
}

pub(crate) async fn load() -> Result<(Workspace, Option<PathBuf>), Failure> {
    tokio::task::spawn_blocking(|| {
        let path = config_path()?;
        let mut workspace = load_at(&path)?;
        workspace.normalize();
        let mut pending_directory = None;
        if let Some(path) = std::env::args_os().nth(1) {
            let path = PathBuf::from(path);
            if path.is_dir()
                && let Ok(directory) = path.canonicalize()
                && !workspace.select_directory(&directory)
            {
                pending_directory = Some(directory);
            }
        }
        Ok((workspace, pending_directory))
    })
    .await
    .map_err(|_| Failure::Storage)?
}

fn load_at(path: &Path) -> Result<Workspace, Failure> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Workspace::default());
        }
        Err(_) => return Err(Failure::Storage),
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| Failure::Storage)?;
    let legacy = value.get("spaces").is_none() && value.get("projects").is_some();
    let workspace = serde_json::from_value(value).map_err(|_| Failure::Storage)?;
    if legacy {
        let parent = path.parent().ok_or(Failure::Storage)?;
        let mut temporary =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| Failure::Storage)?;
        temporary.write_all(&bytes).map_err(|_| Failure::Storage)?;
        temporary
            .as_file()
            .sync_all()
            .map_err(|_| Failure::Storage)?;
        if let Err(error) = temporary.persist_noclobber(path.with_extension("v1.json"))
            && error.error.kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(Failure::Storage);
        }
    }
    Ok(workspace)
}

pub(crate) async fn save(workspace: Workspace) -> Result<(), Failure> {
    tokio::task::spawn_blocking(move || save_at(&config_path()?, &workspace))
        .await
        .map_err(|_| Failure::Storage)?
}

fn save_at(path: &Path, workspace: &Workspace) -> Result<(), Failure> {
    let parent = path.parent().ok_or(Failure::Storage)?;
    std::fs::create_dir_all(parent).map_err(|_| Failure::Storage)?;
    let bytes = serde_json::to_vec_pretty(workspace).map_err(|_| Failure::Storage)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| Failure::Storage)?;
    temporary.write_all(&bytes).map_err(|_| Failure::Storage)?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| Failure::Storage)?;
    temporary.persist(path).map_err(|_| Failure::Storage)?;
    Ok(())
}

pub(crate) async fn choose_directories() -> Vec<PathBuf> {
    let selected = rfd::AsyncFileDialog::new().pick_folders().await;
    let mut directories = Vec::new();
    for handle in selected.unwrap_or_default() {
        if let Ok(directory) = tokio::fs::canonicalize(handle.path()).await
            && !directories.contains(&directory)
        {
            directories.push(directory);
        }
    }
    directories
}

#[cfg(test)]
pub(crate) async fn entries(root: PathBuf, directory: PathBuf) -> Result<Vec<FileEntry>, Failure> {
    tokio::task::spawn_blocking(move || entries_at(&root, &directory))
        .await
        .map_err(|_| Failure::Read)?
}

pub(super) fn entries_at(root: &Path, directory: &Path) -> Result<Vec<FileEntry>, Failure> {
    let root = root.canonicalize().map_err(|_| Failure::Read)?;
    let resolved = directory.canonicalize().map_err(|_| Failure::Read)?;
    if !resolved.starts_with(&root) {
        return Err(Failure::Read);
    }
    let mut result = Vec::new();
    for entry in std::fs::read_dir(resolved).map_err(|_| Failure::Read)? {
        let entry = entry.map_err(|_| Failure::Read)?;
        let kind = entry.file_type().map_err(|_| Failure::Read)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == ".git" || kind.is_symlink() {
            continue;
        }
        if !kind.is_dir() && !kind.is_file() {
            continue;
        }
        result.push(FileEntry {
            path: directory.join(entry.file_name()),
            name,
            directory: kind.is_dir(),
        });
        if result.len() > 5000 {
            return Err(Failure::TooLarge);
        }
    }
    result.sort_by(|a, b| {
        b.directory
            .cmp(&a.directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(result)
}

pub(crate) async fn read(root: PathBuf, path: PathBuf) -> Result<String, Failure> {
    tokio::task::spawn_blocking(move || {
        let root = root.canonicalize().map_err(|_| Failure::Read)?;
        let path = path.canonicalize().map_err(|_| Failure::Read)?;
        if !path.starts_with(root) {
            return Err(Failure::Read);
        }
        let file = std::fs::File::open(path).map_err(|_| Failure::Read)?;
        if !file.metadata().map_err(|_| Failure::Read)?.is_file() {
            return Err(Failure::Read);
        }
        let mut bytes = Vec::new();
        file.take(PREVIEW_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Failure::Read)?;
        decode(bytes)
    })
    .await
    .map_err(|_| Failure::Read)?
}

pub(crate) fn decode(bytes: Vec<u8>) -> Result<String, Failure> {
    if bytes.len() as u64 > PREVIEW_LIMIT {
        return Err(Failure::TooLarge);
    }
    if bytes.contains(&0) {
        return Err(Failure::Binary);
    }
    String::from_utf8(bytes).map_err(|_| Failure::Binary)
}

#[cfg(test)]
mod tests;
