//! Select branch identities independently from the history filter.

use crate::domain::repository::{
    ReferenceKind,
    project::{BranchKey, RepositorySet},
};
use iced::{Point, keyboard, window};
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
