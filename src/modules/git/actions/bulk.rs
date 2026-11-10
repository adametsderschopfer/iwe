//! Confirm branch batches and execute each operation once in its owning repository.

use crate::domain::git::{Error, Operation};
use crate::{
    domain::repository::{Branch, project::BranchKey},
    i18n::Key,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Push,
    Delete,
    Pull,
}

impl Action {
    pub(crate) fn label(self) -> Key {
        match self {
            Self::Push => Key::GitBulkPush,
            Self::Delete => Key::GitBulkDelete,
            Self::Pull => Key::GitBulkPull,
        }
    }
    pub(crate) fn single(self) -> super::Action {
        match self {
            Self::Push => super::Action::Push,
            Self::Delete => super::Action::Delete,
            Self::Pull => super::Action::Update,
        }
    }
}

#[derive(Default)]
pub(crate) struct State {
    pub menu: Option<iced::Point>,
    pub dialog: Option<Dialog>,
}

pub(crate) struct Dialog {
    pub action: Action,
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone)]
pub(crate) struct Target {
    pub key: BranchKey,
    pub branch: Branch,
    pub remote: String,
    pub error: Option<Error>,
}

impl Target {
    pub(crate) fn operation(&self, action: Action) -> Operation {
        match action {
            Action::Pull => Operation::Update(self.key.reference.clone()),
            Action::Delete => Operation::Delete(self.key.reference.clone()),
            Action::Push => Operation::Push {
                source: self.key.reference.clone(),
                remote: self.remote.clone(),
                name: self.branch.name.clone(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Outcome {
    pub(crate) target: Target,
    pub(crate) result: Result<(), Error>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Choose(Action),
    Remote(PathBuf, String),
    Submit,
    Finished(u64, u64, Vec<Outcome>),
}
