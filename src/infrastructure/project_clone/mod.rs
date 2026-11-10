//! Clone explicit remote URLs into newly reserved directories.

use super::process;
use std::path::{Path, PathBuf};

pub(crate) use crate::domain::git::CloneError as Error;

pub(crate) fn repository_name(remote: &str) -> Option<String> {
    let remote = remote.trim();
    if remote.is_empty() || remote.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let path = if remote.starts_with("https://") || remote.starts_with("ssh://") {
        let url = url::Url::parse(remote).ok()?;
        if url.host_str().is_none()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.password().is_some()
            || (url.scheme() == "https" && !url.username().is_empty())
        {
            return None;
        }
        url.path().to_owned()
    } else {
        let (host, path) = remote.split_once(':')?;
        let (user, host) = host.split_once('@')?;
        if user.is_empty()
            || host.is_empty()
            || host.starts_with('-')
            || !host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-'))
            || !user
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        {
            return None;
        }
        path.to_owned()
    };
    let name = path.trim_end_matches('/').rsplit('/').next()?;
    let name = name.strip_suffix(".git").unwrap_or(name);
    if name.is_empty()
        || name == "."
        || name == ".."
        || name
            .chars()
            .any(|c| matches!(c, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control())
    {
        return None;
    }
    Some(name.to_owned())
}

pub(crate) fn default_parent() -> PathBuf {
    if let Some(path) = std::env::var_os("IWE_DATA_DIR") {
        return PathBuf::from(path).join("IWE Code");
    }
    directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().join("IWE Code"))
        .unwrap_or_default()
}

pub(crate) async fn choose_parent(initial: PathBuf) -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_directory(initial)
        .pick_folder()
        .await
        .map(|folder| folder.path().to_owned())
}

pub(crate) async fn clone(remote: String, destination: PathBuf) -> Result<PathBuf, Error> {
    repository_name(&remote).ok_or(Error::InvalidRemote)?;
    clone_into(remote.trim(), &destination).await
}

async fn clone_into(remote: &str, destination: &Path) -> Result<PathBuf, Error> {
    if !destination.is_absolute() || destination.file_name().is_none() {
        return Err(Error::Destination);
    }
    let parent = destination.parent().ok_or(Error::Destination)?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|_| Error::Destination)?;
    // Reserve only a new directory. Existing directories and links remain untouched.
    tokio::fs::create_dir(destination).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            Error::Exists
        } else {
            Error::Destination
        }
    })?;
    let target = destination.to_str().ok_or(Error::Destination)?;
    process::run_with_timeout(
        "git",
        &[
            "-c",
            "protocol.allow=never",
            "-c",
            "protocol.https.allow=always",
            "-c",
            "protocol.ssh.allow=always",
            "clone",
            "--quiet",
            "--",
            remote,
            target,
        ],
        None,
        std::time::Duration::from_secs(180),
    )
    .await
    .map_err(|_| Error::Failed)?;
    tokio::fs::canonicalize(destination)
        .await
        .map_err(|_| Error::Destination)
}

#[cfg(test)]
mod tests;
