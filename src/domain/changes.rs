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
    pub(crate) fn from_git(status: char) -> Self {
        match status {
            'A' => Self::Added,
            'D' => Self::Deleted,
            'R' => Self::Renamed,
            'U' => Self::Conflict,
            _ => Self::Modified,
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
    pub roots: Vec<PathBuf>,
    pub failures: Vec<(PathBuf, crate::domain::Failure)>,
    pub comparison_scope: Option<PathBuf>,
    pub comparison_roots: Vec<PathBuf>,
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
        Self {
            files,
            folders,
            ..Self::default()
        }
    }

    pub(crate) fn status(&self, path: &Path, directory: bool) -> Option<FileStatus> {
        self.files
            .get(path)
            .map(|change| change.status)
            .or_else(|| directory.then(|| self.folders.get(path).copied()).flatten())
    }
}

/// Bound per-line comparison metadata independently of source byte limits.
pub(crate) const PREVIEW_LINES: usize = 100_000;

#[derive(Debug, Clone, Default)]
pub(crate) struct Source {
    pub text: String,
    pub previous: Option<String>,
    pub deleted: bool,
    pub comparison_failed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextCursor {
    pub offset: u64,
    pub line: usize,
}

impl Default for TextCursor {
    fn default() -> Self {
        Self { offset: 0, line: 1 }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TextPage {
    pub cursor: TextCursor,
    pub next: Option<TextCursor>,
    pub version: Option<(u64, u128)>,
}
