//! Read and validate external module packages before installation.

use super::Error;
use crate::domain::modules::{Manifest, SYSTEM_MODULE_IDS, current_target};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

/// Largest accepted module package.
pub(crate) const MAX_PACKAGE: u64 = 128 * 1024 * 1024;
/// Largest accepted manifest file.
const MAX_MANIFEST: u64 = 256 * 1024;
/// Largest accepted unpacked package content.
const MAX_UNPACKED: u64 = MAX_PACKAGE * 3;
/// Largest accepted file count inside a package.
const MAX_ENTRIES: usize = 1024;

/// Validated package content staged for installation.
#[derive(Debug, PartialEq)]
pub(crate) struct Package {
    pub manifest: Manifest,
    /// Staging directory that holds the unpacked package.
    pub directory: PathBuf,
    /// Executable path inside the staging directory.
    pub entry: PathBuf,
}

/// Read only the manifest from a package archive.
pub(crate) fn manifest(archive: &Path) -> Result<Manifest, Error> {
    let size = fs::metadata(archive).map_err(|_| Error::Invalid)?.len();
    if size > MAX_PACKAGE {
        return Err(Error::TooLarge);
    }
    let file = File::open(archive).map_err(|_| Error::Invalid)?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut bytes: Option<Vec<u8>> = None;
    for entry in archive.entries().map_err(|_| Error::Invalid)? {
        let entry = entry.map_err(|_| Error::Invalid)?;
        if entry.path().map_err(|_| Error::Invalid)? != Path::new("manifest.json") {
            continue;
        }
        if bytes.is_some() || entry.size() > MAX_MANIFEST {
            return Err(Error::Invalid);
        }
        let mut content = Vec::new();
        entry
            .take(MAX_MANIFEST + 1)
            .read_to_end(&mut content)
            .map_err(|_| Error::Invalid)?;
        if content.len() as u64 > MAX_MANIFEST {
            return Err(Error::TooLarge);
        }
        bytes = Some(content);
    }
    let manifest: Manifest =
        serde_json::from_slice(&bytes.ok_or(Error::Invalid)?).map_err(|_| Error::Invalid)?;
    check(&manifest)?;
    Ok(manifest)
}

/// Unpack a package archive into `staging` and validate its manifest.
pub(crate) fn extract(archive: &Path, staging: &Path) -> Result<Package, Error> {
    let size = fs::metadata(archive).map_err(|_| Error::Invalid)?.len();
    if size > MAX_PACKAGE {
        return Err(Error::TooLarge);
    }
    let file = File::open(archive).map_err(|_| Error::Invalid)?;
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(file));
    let mut total = 0_u64;
    let mut seen = BTreeSet::new();
    for entry in archive.entries().map_err(|_| Error::Invalid)? {
        let mut entry = entry.map_err(|_| Error::Invalid)?;
        let path = entry.path().map_err(|_| Error::Invalid)?.into_owned();
        if !valid_path(&path) {
            return Err(Error::Invalid);
        }
        if !seen.insert(path.clone()) || seen.len() > MAX_ENTRIES {
            return Err(Error::Invalid);
        }
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err(Error::Invalid);
        }
        total = total
            .checked_add(entry.size())
            .filter(|total| *total <= MAX_UNPACKED)
            .ok_or(Error::TooLarge)?;
        let output = staging.join(&path);
        if kind.is_dir() {
            fs::create_dir_all(&output).map_err(|_| Error::Storage)?;
            continue;
        }
        let parent = output.parent().ok_or(Error::Invalid)?;
        fs::create_dir_all(parent).map_err(|_| Error::Storage)?;
        let mut target = File::options()
            .create_new(true)
            .write(true)
            .open(&output)
            .map_err(|_| Error::Storage)?;
        std::io::copy(&mut entry, &mut target).map_err(|_| Error::Storage)?;
    }
    validate(staging)
}

fn validate(staging: &Path) -> Result<Package, Error> {
    let manifest_path = staging.join("manifest.json");
    let mut bytes = Vec::new();
    File::open(&manifest_path)
        .map_err(|_| Error::Invalid)?
        .take(MAX_MANIFEST + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Invalid)?;
    if bytes.len() as u64 > MAX_MANIFEST {
        return Err(Error::TooLarge);
    }
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
    check(&manifest)?;
    let target = current_target();
    let name = manifest.entry_name().ok_or(Error::UnsupportedTarget)?;
    let entry = Path::new("binaries").join(target).join(name);
    let entry_path = staging.join(&entry);
    if !entry_path.is_file() {
        return Err(Error::Invalid);
    }
    Ok(Package {
        manifest,
        directory: staging.to_owned(),
        entry,
    })
}

fn check(manifest: &Manifest) -> Result<(), Error> {
    if !manifest.is_valid(&SYSTEM_MODULE_IDS) {
        return Err(Error::Invalid);
    }
    if manifest.min_app_version > app_version() {
        return Err(Error::Incompatible);
    }
    manifest.entry_name().ok_or(Error::UnsupportedTarget)?;
    Ok(())
}

fn app_version() -> semver::Version {
    semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or(semver::Version::new(1, 0, 0))
}

fn valid_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && path.components().count() <= 16
}

#[cfg(test)]
pub(in crate::infrastructure::modules) mod testing;

#[cfg(test)]
mod tests;
