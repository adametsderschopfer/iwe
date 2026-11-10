//! Refresh sidebar indicators independently of the selected project.

use super::{App, Message};
use crate::{
    domain::{Failure, changes::Worktree},
    infrastructure::repository,
};
use iced::{
    Subscription, Task,
    futures::{StreamExt, stream},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Default)]
pub(crate) struct State {
    known: BTreeMap<PathBuf, bool>,
    pending: bool,
    request: u64,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Refresh,
    Loaded(u64, u64, PathBuf, Result<bool, Failure>),
    Finished(u64),
}

impl App {
    pub(super) fn project_change_subscription(&self) -> Subscription<Message> {
        if self.module_enabled(crate::modules::ModuleId::Git)
            && self.loaded
            && self
                .workspace
                .spaces
                .iter()
                .any(|space| !space.projects.is_empty())
        {
            iced::time::every(Duration::from_secs(10))
                .map(|_| Message::ProjectChanges(Event::Refresh))
        } else {
            Subscription::none()
        }
    }

    pub(crate) fn project_has_changes(&self, root: &Path) -> bool {
        if self
            .workspace
            .directory()
            .is_some_and(|selected| selected == root)
            && self.changes.loaded
            && self.changes.error.is_none()
            && (!self.changes.status.files.is_empty() || self.changes.status.failures.is_empty())
        {
            return !self.changes.status.files.is_empty();
        }
        self.project_changes
            .known
            .get(root)
            .copied()
            .unwrap_or(false)
    }

    pub(super) fn remember_project_changes(&mut self, root: PathBuf, status: &Worktree) {
        if !status.files.is_empty() || status.failures.is_empty() {
            self.project_changes
                .known
                .insert(root, !status.files.is_empty());
        }
    }

    pub(super) fn refresh_project_changes(&mut self) -> Task<Message> {
        if !self.module_enabled(crate::modules::ModuleId::Git)
            || !self.loaded
            || self.project_changes.pending
        {
            return Task::none();
        }
        let roots: BTreeSet<_> = self
            .workspace
            .spaces
            .iter()
            .flat_map(|space| {
                space
                    .projects
                    .iter()
                    .map(|project| project.directory.clone())
            })
            .collect();
        self.project_changes
            .known
            .retain(|root, _| roots.contains(root));
        let roots: Vec<_> = roots
            .into_iter()
            .filter(|root| Some(root) != self.workspace.directory())
            .collect();
        if roots.is_empty() {
            return Task::none();
        }
        self.project_changes.pending = true;
        self.project_changes.request += 1;
        let request = self.project_changes.request;
        let generation = self.generation;
        let results = stream::iter(roots)
            .map(move |root| async move {
                let result = repository::working_status(root.clone())
                    .await
                    .and_then(|status| {
                        if status.files.is_empty() && !status.failures.is_empty() {
                            Err(status.failures[0].1)
                        } else {
                            Ok(!status.files.is_empty())
                        }
                    });
                Event::Loaded(request, generation, root, result)
            })
            .buffer_unordered(2);
        Task::run(results, Message::ProjectChanges).chain(Task::done(Message::ProjectChanges(
            Event::Finished(request),
        )))
    }

    pub(super) fn update_project_changes(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Refresh => return self.refresh_project_changes(),
            Event::Loaded(request, generation, root, result)
                if request == self.project_changes.request
                    && generation == self.generation
                    && self.project_changes.pending =>
            {
                if self.workspace.directory() == Some(&root)
                    || !self.workspace.spaces.iter().any(|space| {
                        space
                            .projects
                            .iter()
                            .any(|project| project.directory == root)
                    })
                {
                    return Task::none();
                }
                match result {
                    Ok(dirty) => {
                        self.project_changes.known.insert(root, dirty);
                    }
                    Err(Failure::NotRepository) => {
                        self.project_changes.known.remove(&root);
                    }
                    Err(_) => {}
                }
            }
            Event::Finished(request) if request == self.project_changes.request => {
                self.project_changes.pending = false;
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;

impl State {
    pub(crate) fn suspend(&mut self) {
        let request = self.request + 1;
        *self = Self {
            request,
            ..Self::default()
        };
    }
}
