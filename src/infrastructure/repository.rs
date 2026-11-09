//! Read Git metadata and historical blobs without modifying the working tree.

mod changes;
mod discovery;
mod project;
pub(crate) use changes::{
    commit_comparison, commit_text, working_comparison, working_file, working_status, working_text,
    working_page,
};
pub(crate) use project::{load_project, load_project_history};

use super::{
    process::{self, Failure},
    workspace,
};
use crate::domain::repository::{
    Branch, ChangedFile, Commit, GraphRow, HistoryCursor, HistoryPage, ReferenceKind, Remote,
    Repository, assign_graph,
};
use std::path::{Path, PathBuf};

const HISTORY_PAGE_SIZE: usize = 250;

async fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, Failure> {
    let mut arguments = vec![
        "--no-pager",
        "--no-optional-locks",
        "-c",
        "core.fsmonitor=false",
        "-c",
        "color.ui=false",
    ];
    arguments.extend_from_slice(args);
    process::run("git", &arguments, Some(root)).await
}

pub(crate) async fn load(
    directory: PathBuf,
    reference: Option<String>,
) -> Result<Repository, Failure> {
    load_reference(directory, reference, false).await
}

async fn load_reference(
    directory: PathBuf,
    reference: Option<String>,
    all: bool,
) -> Result<Repository, Failure> {
    let root = discovery::root(&directory).await?;
    let branches = git(
        &root,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(HEAD)%00%(refname)%00%(refname:short)%00%(symref)",
            "refs/heads",
            "refs/remotes",
            "refs/tags",
        ],
    )
    .await?;
    let branches = String::from_utf8_lossy(&branches)
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split('\0').collect();
            if fields.len() != 4 || !fields[3].is_empty() {
                return None;
            }
            let (kind, name) = if let Some(name) = fields[1].strip_prefix("refs/tags/") {
                (ReferenceKind::Tag, name)
            } else if let Some(name) = fields[1].strip_prefix("refs/remotes/") {
                (ReferenceKind::Remote, name)
            } else {
                (
                    ReferenceKind::Local,
                    fields[1].strip_prefix("refs/heads/").unwrap_or(fields[2]),
                )
            };
            Some(Branch {
                name: name.to_owned(),
                reference: fields[1].to_owned(),
                current: fields[0] == "*",
                kind,
            })
        })
        .collect::<Vec<_>>();
    let head = git(&root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .await
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_owned())
        .unwrap_or_default();
    let mut args = vec!["rev-parse", "--revs-only"];
    if let Some(reference) = &reference {
        if !branches.iter().any(|branch| &branch.reference == reference) {
            return Err(Failure::Read);
        }
        args.push(reference);
    } else if all {
        args.extend(["--all", "HEAD"]);
    } else {
        args.push("HEAD");
    }
    let revisions = git(&root, &args).await?;
    let mut revisions: Vec<_> = String::from_utf8_lossy(&revisions)
        .lines()
        .filter(|id| valid_id(id))
        .map(str::to_owned)
        .collect();
    revisions.sort();
    revisions.dedup();
    let history = load_history(
        root.clone(),
        HistoryCursor {
            revisions,
            offset: 0,
            lanes: Vec::new(),
        },
    )
    .await?;
    let remote_names = git(&root, &["remote"]).await?;
    let remote_names = String::from_utf8_lossy(&remote_names);
    let selected_remote = if remote_names.lines().any(|name| name == "origin") {
        Some("origin")
    } else {
        remote_names.lines().next()
    };
    let remote = if let Some(name) = selected_remote {
        git(&root, &["remote", "get-url", name])
            .await
            .ok()
            .and_then(|bytes| Remote::parse(String::from_utf8_lossy(&bytes).trim()))
    } else {
        None
    };
    Ok(Repository {
        root,
        branches,
        commits: history.commits,
        next_history: history.next,
        remote,
        remotes: remote_names.lines().map(str::to_owned).collect(),
        head,
    })
}

