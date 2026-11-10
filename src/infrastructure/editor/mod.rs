//! Save local snapshots without truncating the destination on write failure.

use crate::domain::editor::LIMIT;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    Access,
    Changed,
    TooLarge,
    Symlink,
}

pub(crate) fn save(path: &Path, expected: &str, text: &str) -> Result<(), Error> {
    if text.len() > LIMIT {
        return Err(Error::TooLarge);
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| Error::Access)?;
    if metadata.file_type().is_symlink() {
        return Err(Error::Symlink);
    }
    if !metadata.is_file() || metadata.permissions().readonly() {
        return Err(Error::Access);
    }
    let parent = path.parent().ok_or(Error::Access)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| Error::Access)?;
    temporary
        .write_all(text.as_bytes())
        .map_err(|_| Error::Access)?;
    temporary
        .as_file()
        .set_permissions(metadata.permissions())
        .map_err(|_| Error::Access)?;
    temporary.as_file().sync_all().map_err(|_| Error::Access)?;
    // Recheck after preparing the replacement. Keep the draft if another writer changed the file.
    let metadata = fs::symlink_metadata(path).map_err(|_| Error::Access)?;
    if metadata.file_type().is_symlink() {
        return Err(Error::Symlink);
    }
    let mut actual = Vec::new();
    fs::File::open(path)
        .map_err(|_| Error::Access)?
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut actual)
        .map_err(|_| Error::Access)?;
    if actual != expected.as_bytes() {
        return Err(Error::Changed);
    }
    temporary.persist(path).map_err(|_| Error::Access)?;
    Ok(())
}

#[cfg(test)]
mod tests;
