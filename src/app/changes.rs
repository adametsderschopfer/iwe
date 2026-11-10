//! Refresh Git decorations without interrupting navigation or source scrolling.

use super::{App, Message};
use crate::{
    domain::{Failure, changes::Worktree},
    infrastructure::repository,
    ui::code::{Document, syntax_for},
};
use iced::{Subscription, Task};
use std::{collections::BTreeSet, path::PathBuf, time::Duration};

#[derive(Default)]
pub(crate) struct State {
    pub status: Worktree,
    pub error: Option<Failure>,
    pub loaded: bool,
    pending: bool,
    disabled: bool,
    request: u64,
    pub(super) preview_pending: Option<u64>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Refresh,
    RefreshManually,
    Loaded(u64, u64, Result<Worktree, Failure>),
    PreviewLoaded(u64, u64, Result<Option<Box<Document>>, Failure>),
}

impl App {
    pub(super) fn toggle_worktree_folder(&mut self, repository: Option<PathBuf>, path: String) {
        let prefix = repository
            .as_ref()
            .map(|root| format!("repo:{}:", root.display()))
            .unwrap_or_default();
        let key = format!("{prefix}{path}");
        if self.collapsed_worktree_folders.remove(&key) {
            return;
        }
        self.collapsed_worktree_folders.insert(key);
        let Some(root) = repository.as_ref().or_else(|| self.workspace.directory()) else {
            return;
        };
        let descendant_prefix = format!("{path}/");
        for file in self.changes.status.files.keys() {
            if repository.is_some()
                && self
                    .changes
                    .status
                    .roots
                    .iter()
                    .filter(|candidate| file.starts_with(candidate))
                    .max_by_key(|candidate| candidate.components().count())
                    != Some(root)
            {
                continue;
            }
            let Ok(relative) = file.strip_prefix(root) else {
                continue;
            };
            for folder in relative
                .ancestors()
                .skip(1)
                .filter(|folder| !folder.as_os_str().is_empty())
            {
                let folder = folder
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                if path.is_empty() || folder == path || folder.starts_with(&descendant_prefix) {
                    self.collapsed_worktree_folders
                        .insert(format!("{prefix}{folder}"));
                }
            }
        }
    }

    pub(super) fn change_subscription(&self) -> Subscription<Message> {
        if self.loaded && self.workspace.directory().is_some() && !self.changes.disabled {
            iced::time::every(Duration::from_secs(3)).map(|_| Message::Worktree(Event::Refresh))
        } else {
            Subscription::none()
        }
    }

    pub(super) fn refresh_changes(&mut self) -> Task<Message> {
        if self.changes.pending {
            return Task::none();
        }
        let Some(root) = self.workspace.directory().cloned() else {
            return Task::none();
        };
        self.changes.pending = true;
        self.changes.request += 1;
        let (generation, request) = (self.generation, self.changes.request);
        Task::perform(repository::working_status(root), move |result| {
            Message::Worktree(Event::Loaded(generation, request, result))
        })
    }

    pub(super) fn update_changes(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Refresh => return self.refresh_changes(),
            Event::RefreshManually => {
                self.changes.disabled = false;
                return self.refresh_changes();
            }
            Event::Loaded(generation, request, result)
                if generation == self.generation && request == self.changes.request =>
            {
                self.changes.pending = false;
                self.changes.loaded = true;
                self.changes.error = result.as_ref().err().copied();
                match result {
                    Ok(status) => {
                        if let Some(root) = self.workspace.directory().cloned() {
                            self.remember_project_changes(root, &status);
                        }
                        let previous = std::mem::replace(&mut self.changes.status, status);
                        let mut folders = BTreeSet::new();
                        for path in previous
                            .files
                            .keys()
                            .chain(self.changes.status.files.keys())
                        {
                            if previous.files.get(path) != self.changes.status.files.get(path) {
                                let original = self
                                    .changes
                                    .status
                                    .files
                                    .get(path)
                                    .or_else(|| previous.files.get(path))
                                    .and_then(|change| change.original.as_ref());
                                for path in std::iter::once(path).chain(original) {
                                    folders.extend(
                                        path.ancestors()
                                            .skip(1)
                                            .filter(|parent| self.tree.contains_key(*parent))
                                            .map(std::path::Path::to_owned),
                                    );
                                }
                            }
                        }
                        let refresh =
                            Task::batch(folders.into_iter().map(|path| self.refresh_folder(path)));
                        return Task::batch([refresh, self.refresh_preview()]);
                    }
                    Err(error @ (Failure::NotRepository | Failure::MissingTool)) => {
                        if error == Failure::NotRepository
                            && let Some(root) = self.workspace.directory().cloned()
                        {
                            self.remember_project_changes(root, &Worktree::default());
                        }
                        self.changes.status = Worktree::default();
                        self.changes.disabled = true;
                    }
                    Err(_) => {}
                }
            }
            Event::PreviewLoaded(generation, request, result) if generation == self.generation => {
                if self.changes.preview_pending == Some(request) {
                    self.changes.preview_pending = None;
                }
                match result {
                    Ok(Some(document)) => return self.finish_preview(request, Ok(*document)),
                    Err(Failure::TooLarge) => return self.reload_large_preview(request),
                    result => {
                        if let Some(preview) = self.preview_for_request(request) {
                            preview.error = result.err();
                        }
                    }
                }
            }
            _ => {}
        }
        Task::none()
    }

    pub(super) fn refresh_preview(&mut self) -> Task<Message> {
        if self.preview().loading
            || self
                .preview()
                .document
                .as_ref()
                .is_some_and(|document| document.page.is_some())
            || self.preview().revision.is_some()
            || self.changes.preview_pending.is_some()
        {
            return Task::none();
        }
        let (Some(root), Some(path), Some(document)) = (
            self.workspace.directory().cloned(),
            self.preview().path.clone(),
            self.preview().document.as_ref(),
        ) else {
            return Task::none();
        };
        let (generation, request) = (self.generation, self.preview().id);
        let fingerprint = document.fingerprint;
        self.changes.preview_pending = Some(request);
        Task::perform(
            async move {
                let extension = syntax_for(&path);
                let source = repository::working_file(root, path).await?;
                if Document::fingerprint(&source) == fingerprint {
                    return Ok(None);
                }
                super::prepare_document(Ok(source), extension)
                    .await
                    .map(Some)
            },
            move |result| {
                Message::Worktree(Event::PreviewLoaded(
                    generation,
                    request,
                    result.map(|document| document.map(Box::new)),
                ))
            },
        )
    }
}

#[cfg(test)]
mod tests;
