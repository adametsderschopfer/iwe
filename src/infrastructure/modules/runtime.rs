//! Run one external module process and exchange protocol lines.

use super::protocol::{self, HostMessage, ModuleMessage};
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

enum Control {
    Line(String),
    Close,
}

/// One running external module process.
pub(crate) struct Session {
    controls: mpsc::SyncSender<Control>,
    output: tokio::sync::Mutex<tokio::sync::mpsc::Receiver<ModuleMessage>>,
    stop: mpsc::Sender<()>,
}

impl Session {
    /// Spawn a module process and send the initialize message.
    pub(crate) fn start(executable: &Path, initialize: &HostMessage) -> Result<Arc<Self>, ()> {
        let mut child = Command::new(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| ())?;
        let mut stdin = child.stdin.take().ok_or(())?;
        let stdout = child.stdout.take().ok_or(())?;

        let (controls, requests) = mpsc::sync_channel(64);
        let shutdown_writer = controls.clone();
        let (stop, stopped) = mpsc::channel();
        thread::Builder::new()
            .name("module-lifecycle".into())
            .spawn(move || {
                loop {
                    match stopped.recv_timeout(Duration::from_millis(100)) {
                        Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            let _ = child.kill();
                            let _ = child.wait();
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => match child.try_wait() {
                            Ok(None) => {}
                            _ => break,
                        },
                    }
                }
                // Close stdin so the writer and reader threads finish.
                let _ = shutdown_writer.send(Control::Close);
            })
            .map_err(|_| ())?;

        let first = protocol::encode(initialize);
        thread::Builder::new()
            .name("module-writer".into())
            .spawn(move || {
                let mut alive = write_line(&mut stdin, &first);
                while alive.is_ok() {
                    match requests.recv() {
                        Ok(Control::Line(line)) => alive = write_line(&mut stdin, &line),
                        Ok(Control::Close) | Err(_) => break,
                    }
                }
                // Dropping stdin closes the pipe and ends the module.
            })
            .map_err(|_| ())?;

        let (events, output) = tokio::sync::mpsc::channel(64);
        thread::Builder::new()
            .name("module-reader".into())
            .spawn(move || {
                for line in BufReader::new(stdout).lines() {
                    let Ok(line) = line else {
                        break;
                    };
                    let Some(message) = protocol::parse(&line) else {
                        continue;
                    };
                    if events.blocking_send(message).is_err() {
                        break;
                    }
                }
                let _ = events.blocking_send(ModuleMessage::Exited);
            })
            .map_err(|_| ())?;

        Ok(Arc::new(Self {
            controls,
            output: tokio::sync::Mutex::new(output),
            stop,
        }))
    }

    /// Receive the next module message, or `None` when the stream ended.
    pub(crate) async fn next(&self) -> Option<ModuleMessage> {
        self.output.lock().await.recv().await
    }

    /// Send a protocol message to the module process.
    pub(crate) fn send(&self, message: &HostMessage) {
        let _ = self.controls.send(Control::Line(protocol::encode(message)));
    }

    /// Ask the module to exit and close its input.
    pub(crate) fn shutdown(&self) {
        self.send(&HostMessage::Shutdown);
        let _ = self.controls.send(Control::Close);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

fn write_line<W: Write>(writer: &mut W, line: &str) -> std::io::Result<()> {
    writer.write_all(line.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}
