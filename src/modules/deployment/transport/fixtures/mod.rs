//! Serve isolated protocol fixtures on loopback sockets without user credentials.

#[cfg(unix)]
pub(super) mod agent;
mod ssh;
pub(super) use ssh::Ssh;

use super::*;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    thread,
};
use suppaftp::rustls;

type Files = Arc<Mutex<BTreeMap<String, Vec<u8>>>>;
pub(super) struct Ftp {
    pub port: u16,
    pub roots: rustls::RootCertStore,
    pub files: Files,
    worker: Option<thread::JoinHandle<()>>,
}

enum Wire {
    Plain(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ServerConnection, TcpStream>>),
}
impl Read for Wire {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(bytes),
            Self::Tls(s) => s.read(bytes),
        }
    }
}
impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(bytes),
            Self::Tls(s) => s.write(bytes),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}
fn secure(socket: TcpStream, config: Arc<rustls::ServerConfig>) -> Wire {
    Wire::Tls(Box::new(rustls::StreamOwned::new(
        rustls::ServerConnection::new(config).unwrap(),
        socket,
    )))
}
fn reply(reader: &mut BufReader<Wire>, value: &str) -> std::io::Result<()> {
    write!(reader.get_mut(), "{value}\r\n")?;
    reader.get_mut().flush()
}

impl Ftp {
    pub(super) fn start(protocol: Protocol) -> Self {
        let certificate =
            rcgen::generate_simple_self_signed(vec!["localhost".into(), "127.0.0.1".into()])
                .unwrap();
        let mut roots = rustls::RootCertStore::empty();
        roots.add(certificate.cert.der().clone()).unwrap();
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![certificate.cert.der().clone()],
            rustls::pki_types::PrivatePkcs8KeyDer::from(certificate.signing_key.serialize_der())
                .into(),
        )
        .unwrap();
        let config = Arc::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let files: Files = Arc::new(Mutex::new(BTreeMap::from([(
            "/hello.txt".into(),
            b"server\n".to_vec(),
        )])));
        let saved = Arc::clone(&files);
        let worker = thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let wire = if protocol == Protocol::FtpsImplicit {
                secure(socket, Arc::clone(&config))
            } else {
                Wire::Plain(socket)
            };
            let mut reader = BufReader::new(wire);
            if reply(&mut reader, "220 fixture").is_err() {
                return;
            }
            let mut passive = None;
            let mut encrypted_data = protocol == Protocol::FtpsImplicit;
            loop {
                let mut line = String::new();
                if !matches!(reader.read_line(&mut line), Ok(n) if n > 0) {
                    break;
                }
                let (command, argument) = line
                    .trim_end()
                    .split_once(' ')
                    .unwrap_or((line.trim_end(), ""));
                let result = match command {
                    "AUTH" => {
                        if reply(&mut reader, "234 proceed").is_err() {
                            break;
                        }
                        let Wire::Plain(socket) = reader.into_inner() else {
                            break;
                        };
                        reader = BufReader::new(secure(socket, Arc::clone(&config)));
                        Ok(())
                    }
                    "USER" => reply(&mut reader, "331 password"),
                    "PASS" => reply(
                        &mut reader,
                        if argument == "fixture" {
                            "230 signed in"
                        } else {
                            "530 denied"
                        },
                    ),
                    "PBSZ" | "TYPE" => reply(&mut reader, "200 ready"),
                    "PROT" => {
                        encrypted_data = argument == "P";
                        reply(&mut reader, "200 ready")
                    }
                    "PWD" => reply(&mut reader, "257 \"/\""),
                    "PASV" | "EPSV" => {
                        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                        let port = listener.local_addr().unwrap().port();
                        passive = Some(listener);
                        reply(
                            &mut reader,
                            &if command == "PASV" {
                                format!("227 passive (127,0,0,1,{},{})", port / 256, port % 256)
                            } else {
                                format!("229 passive (|||{port}|)")
                            },
                        )
                    }
                    "LIST" | "RETR" | "STOR" => {
                        if reply(&mut reader, "150 opening").is_err() {
                            break;
                        }
                        let Some(listener) = passive.take() else {
                            break;
                        };
                        let (socket, _) = listener.accept().unwrap();
                        socket
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        socket
                            .set_write_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut data = if encrypted_data {
                            secure(socket, Arc::clone(&config))
                        } else {
                            Wire::Plain(socket)
                        };
                        let result = match command {
                            "LIST" => {
                                let files = saved.lock().unwrap();
                                let listing = files
                                    .iter()
                                    .map(|(path, bytes)| {
                                        format!(
                                            "-rw-r--r-- 1 user group {} Jan 01 2026 {}\r\n",
                                            bytes.len(),
                                            path.trim_start_matches('/')
                                        )
                                    })
                                    .collect::<String>();
                                data.write_all(listing.as_bytes())
                            }
                            "RETR" => data.write_all(saved.lock().unwrap().get(argument).unwrap()),
                            _ => {
                                let mut bytes = Vec::new();
                                let result = data.read_to_end(&mut bytes).map(|_| ());
                                saved.lock().unwrap().insert(argument.into(), bytes);
                                result
                            }
                        };
                        if result.is_err() {
                            break;
                        }
                        let _ = data.flush();
                        if let Wire::Tls(stream) = &mut data {
                            stream.conn.send_close_notify();
                            let _ = stream.flush();
                        }
                        drop(data);
                        reply(&mut reader, "226 complete")
                    }
                    "QUIT" => {
                        let _ = reply(&mut reader, "221 bye");
                        break;
                    }
                    _ => reply(&mut reader, "502 unavailable"),
                };
                if result.is_err() {
                    break;
                }
            }
        });
        Self {
            port,
            roots,
            files,
            worker: Some(worker),
        }
    }
}
impl Drop for Ftp {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}
