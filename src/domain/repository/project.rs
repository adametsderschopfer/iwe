//! Keep repository identities separate in a shared project history.

use super::{Commit, GraphRow, HistoryCursor, Repository};
use crate::domain::Failure;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BranchKey {
    pub root: PathBuf,
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommitKey {
    pub root: PathBuf,
    pub id: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ScopedCommit {
    pub root: PathBuf,
    pub commit: Commit,
}

impl ScopedCommit {
    pub(crate) fn key(&self) -> CommitKey {
        CommitKey {
            root: self.root.clone(),
            id: self.commit.id.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HistorySource {
    pub root: PathBuf,
    pub buffered: VecDeque<Commit>,
    pub next: Option<HistoryCursor>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectHistoryCursor {
    pub sources: Vec<HistorySource>,
    pub lanes: Vec<CommitKey>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectHistoryPage {
    pub commits: Vec<ScopedCommit>,
    pub next: Option<ProjectHistoryCursor>,
}

#[derive(Debug, Clone)]
pub(crate) struct RepositorySet {
    pub repositories: Vec<Repository>,
    pub active_root: PathBuf,
    pub commits: Vec<ScopedCommit>,
    pub next_history: Option<ProjectHistoryCursor>,
    pub failures: Vec<(PathBuf, Failure)>,
}

impl RepositorySet {
    pub(crate) fn get(&self, root: &Path) -> Option<&Repository> {
        self.repositories.iter().find(|repo| repo.root == root)
    }

    pub(crate) fn active(&self) -> Option<&Repository> {
        self.get(&self.active_root)
    }

    pub(crate) fn active_mut(&mut self) -> Option<&mut Repository> {
        self.repositories
            .iter_mut()
            .find(|repo| repo.root == self.active_root)
    }

    pub(crate) fn commit(&self, key: &CommitKey) -> Option<&Commit> {
        self.commits
            .iter()
            .find(|entry| entry.root == key.root && entry.commit.id == key.id)
            .map(|entry| &entry.commit)
    }
}

impl From<Repository> for RepositorySet {
    fn from(mut repo: Repository) -> Self {
        let root = repo.root.clone();
        let commits = std::mem::take(&mut repo.commits)
            .into_iter()
            .map(|commit| ScopedCommit {
                root: root.clone(),
                commit,
            })
            .collect();
        let next_history = repo.next_history.take().map(|cursor| ProjectHistoryCursor {
            lanes: cursor
                .lanes
                .iter()
                .map(|id| CommitKey {
                    root: root.clone(),
                    id: id.clone(),
                })
                .collect(),
            sources: vec![HistorySource {
                root: root.clone(),
                buffered: VecDeque::new(),
                next: Some(cursor),
            }],
        });
        Self {
            repositories: vec![repo],
            active_root: root,
            commits,
            next_history,
            failures: Vec::new(),
        }
    }
}

pub(crate) fn assign_graph(
    commits: &mut [ScopedCommit],
    mut lanes: Vec<CommitKey>,
) -> Vec<CommitKey> {
    for entry in commits {
        let previous = lanes.clone();
        let key = entry.key();
        let lane = lanes
            .iter()
            .position(|value| value == &key)
            .unwrap_or_else(|| {
                lanes.push(key);
                lanes.len() - 1
            });
        let before = lanes.clone();
        lanes.remove(lane);
        let parents: Vec<_> = entry
            .commit
            .parents
            .iter()
            .map(|id| CommitKey {
                root: entry.root.clone(),
                id: id.clone(),
            })
            .collect();
        for (index, parent) in parents.iter().enumerate() {
            if !lanes.contains(parent) {
                lanes.insert((lane + index).min(lanes.len()), parent.clone());
            }
        }
        let incoming = previous
            .iter()
            .enumerate()
            .map(|(index, _)| (index, index))
            .collect();
        let mut outgoing = Vec::new();
        for (index, id) in before.iter().enumerate() {
            if index != lane
                && let Some(next) = lanes.iter().position(|target| target == id)
            {
                outgoing.push((index, next));
            }
        }
        for parent in parents {
            if let Some(next) = lanes.iter().position(|target| target == &parent) {
                outgoing.push((lane, next));
            }
        }
        entry.commit.graph = GraphRow {
            lane,
            incoming,
            outgoing,
        };
    }
    lanes
}

#[cfg(test)]
mod tests;
