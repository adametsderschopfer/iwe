//! Coordinate explicit repository actions and discard results from other projects.

pub(crate) mod bulk;

use super::{App, Message as AppMessage};
use crate::{
    domain::{
        git::{Error, Operation},
        repository::{Branch, ReferenceKind, project::BranchKey},
    },
    i18n::Key,
    infrastructure::git,
};
use iced::{Point, Task};
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct State {
    pub menu: Option<(BranchKey, Branch, Point)>,
    pub dialog: Option<Dialog>,
    pub pending: bool,
    pub bulk: bulk::State,
    completion: Option<(Key, Option<String>)>,
    request: u64,
}

impl State {
    pub(crate) fn has_dialog(&self) -> bool {
        self.dialog.is_some() || self.bulk.dialog.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Checkout,
    Create,
    Update,
    Push,
    Rename,
    Delete,
}

pub(crate) struct Dialog {
    pub root: PathBuf,
    pub action: Action,
    pub branch: Branch,
    pub name: String,
    pub remote: String,
    pub checkout: bool,
    pub error: Option<Error>,
}

#[derive(Clone, Debug)]
pub(crate) enum Message {
    Menu(BranchKey, Point),
    Bulk(bulk::Event),
    Close,
    Fetch,
    Choose(Action, BranchKey),
    Name(String),
    Remote(String),
    Checkout(bool),
    Submit,
    Finished(u64, PathBuf, Result<(), Error>),
    Fetched(u64, u64, Vec<(PathBuf, Result<(), Error>)>),
}

impl App {
    pub(super) fn update_git(&mut self, message: Message) -> Task<AppMessage> {
        match message {
            Message::Bulk(event) => return self.update_bulk_git(event),
            Message::Fetched(request, generation, results)
                if self.git_actions.pending && request == self.git_actions.request =>
            {
                self.git_actions.pending = false;
                if generation != self.generation {
                    return Task::none();
                }
                for (root, result) in results {
                    let context = root
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned());
                    match result {
                        Ok(()) => self.notifications.success(Key::GitFetchDone, context),
                        Err(error) => self.notifications.error(error_key(error), context),
                    }
                }
                return self.load_directory();
            }
            Message::Finished(request, root, result)
                if self.git_actions.pending && request == self.git_actions.request =>
            {
                self.git_actions.pending = false;
                let completion = self.git_actions.completion.take();
                if self
                    .repository
                    .as_ref()
                    .is_none_or(|repos| repos.get(&root).is_none())
                {
                    self.git_actions.dialog = None;
                    return Task::none();
                }
                match result {
                    Ok(()) => {
                        if let Some((message, context)) = completion {
                            self.notifications.success(message, context);
                        }
                        self.git_actions.dialog = None;
                        return self.load_directory();
                    }
                    Err(error) => {
                        self.notifications.error(
                            error_key(error),
                            completion.and_then(|(_, context)| context),
                        );
                        if let Some(dialog) = &mut self.git_actions.dialog {
                            dialog.error = Some(error);
                        }
                        return Task::batch([self.load_repository(), self.refresh_changes()]);
                    }
                }
            }
            Message::Close if !self.git_actions.pending => {
                self.branch_selection.selected.clear();
                self.git_actions.menu = None;
                self.git_actions.dialog = None;
                self.git_actions.bulk.menu = None;
                self.git_actions.bulk.dialog = None;
            }
            _ if self.git_actions.pending
                || self.space_draft.is_some()
                || self.auth_dialog.is_some() => {}
            Message::Menu(key, position) => {
                if self.branch_selection.selected.contains(&key) {
                    self.git_actions.menu = None;
                    self.git_actions.bulk.menu = Some(position);
                    return Task::none();
                }
                self.branch_selection.selected.clear();
                self.git_actions.bulk.menu = None;
                if let Some(branch) = self
                    .repository
                    .as_ref()
                    .and_then(|repos| repos.get(&key.root))
                    .and_then(|repo| {
                        repo.branches
                            .iter()
                            .find(|branch| branch.reference == key.reference)
                    })
                    .cloned()
                {
                    self.project_menu = None;
                    self.space_menu = None;
                    self.git_actions.menu = Some((key, branch, position));
                }
            }
            Message::Fetch => return self.fetch_repositories(),
            Message::Choose(action, key) => {
                self.git_actions.menu = None;
                let Some(repo) = self
                    .repository
                    .as_ref()
                    .and_then(|repos| repos.get(&key.root))
                else {
                    return Task::none();
                };
                let Some(branch) = repo
                    .branches
                    .iter()
                    .find(|branch| branch.reference == key.reference)
                    .cloned()
                else {
                    return Task::none();
                };
                if !allowed(action, &branch) {
                    return Task::none();
                }
                match action {
                    Action::Checkout => {
                        return self.run_git(key.root, Operation::Checkout(key.reference));
                    }
                    Action::Update => {
                        return self.run_git(key.root, Operation::Update(key.reference));
                    }
                    _ => {
                        self.git_actions.dialog = Some(Dialog {
                            root: key.root,
                            name: if action == Action::Create {
                                String::new()
                            } else {
                                branch.name.clone()
                            },
                            remote: repo
                                .remotes
                                .iter()
                                .find(|name| name.as_str() == "origin")
                                .or(repo.remotes.first())
                                .cloned()
                                .unwrap_or_default(),
                            action,
                            branch,
                            checkout: true,
                            error: None,
                        });
                    }
                }
            }
            Message::Name(value) => {
                if let Some(dialog) = &mut self.git_actions.dialog {
                    dialog.name = value;
                    dialog.error = None;
                }
            }
            Message::Remote(value) => {
                if let Some(dialog) = &mut self.git_actions.dialog {
                    dialog.remote = value;
                }
            }
            Message::Checkout(value) => {
                if let Some(dialog) = &mut self.git_actions.dialog {
                    dialog.checkout = value;
                }
            }
            Message::Submit => {
                let Some(dialog) = &self.git_actions.dialog else {
                    return Task::none();
                };
                let source = dialog.branch.reference.clone();
                let name = dialog.name.trim().to_owned();
                let operation = match dialog.action {
                    Action::Create => Operation::Create {
                        source,
                        name,
                        checkout: dialog.checkout,
                    },
                    Action::Rename => Operation::Rename { source, name },
                    Action::Push => Operation::Push {
                        source,
                        name,
                        remote: dialog.remote.clone(),
                    },
                    Action::Delete => Operation::Delete(source),
                    _ => return Task::none(),
                };
                return self.run_git(dialog.root.clone(), operation);
            }
            _ => {}
        }
        Task::none()
    }

