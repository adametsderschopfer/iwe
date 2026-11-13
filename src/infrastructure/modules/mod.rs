//! Install external module packages and run their processes.

pub(crate) mod install;
pub(crate) mod package;
pub(crate) mod protocol;
pub(crate) mod registry;
pub(crate) mod runtime;

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    /// The package or its manifest is malformed.
    Invalid,
    /// The package exceeds the size limit.
    TooLarge,
    /// The package has no binary for this platform.
    UnsupportedTarget,
    /// The package needs a newer application or protocol version.
    Incompatible,
    /// An update must use a newer module version.
    OlderVersion,
    /// The stored registry cannot be read.
    Storage,
}

/// Root directory for installed module files.
pub(crate) fn modules_root() -> Result<PathBuf, Error> {
    data_root()
        .map(|root| root.join("modules"))
        .ok_or(Error::Storage)
}

fn data_root() -> Option<PathBuf> {
    std::env::var_os("IWE_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            directories::ProjectDirs::from("", "", "iwe")
                .map(|dirs| dirs.data_local_dir().to_owned())
        })
}

/// Installation directory for one module version.
pub(crate) fn module_directory(id: &str, version: &str) -> Result<PathBuf, Error> {
    if !valid_directory_name(id) || !valid_directory_name(version) {
        return Err(Error::Invalid);
    }
    Ok(modules_root()?.join(id).join(version))
}

fn valid_directory_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
        && !value.starts_with('.')
        && !value.contains("..")
}
