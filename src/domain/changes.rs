//! Describe file status and source comparisons without filesystem access.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FileStatus {
    Untracked,
    Added,
    Deleted,
    Renamed,
    Modified,
    Conflict,
}

impl FileStatus {
    pub(crate) fn badge(self) -> &'static str {
        match self {
            Self::Untracked => "U",
            Self::Added => "A",
            Self::Deleted => "D",
            Self::Renamed => "R",
            Self::Modified => "M",
            Self::Conflict => "!",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileChange {
    pub status: FileStatus,
    pub original: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Worktree {
    pub files: BTreeMap<PathBuf, FileChange>,
    pub folders: BTreeMap<PathBuf, FileStatus>,
}

impl Worktree {
    pub(crate) fn new(scope: &Path, files: BTreeMap<PathBuf, FileChange>) -> Self {
        let mut folders = BTreeMap::<PathBuf, FileStatus>::new();
        for (path, change) in &files {
            for parent in path
                .ancestors()
                .skip(1)
                .take_while(|path| path.starts_with(scope))
            {
                folders
                    .entry(parent.to_owned())
                    .and_modify(|status| *status = (*status).max(change.status))
                    .or_insert(change.status);
            }
        }
        Self { files, folders }
    }

    pub(crate) fn status(&self, path: &Path, directory: bool) -> Option<FileStatus> {
        self.files
            .get(path)
            .map(|change| change.status)
            .or_else(|| directory.then(|| self.folders.get(path).copied()).flatten())
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Source {
    pub text: String,
    pub previous: Option<String>,
    pub deleted: bool,
    pub comparison_failed: bool,
}
