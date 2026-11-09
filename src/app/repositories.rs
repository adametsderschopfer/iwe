//! Load project repositories and route history reads to their original roots.

use super::{App, Message};
use crate::{
    domain::repository::{
        Repository,
        project::{CommitKey, RepositorySet},
    },
    infrastructure::repository,
};
use iced::{Task, widget::operation};

impl App {
    pub(super) fn load_repository(&mut self) -> Task<Message> {
        let Some(path) = self.workspace.directory().cloned() else {
            return Task::none();
        };
        self.loading = true;
        self.history_loading = false;
        self.history_error = None;
        self.repo_request += 1;
        let (generation, request) = (self.generation, self.repo_request);
        let load = Task::perform(
            repository::load_project(path, self.branch.clone()),
            move |result| Message::RepositoryLoaded(generation, request, result),
        );
        Task::batch([load, self.reset_history_scroll()])
    }

    pub(super) fn reset_history_scroll(&mut self) -> Task<Message> {
        self.history_scroll_y = 0.;
        operation::scroll_to("commit-history", operation::AbsoluteOffset { x: 0., y: 0. })
    }

    pub(super) fn load_more_history(&mut self) -> Task<Message> {
        if self.loading || self.history_loading {
            return Task::none();
        }
        let Some(repo) = self.repository.as_ref() else {
            return Task::none();
        };
        let Some(cursor) = repo.next_history.clone() else {
            return Task::none();
        };
        let offset = repo.commits.len();
        let (generation, request) = (self.generation, self.repo_request);
        self.history_loading = true;
        self.history_error = None;
        Task::perform(repository::load_project_history(cursor), move |result| {
            Message::HistoryLoaded(generation, request, offset, result)
        })
    }

    pub(super) fn select_commit(&mut self, id: CommitKey) -> Task<Message> {
        let Some(repo) = &self.repository else {
            return Task::none();
        };
        let Some(commit) = repo.commit(&id).cloned() else {
            return Task::none();
        };
        let root = id.root.clone();
        self.selected_commit = Some(id);
        self.show_commit_files();
        self.collapsed_commit_folders.clear();
        self.changed_files.clear();
        self.changes_error = None;
        self.changes_loading = true;
        self.commit_request += 1;
        let (generation, request) = (self.generation, self.commit_request);
        Task::perform(repository::changed_files(root, commit), move |result| {
            Message::ChangesLoaded(generation, request, result)
        })
    }

    pub(crate) fn active_repository(&self) -> Option<&Repository> {
        self.repository.as_ref().and_then(RepositorySet::active)
    }
}

#[cfg(test)]
mod tests;
