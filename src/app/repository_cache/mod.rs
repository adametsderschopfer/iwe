//! Keep initial Git snapshots for recently opened projects in memory.

use crate::domain::repository::project::{BranchKey, RepositorySet};
use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
};

const MAX_PROJECTS: usize = 8;

#[derive(Clone)]
pub(super) struct Snapshot {
    pub repository: RepositorySet,
    pub branch: Option<BranchKey>,
    pub folders: BTreeSet<String>,
}

#[derive(Default)]
pub(super) struct Cache {
    entries: VecDeque<(PathBuf, Snapshot)>,
}

impl Cache {
    pub(super) fn insert(&mut self, root: PathBuf, snapshot: Snapshot) {
        self.remove(&root);
        self.entries.push_back((root, snapshot));
        while self.entries.len() > MAX_PROJECTS {
            self.entries.pop_front();
        }
    }

    pub(super) fn get(&mut self, root: &Path) -> Option<Snapshot> {
        let index = self.entries.iter().position(|(path, _)| path == root)?;
        let entry = self.entries.remove(index)?;
        let snapshot = entry.1.clone();
        self.entries.push_back(entry);
        Some(snapshot)
    }

    pub(super) fn remember_folders(
        &mut self,
        root: &Path,
        branch: &Option<BranchKey>,
        folders: &BTreeSet<String>,
    ) {
        if let Some((_, snapshot)) = self
            .entries
            .iter_mut()
            .find(|(path, snapshot)| path == root && &snapshot.branch == branch)
        {
            snapshot.folders.clone_from(folders);
        }
    }

    pub(super) fn remove(&mut self, root: &Path) {
        self.entries.retain(|(path, _)| path != root);
    }
}

#[cfg(test)]
mod tests;
