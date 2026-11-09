//! Discover the opened repository and direct child repositories without recursion.

use super::{Failure, git};
use std::path::{Path, PathBuf};

pub(super) struct Discovery {
    pub scope: PathBuf,
    pub roots: Vec<PathBuf>,
    pub failures: Vec<(PathBuf, Failure)>,
}

pub(super) async fn root(directory: &Path) -> Result<PathBuf, Failure> {
    let bytes = git(directory, &["rev-parse", "--show-toplevel"])
        .await
        .map_err(|error| {
            if error == Failure::MissingTool {
                error
            } else {
                Failure::NotRepository
            }
        })?;
    let value = String::from_utf8(bytes).map_err(|_| Failure::Read)?;
    tokio::fs::canonicalize(PathBuf::from(value.trim_end_matches(['\r', '\n'])))
        .await
        .map_err(|_| Failure::Read)
}

pub(super) async fn discover(directory: &Path) -> Result<Discovery, Failure> {
    let scope = tokio::fs::canonicalize(directory)
        .await
        .map_err(|_| Failure::Read)?;
    let mut roots = Vec::new();
    let mut failures = Vec::new();
    let mut has_parent = false;
    for parent in scope.ancestors() {
        if tokio::fs::try_exists(parent.join(".git"))
            .await
            .unwrap_or(false)
        {
            has_parent = true;
            break;
        }
    }
    if has_parent {
        match root(&scope).await {
            Ok(root) => roots.push(root),
            Err(Failure::NotRepository) => {
                if tokio::fs::try_exists(scope.join(".git"))
                    .await
                    .unwrap_or(false)
                {
                    failures.push((scope.clone(), Failure::NotRepository));
                }
            }
            Err(error) => return Err(error),
        }
    }
    let mut entries = tokio::fs::read_dir(&scope)
        .await
        .map_err(|_| Failure::Read)?;
    let mut children = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(|_| Failure::Read)? {
        let child = entry.path();
        if entry.file_name() == ".git" {
            continue;
        }
        match entry.file_type().await {
            Ok(kind) if kind.is_dir() => {}
            Ok(_) => continue,
            Err(_) => {
                failures.push((child, Failure::Read));
                continue;
            }
        }
        match tokio::fs::try_exists(child.join(".git")).await {
            Ok(true) => children.push(child),
            Ok(false) => {}
            Err(_) => failures.push((child, Failure::Read)),
        }
    }
    children.sort();
    for child in children {
        match root(&child).await {
            Ok(root) if root == child && !roots.contains(&root) => roots.push(root),
            Ok(_) => {}
            Err(Failure::MissingTool) => return Err(Failure::MissingTool),
            Err(error) => failures.push((child, error)),
        }
    }
    if roots.is_empty() {
        return Err(failures
            .first()
            .map_or(Failure::NotRepository, |(_, error)| *error));
    }
    Ok(Discovery {
        scope,
        roots,
        failures,
    })
}

pub(super) fn owner<'a>(roots: &'a [PathBuf], path: &Path) -> Option<&'a PathBuf> {
    roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
}
