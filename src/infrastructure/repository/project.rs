//! Merge bounded history streams while preserving each repository's topology.

use super::{Failure, HISTORY_PAGE_SIZE, discovery, load, load_history, load_reference};
use crate::domain::repository::project::{
    BranchKey, HistorySource, ProjectHistoryCursor, ProjectHistoryPage, RepositorySet,
    ScopedCommit, assign_graph,
};
use std::path::PathBuf;

pub(crate) async fn load_project(
    directory: PathBuf,
    selection: Option<BranchKey>,
) -> Result<RepositorySet, Failure> {
    let discovery = discovery::discover(&directory).await?;
    let mut failures = discovery.failures;
    let multiple = discovery.roots.len() > 1;
    let mut repositories = Vec::new();
    for root in discovery.roots {
        let reference = selection
            .as_ref()
            .filter(|key| key.root == root)
            .map(|key| key.reference.clone());
        let mut loaded = if multiple && reference.is_none() {
            load_reference(root.clone(), None, true).await
        } else {
            load(root.clone(), reference.clone()).await
        };
        if reference.is_some() && loaded.as_ref().is_err_and(|error| *error == Failure::Read) {
            loaded = load_reference(root.clone(), None, multiple).await;
        }
        match loaded {
            Ok(repo) => repositories.push(repo),
            Err(error) => failures.push((root, error)),
        }
    }
    let Some(first) = repositories.first() else {
        return Err(failures
            .first()
            .map_or(Failure::NotRepository, |(_, error)| *error));
    };
    let selected = selection.as_ref().filter(|key| {
        repositories.iter().any(|repo| {
            repo.root == key.root
                && repo
                    .branches
                    .iter()
                    .any(|branch| branch.reference == key.reference)
        })
    });
    let active_root = selected.map_or_else(|| first.root.clone(), |key| key.root.clone());
    let sources = repositories
        .iter_mut()
        .filter(|repo| selected.is_none_or(|key| repo.root == key.root))
        .map(|repo| HistorySource {
            root: repo.root.clone(),
            buffered: std::mem::take(&mut repo.commits).into(),
            next: repo.next_history.take(),
        })
        .collect();
    for repo in &mut repositories {
        repo.commits.clear();
        repo.next_history = None;
    }
    let history = load_project_history(ProjectHistoryCursor {
        sources,
        lanes: Vec::new(),
    })
    .await?;
    Ok(RepositorySet {
        repositories,
        active_root,
        commits: history.commits,
        next_history: history.next,
        failures,
    })
}

pub(crate) async fn load_project_history(
    mut cursor: ProjectHistoryCursor,
) -> Result<ProjectHistoryPage, Failure> {
    let mut commits = Vec::with_capacity(HISTORY_PAGE_SIZE);
    while commits.len() < HISTORY_PAGE_SIZE {
        // Refill before choosing a head so older rows never precede an unread newer page.
        for source in &mut cursor.sources {
            if source.buffered.is_empty()
                && let Some(next) = source.next.take()
            {
                let page = load_history(source.root.clone(), next).await?;
                source.buffered = page.commits.into();
                source.next = page.next;
            }
        }
        let source = cursor
            .sources
            .iter_mut()
            .filter(|source| !source.buffered.is_empty())
            .max_by(|left, right| {
                left.buffered
                    .front()
                    .unwrap()
                    .timestamp
                    .cmp(&right.buffered.front().unwrap().timestamp)
                    .then_with(|| right.root.cmp(&left.root))
            });
        let Some(source) = source else { break };
        commits.push(ScopedCommit {
            root: source.root.clone(),
            commit: source.buffered.pop_front().unwrap(),
        });
    }
    cursor.lanes = assign_graph(&mut commits, cursor.lanes);
    let more = cursor
        .sources
        .iter()
        .any(|source| !source.buffered.is_empty() || source.next.is_some());
    Ok(ProjectHistoryPage {
        commits,
        next: more.then_some(cursor),
    })
}

#[cfg(test)]
mod tests;

/// Read provider endpoints without loading Git history or branch lists.
pub(crate) async fn load_project_remotes(directory: PathBuf) -> Result<RepositorySet, Failure> {
    let found = discovery::discover(&directory).await?;
    let mut failures = found.failures;
    let mut repositories = Vec::new();
    for root in found.roots {
        match super::load_remote(root.clone()).await {
            Ok(repository) => repositories.push(repository),
            Err(error) => failures.push((root, error)),
        }
    }
    let active_root = repositories
        .first()
        .ok_or(Failure::NotRepository)?
        .root
        .clone();
    Ok(RepositorySet {
        repositories,
        active_root,
        commits: Vec::new(),
        next_history: None,
        failures,
    })
}
