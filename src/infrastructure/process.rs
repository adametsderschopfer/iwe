//! Run bounded, noninteractive subprocesses away from the UI thread.

use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command};

pub(crate) use crate::domain::Failure;

pub(crate) fn executable(name: &str) -> PathBuf {
    #[cfg(target_os = "windows")]
    if name == "git"
        && let Some(candidate) = windows_git()
    {
        return candidate;
    }
    #[cfg(target_os = "macos")]
    for prefix in ["/opt/homebrew/bin", "/usr/local/bin"] {
        let candidate = Path::new(prefix).join(name);
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from(name)
}

#[cfg(target_os = "windows")]
fn windows_git() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            if directory.as_os_str().is_empty() {
                continue;
            }
            let candidate = directory.join("git.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let mut roots = Vec::new();
    for variable in ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(directory) = std::env::var_os(variable)
            && !directory.is_empty()
        {
            roots.push(PathBuf::from(directory).join("Git"));
        }
    }
    if let Some(directory) = std::env::var_os("LOCALAPPDATA")
        && !directory.is_empty()
    {
        roots.push(PathBuf::from(directory).join("Programs").join("Git"));
    }
    for root in roots {
        for folder in ["cmd", "bin"] {
            let candidate = root.join(folder).join("git.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

pub(crate) async fn run(name: &str, args: &[&str], cwd: Option<&Path>) -> Result<Vec<u8>, Failure> {
    run_with_timeout(name, args, cwd, Duration::from_secs(25)).await
}

pub(crate) async fn read_range(
    name: &str, args: &[&str], cwd: &Path, offset: u64, length: usize,
) -> Result<Vec<u8>, Failure> {
    if length > super::workspace::pages::READ_BYTES { return Err(Failure::TooLarge); }
    let mut command = Command::new(executable(name));
    command.args(args).current_dir(cwd)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null())
        .env("GIT_TERMINAL_PROMPT", "0").env("GIT_OPTIONAL_LOCKS", "0")
        .kill_on_drop(true);
    #[cfg(target_os = "windows")]
    command.creation_flags(0x08000000);
    let operation = async {
        let mut child = command.spawn().map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound { Failure::MissingTool }
            else { Failure::Unavailable }
        })?;
        let mut stdout = child.stdout.take().ok_or(Failure::Read)?;
        let skipped = tokio::io::copy(&mut (&mut stdout).take(offset), &mut tokio::io::sink())
            .await.map_err(|_| Failure::Read)?;
        if skipped != offset { return Err(Failure::Read); }
        let mut bytes = Vec::with_capacity(length);
        (&mut stdout).take(length as u64).read_to_end(&mut bytes)
            .await.map_err(|_| Failure::Read)?;
        // Stop the producer after the requested range instead of buffering the remainder.
        let _ = child.kill().await;
        let _ = child.wait().await;
        if bytes.len() != length { return Err(Failure::Read); }
        Ok(bytes)
    };
    tokio::time::timeout(Duration::from_secs(25), operation).await
        .map_err(|_| Failure::Unavailable)?
}

pub(crate) async fn run_with_timeout(
    name: &str,
    args: &[&str],
    cwd: Option<&Path>,
    timeout: Duration,
) -> Result<Vec<u8>, Failure> {
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
    tokio::time::timeout(timeout, operation)
        .await
        .map_err(|_| Failure::Unavailable)?
}
