//! Store secrets in private files outside workspace metadata.

use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::{
    fs,
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

pub(crate) fn path(group: &str, key: &str) -> io::Result<PathBuf> {
    let root = std::env::var_os("IWE_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            directories::ProjectDirs::from("", "", "iwe")
                .map(|dirs| dirs.data_local_dir().to_owned())
        })
        .ok_or(ErrorKind::NotFound)?;
    let hash = Sha256::digest(key.as_bytes());
    Ok(root.join("secrets").join(format!("{group}-{hash:x}")))
}

fn parent(path: &Path) -> io::Result<&Path> {
    path.parent().ok_or_else(|| ErrorKind::InvalidInput.into())
}

fn check_directory(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() {
        return Err(ErrorKind::InvalidData.into());
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o777 != 0o700 {
        return Err(ErrorKind::PermissionDenied.into());
    }
    Ok(())
}

pub(crate) fn read(path: &Path) -> io::Result<Option<Zeroizing<Vec<u8>>>> {
    let metadata = match check_directory(parent(path)?).and_then(|()| fs::symlink_metadata(path)) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() {
        return Err(ErrorKind::InvalidData.into());
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o777 != 0o600 {
        return Err(ErrorKind::PermissionDenied.into());
    }
    match fs::read(path) {
        Ok(bytes) => Ok(Some(Zeroizing::new(bytes))),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn write(path: &Path, value: &[u8]) -> io::Result<()> {
    let directory = parent(path)?;
    let root = parent(directory)?;
    fs::create_dir_all(root)?;
    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let builder = {
        let mut builder = builder;
        builder.mode(0o700);
        builder
    };
    match builder.create(directory) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    check_directory(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    #[cfg(unix)]
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(value)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

pub(crate) fn delete(path: &Path) -> io::Result<()> {
    match check_directory(parent(path)?).and_then(|()| fs::remove_file(path)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests;
