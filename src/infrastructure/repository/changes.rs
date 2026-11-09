//! Read working-tree status and bounded source versions for comparison.

use super::{blob, discovery, git, valid_id, workspace};
use crate::domain::{
    Failure,
    changes::{FileChange, FileStatus, Source, Worktree},
    repository::{ChangedFile, Commit},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

fn parse_status(bytes: &[u8], root: &Path) -> BTreeMap<PathBuf, FileChange> {
    let mut result = BTreeMap::new();
    let mut fields = bytes.split(|byte| *byte == 0);
    while let Some(field) = fields.next() {
        if field.len() < 4 {
            continue;
        }
        let xy = &field[..2];
        let renamed = xy.contains(&b'R') || xy.contains(&b'C');
        let original = renamed
            .then(|| fields.next())
            .flatten()
            .and_then(|path| std::str::from_utf8(path).ok())
            .map(|path| root.join(path));
        let Ok(path) = std::str::from_utf8(&field[3..]) else {
            continue;
        };
        let status = if xy == b"??" {
            FileStatus::Untracked
        } else if xy.contains(&b'U') || xy == b"AA" || xy == b"DD" {
            FileStatus::Conflict
        } else if xy.contains(&b'D') {
            FileStatus::Deleted
        } else if renamed {
            FileStatus::Renamed
        } else if xy.contains(&b'A') {
            FileStatus::Added
        } else if xy.contains(&b'M') || xy.contains(&b'T') {
            FileStatus::Modified
        } else {
            continue;
        };
        result.insert(root.join(path), FileChange { status, original });
    }
    result
}

pub(crate) async fn working_status(directory: PathBuf) -> Result<Worktree, Failure> {
    let discovery = discovery::discover(&directory).await?;
    let mut files = BTreeMap::new();
    let mut roots = Vec::new();
    let mut failures = discovery.failures;
    for root in &discovery.roots {
        let bytes = match git(
            root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
        )
        .await
        {
            Ok(bytes) => bytes,
            Err(error) => {
                failures.push((root.clone(), error));
                continue;
            }
        };
        roots.push(
            root.strip_prefix(&discovery.scope)
                .map_or_else(|_| directory.clone(), |path| directory.join(path)),
        );
        for (path, mut change) in parse_status(&bytes, root) {
            if discovery::owner(&discovery.roots, &path) != Some(root) {
                continue;
            }
            let Ok(relative) = path.strip_prefix(&discovery.scope) else {
                continue;
            };
            change.original = change.original.and_then(|path| {
                path.strip_prefix(&discovery.scope)
                    .ok()
                    .map(|path| directory.join(path))
            });
            files.insert(directory.join(relative), change);
        }
    }
    if roots.is_empty() {
        return Err(failures.first().map_or(Failure::Read, |(_, error)| *error));
    }
    let mut status = Worktree::new(&directory, files);
    status.roots = roots;
    status.failures = failures;
    Ok(status)
}

async fn baseline(root: &Path, revision: &str, path: &str) -> Result<Option<String>, Failure> {
    if !valid_id(revision) {
        return Err(Failure::Read);
    }
    let entry = git(
        root,
        &["--literal-pathspecs", "ls-tree", "-z", revision, "--", path],
    )
    .await?;
    if entry.is_empty() {
        return Ok(None);
    }
    // Submodule entries contain commit objects instead of source blobs.
    if entry
        .split(|byte| *byte == b' ')
        .nth(1)
        .is_none_or(|kind| kind != b"blob")
    {
        return Ok(None);
    }
    blob(root.to_owned(), revision.to_owned(), path.to_owned())
        .await
        .map(Some)
}

async fn working_baseline(directory: &Path, path: &Path) -> Result<Option<String>, Failure> {
    let discovery = match discovery::discover(directory).await {
        Ok(discovery) => discovery,
        Err(Failure::NotRepository | Failure::MissingTool) => return Ok(None),
        Err(error) => return Err(error),
    };
    let absolute = discovery
        .scope
        .join(path.strip_prefix(directory).map_err(|_| Failure::Read)?);
    let Some(root) = discovery::owner(&discovery.roots, &absolute) else {
        return Ok(None);
    };
    absolute.strip_prefix(root).map_err(|_| Failure::Read)?;
    // A path filter hides the deleted side of staged renames.
    let status = git(
        root,
        &[
            "--literal-pathspecs",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ],
    )
    .await?;
    let changes = parse_status(&status, root);
    let change = changes.get(&absolute);
    let head = git(root, &["rev-parse", "--revs-only", "HEAD"]).await?;
    let head = String::from_utf8(head).map_err(|_| Failure::Read)?;
    let Some(head) = head.lines().find(|line| valid_id(line)) else {
        return Ok(change.map(|_| String::new()));
    };
    let old_path = change
        .and_then(|change| change.original.as_ref())
        .unwrap_or(&absolute);
    let old_path = old_path
        .strip_prefix(root)
        .map_err(|_| Failure::Read)?
        .components()
        .map(|part| part.as_os_str().to_str().ok_or(Failure::Read))
        .collect::<Result<Vec<_>, _>>()?
        .join("/");
    let before = baseline(root, head, &old_path).await?;
    Ok(before.or_else(|| change.map(|_| String::new())))
}

pub(crate) async fn working_file(directory: PathBuf, path: PathBuf) -> Result<Source, Failure> {
    let text = working_text(directory.clone(), path.clone()).await?;
    Ok(working_comparison(directory, path, text).await)
}

pub(crate) async fn working_text(directory: PathBuf, path: PathBuf) -> Result<String, Failure> {
    match workspace::read(directory.clone(), path.clone()).await {
        Ok(text) => Ok(text),
        Err(error) => {
            if error == Failure::Read
                && !tokio::fs::try_exists(&path)
                    .await
                    .map_err(|_| Failure::Read)?
            {
                let status = working_status(directory.clone()).await?;
                if status
                    .files
                    .get(&path)
                    .is_some_and(|change| change.status == FileStatus::Deleted)
                {
                    return working_baseline(&directory, &path)
                        .await?
                        .ok_or(Failure::Read);
                }
            }
            Err(error)
        }
    }
}

pub(crate) async fn working_page(
    directory: PathBuf,
    path: PathBuf,
    cursor: crate::domain::changes::TextCursor,
    version: Option<(u64, u128)>,
) -> Result<(String, crate::domain::changes::TextPage), Failure> {
    if tokio::fs::try_exists(&path)
        .await
        .map_err(|_| Failure::Read)?
    {
        return workspace::pages::read(directory, path, cursor, version).await;
    }
    let status = working_status(directory.clone()).await?;
    if !status
        .files
        .get(&path)
        .is_some_and(|change| change.status == FileStatus::Deleted)
    {
        return Err(Failure::Read);
    }
    let discovery = discovery::discover(&directory).await?;
    let absolute = discovery
        .scope
        .join(path.strip_prefix(&directory).map_err(|_| Failure::Read)?);
    let root = discovery::owner(&discovery.roots, &absolute).ok_or(Failure::Read)?;
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| Failure::Read)?
        .components()
        .map(|part| part.as_os_str().to_str().ok_or(Failure::Read))
        .collect::<Result<Vec<_>, _>>()?
        .join("/");
    let head = git(root, &["rev-parse", "--verify", "HEAD"]).await?;
    let head = String::from_utf8(head).map_err(|_| Failure::Read)?;
    super::blob_page(root.clone(), head.trim().to_owned(), relative, cursor).await
}

