//! Execute explicit Git operations using validated references and bounded processes.

use crate::domain::git::{Error, Operation};
use crate::infrastructure::process;
use std::path::{Path, PathBuf};

async fn run(root: &Path, args: &[&str]) -> Result<String, Error> {
    let mut options = vec![
        "--no-pager",
        "-c",
        "color.ui=false",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "merge.autoStash=false",
        "-c",
        "rebase.autoStash=false",
    ];
    options.extend_from_slice(args);
    let bytes = process::run_with_timeout(
        "git",
        &options,
        Some(root),
        std::time::Duration::from_secs(180),
    )
    .await
    .map_err(|_| Error::Failed)?;
    String::from_utf8(bytes).map_err(|_| Error::Failed)
}

async fn reference(root: &Path, value: &str) -> Result<(), Error> {
    if !["refs/heads/", "refs/remotes/", "refs/tags/"]
        .iter()
        .any(|prefix| value.starts_with(prefix))
    {
        return Err(Error::InvalidName);
    }
    run(root, &["check-ref-format", value])
        .await
        .map_err(|_| Error::InvalidName)?;
    run(root, &["show-ref", "--verify", "--quiet", value]).await?;
    Ok(())
}

async fn name(root: &Path, value: &str) -> Result<(), Error> {
    if value.is_empty() || value.starts_with('-') || value == "HEAD" {
        return Err(Error::InvalidName);
    }
    run(root, &["check-ref-format", &format!("refs/heads/{value}")])
        .await
        .map_err(|_| Error::InvalidName)?;
    Ok(())
}

async fn clean(root: &Path) -> Result<(), Error> {
    if !run(
        root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )
    .await?
    .is_empty()
    {
        return Err(Error::Dirty);
    }
    Ok(())
}

async fn remote(root: &Path, value: &str) -> Result<(), Error> {
    if !run(root, &["remote"])
        .await?
        .lines()
        .any(|item| item == value)
    {
        return Err(Error::InvalidName);
    }
    Ok(())
}

async fn remote_branch(root: &Path, value: &str) -> Result<(String, String), Error> {
    let remotes = run(root, &["remote"]).await?;
    let mut remotes = remotes.lines().collect::<Vec<_>>();
    remotes.sort_by_key(|name| std::cmp::Reverse(name.len()));
    for remote in remotes {
        if let Some(branch) = value.strip_prefix(&format!("refs/remotes/{remote}/")) {
            return Ok((remote.to_owned(), branch.to_owned()));
        }
    }
    Err(Error::InvalidName)
}

pub(crate) async fn execute(root: PathBuf, operation: Operation) -> Result<(), Error> {
    match operation {
        Operation::Fetch => {
            run(&root, &["fetch", "--all"]).await?;
        }
        Operation::Checkout(source) => {
            reference(&root, &source).await?;
            clean(&root).await?;
            if let Some(local) = source.strip_prefix("refs/heads/") {
                run(&root, &["switch", "--no-overwrite-ignore", "--", local]).await?;
            } else if source.starts_with("refs/remotes/") {
                let (_, branch) = remote_branch(&root, &source).await?;
                name(&root, &branch).await?;
                run(
                    &root,
                    &[
                        "switch",
                        "--no-overwrite-ignore",
                        "--track",
                        "--create",
                        &branch,
                        &source,
                    ],
                )
                .await?;
            } else {
                run(
                    &root,
                    &["switch", "--no-overwrite-ignore", "--detach", "--", &source],
                )
                .await?;
            }
        }
        Operation::Create {
            source,
            name: new,
            checkout,
        } => {
            reference(&root, &source).await?;
            name(&root, &new).await?;
            if checkout {
                clean(&root).await?;
                run(
                    &root,
                    &["switch", "--no-overwrite-ignore", "--create", &new, &source],
                )
                .await?;
            } else {
                run(&root, &["branch", "--", &new, &source]).await?;
            }
        }
        Operation::Update(source) => {
            reference(&root, &source).await?;
            if source.starts_with("refs/remotes/") {
                let (remote, _) = remote_branch(&root, &source).await?;
                run(&root, &["fetch", "--", &remote]).await?;
                return Ok(());
            }
            if !source.starts_with("refs/heads/") {
                return Err(Error::InvalidName);
            }
            let upstream = run(
                &root,
                &[
                    "for-each-ref",
                    "--format=%(upstream:remotename)%00%(upstream:remoteref)",
                    &source,
                ],
            )
            .await?;
            let (remote_name, upstream_ref) = upstream
                .trim_end()
                .split_once('\0')
                .ok_or(Error::MissingUpstream)?;
            if remote_name.is_empty() || !upstream_ref.starts_with("refs/heads/") {
                return Err(Error::MissingUpstream);
            }
            if remote_name != "." {
                remote(&root, remote_name).await?;
            }
            let current = run(&root, &["symbolic-ref", "--quiet", "HEAD"])
                .await
                .unwrap_or_default();
            if current.trim() == source {
                clean(&root).await?;
                run(
                    &root,
                    &[
                        "pull",
                        "--ff-only",
                        "--no-rebase",
                        "--no-autostash",
                        "--",
                        remote_name,
                        upstream_ref,
                    ],
                )
                .await?;
            } else {
                // Git rejects non-fast-forward updates and branches checked out in any worktree.
                run(
                    &root,
                    &[
                        "fetch",
                        "--no-tags",
                        "--",
                        remote_name,
                        &format!("{upstream_ref}:{source}"),
                    ],
                )
                .await?;
            }
        }
        Operation::Push {
            source,
            remote: target,
            name: destination,
        } => {
            reference(&root, &source).await?;
            if !source.starts_with("refs/heads/") {
                return Err(Error::InvalidName);
            }
            name(&root, &destination).await?;
            remote(&root, &target).await?;
            run(
                &root,
                &[
                    "-c",
                    &format!("remote.{target}.mirror=false"),
                    "push",
                    "--set-upstream",
                    "--no-follow-tags",
                    "--no-mirror",
                    "--",
                    &target,
                    &format!("{source}:refs/heads/{destination}"),
                ],
            )
            .await?;
        }
        Operation::Rename { source, name: new } => {
            reference(&root, &source).await?;
            name(&root, &new).await?;
            let old = source
                .strip_prefix("refs/heads/")
                .ok_or(Error::InvalidName)?;
            run(&root, &["branch", "--move", "--", old, &new]).await?;
        }
        Operation::Delete(source) => {
            reference(&root, &source).await?;
            if let Some(local) = source.strip_prefix("refs/heads/") {
                let current = run(&root, &["symbolic-ref", "--quiet", "HEAD"])
                    .await
                    .unwrap_or_default();
                if current.trim() == source {
                    return Err(Error::CurrentBranch);
                }
                let worktrees = run(&root, &["worktree", "list", "--porcelain", "-z"]).await?;
                if worktrees
                    .split('\0')
                    .any(|field| field.strip_prefix("branch ") == Some(source.as_str()))
                {
                    return Err(Error::BranchInWorktree);
                }
                run(&root, &["branch", "--delete", "--", local]).await?;
            } else if source.starts_with("refs/remotes/") {
                let (remote, branch) = remote_branch(&root, &source).await?;
                run(
                    &root,
                    &[
                        "-c",
                        &format!("remote.{remote}.mirror=false"),
                        "push",
                        "--no-follow-tags",
                        "--no-mirror",
                        "--",
                        &remote,
                        &format!(":refs/heads/{branch}"),
                    ],
                )
                .await?;
            } else {
                return Err(Error::InvalidName);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