    fn run_git(&mut self, root: PathBuf, operation: Operation) -> Task<AppMessage> {
        let Some(repo) = self.repository.as_ref().and_then(|repos| repos.get(&root)) else {
            return Task::none();
        };
        if self.git_actions.pending {
            return Task::none();
        }
        let branch_name = |reference: &str| {
            let name = repo
                .branches
                .iter()
                .find(|branch| branch.reference == reference)
                .map_or_else(|| reference.to_owned(), |branch| branch.name.clone());
            Some(name)
        };
        self.git_actions.completion = Some(match &operation {
            Operation::Fetch => (
                Key::GitFetchDone,
                self.workspace.project().map(|project| project.name.clone()),
            ),
            Operation::Checkout(reference) => (Key::GitCheckoutDone, branch_name(reference)),
            Operation::Update(reference) => (Key::GitUpdateDone, branch_name(reference)),
            Operation::Delete(reference) => (Key::GitDeleteDone, branch_name(reference)),
            Operation::Create { name, .. } => (Key::GitCreateDone, Some(name.clone())),
            Operation::Rename { name, .. } => (Key::GitRenameDone, Some(name.clone())),
            Operation::Push { name, .. } => (Key::GitPushDone, Some(name.clone())),
        });
        if self
            .repository
            .as_ref()
            .is_some_and(|repos| repos.repositories.len() > 1)
            && let Some((_, Some(context))) = &mut self.git_actions.completion
        {
            *context = format!(
                "{} · {context}",
                root.file_name().unwrap_or_default().to_string_lossy()
            );
        }
        self.git_actions.pending = true;
        self.git_actions.request += 1;
        let request = self.git_actions.request;
        Task::perform(git::execute(root.clone(), operation), move |result| {
            AppMessage::Git(Message::Finished(request, root.clone(), result))
        })
    }

    fn fetch_repositories(&mut self) -> Task<AppMessage> {
        let Some(repos) = &self.repository else {
            return Task::none();
        };
        if repos.repositories.len() == 1 {
            return self.run_git(repos.active_root.clone(), Operation::Fetch);
        }
        let roots: Vec<_> = repos
            .repositories
            .iter()
            .map(|repo| repo.root.clone())
            .collect();
        self.git_actions.pending = true;
        self.git_actions.request += 1;
        let (request, generation) = (self.git_actions.request, self.generation);
        Task::perform(
            async move {
                let mut results = Vec::new();
                for root in roots {
                    let result = git::execute(root.clone(), Operation::Fetch).await;
                    results.push((root, result));
                }
                results
            },
            move |results| AppMessage::Git(Message::Fetched(request, generation, results)),
        )
    }
}

pub(crate) fn allowed(action: Action, branch: &Branch) -> bool {
    match action {
        Action::Checkout => !branch.current,
        Action::Create => true,
        Action::Update => branch.kind != ReferenceKind::Tag,
        Action::Push | Action::Rename => branch.kind == ReferenceKind::Local,
        Action::Delete => branch.kind != ReferenceKind::Tag && !branch.current,
    }
}

pub(crate) fn error_key(error: Error) -> Key {
    match error {
        Error::Failed => Key::GitOperationFailed,
        Error::InvalidName => Key::GitInvalidName,
        Error::MissingUpstream => Key::GitMissingUpstream,
        Error::Dirty => Key::GitDirty,
        Error::CurrentBranch => Key::GitCurrentBranch,
        Error::BranchInWorktree => Key::GitBranchInWorktree,
    }
}

#[cfg(test)]
mod tests;
