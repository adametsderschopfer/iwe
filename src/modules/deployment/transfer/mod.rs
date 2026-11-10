//! Run bounded transfers and preserve local files until downloads finish.

use super::transport::{self, Entry, Progress, Secret, Transport};
use crate::domain::deployment::{Connection, Connections, Error, Profile, contains, remote_path};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

const PREVIEW_LIMIT: u64 = 2 * 1024 * 1024;
const DEPTH_LIMIT: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Operation {
    Test,
    List(String),
    Preview(String),
    Compare(PathBuf),
    Upload(PathBuf),
    Download(String, bool),
}

#[derive(Debug, Clone)]
pub(crate) enum Output {
    Connected,
    Listed(String, Vec<Entry>),
    Preview {
        profile_id: String,
        label: String,
        path: PathBuf,
        remote: String,
        text: Arc<String>,
        previous: Option<Arc<String>>,
    },
    Transferred,
}

#[derive(Debug, Clone)]
pub(crate) struct Request {
    pub connection: Connection,
    pub settings: Connections,
    pub profile: Profile,
    pub operation: Operation,
    pub secret: Option<Secret>,
}

pub(crate) fn execute(request: Request, progress: &Progress) -> Result<Output, Error> {
    progress.check()?;
    let mut transport = transport::connect(
        &request.connection,
        &request.settings,
        request.secret.as_ref(),
    )?;
    run(
        transport.as_mut(),
        &request.profile,
        &request.operation,
        progress,
    )
}

pub(crate) fn run(
    transport: &mut dyn Transport,
    profile: &Profile,
    operation: &Operation,
    progress: &Progress,
) -> Result<Output, Error> {
    progress.check()?;
    match operation {
        Operation::Test => Ok(Output::Connected),
        Operation::List(path) => {
            check_remote(transport, path, false)?;
            if !contains(&remote_path(&profile.root)?, &remote_path(path)?) {
                return Err(Error::Mapping);
            }
            let mut entries = transport.list(path)?;
            entries.sort_by(|a, b| {
                b.directory
                    .cmp(&a.directory)
                    .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            });
            Ok(Output::Listed(path.clone(), entries))
        }
        Operation::Preview(remote) => {
            if !contains(&remote_path(&profile.root)?, &remote_path(remote)?) {
                return Err(Error::Mapping);
            }
            let text = read_remote(transport, remote, progress)?;
            Ok(Output::Preview {
                profile_id: profile.id.clone(),
                label: profile.name.clone(),
                path: PathBuf::from(remote),
                remote: remote.clone(),
                text: Arc::new(text),
                previous: None,
            })
        }
        Operation::Compare(local) => {
            let remote = profile.to_remote(local)?;
            check_local(&profile.project, local, false)?;
            let before = fs::metadata(local).map_err(|_| Error::Access)?;
            if before.len() > PREVIEW_LIMIT {
                return Err(Error::TooLarge);
            }
            let text = read_local_text(local)?;
            let previous = read_remote(transport, &remote, progress)?;
            if version(local)? != Some((before.len(), before.modified().ok())) {
                return Err(Error::Changed);
            }
            Ok(Output::Preview {
                profile_id: profile.id.clone(),
                label: profile.name.clone(),
                path: local.clone(),
                remote,
                text: Arc::new(text),
                previous: Some(Arc::new(previous)),
            })
        }
        Operation::Upload(local) => {
            profile.validate()?;
            let mut files = Vec::new();
            collect_local(profile, local, &mut files, 0, progress)?;
            for (local, remote, directory) in files {
                progress.check()?;
                check_local(&profile.project, &local, false)?;
                if directory {
                    ensure_directory(transport, &remote)?;
                } else {
                    let parent = remote.rsplit_once('/').ok_or(Error::Mapping)?.0;
                    ensure_directory(transport, if parent.is_empty() { "/" } else { parent })?;
                    check_remote(transport, &remote, true)?;
                    let mut input = fs::File::open(&local).map_err(|_| Error::Access)?;
                    let before = version(&local)?;
                    transport.write(&remote, &mut input, progress)?;
                    if before != version(&local)? {
                        return Err(Error::Changed);
                    }
                    progress
                        .files
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            Ok(Output::Transferred)
        }
        Operation::Download(remote, overwrite) => {
            let mut files = Vec::new();
            collect_remote(transport, profile, remote, &mut files, 0, progress)?;
            let mut expected = Vec::new();
            for (local, _, directory) in &files {
                check_local(&profile.project, local, true)?;
                let version = if *directory { None } else { version(local)? };
                if version.is_some() && !overwrite {
                    return Err(Error::Overwrite);
                }
                expected.push(version);
            }
            for ((local, remote, directory), expected) in files.into_iter().zip(expected) {
                progress.check()?;
                check_local(&profile.project, &local, true)?;
                check_remote(transport, &remote, false)?;
                if directory {
                    fs::create_dir_all(&local).map_err(|_| Error::Access)?;
                } else {
                    let parent = local.parent().ok_or(Error::Mapping)?;
                    fs::create_dir_all(parent).map_err(|_| Error::Access)?;
                    let mut temporary =
                        tempfile::NamedTempFile::new_in(parent).map_err(|_| Error::Access)?;
                    transport.read(&remote, temporary.as_file_mut(), progress)?;
                    progress.check()?;
                    temporary.as_file().sync_all().map_err(|_| Error::Access)?;
                    check_local(&profile.project, &local, true)?;
                    if version(&local)? != expected {
                        return Err(Error::Changed);
                    }
                    if expected.is_some() {
                        if let Ok(metadata) = fs::metadata(&local) {
                            temporary
                                .as_file()
                                .set_permissions(metadata.permissions())
                                .map_err(|_| Error::Access)?;
                        }
                        temporary.persist(&local).map_err(|_| Error::Access)?;
                    } else {
                        temporary
                            .persist_noclobber(&local)
                            .map_err(|_| Error::Changed)?;
                    }
                    progress
                        .files
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            Ok(Output::Transferred)
        }
    }
}

type Version = Option<(u64, Option<std::time::SystemTime>)>;
fn version(path: &Path) -> Result<Version, Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(Some((metadata.len(), metadata.modified().ok()))),
        Ok(_) => Err(Error::Access),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(Error::Access),
    }
}

fn read_local_text(path: &Path) -> Result<String, Error> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| Error::Access)?
        .take(PREVIEW_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Access)?;
    decode(bytes)
}
fn decode(bytes: Vec<u8>) -> Result<String, Error> {
    if bytes.len() as u64 > PREVIEW_LIMIT {
        return Err(Error::TooLarge);
    }
    if bytes.contains(&0) {
        return Err(Error::Binary);
    }
    String::from_utf8(bytes).map_err(|_| Error::Binary)
}

