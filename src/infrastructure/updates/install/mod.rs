//! Replace complete application bundles or standalone executables after shutdown.

use super::{Error, MAX_PACKAGE, PUBLIC_KEY, Release, platform, read_bounded, verified_release};
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Child, Command, ExitCode, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tempfile::TempDir;

#[derive(Debug)]
pub(crate) struct Prepared {
    pub version: String,
    directory: PathBuf,
    // Keep this lock until the application's event loop has returned.
    _lock: File,
    handed_off: AtomicBool,
}
impl Drop for Prepared {
    fn drop(&mut self) {
        if !self.handed_off.load(Ordering::Acquire) {
            let _ = self._lock.unlock();
            let _ = fs::remove_dir_all(&self.directory);
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Plan {
    target: PathBuf,
    tag: String,
}

pub(super) fn current_target() -> Result<PathBuf, Error> {
    let executable = std::env::current_exe()
        .map_err(|_| Error::Install)?
        .canonicalize()
        .map_err(|_| Error::Install)?;
    target_for(&executable, cfg!(target_os = "macos"))
}

fn target_for(executable: &Path, macos: bool) -> Result<PathBuf, Error> {
    if macos {
        let macos_dir = executable.parent().ok_or(Error::Unsupported)?;
        let contents = macos_dir.parent().ok_or(Error::Unsupported)?;
        let bundle = contents.parent().ok_or(Error::Unsupported)?;
        if macos_dir.file_name() != Some("MacOS".as_ref())
            || contents.file_name() != Some("Contents".as_ref())
            || bundle.extension() != Some("app".as_ref())
        {
            return Err(Error::Unsupported);
        }
        return Ok(bundle.to_owned());
    }
    Ok(executable.to_owned())
}

pub(super) fn stage_directory(target: &Path) -> Result<TempDir, Error> {
    tempfile::Builder::new()
        .prefix(".iwe-update-")
        .tempdir_in(target.parent().ok_or(Error::Install)?)
        .map_err(|_| Error::Install)
}

pub(super) fn prepare(
    stage: TempDir,
    target: PathBuf,
    release: Release,
) -> Result<Prepared, Error> {
    let root = payload_root(platform()?);
    extract(
        &stage.path().join("package.tar.gz"),
        &stage.path().join("new"),
        root,
    )?;
    let source = stage.path().join("new").join(root);
    validate_payload(&source, cfg!(target_os = "macos"))?;
    fs::write(stage.path().join("updates.json"), &release.manifest).map_err(|_| Error::Install)?;
    fs::write(stage.path().join("updates.sig"), &release.signature).map_err(|_| Error::Install)?;
    let plan = serde_json::to_vec(&Plan {
        target,
        tag: release.tag,
    })
    .map_err(|_| Error::Install)?;
    fs::write(stage.path().join("plan.json"), plan).map_err(|_| Error::Install)?;
    let lock = File::create(stage.path().join("running.lock")).map_err(|_| Error::Install)?;
    lock.lock().map_err(|_| Error::Install)?;
    Ok(Prepared {
        version: release.version,
        directory: stage.keep(),
        _lock: lock,
        handed_off: AtomicBool::new(false),
    })
}

pub(crate) fn start_install(prepared: &Prepared) -> Result<(), Error> {
    let helper = prepared.directory.join(if cfg!(windows) {
        "helper.exe"
    } else {
        "helper"
    });
    fs::copy(
        std::env::current_exe().map_err(|_| Error::Install)?,
        &helper,
    )
    .map_err(|_| Error::Install)?;
    let mut command = Command::new(helper);
    command
        .arg("--iwe-install-update")
        .arg(&prepared.directory)
        .env_remove("IWE_GRAPHICS_CHILD")
        .env_remove("WGPU_BACKEND")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().map_err(|_| Error::Install)?;
    if wait_for_helper(&mut child, &prepared.directory, Duration::from_secs(10)).is_err() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Error::Install);
    }
    prepared.handed_off.store(true, Ordering::Release);
    Ok(())
}

// Do not shut down the user's application until the helper accepts the handoff.
fn wait_for_helper(child: &mut Child, directory: &Path, timeout: Duration) -> Result<(), Error> {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().map_err(|_| Error::Install)?.is_some() {
            return Err(Error::Install);
        }
        if directory.join("helper.ready").is_file() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(Error::Install);
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

pub(crate) fn startup() -> Option<ExitCode> {
    let mut args = std::env::args_os().skip(1);
    let action = args.next()?;
    if action == "--iwe-install-update" {
        let directory = args.next().map(PathBuf::from)?;
        let result = helper(&directory);
        return Some(if result.is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }
    if action == "--iwe-update-cleanup"
        && let Some(directory) = args.next().map(PathBuf::from)
    {
        // A restarted launcher and its renderer may both attempt this cleanup.
        if let Ok(target) = current_target()
            && valid_stage(&directory, &target)
        {
            for _ in 0..50 {
                if fs::remove_dir_all(&directory).is_ok() || !directory.exists() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    None
}

fn valid_stage(directory: &Path, target: &Path) -> bool {
    directory.is_absolute()
        && directory.parent() == target.parent()
        && directory
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(".iwe-update-"))
        && fs::symlink_metadata(directory)
            .is_ok_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
}

fn helper_directory(own: &Path, directory: &Path) -> Result<PathBuf, Error> {
    let directory = directory.canonicalize().map_err(|_| Error::Install)?;
    let own = own.canonicalize().map_err(|_| Error::Install)?;
    if own.parent() != Some(directory.as_path()) {
        return Err(Error::Invalid);
    }
    Ok(directory)
}

fn helper(directory: &Path) -> Result<(), Error> {
    let own = std::env::current_exe().map_err(|_| Error::Install)?;
    let directory = helper_directory(&own, directory)?;
    let directory = directory.as_path();
    let plan: Plan = serde_json::from_slice(&read_bounded(directory.join("plan.json"), 16 * 1024)?)
        .map_err(|_| Error::Invalid)?;
    if !valid_stage(directory, &plan.target) || !super::newer(&plan.tag, env!("CARGO_PKG_VERSION"))?
    {
        return Err(Error::Invalid);
    }
    let lock = File::options()
        .read(true)
        .write(true)
        .open(directory.join("running.lock"))
        .map_err(|_| Error::Install)?;
    fs::write(directory.join("helper.ready"), b"ready").map_err(|_| Error::Install)?;
    let deadline = Instant::now() + Duration::from_secs(120);
    while lock.try_lock().is_err() {
        if Instant::now() >= deadline {
            return Err(Error::Install);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let result = apply(directory, &plan);
    if result.is_err() {
        let _ = restart(&plan.target, directory, true);
    }
    result
}

fn apply(directory: &Path, plan: &Plan) -> Result<(), Error> {
    let bytes = read_bounded(directory.join("updates.json"), 64 * 1024)?;
    let signature = read_bounded(directory.join("updates.sig"), 64)?;
    let release = verified_release(&bytes, &signature, &plan.tag, platform()?, PUBLIC_KEY)?;
    let archive = directory.join("package.tar.gz");
    verify_package(&archive, &release)?;
    // Extract again after shutdown so modified staged files cannot bypass verification.
    let unpacked = directory.join("verified");
    extract(&archive, &unpacked, payload_root(platform()?))?;
    let source = unpacked.join(payload_root(platform()?));
    validate_payload(&source, cfg!(target_os = "macos"))?;
    if fs::symlink_metadata(&plan.target)
        .map_err(|_| Error::Install)?
        .file_type()
        .is_symlink()
    {
        return Err(Error::Invalid);
    }
    let backup = directory.join("previous");
    replace(&source, &plan.target, &backup, || {
        restart(&plan.target, directory, false)
    })
}

fn verify_package(archive: &Path, release: &Release) -> Result<(), Error> {
    let mut file = File::open(archive).map_err(|_| Error::Install)?;
    if file.metadata().map_err(|_| Error::Install)?.len() != release.size {
        return Err(Error::Invalid);
    }
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|_| Error::Install)?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    if format!("{:x}", hash.finalize()) != release.sha256 {
        return Err(Error::Invalid);
    }
    Ok(())
}

fn replace(
    source: &Path,
    target: &Path,
    backup: &Path,
    launch: impl FnOnce() -> Result<(), Error>,
) -> Result<(), Error> {
    // The graphics launcher exits just after its renderer releases running.lock.
    // Windows can keep its executable locked during this short interval.
    rename_after_shutdown(target, backup, Duration::from_secs(10))?;
    if fs::rename(source, target).is_err() {
        fs::rename(backup, target).map_err(|_| Error::Install)?;
        return Err(Error::Install);
    }
    if launch().is_err() {
        fs::rename(target, source).map_err(|_| Error::Install)?;
        fs::rename(backup, target).map_err(|_| Error::Install)?;
        return Err(Error::Install);
    }
    Ok(())
}

fn rename_after_shutdown(
    source: &Path,
    destination: &Path,
    timeout: Duration,
) -> Result<(), Error> {
    retry_busy(timeout, || fs::rename(source, destination))
}

fn retry_busy(
    timeout: Duration,
    mut operation: impl FnMut() -> std::io::Result<()>,
) -> Result<(), Error> {
    let deadline = Instant::now() + timeout;
    loop {
        match operation() {
            Ok(()) => return Ok(()),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::WouldBlock
                ) || cfg!(windows) && matches!(error.raw_os_error(), Some(32 | 33)) =>
            {
                if Instant::now() >= deadline {
                    return Err(Error::Install);
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => return Err(Error::Install),
        }
    }
}

fn restart(target: &Path, directory: &Path, failed: bool) -> Result<(), Error> {
    let executable = if cfg!(target_os = "macos") {
        target.join("Contents/MacOS/iwe")
    } else {
        target.to_owned()
    };
    let mut command = Command::new(executable);
    command
        .arg("--iwe-update-cleanup")
        .arg(directory)
        .env_remove("IWE_GRAPHICS_CHILD")
        .env_remove("WGPU_BACKEND")
        .env("IWE_UPDATE_FAILED", if failed { "1" } else { "0" })
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn().map_err(|_| Error::Install)?;
    Ok(())
}

fn payload_root(platform: &str) -> &'static str {
    if platform.starts_with("macos-") {
        "IWE.app"
    } else if platform.starts_with("windows-") {
        "iwe.exe"
    } else {
        "iwe"
    }
}

fn validate_payload(source: &Path, macos: bool) -> Result<(), Error> {
    let executable = if macos {
        source.join("Contents/MacOS/iwe")
    } else {
        source.to_owned()
    };
    if !executable.is_file() {
        return Err(Error::Invalid);
    }
    if macos && !source.join("Contents/Info.plist").is_file() {
        return Err(Error::Invalid);
    }
    Ok(())
}

fn extract(archive: &Path, destination: &Path, root: &str) -> Result<(), Error> {
    fs::create_dir(destination).map_err(|_| Error::Install)?;
    let file = File::open(archive).map_err(|_| Error::Install)?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    let mut total = 0_u64;
    let mut seen = BTreeSet::new();
    for entry in archive.entries().map_err(|_| Error::Invalid)? {
        let mut entry = entry.map_err(|_| Error::Invalid)?;
        let path = entry.path().map_err(|_| Error::Invalid)?.into_owned();
        let mut components = path.components();
        if components.next() != Some(Component::Normal(root.as_ref()))
            || components.any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(Error::Invalid);
        }
        if !seen.insert(path.to_string_lossy().to_lowercase()) || seen.len() > 4096 {
            return Err(Error::Invalid);
        }
        let kind = entry.header().entry_type();
        if !kind.is_file() && !kind.is_dir() {
            return Err(Error::Invalid);
        }
        total = total.checked_add(entry.size()).ok_or(Error::Invalid)?;
        if total > MAX_PACKAGE * 3 {
            return Err(Error::Invalid);
        }
        let output = destination.join(path);
        if kind.is_dir() {
            fs::create_dir_all(output).map_err(|_| Error::Install)?;
            continue;
        }
        fs::create_dir_all(output.parent().ok_or(Error::Invalid)?).map_err(|_| Error::Install)?;
        let mut file = File::options()
            .create_new(true)
            .write(true)
            .open(&output)
            .map_err(|_| Error::Invalid)?;
        std::io::copy(&mut entry, &mut file).map_err(|_| Error::Invalid)?;
        file.flush().map_err(|_| Error::Install)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = entry.header().mode().map_err(|_| Error::Invalid)? & 0o777;
            fs::set_permissions(output, fs::Permissions::from_mode(mode))
                .map_err(|_| Error::Install)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
