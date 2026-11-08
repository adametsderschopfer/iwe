//! Run bounded, noninteractive subprocesses away from the UI thread.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};

pub(crate) use crate::domain::Failure;

pub(crate) fn executable(name: &str) -> PathBuf {
    #[cfg(target_os = "macos")]
    for prefix in ["/opt/homebrew/bin", "/usr/local/bin"] {
        let candidate = Path::new(prefix).join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from(name)
}

pub(crate) async fn run(name: &str, args: &[&str], cwd: Option<&Path>) -> Result<Vec<u8>, Failure> {
    let mut command = Command::new(executable(name));
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("GH_PROMPT_DISABLED", "1")
        .env("NO_COLOR", "1")
        .env("GLAB_SEND_TELEMETRY", "false")
        .kill_on_drop(true);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000);
    let operation = async {
        let mut child = command.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Failure::MissingTool
            } else {
                Failure::Unavailable
            }
        })?;
        let mut output = Vec::new();
        let mut errors = Vec::new();
        let stdout = child.stdout.take().ok_or(Failure::Unavailable)?;
        let stderr = child.stderr.take().ok_or(Failure::Unavailable)?;
        let (out, err) = tokio::join!(
            async {
                stdout
                    .take(4 * 1024 * 1024 + 1)
                    .read_to_end(&mut output)
                    .await
            },
            async { stderr.take(64 * 1024 + 1).read_to_end(&mut errors).await },
        );
        out.map_err(|_| Failure::Read)?;
        err.map_err(|_| Failure::Read)?;
        if output.len() > 4 * 1024 * 1024 || errors.len() > 64 * 1024 {
            return Err(Failure::TooLarge);
        }
        let status = child.wait().await.map_err(|_| Failure::Unavailable)?;
        if status.success() {
            return Ok(output);
        }
        // Never log CLI output: it can contain credentials or private repository data.
        eprintln!("iwe: {name} exited with {status}");
        let message = String::from_utf8_lossy(&errors).to_ascii_lowercase();
        if message.contains("401")
            || message.contains("403")
            || message.contains("404")
            || message.contains("auth login")
            || message.contains("authentication")
        {
            Err(Failure::Access)
        } else {
            Err(Failure::Unavailable)
        }
    };
    tokio::time::timeout(Duration::from_secs(25), operation)
        .await
        .map_err(|_| Failure::Unavailable)?
}