struct PreviewBuffer(Vec<u8>);
impl Write for PreviewBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.len() + bytes.len() > PREVIEW_LIMIT as usize {
            return Err(std::io::Error::other("preview limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn read_remote(
    transport: &mut dyn Transport,
    remote: &str,
    progress: &Progress,
) -> Result<String, Error> {
    check_remote(transport, remote, false)?;
    let metadata = transport.stat(remote)?.ok_or(Error::Access)?;
    if metadata.directory {
        return Err(Error::Unsupported);
    }
    if metadata.size > PREVIEW_LIMIT {
        return Err(Error::TooLarge);
    }
    let mut bytes = PreviewBuffer(Vec::new());
    transport.read(remote, &mut bytes, progress)?;
    decode(bytes.0)
}

pub(crate) fn check_local(root: &Path, path: &Path, missing: bool) -> Result<(), Error> {
    if fs::symlink_metadata(root)
        .map_err(|_| Error::Access)?
        .file_type()
        .is_symlink()
    {
        return Err(Error::Access);
    }
    let tail = path.strip_prefix(root).map_err(|_| Error::Mapping)?;
    let mut current = root.to_path_buf();
    for component in tail.components() {
        if !matches!(
            component,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        ) {
            return Err(Error::Mapping);
        }
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => return Err(Error::Access),
            Ok(_) => {}
            Err(error) if missing && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Error::Access),
        }
    }
    Ok(())
}

pub(crate) fn check_remote(
    transport: &mut dyn Transport,
    path: &str,
    missing: bool,
) -> Result<(), Error> {
    let path = remote_path(path)?;
    let mut current = String::new();
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        current.push('/');
        current.push_str(segment);
        match transport.stat(&current)? {
            Some(entry) if entry.symlink => return Err(Error::Access),
            Some(entry) if current != path && !entry.directory => return Err(Error::Access),
            None if !missing => return Err(Error::Access),
            _ => {}
        }
    }
    Ok(())
}

fn ensure_directory(transport: &mut dyn Transport, path: &str) -> Result<(), Error> {
    let path = remote_path(path)?;
    let mut current = String::new();
    for segment in path.split('/').filter(|s| !s.is_empty()) {
        current.push('/');
        current.push_str(segment);
        match transport.stat(&current)? {
            Some(entry) if entry.symlink || !entry.directory => return Err(Error::Access),
            None => transport.mkdir(&current)?,
            _ => {}
        }
    }
    Ok(())
}

type FilePlan = Vec<(PathBuf, String, bool)>;
fn collect_local(
    profile: &Profile,
    path: &Path,
    files: &mut FilePlan,
    depth: usize,
    progress: &Progress,
) -> Result<(), Error> {
    progress.check()?;
    if depth > DEPTH_LIMIT || files.len() >= transport::ENTRY_LIMIT {
        return Err(Error::TooLarge);
    }
    if profile.check_local(path) == Err(Error::Excluded) {
        return Ok(());
    }
    check_local(&profile.project, path, false)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| Error::Access)?;
    if metadata.file_type().is_symlink() {
        return Err(Error::Access);
    }
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(Error::Unsupported);
    }
    if let Ok(remote) = profile.to_remote(path) {
        files.push((path.to_path_buf(), remote, metadata.is_dir()));
    } else if !metadata.is_dir() {
        return Err(Error::Mapping);
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|_| Error::Access)? {
            let entry = entry.map_err(|_| Error::Access)?;
            if entry.file_type().map_err(|_| Error::Access)?.is_symlink() {
                continue;
            }
            collect_local(profile, &entry.path(), files, depth + 1, progress)?;
        }
    }
    Ok(())
}

fn collect_remote(
    transport: &mut dyn Transport,
    profile: &Profile,
    remote: &str,
    files: &mut FilePlan,
    depth: usize,
    progress: &Progress,
) -> Result<(), Error> {
    progress.check()?;
    if depth > DEPTH_LIMIT || files.len() >= transport::ENTRY_LIMIT {
        return Err(Error::TooLarge);
    }
    let local = match profile.to_local(remote) {
        Err(Error::Excluded) => return Ok(()),
        result => result?,
    };
    check_remote(transport, remote, false)?;
    let entry = transport.stat(remote)?.ok_or(Error::Access)?;
    files.push((local, remote.to_owned(), entry.directory));
    if entry.directory {
        for entry in transport.list(remote)? {
            if entry.symlink {
                continue;
            }
            collect_remote(transport, profile, &entry.path, files, depth + 1, progress)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
