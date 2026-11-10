//! Confirm branch batches and execute each operation once in its owning repository.

use super::{App, AppMessage, Error, Operation, allowed};
use crate::{
    domain::repository::{Branch, project::BranchKey},
    i18n::{FluentArgs, Key},
    infrastructure::git,
};
use iced::Task;
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
    fn single(self) -> super::Action {
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
    fn operation(&self, action: Action) -> Operation {
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
    target: Target,
    result: Result<(), Error>,
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Choose(Action),
    Remote(PathBuf, String),
    Submit,
    Finished(u64, u64, Vec<Outcome>),
}

impl App {
    fn selected_branch_targets(&self) -> Vec<Target> {
        let Some(repos) = &self.repository else {
            return Vec::new();
        };
        self.branch_selection
            .selected
            .iter()
            .filter_map(|key| {
                let repo = repos.get(&key.root)?;
                let branch = repo
                    .branches
                    .iter()
                    .find(|branch| branch.reference == key.reference)?
                    .clone();
                let remote = repo
                    .remotes
                    .iter()
                    .find(|remote| remote.as_str() == "origin")
                    .or(repo.remotes.first())
                    .cloned()
                    .unwrap_or_default();
                Some(Target {
                    key: key.clone(),
                    branch,
                    remote,
                    error: None,
                })
            })
            .collect()
    }

    pub(crate) fn bulk_allowed(&self, action: Action) -> bool {
        let targets = self.selected_branch_targets();
        !self.git_actions.pending
            && !targets.is_empty()
            && targets.len() == self.branch_selection.selected.len()
            && targets.iter().all(|target| {
                allowed(action.single(), &target.branch)
                    && (action != Action::Push || !target.remote.is_empty())
            })
    }

    pub(super) fn update_bulk_git(&mut self, event: Event) -> Task<AppMessage> {
        match event {
            Event::Finished(request, generation, outcomes)
                if self.git_actions.pending
                    && request == self.git_actions.request
                    && self.git_actions.bulk.dialog.is_some() =>
            {
                self.git_actions.pending = false;
                let Some(mut dialog) = self.git_actions.bulk.dialog.take() else {
                    return Task::none();
                };
                if generation != self.generation {
                    return Task::none();
                }
                let mut success = 0;
                let mut failures = Vec::new();
                for outcome in outcomes {
                    match outcome.result {
                        Ok(()) => success += 1,
                        Err(error) => {
                            let mut target = outcome.target;
                            target.error = Some(error);
                            failures.push(target);
                        }
                    }
                }
                let mut args = FluentArgs::new();
                args.set("success", success);
                args.set("failed", failures.len());
                let summary = Some(self.i18n.format(Key::GitBulkResult, &args));
                if failures.is_empty() {
                    self.notifications.success(Key::GitBulkDone, summary);
                } else {
                    self.notifications.error(Key::GitBulkFailed, summary);
                    dialog.targets = failures;
                    self.git_actions.bulk.dialog = Some(dialog);
                }
                return self.load_directory();
            }
            _ if self.git_actions.pending
                || self.space_draft.is_some()
                || self.auth_dialog.is_some()
                || self.line_navigation.is_some() => {}
            Event::Choose(action)
                if self.bulk_allowed(action) && !self.git_actions.has_dialog() =>
            {
                let targets = self.selected_branch_targets();
                self.git_actions.menu = None;
                self.git_actions.bulk.menu = None;
                self.git_actions.bulk.dialog = Some(Dialog { action, targets });
            }
            Event::Remote(root, remote) => {
                if self
                    .repository
                    .as_ref()
                    .and_then(|repos| repos.get(&root))
                    .is_some_and(|repo| repo.remotes.contains(&remote))
                    && let Some(dialog) = &mut self.git_actions.bulk.dialog
                {
                    for target in &mut dialog.targets {
                        if target.key.root == root {
                            target.remote = remote.clone();
                            target.error = None;
                        }
                    }
                }
            }
            Event::Submit => {
                let Some(dialog) = &mut self.git_actions.bulk.dialog else {
                    return Task::none();
                };
                if dialog.targets.is_empty()
                    || dialog.targets.iter().any(|target| {
                        !allowed(dialog.action.single(), &target.branch)
                            || (dialog.action == Action::Push && target.remote.is_empty())
                    })
                {
                    return Task::none();
                }
                for target in &mut dialog.targets {
                    target.error = None;
                }
                let action = dialog.action;
                let targets = dialog.targets.clone();
                self.git_actions.pending = true;
                self.git_actions.request += 1;
                let (request, generation) = (self.git_actions.request, self.generation);
                return Task::perform(
                    async move {
                        let mut outcomes = Vec::new();
                        for target in targets {
                            let result =
                                git::execute(target.key.root.clone(), target.operation(action))
                                    .await;
                            outcomes.push(Outcome { target, result });
                        }
                        outcomes
                    },
                    move |outcomes| {
                        AppMessage::Git(super::Message::Bulk(Event::Finished(
                            request, generation, outcomes,
                        )))
                    },
                );
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
