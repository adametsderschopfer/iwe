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
    pub(super) fn reveal_repository_branches(&mut self, root: std::path::PathBuf) -> Task<Message> {
        let Some(repo) = self.repository.as_ref().and_then(|repos| repos.get(&root)) else {
            return Task::none();
        };
        let branch = repo
            .branches
            .iter()
            .find(|branch| branch.current)
            .map(|branch| crate::domain::repository::project::BranchKey {
                root: root.clone(),
                reference: branch.reference.clone(),
            });
        self.repository_menu = None;
        self.expanded_branch_folders
            .insert(format!("repo:{}", root.display()));
        self.expanded_branch_folders
            .insert(format!("{}:false", root.display()));
        self.expanded_branch_folders.insert("false".into());
        let history = if let Some(branch) = branch {
            self.update(Message::SelectBranch(branch))
        } else {
            Task::none()
        };
        Task::batch([history, self.update(Message::RevealGitBranches)])
    }

    pub(super) fn cache_repository(&mut self) {
        if let (Some(root), Some(repository)) =
            (self.workspace.directory(), self.repository.as_ref())
        {
            self.repository_cache.insert(
                root.clone(),
                super::repository_cache::Snapshot {
                    repository: repository.clone(),
                    branch: self.branch.clone(),
                    folders: self.expanded_branch_folders.clone(),
                },
            );
        }
    }

    pub(super) fn restore_cached_repository(&mut self) -> bool {
        let Some(root) = self.workspace.directory().cloned() else {
            return false;
        };
        let Some(snapshot) = self.repository_cache.get(&root) else {
            return false;
        };
        self.repository = Some(snapshot.repository);
        self.branch = snapshot.branch;
        self.expanded_branch_folders = snapshot.folders;
        true
    }

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
