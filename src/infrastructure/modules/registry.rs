//! Persist the installed external module registry next to workspace metadata.

use super::Error;
use crate::domain::modules::ModuleRegistry;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(super) fn path() -> Result<PathBuf, Error> {
    if let Some(path) = std::env::var_os("IWE_DATA_DIR") {
        return Ok(PathBuf::from(path).join("modules.json"));
    }
    directories::ProjectDirs::from("", "", "iwe")
        .map(|dirs| dirs.config_dir().join("modules.json"))
        .ok_or(Error::Storage)
}

/// Load the registry. Missing files produce an empty registry.
pub(crate) fn load() -> Result<ModuleRegistry, Error> {
    load_at(&path()?)
}

pub(super) fn load_at(path: &Path) -> Result<ModuleRegistry, Error> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ModuleRegistry::default());
        }
        Err(_) => return Err(Error::Storage),
    };
    serde_json::from_slice(&bytes).map_err(|_| Error::Storage)
}

/// Save the registry through an atomic file replacement.
pub(super) fn save_at(path: &Path, registry: &ModuleRegistry) -> Result<(), Error> {
    let parent = path.parent().ok_or(Error::Storage)?;
    fs::create_dir_all(parent).map_err(|_| Error::Storage)?;
    let bytes = serde_json::to_vec_pretty(registry).map_err(|_| Error::Storage)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| Error::Storage)?;
    temporary.write_all(&bytes).map_err(|_| Error::Storage)?;
    temporary.as_file().sync_all().map_err(|_| Error::Storage)?;
    temporary.persist(path).map_err(|_| Error::Storage)?;
    Ok(())
}

#[cfg(test)]
mod tests;
