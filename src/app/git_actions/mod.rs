//! Coordinate explicit repository actions and discard results from other projects.

use super::{App, Message as AppMessage};
use crate::{
    domain::{
        git::{Error, Operation},
        repository::{Branch, ReferenceKind},
    },
    i18n::Key,
    infrastructure::git,
};
use iced::{Point, Task};
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct State {
    pub menu: Option<(Branch, Point)>,
    pub dialog: Option<Dialog>,
    pub pending: bool,
    request: u64,
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
    pub action: Action,
    pub branch: Branch,
    pub name: String,
    pub remote: String,
    pub checkout: bool,
    pub error: Option<Error>,
}

#[derive(Clone, Debug)]
pub(crate) enum Message {
    Menu(String, Point),
    Close,
    Fetch,
    Choose(Action, String),
    Name(String),
    Remote(String),
    Checkout(bool),
    Submit,
    Finished(u64, PathBuf, Result<(), Error>),
}

impl App {
    pub(super) fn update_git(&mut self, message: Message) -> Task<AppMessage> {
        match message {
            Message::Finished(request, root, result) if request == self.git_actions.request => {
                self.git_actions.pending = false;
                if self
                    .repository
                    .as_ref()
                    .is_none_or(|repo| repo.root != root)
                {
                    self.git_actions.dialog = None;
                    return Task::none();
                }
                match result {
                    Ok(()) => {
                        self.git_actions.dialog = None;
                        return self.load_directory();
                    }
                    Err(error) => {
                        if let Some(dialog) = &mut self.git_actions.dialog {
                            dialog.error = Some(error);
                        } else {
                            self.notice = Some(error_key(error));
                        }
                        return Task::batch([self.load_repository(), self.refresh_changes()]);
                    }
                }
            }
            Message::Close if !self.git_actions.pending => {
                self.git_actions.menu = None;
                self.git_actions.dialog = None;
            }
            _ if self.git_actions.pending
                || self.space_draft.is_some()
                || self.auth_dialog.is_some() => {}
            Message::Menu(reference, position) => {
                if let Some(branch) = self
                    .repository
                    .as_ref()
                    .and_then(|repo| {
                        repo.branches
                            .iter()
                            .find(|branch| branch.reference == reference)
                    })
                    .cloned()
                {
                    self.project_menu = None;
                    self.git_actions.menu = Some((branch, position));
                }
            }
            Message::Fetch => return self.run_git(Operation::Fetch),
            Message::Choose(action, reference) => {
                self.git_actions.menu = None;
                let Some(repo) = &self.repository else {
                    return Task::none();
                };
                let Some(branch) = repo
                    .branches
                    .iter()
                    .find(|branch| branch.reference == reference)
                    .cloned()
                else {
                    return Task::none();
                };
                if !allowed(action, &branch) {
                    return Task::none();
                }
                match action {
                    Action::Checkout => return self.run_git(Operation::Checkout(reference)),
                    Action::Update => return self.run_git(Operation::Update(reference)),
                    _ => {
                        self.git_actions.dialog = Some(Dialog {
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
                return self.run_git(operation);
            }
            _ => {}
        }
        Task::none()
    }

    fn run_git(&mut self, operation: Operation) -> Task<AppMessage> {
        let Some(repo) = &self.repository else {
            return Task::none();
        };
        if self.git_actions.pending {
            return Task::none();
        }
        let root = repo.root.clone();
        self.git_actions.pending = true;
        self.git_actions.request += 1;
        let request = self.git_actions.request;
        Task::perform(git::execute(root.clone(), operation), move |result| {
            AppMessage::Git(Message::Finished(request, root.clone(), result))
        })
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
    }
}

#[cfg(test)]
mod tests;
