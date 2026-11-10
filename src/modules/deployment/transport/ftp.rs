use super::{Entry, Progress, Secret, Transport, child_path, copy, socket};
use crate::domain::deployment::{Connection, Error, Protocol};
use std::{
    io::{Read, Write},
    sync::Arc,
};
use suppaftp::{RustlsConnector, RustlsFtpStream, list::File, rustls};

pub(super) struct Ftp {
    stream: RustlsFtpStream,
}

#[derive(Debug)]
struct TimedConnector(RustlsConnector);
impl suppaftp::TlsConnector for TimedConnector {
    type Stream = <RustlsConnector as suppaftp::TlsConnector>::Stream;
    fn connect(
        &self,
        domain: &str,
        stream: std::net::TcpStream,
    ) -> suppaftp::FtpResult<Self::Stream> {
        stream
            .set_read_timeout(Some(super::TIMEOUT))
            .map_err(suppaftp::FtpError::ConnectionError)?;
        stream
            .set_write_timeout(Some(super::TIMEOUT))
            .map_err(suppaftp::FtpError::ConnectionError)?;
        self.0.connect(domain, stream)
    }
}

impl Ftp {
    pub(super) fn connect(connection: &Connection, secret: &Secret) -> Result<Self, Error> {
        let roots =
            rustls::RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        Self::connect_with_roots(connection, secret, roots)
    }

    pub(super) fn connect_with_roots(
        connection: &Connection,
        secret: &Secret,
        roots: rustls::RootCertStore,
    ) -> Result<Self, Error> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .map_err(|_| Error::Connection)?
            .with_root_certificates(roots)
            .with_no_client_auth();
        let tls = TimedConnector(RustlsConnector::from(Arc::new(config)));
        let mut stream = if connection.protocol == Protocol::FtpsImplicit {
            RustlsFtpStream::connect_secure_implicit(
                (connection.host.as_str(), connection.port),
                tls,
                &connection.host,
            )
            .map_err(|_| Error::Connection)?
        } else {
            let stream =
                RustlsFtpStream::connect_with_stream(socket(&connection.host, connection.port)?)
                    .map_err(|_| Error::Connection)?;
            if connection.protocol == Protocol::Ftps {
                stream
                    .into_secure(tls, &connection.host)
                    .map_err(|_| Error::Connection)?
            } else {
                stream
            }
        };
        stream
            .get_ref()
            .set_read_timeout(Some(super::TIMEOUT))
            .map_err(|_| Error::Connection)?;
        stream
            .get_ref()
            .set_write_timeout(Some(super::TIMEOUT))
            .map_err(|_| Error::Connection)?;
        stream = stream.passive_stream_builder(|address| {
            let stream = std::net::TcpStream::connect_timeout(&address, super::TIMEOUT)
                .map_err(suppaftp::FtpError::ConnectionError)?;
            stream
                .set_read_timeout(Some(super::TIMEOUT))
                .map_err(suppaftp::FtpError::ConnectionError)?;
            stream
                .set_write_timeout(Some(super::TIMEOUT))
                .map_err(suppaftp::FtpError::ConnectionError)?;
            Ok(stream)
        });
        stream.set_passive_nat_workaround(true);
        stream
            .login(connection.username.as_str(), secret.0.as_str())
            .map_err(|_| Error::Credentials)?;
        stream
            .transfer_type(suppaftp::types::FileType::Binary)
            .map_err(|_| Error::Connection)?;
        Ok(Self { stream })
    }
}

impl Transport for Ftp {
    fn list(&mut self, path: &str) -> Result<Vec<Entry>, Error> {
        // LIST preserves symbolic-link information that some MLSD servers resolve away.
        let lines = self.stream.list(Some(path)).map_err(|_| Error::Access)?;
        if lines.len() > super::ENTRY_LIMIT {
            return Err(Error::TooLarge);
        }
        let mut entries = Vec::new();
        for line in lines {
            let file = File::try_from(line.as_str()).map_err(|_| Error::Unsupported)?;
            if matches!(file.name(), "." | "..") {
                continue;
            }
            entries.push(Entry {
                path: child_path(path, file.name())?,
                name: file.name().to_owned(),
                directory: file.is_directory(),
                symlink: !file.is_directory() && !file.is_file(),
                size: file.size() as u64,
            });
        }
        Ok(entries)
    }
    fn stat(&mut self, path: &str) -> Result<Option<Entry>, Error> {
        if path == "/" {
            return Ok(Some(Entry {
                path: path.to_owned(),
                name: String::new(),
                directory: true,
                symlink: false,
                size: 0,
            }));
        }
        let (parent, _) = path.rsplit_once('/').ok_or(Error::Mapping)?;
        Ok(self
            .list(if parent.is_empty() { "/" } else { parent })?
            .into_iter()
            .find(|entry| entry.path == path))
    }
    fn mkdir(&mut self, path: &str) -> Result<(), Error> {
        self.stream.mkdir(path).map_err(|_| Error::Access)
    }
    fn read(
        &mut self,
        path: &str,
        output: &mut dyn Write,
        progress: &Progress,
    ) -> Result<(), Error> {
        let mut stream = self
            .stream
            .retr_as_stream(path)
            .map_err(|_| Error::Access)?;
        copy(&mut stream, output, progress)?;
        stream.finish().map_err(|_| Error::Connection)
    }
    fn write(
        &mut self,
        path: &str,
        input: &mut dyn Read,
        progress: &Progress,
    ) -> Result<(), Error> {
        let mut stream = self
            .stream
            .put_with_stream(path)
            .map_err(|_| Error::Access)?;
        copy(input, &mut stream, progress)?;
        stream.finish().map_err(|_| Error::Connection)
    }
}
