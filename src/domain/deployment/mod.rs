//! Define deployment settings and reversible path mappings without external access.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Protocol {
    #[default]
    Sftp,
    Ftp,
    Ftps,
    FtpsImplicit,
}

impl Protocol {
    pub(crate) fn port(self) -> u16 {
        match self {
            Self::Sftp => 22,
            Self::FtpsImplicit => 990,
            _ => 21,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Authentication {
    #[default]
    Password,
    Agent,
    Key(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Connection {
    pub id: String,
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub authentication: Authentication,
    #[serde(default)]
    pub trusted_host_key: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PrivateKey {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Connections {
    pub connections: Vec<Connection>,
    pub keys: Vec<PrivateKey>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Mapping {
    pub local: PathBuf,
    pub remote: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Profile {
    pub id: String,
    pub name: String,
    pub project: PathBuf,
    pub connection: String,
    pub root: String,
    pub mappings: Vec<Mapping>,
    pub excluded: Vec<PathBuf>,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Error {
    Configuration,
    Mapping,
    Excluded,
    Access,
    Connection,
    Credentials,
    HostUnknown(String),
    HostChanged,
    Cancelled,
    TooLarge,
    Binary,
    Overwrite,
    Changed,
    Unsupported,
}

pub(crate) fn safe_text(value: &str) -> bool {
    !value.chars().any(|c| c.is_control())
}

fn relative(path: &Path) -> bool {
    !path.is_absolute()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
        && path
            .to_str()
            .is_some_and(|p| safe_text(p) && !p.contains('\\') && !p.contains(':'))
}

pub(crate) fn remote_path(value: &str) -> Result<String, Error> {
    if !value.starts_with('/')
        || !safe_text(value)
        || value.contains('\\')
        || value.split('/').any(|s| s == "..")
    {
        return Err(Error::Mapping);
    }
    Ok(format!(
        "/{}",
        value
            .split('/')
            .filter(|s| !s.is_empty() && *s != ".")
            .collect::<Vec<_>>()
            .join("/")
    ))
}

pub(crate) fn contains(root: &str, path: &str) -> bool {
    root == "/"
        || path == root
        || path
            .strip_prefix(root)
            .is_some_and(|tail| tail.starts_with('/'))
}

impl Connection {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.name.trim().is_empty()
            || self.host.trim().is_empty()
            || self.port == 0
            || self.username.is_empty()
            || ![&self.name, &self.host, &self.username]
                .into_iter()
                .all(|s| safe_text(s))
        {
            return Err(Error::Configuration);
        }
        Ok(())
    }
}

impl Profile {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.name.trim().is_empty() || self.connection.is_empty() || !self.project.is_absolute()
        {
            return Err(Error::Configuration);
        }
        if self.mappings.is_empty() {
            return Err(Error::Mapping);
        }
        remote_path(&self.root)?;
        for (index, mapping) in self.mappings.iter().enumerate() {
            if !relative(&mapping.local) || !relative(Path::new(&mapping.remote)) {
                return Err(Error::Mapping);
            }
            for previous in &self.mappings[..index] {
                let local = self.project.join(&mapping.local);
                let other = self.project.join(&previous.local);
                let remote = self.mapping_root(mapping)?;
                let previous_remote = self.mapping_root(previous)?;
                if local.starts_with(&other)
                    || other.starts_with(&local)
                    || contains(&remote, &previous_remote)
                    || contains(&previous_remote, &remote)
                {
                    return Err(Error::Mapping);
                }
            }
        }
        if self.excluded.iter().any(|path| !relative(path)) {
            return Err(Error::Mapping);
        }
        Ok(())
    }

    pub(crate) fn mapping_root(&self, mapping: &Mapping) -> Result<String, Error> {
        remote_path(&format!("{}/{}", self.root, mapping.remote))
    }

    pub(crate) fn check_local(&self, local: &Path) -> Result<(), Error> {
        let path = local
            .strip_prefix(&self.project)
            .map_err(|_| Error::Mapping)?;
        if !relative(path) {
            return Err(Error::Mapping);
        }
        if path.components().any(|s| s.as_os_str() == ".git")
            || self
                .excluded
                .iter()
                .any(|excluded| path.starts_with(excluded))
        {
            return Err(Error::Excluded);
        }
        Ok(())
    }

    pub(crate) fn to_remote(&self, local: &Path) -> Result<String, Error> {
        self.validate()?;
        self.check_local(local)?;
        for mapping in &self.mappings {
            if let Ok(tail) = local.strip_prefix(self.project.join(&mapping.local)) {
                let tail = tail
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                return remote_path(&format!("{}/{tail}", self.mapping_root(mapping)?));
            }
        }
        Err(Error::Mapping)
    }

    pub(crate) fn to_local(&self, remote: &str) -> Result<PathBuf, Error> {
        self.validate()?;
        let remote = remote_path(remote)?;
        for mapping in &self.mappings {
            let root = self.mapping_root(mapping)?;
            if contains(&root, &remote) {
                let local = self
                    .project
                    .join(&mapping.local)
                    .join(remote[root.len()..].trim_start_matches('/'));
                self.check_local(&local)?;
                return Ok(local);
            }
        }
        Err(Error::Mapping)
    }
}

#[cfg(test)]
mod tests;
