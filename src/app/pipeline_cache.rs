//! Keep recent pipeline lists in memory without retaining credentials.

use crate::domain::repository::{Pipeline, Remote};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

const MAX_REPOSITORIES: usize = 8;

#[derive(Default)]
pub(super) struct Cache {
    entries: VecDeque<Entry>,
}

struct Entry {
    root: PathBuf,
    remote: Remote,
    runs: Vec<Pipeline>,
}

impl Cache {
    pub(super) fn get(&mut self, root: &Path, remote: &Remote) -> Option<Vec<Pipeline>> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.root.as_path() == root && &entry.remote == remote)?;
        let entry = self.entries.remove(index)?;
        let runs = entry.runs.clone();
        self.entries.push_back(entry);
        Some(runs)
    }

    pub(super) fn insert(&mut self, root: PathBuf, remote: Remote, runs: Vec<Pipeline>) {
        self.entries
            .retain(|entry| entry.root != root || entry.remote != remote);
        self.entries.push_back(Entry { root, remote, runs });
        while self.entries.len() > MAX_REPOSITORIES {
            self.entries.pop_front();
        }
    }

    pub(super) fn clear(&mut self) {
        self.entries.clear();
    }
}
