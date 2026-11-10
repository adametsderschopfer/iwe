use super::{Entry, Progress, Secret, Transport, child_path, copy, socket};
use crate::domain::deployment::{Authentication, Connection, Connections, Error};
use base64::Engine;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::Path,
};

pub(super) struct Sftp {
    _session: ssh2::Session,
    files: ssh2::Sftp,
}

impl Sftp {
    pub(super) fn connect(
        connection: &Connection,
        settings: &Connections,
        secret: &Secret,
    ) -> Result<Self, Error> {
        Self::connect_with_agent(connection, settings, secret, None)
    }

    pub(super) fn connect_with_agent(
        connection: &Connection,
        settings: &Connections,
        secret: &Secret,
        agent_path: Option<&Path>,
    ) -> Result<Self, Error> {
        let mut session = ssh2::Session::new().map_err(|_| Error::Connection)?;
        session.set_tcp_stream(socket(&connection.host, connection.port)?);
        session.set_timeout(super::TIMEOUT.as_millis() as u32);
        session.handshake().map_err(|_| Error::Connection)?;
        let (key, _) = session.host_key().ok_or(Error::Connection)?;
        let fingerprint = format!(
            "SHA256:{}",
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(Sha256::digest(key))
        );
        match &connection.trusted_host_key {
            None => return Err(Error::HostUnknown(fingerprint)),
            Some(known) if *known != fingerprint => return Err(Error::HostChanged),
            _ => {}
        }
        match &connection.authentication {
            Authentication::Password => session.userauth_password(&connection.username, &secret.0),
            Authentication::Agent => {
                let mut agent = session.agent().map_err(|_| Error::Credentials)?;
                if let Some(path) = agent_path {
                    agent
                        .set_identity_path(path)
                        .map_err(|_| Error::Credentials)?;
                }
                agent.connect().map_err(|_| Error::Credentials)?;
                agent.list_identities().map_err(|_| Error::Credentials)?;
                let identities = agent.identities().map_err(|_| Error::Credentials)?;
                if !identities
                    .iter()
                    .any(|identity| agent.userauth(&connection.username, identity).is_ok())
                {
                    return Err(Error::Credentials);
                }
                Ok(())
            }
            Authentication::Key(id) => {
                let key = settings
                    .keys
                    .iter()
                    .find(|key| key.id == *id)
                    .ok_or(Error::Configuration)?;
                session.userauth_pubkey_file(
                    &connection.username,
                    None,
                    &key.path,
                    (!secret.0.is_empty()).then_some(secret.0.as_str()),
                )
            }
        }
        .map_err(|_| Error::Credentials)?;
        let files = session.sftp().map_err(|_| Error::Connection)?;
        Ok(Self {
            _session: session,
            files,
        })
    }
}

fn entry(path: String, stat: ssh2::FileStat) -> Entry {
    Entry {
        name: path.rsplit('/').next().unwrap_or("").to_owned(),
        path,
        directory: stat.is_dir(),
        symlink: !stat.is_file() && !stat.is_dir(),
        size: stat.size.unwrap_or(0),
    }
}

impl Transport for Sftp {
    fn list(&mut self, path: &str) -> Result<Vec<Entry>, Error> {
        let mut directory = self
            .files
            .opendir(Path::new(path))
            .map_err(|_| Error::Access)?;
        let mut entries = Vec::new();
        loop {
            match directory.readdir() {
                Ok((name, stat)) => {
                    let name = name.to_str().ok_or(Error::Unsupported)?;
                    if matches!(name, "." | "..") {
                        continue;
                    }
                    entries.push(entry(child_path(path, name)?, stat));
                    if entries.len() > super::ENTRY_LIMIT {
                        return Err(Error::TooLarge);
                    }
                }
                Err(error) if error.code() == ssh2::ErrorCode::Session(-16) => break,
                Err(_) => return Err(Error::Connection),
            }
        }
        Ok(entries)
    }
    fn stat(&mut self, path: &str) -> Result<Option<Entry>, Error> {
        match self.files.lstat(Path::new(path)) {
            Ok(stat) => Ok(Some(entry(path.to_owned(), stat))),
            Err(error) if error.code() == ssh2::ErrorCode::SFTP(2) => Ok(None),
            Err(_) => Err(Error::Access),
        }
    }
    fn mkdir(&mut self, path: &str) -> Result<(), Error> {
        self.files
            .mkdir(Path::new(path), 0o755)
            .map_err(|_| Error::Access)
    }
    fn read(
        &mut self,
        path: &str,
        output: &mut dyn Write,
        progress: &Progress,
    ) -> Result<(), Error> {
        let mut file = self
            .files
            .open(Path::new(path))
            .map_err(|_| Error::Access)?;
        copy(&mut file, output, progress)
    }
    fn write(
        &mut self,
        path: &str,
        input: &mut dyn Read,
        progress: &Progress,
    ) -> Result<(), Error> {
        let mut file = self
            .files
            .create(Path::new(path))
            .map_err(|_| Error::Access)?;
        copy(input, &mut file, progress)?;
        file.close().map_err(|_| Error::Connection)
    }
}
