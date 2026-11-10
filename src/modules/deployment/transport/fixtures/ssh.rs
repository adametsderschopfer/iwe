use russh::{
    Channel, ChannelId,
    server::{self, Auth, ChannelOpenHandle, Msg, Session, VerifiedPublicKey},
};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode,
};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Mutex},
};

type Files = Arc<Mutex<BTreeMap<String, Vec<u8>>>>;
pub(crate) struct Ssh {
    pub port: u16,
    pub key: russh::keys::PrivateKey,
    pub files: Files,
    task: tokio::task::JoinHandle<()>,
}
impl Ssh {
    pub(crate) async fn start() -> Self {
        let key =
            russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519)
                .unwrap();
        let config = Arc::new(server::Config {
            keys: vec![key.clone()],
            auth_rejection_time: std::time::Duration::ZERO,
            auth_rejection_time_initial: Some(std::time::Duration::ZERO),
            ..Default::default()
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let files: Files = Arc::new(Mutex::new(BTreeMap::from([(
            "/hello.txt".into(),
            b"server\n".to_vec(),
        )])));
        let saved = Arc::clone(&files);
        let task = tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let config = Arc::clone(&config);
                let files = Arc::clone(&saved);
                tokio::spawn(async move {
                    if let Ok(session) = server::run_stream(
                        config,
                        socket,
                        SshSession {
                            channels: HashMap::new(),
                            files,
                        },
                    )
                    .await
                    {
                        let _ = session.await;
                    }
                });
            }
        });
        Self {
            port,
            key,
            files,
            task,
        }
    }
}
impl Drop for Ssh {
    fn drop(&mut self) {
        self.task.abort();
    }
}
struct SshSession {
    channels: HashMap<ChannelId, Channel<Msg>>,
    files: Files,
}
impl server::Handler for SshSession {
    type Error = russh::Error;
    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        Ok(if user == "user" && password == "fixture" {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }
    async fn auth_publickey(
        &mut self,
        _: &str,
        _: VerifiedPublicKey<'_>,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }
    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }
    async fn subsystem_request(
        &mut self,
        id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name == "sftp" {
            let channel = self.channels.remove(&id).unwrap();
            session.channel_success(id)?;
            let handler = FilesSession {
                files: Arc::clone(&self.files),
                listed: false,
            };
            tokio::spawn(async move {
                russh_sftp::server::run(channel.into_stream(), handler).await;
            });
        } else {
            session.channel_failure(id)?;
        }
        Ok(())
    }
}

struct FilesSession {
    files: Files,
    listed: bool,
}
fn ok(id: u32) -> Status {
    Status {
        id,
        status_code: StatusCode::Ok,
        error_message: String::new(),
        language_tag: String::new(),
    }
}
fn attrs(size: u64, directory: bool) -> FileAttributes {
    FileAttributes {
        size: Some(size),
        permissions: Some(if directory { 0o40755 } else { 0o100644 }),
        ..Default::default()
    }
}
impl russh_sftp::server::Handler for FilesSession {
    type Error = StatusCode;
    fn unimplemented(&self) -> StatusCode {
        StatusCode::OpUnsupported
    }
    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, StatusCode> {
        if path == "/" {
            return Ok(Attrs {
                id,
                attrs: attrs(0, true),
            });
        }
        let files = self.files.lock().unwrap();
        let bytes = files.get(&path).ok_or(StatusCode::NoSuchFile)?;
        Ok(Attrs {
            id,
            attrs: attrs(bytes.len() as u64, false),
        })
    }
    async fn open(
        &mut self,
        id: u32,
        filename: String,
        flags: OpenFlags,
        _: FileAttributes,
    ) -> Result<Handle, StatusCode> {
        if flags.contains(OpenFlags::CREATE) {
            self.files
                .lock()
                .unwrap()
                .insert(filename.clone(), Vec::new());
        }
        Ok(Handle {
            id,
            handle: filename,
        })
    }
    async fn close(&mut self, id: u32, _: String) -> Result<Status, StatusCode> {
        Ok(ok(id))
    }
    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, StatusCode> {
        let files = self.files.lock().unwrap();
        let bytes = files.get(&handle).ok_or(StatusCode::NoSuchFile)?;
        let offset = offset as usize;
        if offset >= bytes.len() {
            return Err(StatusCode::Eof);
        }
        Ok(Data {
            id,
            data: bytes[offset..bytes.len().min(offset + len as usize)].to_vec(),
        })
    }
    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, StatusCode> {
        let mut files = self.files.lock().unwrap();
        let bytes = files.entry(handle).or_default();
        let offset = offset as usize;
        bytes.resize(bytes.len().max(offset + data.len()), 0);
        bytes[offset..offset + data.len()].copy_from_slice(&data);
        Ok(ok(id))
    }
    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, StatusCode> {
        self.listed = false;
        Ok(Handle { id, handle: path })
    }
    async fn readdir(&mut self, id: u32, _: String) -> Result<Name, StatusCode> {
        if self.listed {
            return Err(StatusCode::Eof);
        }
        self.listed = true;
        Ok(Name {
            id,
            files: self
                .files
                .lock()
                .unwrap()
                .iter()
                .map(|(path, bytes)| {
                    File::new(
                        path.trim_start_matches('/'),
                        attrs(bytes.len() as u64, false),
                    )
                })
                .collect(),
        })
    }
}
