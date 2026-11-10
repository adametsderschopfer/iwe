//! Keep cached folders visible while a background index refreshes their metadata.

use super::{App, Message};
use crate::{domain::Failure, infrastructure::project_index};
use iced::{Task, futures::stream};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::PathBuf,
    time::Duration,
};

#[derive(Default)]
pub(crate) struct State {
    root: Option<PathBuf>,
    worker: Option<project_index::Handle>,
    epoch: u64,
    request: u64,
    pending: BTreeMap<PathBuf, u64>,
    pub loading: BTreeSet<PathBuf>,
    pub errors: BTreeMap<PathBuf, Failure>,
    pub selected_path: Option<PathBuf>,
    pub menu: Option<Menu>,
    cache: VecDeque<Cached>,
}

pub(crate) struct Menu {
    pub path: PathBuf,
    pub point: iced::Point,
    pub copy_open: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PathFormat {
    Absolute,
    Relative,
}

struct Cached {
    root: PathBuf,
    tree: project_index::Tree,
    expanded: BTreeSet<PathBuf>,
    selected_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    OpenMenu(PathBuf, iced::Point),
    CopyOptions,
    CopyPath(PathFormat),
    DismissMenu,
    Index(u64, project_index::Event),
    Slow(u64, PathBuf, u64),
    Finished(u64),
    Retry(PathBuf),
}

impl App {
    pub(super) fn load_file_tree(&mut self) -> Task<Message> {
        self.files.menu = None;
        let root = self.workspace.directory().cloned();
        if root == self.files.root && self.files.worker.is_some() {
            let expanded: Vec<_> = self.expanded.iter().cloned().collect();
            return Task::batch(expanded.into_iter().map(|path| self.refresh_folder(path)));
        }
        self.files.worker = None;
        self.files.epoch += 1;
        self.files.pending.clear();
        self.files.loading.clear();
        self.files.errors.clear();
        if let Some(previous) = self.files.root.take() {
            self.files.cache.push_front(Cached {
                root: previous,
                tree: std::mem::take(&mut self.tree),
                expanded: std::mem::take(&mut self.expanded),
                selected_path: self.files.selected_path.take(),
            });
        } else {
            self.tree.clear();
            self.expanded.clear();
            self.files.selected_path = None;
        }
        let Some(root) = root else {
            self.files.cache.truncate(4);
            return Task::none();
        };
        if let Some(index) = self
            .files
            .cache
            .iter()
            .position(|cached| cached.root == root)
            && let Some(cached) = self.files.cache.remove(index)
        {
            self.tree = cached.tree;
            self.expanded = cached.expanded;
            self.files.selected_path = cached.selected_path;
        }
        self.files.cache.truncate(4);
        self.expanded.insert(root.clone());
        let (handle, events, worker) = project_index::session(root.clone(), self.tree.clone());
        self.files.root = Some(root);
        self.files.worker = Some(handle);
        let epoch = self.files.epoch;
        let events = stream::unfold(events, |mut receiver| async move {
            receiver.recv().await.map(|event| (event, receiver))
        });
        let expanded: Vec<_> = self.expanded.iter().cloned().collect();
        let refresh = Task::batch(expanded.into_iter().map(|path| self.refresh_folder(path)));
        Task::batch([
            Task::run(events, move |event| {
                Message::Files(Event::Index(epoch, event))
            }),
            Task::perform(project_index::run(worker), move |_| {
                Message::Files(Event::Finished(epoch))
            }),
            refresh,
        ])
    }

    pub(super) fn toggle_folder(&mut self, path: PathBuf) -> Task<Message> {
        self.files.selected_path = Some(path.clone());
        if self.expanded.remove(&path) {
            self.expanded
                .retain(|directory| !directory.starts_with(&path));
            self.files
                .loading
                .retain(|directory| !directory.starts_with(&path));
            return Task::none();
        }
        self.expanded.insert(path.clone());
        self.refresh_folder(path)
    }

    pub(super) fn refresh_folder(&mut self, path: PathBuf) -> Task<Message> {
        let Some(worker) = &self.files.worker else {
            return Task::none();
        };
        self.files.errors.remove(&path);
        let request = if let Some(request) = self.files.pending.get(&path) {
            *request
        } else {
            self.files.request += 1;
            let request = self.files.request;
            self.files.pending.insert(path.clone(), request);
            worker.refresh(path.clone());
            request
        };
        if self.tree.contains_key(&path) {
            return Task::none();
        }
        let epoch = self.files.epoch;
        Task::perform(
            async { tokio::time::sleep(Duration::from_millis(350)).await },
            move |_| Message::Files(Event::Slow(epoch, path.clone(), request)),
        )
    }