pub(crate) async fn working_comparison(directory: PathBuf, path: PathBuf, text: String) -> Source {
    let comparison = working_baseline(&directory, &path).await;
    let deleted = matches!(tokio::fs::try_exists(&path).await, Ok(false))
        && comparison.as_ref().is_ok_and(|before| before.is_some());
    Source {
        text,
        deleted,
        comparison_failed: comparison.is_err(),
        previous: comparison.ok().flatten(),
    }
}

pub(crate) async fn commit_text(
    root: PathBuf,
    commit: Commit,
    file: ChangedFile,
) -> Result<String, Failure> {
    if file.status == 'D' {
        let parent = commit.parents.first().ok_or(Failure::Read)?;
        return blob(root, parent.clone(), file.path).await;
    }
    blob(root, commit.id, file.path).await
}

pub(crate) async fn commit_comparison(
    root: PathBuf,
    commit: Commit,
    file: ChangedFile,
    text: String,
) -> Source {
    if file.status == 'D' {
        return Source {
            text,
            deleted: true,
            ..Source::default()
        };
    }
    let previous = if let Some(parent) = commit.parents.first() {
        baseline(&root, parent, &file.path)
            .await
            .map(|text| Some(text.unwrap_or_default()))
    } else {
        Ok(Some(String::new()))
    };
    Source {
        text,
        comparison_failed: previous.is_err(),
        previous: previous.ok().flatten(),
        ..Source::default()
    }
}

#[cfg(test)]
async fn commit_file(root: PathBuf, commit: Commit, file: ChangedFile) -> Result<Source, Failure> {
    let text = commit_text(root.clone(), commit.clone(), file.clone()).await?;
    Ok(commit_comparison(root, commit, file, text).await)
}

#[cfg(test)]
mod tests;
