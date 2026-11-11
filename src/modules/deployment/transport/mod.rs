//! Isolate network access behind a streaming, synchronous transport boundary.

mod ftp;
mod sftp;

use crate::domain::deployment::{
    Authentication, Connection, Connections, Error, Protocol, safe_text,
};
use crate::infrastructure::secret_files;
use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::Arc,
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use zeroize::Zeroizing;

pub(crate) const TIMEOUT: Duration = Duration::from_secs(15);
pub(crate) const ENTRY_LIMIT: usize = 10_000;

#[derive(Clone, Default)]
pub(crate) struct Secret(pub Zeroizing<String>);
impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub path: String,
    pub name: String,
    pub directory: bool,
    pub symlink: bool,
    pub size: u64,
}

pub(crate) trait Transport: Send {
    fn list(&mut self, path: &str) -> Result<Vec<Entry>, Error>;
    fn stat(&mut self, path: &str) -> Result<Option<Entry>, Error>;
    fn mkdir(&mut self, path: &str) -> Result<(), Error>;
    fn read(
        &mut self,
        path: &str,
        output: &mut dyn Write,
        progress: &Progress,
    ) -> Result<(), Error>;
    fn write(&mut self, path: &str, input: &mut dyn Read, progress: &Progress)
    -> Result<(), Error>;
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Progress {
    pub cancel: CancellationToken,
    pub bytes: Arc<std::sync::atomic::AtomicU64>,
    pub files: Arc<std::sync::atomic::AtomicU64>,
}

impl Progress {
    pub(crate) fn check(&self) -> Result<(), Error> {
        if self.cancel.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

pub(crate) fn copy(
    input: &mut dyn Read,
    output: &mut dyn Write,
    progress: &Progress,
) -> Result<(), Error> {
    let mut buffer = [0; 64 * 1024];
    loop {
        progress.check()?;
        let count = input.read(&mut buffer).map_err(|_| Error::Connection)?;
        if count == 0 {
            break;
        }
        output
            .write_all(&buffer[..count])
            .map_err(|_| Error::Access)?;
        progress
            .bytes
            .fetch_add(count as u64, std::sync::atomic::Ordering::Relaxed);
    }
    output.flush().map_err(|_| Error::Access)
}

pub(crate) fn socket(host: &str, port: u16) -> Result<TcpStream, Error> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|_| Error::Connection)?;
    for address in addresses.take(4) {
        if let Ok(stream) = TcpStream::connect_timeout(&address, TIMEOUT) {
            stream
                .set_read_timeout(Some(TIMEOUT))
                .map_err(|_| Error::Connection)?;
            stream
                .set_write_timeout(Some(TIMEOUT))
                .map_err(|_| Error::Connection)?;
            return Ok(stream);
        }
    }
    Err(Error::Connection)
}

pub(crate) fn secret_id(connection: &Connection) -> &str {
    match &connection.authentication {
        Authentication::Key(id) => id,
        _ => &connection.id,
    }
}

pub(crate) fn store_secret(id: &str, secret: &Secret) -> Result<(), Error> {
    let path = secret_files::path("deployment", id).map_err(|_| Error::Credentials)?;
    secret_files::write(&path, secret.0.as_bytes()).map_err(|_| Error::Credentials)
}

pub(crate) fn read_secret(id: &str) -> Result<Secret, Error> {
    let path = secret_files::path("deployment", id).map_err(|_| Error::Credentials)?;
    let Some(bytes) = secret_files::read(&path).map_err(|_| Error::Credentials)? else {
        return Ok(Secret::default());
    };
    let text = std::str::from_utf8(&bytes).map_err(|_| Error::Credentials)?;
    Ok(Secret(Zeroizing::new(text.to_owned())))
}

pub(crate) fn connect(
    connection: &Connection,
    settings: &Connections,
    supplied: Option<&Secret>,
) -> Result<Box<dyn Transport>, Error> {
    connection.validate()?;
    let secret = if let Some(secret) = supplied {
        secret.clone()
    } else if connection.authentication == Authentication::Agent {
        Secret::default()
    } else {
        read_secret(secret_id(connection))?
    };
    match connection.protocol {
        Protocol::Sftp => Ok(Box::new(sftp::Sftp::connect(
            connection, settings, &secret,
        )?)),
        _ => Ok(Box::new(ftp::Ftp::connect(connection, &secret)?)),
    }
}

pub(crate) fn child_path(parent: &str, name: &str) -> Result<String, Error> {
    if name.is_empty()
        || matches!(name, "." | "..")
        || name.contains(['/', '\\'])
        || !safe_text(name)
    {
        return Err(Error::Mapping);
    }
    crate::domain::deployment::remote_path(&format!("{parent}/{name}"))
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fixtures;
