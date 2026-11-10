//! Cache directory metadata and refresh project trees on a background worker.

use super::workspace;
use crate::domain::{Failure, workspace::FileEntry};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    io::{BufWriter, Read, Write},
    path::{Component, Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc as events;
use tokio_util::sync::CancellationToken;

pub(crate) type Tree = BTreeMap<PathBuf, Vec<FileEntry>>;
const MAX_DIRECTORIES: usize = 4096;
const MAX_ENTRIES: usize = 100_000;
const MAX_CACHE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_DEPTH: usize = 24;

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Cached(Tree),
    Directory(PathBuf, Result<Vec<FileEntry>, Failure>),
}

pub(crate) struct Handle {
    commands: mpsc::Sender<PathBuf>,
    cancelled: CancellationToken,
}

impl Handle {
    pub(crate) fn refresh(&self, path: PathBuf) {
        let _ = self.commands.send(path);
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        self.cancelled.cancel();
    }
}

pub(crate) struct Worker {
    root: PathBuf,
    storage: Option<PathBuf>,
    tree: Tree,
    entry_count: usize,
    commands: mpsc::Receiver<PathBuf>,
    events: events::Sender<Event>,
    cancelled: CancellationToken,
}

pub(crate) fn session(root: PathBuf, tree: Tree) -> (Handle, events::Receiver<Event>, Worker) {
    let (commands, receiver) = mpsc::channel();
    let (sender, events) = events::channel(16);
    let cancelled = CancellationToken::new();
    let storage = cache_path(&root).ok();
    let entry_count = tree.values().map(Vec::len).sum();
    (
        Handle {
            commands,
            cancelled: cancelled.clone(),
        },
        events,
        Worker {
            root,
            storage,
            tree,
            entry_count,
            commands: receiver,
            events: sender,
            cancelled,
        },
    )
}

pub(crate) async fn run(worker: Worker) {
    // A session-long worker must not hold Tokio's blocking pool open during shutdown.
    let (finished, completion) = tokio::sync::oneshot::channel();
    if std::thread::Builder::new()
        .name("project-index".into())
        .spawn(move || {
            worker.run();
            let _ = finished.send(());
        })
        .is_ok()
    {
        let _ = completion.await;
    }
}

