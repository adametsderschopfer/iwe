//! Install, update, and remove external module packages.

use super::{Error, package, registry};
use crate::domain::modules::{InstalledModule, Manifest, ModuleRegistry, current_target};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Install a package file and persist the updated registry.
///
/// Returns the updated registry and the installed module identifier.
pub(crate) fn install(
    archive: &Path,
    current: &ModuleRegistry,
) -> Result<(ModuleRegistry, String), Error> {
    let root = super::modules_root()?;
    install_into(archive, current, &root, &registry::path()?)
}

/// Remove an installed module and persist the updated registry.
pub(crate) fn uninstall(id: &str, current: &ModuleRegistry) -> Result<ModuleRegistry, Error> {
    uninstall_from(id, current, &super::modules_root()?, &registry::path()?)
}

pub(super) fn install_into(
    archive: &Path,
    current: &ModuleRegistry,
    root: &Path,
    registry_file: &Path,
) -> Result<(ModuleRegistry, String), Error> {
    let manifest = package::manifest(archive)?;
    if let Some(installed) = current.get(&manifest.id)
        && manifest.version <= installed.manifest.version
    {
        return Err(Error::OlderVersion);
    }
    let id = manifest.id.clone();
    let version = manifest.version.to_string();
    let home = root.join(&id);
    fs::create_dir_all(&home).map_err(|_| Error::Storage)?;
    let staged = tempfile::Builder::new()
        .prefix(".staging-")
        .tempdir_in(&home)
        .map_err(|_| Error::Storage)?;
    let package = package::extract(archive, staged.path())?;
    if package.manifest != manifest {
        return Err(Error::Invalid);
    }
    let destination = home.join(&version);
    if destination.exists() {
        fs::remove_dir_all(&destination).map_err(|_| Error::Storage)?;
    }
    // Keep the staging content alive across the rename.
    let staged_path = staged.keep();
    mark_executable(&staged_path.join(&package.entry))?;
    fs::rename(&staged_path, &destination).map_err(|_| Error::Storage)?;
    remove_other_versions(&home, &version)?;
    let mut updated = current.clone();
    updated.modules.retain(|module| module.manifest.id != id);
    updated.modules.push(InstalledModule {
        manifest,
        installed_at: now_millis(),
    });
    updated
        .modules
        .sort_by(|a, b| a.manifest.id.cmp(&b.manifest.id));
    registry::save_at(registry_file, &updated).map_err(|_| Error::Storage)?;
    Ok((updated, id))
}

pub(super) fn uninstall_from(
    id: &str,
    current: &ModuleRegistry,
    root: &Path,
    registry_file: &Path,
) -> Result<ModuleRegistry, Error> {
    if current.get(id).is_none() {
        return Err(Error::Invalid);
    }
    match fs::remove_dir_all(root.join(id)) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(Error::Storage),
    }
    let mut updated = current.clone();
    updated.modules.retain(|module| module.manifest.id != id);
    registry::save_at(registry_file, &updated).map_err(|_| Error::Storage)?;
    Ok(updated)
}

/// Installed executable path for a registry record.
pub(crate) fn executable(manifest: &Manifest) -> Result<PathBuf, Error> {
    let name = manifest.entry_name().ok_or(Error::UnsupportedTarget)?;
    let path = super::module_directory(&manifest.id, &manifest.version.to_string())?
        .join("binaries")
        .join(current_target())
        .join(name);
    path.is_file().then_some(path).ok_or(Error::Invalid)
}

fn mark_executable(entry: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(entry, fs::Permissions::from_mode(0o755))
            .map_err(|_| Error::Storage)?;
    }
    #[cfg(not(unix))]
    {
        let _ = entry;
    }
    Ok(())
}

fn remove_other_versions(home: &Path, keep: &str) -> Result<(), Error> {
    let entries = fs::read_dir(home).map_err(|_| Error::Storage)?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name != keep && !name.starts_with(".staging-") {
            fs::remove_dir_all(entry.path()).map_err(|_| Error::Storage)?;
        }
    }
    Ok(())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
