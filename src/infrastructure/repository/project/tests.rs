use super::*;
use crate::infrastructure::repository::{changed_files, working_file, working_status};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

fn command(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn repository(root: &Path, count: usize, base: usize, step: usize) {
    fs::create_dir_all(root).unwrap();
    command(root, &["init", "-b", "main"]);
    let mut input = String::new();
    for number in 1..=count {
        let subject = format!("Commit {number}");
        input.push_str(&format!("commit refs/heads/main\ncommitter Test <test@example.invalid> {} +0000\ndata {}\n{subject}\n\n", base + number * step, subject.len()));
    }
    let mut child = Command::new("git")
        .args(["fast-import", "--quiet"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn discovers_only_direct_children_and_supports_linked_worktrees() {
    let parent = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(parent.path()).unwrap();
    repository(&root.join("backend"), 2, 1_700_000_000, 1);
    repository(&root.join("archive/too-deep"), 1, 1_700_000_000, 1);
    let front = root.join("front");
    // Git cannot create worktrees through Windows verbatim paths. Keep the target relative to its repository.
    command(
        &root.join("backend"),
        &["worktree", "add", "-b", "web", "../front"],
    );
    assert!(front.join(".git").is_file());
    let loaded = load_project(root.clone(), None).await.unwrap();
    assert_eq!(
        loaded
            .repositories
            .iter()
            .map(|repo| repo.root.clone())
            .collect::<Vec<_>>(),
        vec![root.join("backend"), front.clone()]
    );
    assert_eq!(loaded.commits.len(), 4);
    let key = BranchKey {
        root: front.clone(),
        reference: "refs/heads/main".into(),
    };
    let selected = load_project(root, Some(key)).await.unwrap();
    assert!(selected.commits.iter().all(|entry| entry.root == front));
    assert_eq!(selected.active().unwrap().root, front);
}

#[tokio::test]
async fn merged_pages_keep_order_and_frozen_revisions_without_losing_commits() {
    let parent = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(parent.path()).unwrap();
    let backend = root.join("backend");
    let front = root.join("front");
    repository(&backend, 315, 1_700_000_000, 2);
    repository(&front, 400, 1_700_000_001, 2);
    let loaded = load_project(root.clone(), None).await.unwrap();
    assert_eq!(loaded.commits.len(), HISTORY_PAGE_SIZE);
    let mut commits = loaded.commits;
    let mut next = loaded.next_history;
    let new_commit = command(
        &backend,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit-tree",
            "HEAD^{tree}",
            "-p",
            "HEAD",
            "-m",
            "After snapshot",
        ],
    );
    command(&backend, &["update-ref", "refs/heads/main", &new_commit]);
    while let Some(cursor) = next {
        let page = load_project_history(cursor).await.unwrap();
        commits.extend(page.commits);
        next = page.next;
    }
    assert_eq!(commits.len(), 715);
    assert!(
        commits
            .windows(2)
            .all(|pair| pair[0].commit.timestamp >= pair[1].commit.timestamp)
    );
    assert!(commits.iter().all(|entry| entry.commit.id != new_commit));
    let unique: std::collections::BTreeSet<_> = commits
        .iter()
        .map(|entry| (&entry.root, &entry.commit.id))
        .collect();
    assert_eq!(unique.len(), 715);
    assert_eq!(
        commits.iter().filter(|entry| entry.root == backend).count(),
        315
    );
}

#[tokio::test]
async fn nested_status_and_comparisons_use_the_owning_repository() {
    let parent = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(parent.path()).unwrap();
    for name in ["backend", "front"] {
        let child = root.join(name);
        repository(&child, 1, 1_700_000_000, 1);
        fs::write(child.join("same.txt"), format!("{name} before\n")).unwrap();
        fs::write(child.join("deleted.txt"), format!("{name} deleted\n")).unwrap();
        command(&child, &["add", "."]);
        command(
            &child,
            &[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-m",
                "Files",
            ],
        );
        fs::write(child.join("same.txt"), format!("{name} after\n")).unwrap();
        fs::remove_file(child.join("deleted.txt")).unwrap();
    }
    let status = working_status(root.clone()).await.unwrap();
    assert_eq!(status.roots.len(), 2);
    assert_eq!(status.files.len(), 4);
    for name in ["backend", "front"] {
        let file = working_file(root.clone(), root.join(name).join("same.txt"))
            .await
            .unwrap();
        assert_eq!(file.previous, Some(format!("{name} before\n")));
        assert_eq!(file.text, format!("{name} after\n"));
        let deleted = working_file(root.clone(), root.join(name).join("deleted.txt"))
            .await
            .unwrap();
        assert!(deleted.deleted);
        assert_eq!(deleted.text, format!("{name} deleted\n"));
    }
    let loaded = load_project(root, None).await.unwrap();
    for entry in loaded
        .commits
        .iter()
        .filter(|entry| entry.commit.subject == "Files")
    {
        let files = changed_files(entry.root.clone(), entry.commit.clone())
            .await
            .unwrap();
        assert_eq!(files.len(), 2);
    }
}

#[tokio::test]
async fn parent_and_child_repositories_do_not_duplicate_nested_status() {
    let parent = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(parent.path()).unwrap();
    repository(&root, 1, 1_700_000_000, 1);
    repository(&root.join("backend"), 1, 1_700_000_000, 1);
    fs::write(root.join("backend/new.txt"), "new").unwrap();
    fs::write(root.join("root.txt"), "root").unwrap();
    let loaded = load_project(root.clone(), None).await.unwrap();
    assert_eq!(loaded.repositories.len(), 2);
    let status = working_status(root.clone()).await.unwrap();
    assert_eq!(
        status.files.keys().cloned().collect::<Vec<_>>(),
        vec![root.join("backend/new.txt"), root.join("root.txt")]
    );
}

#[tokio::test]
async fn broken_child_does_not_hide_a_valid_repository() {
    let parent = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(parent.path()).unwrap();
    repository(&root.join("backend"), 1, 1_700_000_000, 1);
    fs::create_dir(root.join("broken")).unwrap();
    fs::write(root.join("broken/.git"), "gitdir: missing\n").unwrap();
    let loaded = load_project(root.clone(), None).await.unwrap();
    assert_eq!(loaded.repositories.len(), 1);
    assert_eq!(loaded.failures.len(), 1);
    assert_eq!(working_status(root).await.unwrap().failures.len(), 1);
}

#[tokio::test]
async fn a_removed_branch_restores_all_repository_history() {
    let parent = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(parent.path()).unwrap();
    for name in ["backend", "front"] {
        repository(&root.join(name), 2, 1_700_000_000, 1);
    }
    let loaded = load_project(
        root.clone(),
        Some(BranchKey {
            root: root.join("front"),
            reference: "refs/heads/removed".into(),
        }),
    )
    .await
    .unwrap();
    assert_eq!(loaded.repositories.len(), 2);
    assert_eq!(loaded.commits.len(), 4);
    assert!(loaded.failures.is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn discovery_does_not_follow_child_directory_symlinks() {
    let parent = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(parent.path()).unwrap();
    repository(&root.join("backend"), 1, 1_700_000_000, 1);
    repository(outside.path(), 1, 1_700_000_000, 1);
    std::os::unix::fs::symlink(outside.path(), root.join("linked")).unwrap();
    let loaded = load_project(root.clone(), None).await.unwrap();
    assert_eq!(loaded.repositories.len(), 1);
    assert_eq!(loaded.repositories[0].root, root.join("backend"));
}

#[tokio::test]
async fn pipeline_discovery_does_not_load_git_history_or_branches() {
    let directory = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(directory.path()).unwrap();
    repository(&root, 3, 1_700_000_000, 1);
    command(
        &root,
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/example/test.git",
        ],
    );
    let loaded = load_project_remotes(root).await.unwrap();
    assert!(loaded.commits.is_empty());
    assert!(loaded.next_history.is_none());
    let repo = loaded.active().unwrap();
    assert!(repo.branches.is_empty());
    assert!(repo.commits.is_empty());
    assert_eq!(repo.remote.as_ref().unwrap().project, "example/test");
    assert_eq!(repo.remotes, ["origin"]);
}