pub(crate) async fn load_history(
    root: PathBuf,
    cursor: HistoryCursor,
) -> Result<HistoryPage, Failure> {
    if cursor.revisions.iter().any(|id| !valid_id(id)) {
        return Err(Failure::Read);
    }
    if cursor.revisions.is_empty() {
        return Ok(HistoryPage {
            commits: Vec::new(),
            next: None,
        });
    }
    let limit = (HISTORY_PAGE_SIZE + 1).to_string();
    let skip = cursor.offset.to_string();
    let mut args = vec![
        "log",
        "--topo-order",
        "--date=format:%Y-%m-%d %H:%M",
        "-n",
        &limit,
        "--skip",
        &skip,
        "--format=%H%x00%P%x00%s%x00%an%x00%ad%x00%D%x00%ct%x00",
    ];
    args.extend(cursor.revisions.iter().map(String::as_str));
    args.push("--");
    let mut commits = parse_log(&git(&root, &args).await?);
    let has_more = commits.len() > HISTORY_PAGE_SIZE;
    commits.truncate(HISTORY_PAGE_SIZE);
    let offset = cursor.offset + commits.len();
    let lanes = assign_graph(&mut commits, cursor.lanes);
    Ok(HistoryPage {
        commits,
        next: has_more.then_some(HistoryCursor {
            revisions: cursor.revisions,
            offset,
            lanes,
        }),
    })
}

fn parse_log(bytes: &[u8]) -> Vec<Commit> {
    let text = String::from_utf8_lossy(bytes);
    let mut fields = text.split('\0');
    let mut result = Vec::new();
    while let (
        Some(id),
        Some(parents),
        Some(subject),
        Some(author),
        Some(date),
        Some(refs),
        Some(timestamp),
    ) = (
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
        fields.next(),
    ) {
        let id = id.trim();
        if !valid_id(id) {
            continue;
        }
        result.push(Commit {
            id: id.to_owned(),
            parents: parents.split_whitespace().map(str::to_owned).collect(),
            subject: subject.to_owned(),
            author: author.to_owned(),
            date: date.to_owned(),
            timestamp: timestamp.parse().unwrap_or_default(),
            refs: refs.to_owned(),
            graph: GraphRow::default(),
        });
    }
    result
}

fn valid_id(id: &str) -> bool {
    matches!(id.len(), 40 | 64) && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) async fn changed_files(
    root: PathBuf,
    commit: Commit,
) -> Result<Vec<ChangedFile>, Failure> {
    if !valid_id(&commit.id) {
        return Err(Failure::Read);
    }
    let mut args = vec![
        "diff-tree",
        "--no-commit-id",
        "--name-status",
        "--no-renames",
        "--root",
        "-r",
        "-z",
    ];
    if let Some(parent) = commit.parents.first() {
        args.push(parent);
    }
    args.extend([commit.id.as_str(), "--"]);
    let bytes = git(&root, &args).await?;
    let fields: Vec<_> = bytes.split(|byte| *byte == 0).collect();
    Ok(fields
        .chunks_exact(2)
        .filter_map(|pair| {
            Some(ChangedFile {
                status: char::from(*pair[0].first()?),
                path: String::from_utf8(pair[1].to_vec()).ok()?,
            })
        })
        .collect())
}

pub(crate) async fn blob(root: PathBuf, revision: String, path: String) -> Result<String, Failure> {
    if !valid_id(&revision) {
        return Err(Failure::Read);
    }
    let object = format!("{revision}:{path}");
    let size = git(&root, &["cat-file", "-s", &object]).await?;
    let size = String::from_utf8_lossy(&size)
        .trim()
        .parse::<u64>()
        .map_err(|_| Failure::Read)?;
    if size > workspace::PREVIEW_LIMIT {
        return Err(Failure::TooLarge);
    }
    workspace::decode(git(&root, &["cat-file", "blob", &object]).await?)
}

pub(crate) async fn blob_page(
    root: PathBuf, revision: String, path: String, cursor: crate::domain::changes::TextCursor,
) -> Result<(String, crate::domain::changes::TextPage), Failure> {
    if !valid_id(&revision) { return Err(Failure::Read); }
    let object = format!("{revision}:{path}");
    let size = git(&root, &["cat-file", "-s", &object]).await?;
    let size = String::from_utf8_lossy(&size).trim().parse::<u64>()
        .map_err(|_| Failure::Read)?;
    if cursor.offset > size { return Err(Failure::Read); }
    let length = (size - cursor.offset).min(workspace::pages::READ_BYTES as u64) as usize;
    let bytes = process::read_range(
        "git", &["--no-pager", "cat-file", "blob", &object], &root, cursor.offset, length,
    ).await?;
    workspace::pages::decode(bytes, cursor, size, None)
}

#[cfg(test)]
mod tests;