impl Worker {
    fn run(mut self) {
        let storage = self.storage.take();
        let mut dirty = !self.tree.is_empty();
        if self.tree.is_empty()
            && let Some(tree) = storage.as_ref().and_then(|path| load_at(path, &self.root))
        {
            self.tree = tree;
            self.entry_count = self.tree.values().map(Vec::len).sum();
            if !self.send(Event::Cached(self.tree.clone())) {
                return;
            }
        }
        let mut queue = VecDeque::from([self.root.clone()]);
        let mut queued = BTreeSet::from([self.root.clone()]);
        let mut visited = BTreeSet::new();
        let mut saved = Instant::now();
        while !self.cancelled.is_cancelled() {
            let requested = match self.commands.try_recv() {
                Ok(path) => Some(path),
                Err(mpsc::TryRecvError::Disconnected) => break,
                Err(mpsc::TryRecvError::Empty) => None,
            };
            let foreground = requested.is_some();
            let path = requested.or_else(|| queue.pop_front());
            let Some(path) = path else {
                if dirty {
                    self.save(storage.as_deref());
                    dirty = false;
                    saved = Instant::now();
                }
                match self.commands.recv_timeout(Duration::from_millis(250)) {
                    Ok(path) => {
                        // Read opened folders before resuming speculative traversal.
                        let result = self.read(&path, &mut queue, &mut queued, true);
                        dirty |= result;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                continue;
            };
            if !valid_directory(&self.root, &path) {
                continue;
            }
            if !foreground && (!visited.insert(path.clone()) || visited.len() > MAX_DIRECTORIES) {
                continue;
            }
            dirty |= self.read(&path, &mut queue, &mut queued, foreground);
            if dirty && saved.elapsed() >= Duration::from_secs(1) {
                self.save(storage.as_deref());
                dirty = false;
                saved = Instant::now();
            }
        }
    }

    fn read(
        &mut self,
        path: &Path,
        queue: &mut VecDeque<PathBuf>,
        queued: &mut BTreeSet<PathBuf>,
        foreground: bool,
    ) -> bool {
        if !valid_directory(&self.root, path) || self.cancelled.is_cancelled() {
            return false;
        }
        let count = self.entry_count;
        if !foreground && count >= MAX_ENTRIES && !self.tree.contains_key(path) {
            return false;
        }
        let result = workspace::entries_at(&self.root, path);
        if self.cancelled.is_cancelled() {
            return false;
        }
        let mut changed = false;
        if let Ok(entries) = &result {
            let previous = self.tree.get(path);
            if !foreground && count - previous.map_or(0, Vec::len) + entries.len() > MAX_ENTRIES {
                return false;
            }
            changed = previous != Some(entries);
            if changed {
                let children: BTreeSet<_> = entries
                    .iter()
                    .filter(|entry| entry.directory)
                    .filter_map(|entry| entry.path.file_name())
                    .collect();
                let obsolete: Vec<_> = self
                    .tree
                    .range(path.to_path_buf()..)
                    .take_while(|(directory, _)| directory.starts_with(path))
                    .filter(|(directory, _)| {
                        *directory != path
                            && directory
                                .strip_prefix(path)
                                .ok()
                                .and_then(|relative| relative.components().next())
                                .is_some_and(|child| !children.contains(child.as_os_str()))
                    })
                    .map(|(directory, _)| directory.clone())
                    .collect();
                for directory in obsolete {
                    if let Some(removed) = self.tree.remove(&directory) {
                        self.entry_count -= removed.len();
                    }
                }
                let previous = self.tree.insert(path.to_path_buf(), entries.clone());
                self.entry_count =
                    self.entry_count - previous.map_or(0, |entries| entries.len()) + entries.len();
            }
            for entry in entries {
                if entry.directory
                    && queued.len() < MAX_DIRECTORIES
                    && prefetch(&self.root, &entry.path)
                    && queued.insert(entry.path.clone())
                {
                    queue.push_back(entry.path.clone());
                }
            }
        }
        // Unchanged foreground reads still finish a pending request.
        if changed || foreground || result.is_err() || path == self.root {
            self.send(Event::Directory(path.to_path_buf(), result));
        }
        changed
    }

    fn send(&self, mut event: Event) -> bool {
        while !self.cancelled.is_cancelled() {
            match self.events.try_send(event) {
                Ok(()) => return true,
                Err(events::error::TrySendError::Full(pending)) => {
                    event = pending;
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(events::error::TrySendError::Closed(_)) => {
                    self.cancelled.cancel();
                    return false;
                }
            }
        }
        false
    }

    fn save(&self, path: Option<&Path>) {
        if !self.cancelled.is_cancelled()
            && let Some(path) = path
        {
            let _ = save_at(path, &self.root, &self.tree);
        }
    }
}

fn prefetch(root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };
    relative.components().count() <= MAX_DEPTH
        && !relative.components().any(|part| {
            matches!(
                part.as_os_str().to_str(),
                Some(
                    ".git"
                        | "node_modules"
                        | "target"
                        | "vendor"
                        | "dist"
                        | "build"
                        | ".next"
                        | ".nuxt"
                        | ".venv"
                        | "venv"
                        | "__pycache__"
                        | ".cache"
                )
            )
        })
}

fn valid_directory(root: &Path, path: &Path) -> bool {
    path.starts_with(root)
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
}

fn cache_path(root: &Path) -> Result<PathBuf, Failure> {
    let directory = if let Some(directory) = std::env::var_os("IWE_DATA_DIR") {
        PathBuf::from(directory).join("cache")
    } else {
        directories::ProjectDirs::from("", "", "iwe")
            .ok_or(Failure::Storage)?
            .cache_dir()
            .to_path_buf()
    };
    let key = Sha256::digest(root.as_os_str().as_encoded_bytes());
    Ok(directory.join("trees").join(format!("{key:x}.json")))
}

#[derive(Serialize, Deserialize)]
struct Stored {
    version: u8,
    root: PathBuf,
    directories: Tree,
}

#[derive(Serialize)]
struct StoredRef<'a> {
    version: u8,
    root: &'a Path,
    directories: BTreeMap<&'a Path, &'a [FileEntry]>,
}

struct CacheWriter<W> {
    inner: W,
    remaining: u64,
    exceeded: bool,
}

impl<W: Write> Write for CacheWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() as u64 > self.remaining {
            self.exceeded = true;
            return Err(std::io::Error::other(
                "Directory cache exceeds its byte limit",
            ));
        }
        let written = self.inner.write(bytes)?;
        self.remaining -= written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn load_at(path: &Path, root: &Path) -> Option<Tree> {
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_CACHE_BYTES {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_CACHE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_CACHE_BYTES {
        return None;
    }
    let stored: Stored = serde_json::from_slice(&bytes).ok()?;
    if stored.version != 1 || stored.root != root || stored.directories.len() > MAX_DIRECTORIES {
        return None;
    }
    let count: usize = stored.directories.values().map(Vec::len).sum();
    if count > MAX_ENTRIES
        || !stored.directories.iter().all(|(directory, entries)| {
            valid_directory(root, directory)
                && entries.iter().all(|entry| {
                    entry.path.parent() == Some(directory.as_path())
                        && entry
                            .path
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy() == entry.name)
                        && entry.name != ".git"
                        && valid_directory(root, &entry.path)
                })
        })
    {
        return None;
    }
    Some(stored.directories)
}

fn save_at(path: &Path, root: &Path, tree: &Tree) -> Result<(), Failure> {
    let mut directories = BTreeMap::new();
    let mut count = 0;
    for (directory, entries) in tree {
        if directories.len() == MAX_DIRECTORIES || count + entries.len() > MAX_ENTRIES {
            continue;
        }
        count += entries.len();
        directories.insert(directory.as_path(), entries.as_slice());
    }
    let stored = StoredRef {
        version: 1,
        root,
        directories,
    };
    let parent = path.parent().ok_or(Failure::Storage)?;
    std::fs::create_dir_all(parent).map_err(|_| Failure::Storage)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|_| Failure::Storage)?;
    let mut writer = CacheWriter {
        inner: BufWriter::new(file.as_file_mut()),
        remaining: MAX_CACHE_BYTES,
        exceeded: false,
    };
    if serde_json::to_writer(&mut writer, &stored).is_err() {
        return Err(if writer.exceeded {
            Failure::TooLarge
        } else {
            Failure::Storage
        });
    }
    writer.flush().map_err(|_| Failure::Storage)?;
    drop(writer);
    file.as_file().sync_all().map_err(|_| Failure::Storage)?;
    file.persist(path).map_err(|_| Failure::Storage)?;
    Ok(())
}

#[cfg(test)]
mod tests;
