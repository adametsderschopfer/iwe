use super::*;
use std::{fs, process::Command};

fn git_test(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn project() -> tempfile::TempDir {
    let directory = tempfile::tempdir().unwrap();
    git_test(directory.path(), &["init", "-b", "main"]);
    git_test(directory.path(), &["config", "user.name", "Test"]);
    git_test(
        directory.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    directory
}

#[test]
fn nul_status_preserves_paths_and_aggregates_folder_changes() {
    let root = Path::new("/project");
    let files = parse_status(
        b"?? src/new name.rs\0 M src/line\nname.rs\0R  moved.rs\0old.rs\0UU conflict.rs\0",
        root,
    );
    let status = Worktree::new(root, files);
    assert_eq!(
        status.status(&root.join("src/new name.rs"), false),
        Some(FileStatus::Untracked)
    );
    assert_eq!(
        status.status(&root.join("src/line\nname.rs"), false),
        Some(FileStatus::Modified)
    );
    assert_eq!(
        status.status(&root.join("src"), true),
        Some(FileStatus::Modified)
    );
    assert_eq!(
        status.files[&root.join("moved.rs")].original,
        Some(root.join("old.rs"))
    );
    assert_eq!(status.status(root, true), Some(FileStatus::Conflict));
}

#[tokio::test]
async fn working_files_compare_head_and_include_staged_and_untracked_changes() {
    let directory = project();
    let root = directory.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/main.rs"), "original\n").unwrap();
    fs::write(root.join("rename.rs"), "same\n").unwrap();
    fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
    git_test(root, &["add", "."]);
    git_test(root, &["commit", "-m", "Initial"]);
    fs::write(root.join("src/main.rs"), "staged\n").unwrap();
    git_test(root, &["add", "src/main.rs"]);
    fs::write(root.join("src/main.rs"), "working\n").unwrap();
    fs::write(root.join("src/new file.rs"), "created\n").unwrap();
    fs::write(root.join("src/added.rs"), "added\n").unwrap();
    git_test(root, &["add", "src/added.rs"]);
    git_test(root, &["mv", "rename.rs", "renamed.rs"]);
    let status = working_status(root.to_owned()).await.unwrap();
    assert_eq!(
        status.status(&root.join("src/main.rs"), false),
        Some(FileStatus::Modified)
    );
    assert_eq!(
        status.status(&root.join("src/new file.rs"), false),
        Some(FileStatus::Untracked)
    );
    assert_eq!(
        status.status(&root.join("src/added.rs"), false),
        Some(FileStatus::Added)
    );
    let source = working_file(root.to_owned(), root.join("src/main.rs"))
        .await
        .unwrap();
    assert_eq!(source.text, "working\n");
    assert_eq!(source.previous.as_deref(), Some("original\n"));
    let renamed = working_file(root.to_owned(), root.join("renamed.rs"))
        .await
        .unwrap();
    assert_eq!(renamed.previous.as_deref(), Some("same\n"));
    let new = working_file(root.to_owned(), root.join("src/new file.rs"))
        .await
        .unwrap();
    assert_eq!(new.previous.as_deref(), Some(""));
    fs::create_dir(root.join("ignored")).unwrap();
    fs::write(root.join("ignored/file.rs"), "ignored\n").unwrap();
    let ignored = working_file(root.to_owned(), root.join("ignored/file.rs"))
        .await
        .unwrap();
    assert!(ignored.previous.is_none());
    assert!(!ignored.comparison_failed);
    let nested = working_status(root.join("src")).await.unwrap();
    assert_eq!(nested.files.len(), 3);
    assert!(!nested.folders.contains_key(root));
}

#[tokio::test]
async fn commit_previews_compare_the_parent_and_preserve_deleted_content() {
    let directory = project();
    let root = directory.path();
    fs::write(root.join("file.rs"), "before\n").unwrap();
    fs::write(root.join("deleted.rs"), "deleted\n").unwrap();
    git_test(root, &["add", "."]);
    git_test(root, &["commit", "-m", "Initial"]);
    fs::write(root.join("file.rs"), "after\n").unwrap();
    fs::remove_file(root.join("deleted.rs")).unwrap();
    fs::write(root.join("created.rs"), "created\n").unwrap();
    git_test(root, &["add", "."]);
    git_test(root, &["commit", "-m", "Change files"]);
    let repo = super::super::load(root.to_owned(), None).await.unwrap();
    let commit = &repo.commits[0];
    let changes = super::super::changed_files(root.to_owned(), commit.clone())
        .await
        .unwrap();
    for file in changes {
        let source = commit_file(root.to_owned(), commit.clone(), file.clone())
            .await
            .unwrap();
        match file.status {
            'M' => {
                assert_eq!(source.text, "after\n");
                assert_eq!(source.previous.as_deref(), Some("before\n"));
            }
            'A' => {
                assert_eq!(source.text, "created\n");
                assert_eq!(source.previous.as_deref(), Some(""));
            }
            'D' => {
                assert_eq!(source.text, "deleted\n");
                assert!(source.deleted);
            }
            other => panic!("Unexpected fixture status: {other}"),
        }
    }
    let first = commit_file(
        root.to_owned(),
        repo.commits[1].clone(),
        ChangedFile {
            path: "file.rs".to_owned(),
            status: 'A',
        },
    )
    .await
    .unwrap();
    assert_eq!(first.previous.as_deref(), Some(""));
}

#[tokio::test]
async fn unborn_repositories_and_plain_folders_keep_readable_previews() {
    let directory = project();
    let root = directory.path();
    fs::write(root.join("new.rs"), "new\n").unwrap();
    let source = working_file(root.to_owned(), root.join("new.rs"))
        .await
        .unwrap();
    assert_eq!(source.previous.as_deref(), Some(""));
    assert!(!source.comparison_failed);
    let plain = tempfile::tempdir().unwrap();
    fs::write(plain.path().join("plain.rs"), "plain\n").unwrap();
    let source = working_file(plain.path().to_owned(), plain.path().join("plain.rs"))
        .await
        .unwrap();
    assert!(source.previous.is_none());
    assert!(!source.comparison_failed);
}

#[tokio::test]
async fn oversized_baselines_keep_the_current_source_without_false_highlights() {
    let directory = project();
    let root = directory.path();
    fs::write(
        root.join("large.rs"),
        "x".repeat(workspace::PREVIEW_LIMIT as usize + 1),
    )
    .unwrap();
    git_test(root, &["add", "."]);
    git_test(root, &["commit", "-m", "Large baseline"]);
    fs::write(root.join("large.rs"), "small\n").unwrap();
    let source = working_file(root.to_owned(), root.join("large.rs"))
        .await
        .unwrap();
    assert_eq!(source.text, "small\n");
    assert!(source.previous.is_none());
    assert!(source.comparison_failed);
}
