//! Select branch identities independently from the history filter.

use super::{App, Message};
use crate::domain::repository::{
    ReferenceKind,
    project::{BranchKey, RepositorySet},
};
use iced::{Point, Task, keyboard, window};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Default)]
pub(crate) struct State {
    pub selected: BTreeSet<BranchKey>,
    pub modifiers: keyboard::Modifiers,
}

#[derive(Debug, Clone)]
pub(crate) enum Target {
    Branch(BranchKey),
    Group {
        root: PathBuf,
        kind: Option<ReferenceKind>,
        prefix: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum Event {
    Modifiers(window::Id, keyboard::Modifiers),
    Unfocused(window::Id),
    Toggle(Target),
    Menu(Target, Point),
    Clear,
}

impl Target {
    pub(crate) fn keys(&self, repositories: &RepositorySet) -> Vec<BranchKey> {
        let root = match self {
            Self::Branch(key) => &key.root,
            Self::Group { root, .. } => root,
        };
        let Some(repo) = repositories.get(root) else {
            return Vec::new();
        };
        repo.branches
            .iter()
            .filter(|branch| {
                branch.kind != ReferenceKind::Tag
                    && match self {
                        Self::Branch(key) => key.reference == branch.reference,
                        Self::Group { kind, prefix, .. } => {
                            kind.is_none_or(|kind| branch.kind == kind)
                                && prefix.as_ref().is_none_or(|prefix| {
                                    !branch.current && branch.name.starts_with(prefix)
                                })
                        }
                    }
            })
            .map(|branch| BranchKey {
                root: root.clone(),
                reference: branch.reference.clone(),
            })
            .collect()
    }
}

impl State {
    pub(crate) fn command(&self) -> bool {
        self.modifiers.contains(keyboard::Modifiers::COMMAND)
    }

    pub(crate) fn covers(&self, keys: &[BranchKey]) -> bool {
        !keys.is_empty() && keys.iter().all(|key| self.selected.contains(key))
    }

    pub(crate) fn prune(&mut self, repositories: &RepositorySet) {
        self.selected.retain(|key| {
            repositories.get(&key.root).is_some_and(|repo| {
                repo.branches.iter().any(|branch| {
                    branch.reference == key.reference && branch.kind != ReferenceKind::Tag
                })
            })
        });
    }
}

impl App {
    pub(super) fn update_branch_selection(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Modifiers(id, modifiers) if self.window_id == Some(id) => {
                self.branch_selection.modifiers = modifiers
            }
            Event::Unfocused(id) if self.window_id == Some(id) => {
                self.branch_selection.modifiers = keyboard::Modifiers::empty();
                self.branch_selection.selected.clear();
                self.git_actions.bulk.menu = None;
            }
            Event::Clear => {
                self.branch_selection.selected.clear();
                self.git_actions.bulk.menu = None;
            }
            _ if self.git_actions.pending
                || self.git_actions.has_dialog()
                || self.space_draft.is_some()
                || self.auth_dialog.is_some()
                || self.line_navigation.is_some() => {}
            Event::Toggle(target) => {
                if let Some(repo) = &self.repository {
                    let keys = target.keys(repo);
                    if self.branch_selection.covers(&keys) {
                        for key in keys {
                            self.branch_selection.selected.remove(&key);
                        }
                    } else {
                        self.branch_selection.selected.extend(keys);
                    }
                    self.git_actions.menu = None;
                    self.git_actions.bulk.menu = None;
                }
            }
            Event::Menu(target, point) => {
                let matches = self.repository.as_ref().is_some_and(|repo| {
                    target
                        .keys(repo)
                        .iter()
                        .any(|key| self.branch_selection.selected.contains(key))
                });
                self.git_actions.menu = None;
                self.git_actions.bulk.menu = matches.then_some(point);
                if !matches {
                    self.branch_selection.selected.clear();
                }
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