    pub(super) fn update_files(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::OpenMenu(path, point)
                if self.space_draft.is_none()
                    && self.auth_dialog.is_none()
                    && !self.git_actions.has_dialog() =>
            {
                if self
                    .workspace
                    .directory()
                    .is_some_and(|root| path.is_absolute() && path.starts_with(root))
                {
                    self.project_menu = None;
                    self.space_menu = None;
                    self.git_actions.menu = None;
                    self.files.selected_path = Some(path.clone());
                    self.files.menu = Some(Menu {
                        path,
                        point,
                        copy_open: false,
                    });
                }
            }
            Event::CopyOptions => {
                if let Some(menu) = &mut self.files.menu {
                    menu.copy_open = true;
                }
            }
            Event::DismissMenu => self.files.menu = None,
            Event::CopyPath(format) => {
                if let Some(menu) = self.files.menu.take() {
                    let value = match format {
                        PathFormat::Absolute => Some(menu.path.as_path()),
                        PathFormat::Relative => self
                            .path_copy_root(&menu.path)
                            .and_then(|root| menu.path.strip_prefix(root).ok()),
                    };
                    if let Some(value) = value {
                        let value = if value.as_os_str().is_empty() {
                            ".".to_owned()
                        } else {
                            value.to_string_lossy().into_owned()
                        };
                        return iced::clipboard::write(value);
                    }
                }
            }
            Event::Index(epoch, event) if epoch == self.files.epoch => match event {
                project_index::Event::Cached(tree) => {
                    for (path, entries) in tree {
                        self.files.loading.remove(&path);
                        self.tree.entry(path).or_insert(entries);
                    }
                }
                project_index::Event::Directory(path, result) => {
                    self.files.pending.remove(&path);
                    self.files.loading.remove(&path);
                    match result {
                        Ok(entries) => {
                            let retained = |directory: &PathBuf| {
                                directory == &path
                                    || !directory.starts_with(&path)
                                    || entries.iter().any(|entry| {
                                        entry.directory && directory.starts_with(&entry.path)
                                    })
                            };
                            self.tree.retain(|directory, _| retained(directory));
                            self.expanded.retain(retained);
                            self.files
                                .pending
                                .retain(|directory, _| retained(directory));
                            self.files.loading.retain(retained);
                            self.files.errors.retain(|directory, _| retained(directory));
                            if self
                                .files
                                .selected_path
                                .as_ref()
                                .is_some_and(|selected| !retained(selected))
                            {
                                self.files.selected_path = None;
                            }
                            self.files.errors.remove(&path);
                            self.tree.insert(path, entries);
                        }
                        Err(error) if self.expanded.contains(&path) => {
                            self.files.errors.insert(path, error);
                        }
                        Err(_) => {}
                    }
                }
            },
            Event::Slow(epoch, path, request)
                if epoch == self.files.epoch
                    && self.files.pending.get(&path) == Some(&request)
                    && self.expanded.contains(&path)
                    && !self.tree.contains_key(&path) =>
            {
                self.files.loading.insert(path);
            }
            Event::Finished(epoch) if epoch == self.files.epoch => {
                self.files.worker = None;
                for path in std::mem::take(&mut self.files.pending).into_keys() {
                    if self.expanded.contains(&path) {
                        self.files.errors.insert(path, Failure::Read);
                    }
                }
                self.files.loading.clear();
            }
            Event::Retry(path) => {
                if self.files.worker.is_none() {
                    return self.load_file_tree();
                }
                return self.refresh_folder(path);
            }
            _ => {}
        }
        Task::none()
    }

    pub(crate) fn path_repository_root<'a>(
        &'a self,
        path: &std::path::Path,
    ) -> Option<&'a std::path::Path> {
        self.changes
            .status
            .roots
            .iter()
            .map(PathBuf::as_path)
            .chain(
                self.repository
                    .iter()
                    .flat_map(|set| set.repositories.iter().map(|repo| repo.root.as_path())),
            )
            .filter(|root| path.starts_with(root))
            .max_by_key(|root| root.components().count())
    }

    fn path_copy_root<'a>(&'a self, path: &std::path::Path) -> Option<&'a std::path::Path> {
        self.path_repository_root(path)
            .or_else(|| self.workspace.directory().map(PathBuf::as_path))
    }
}

#[cfg(test)]
mod tests;
