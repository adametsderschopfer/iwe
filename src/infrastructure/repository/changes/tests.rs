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
async fn missing_working_files_show_deleted_baselines_but_unknown_paths_stay_unreadable() {
    let directory = project();
    let root = directory.path();
    fs::write(root.join("deleted.rs"), "old contents\n").unwrap();
    git_test(root, &["add", "."]);
    git_test(root, &["commit", "-m", "Initial"]);
    fs::remove_file(root.join("deleted.rs")).unwrap();
    for staged in [false, true] {
        if staged {
            git_test(root, &["add", "-u"]);
        }
        let source = working_file(root.to_owned(), root.join("deleted.rs"))
            .await
            .unwrap();
        assert_eq!(source.text, "old contents\n");
        assert!(source.deleted);
        assert!(!source.comparison_failed);
    }
    assert_eq!(
        working_text(root.to_owned(), root.join("unknown.rs")).await,
        Err(Failure::Read)
    );
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

#[tokio::test]
async fn cached_baselines_follow_head_renames_deletions_and_nested_scopes() {
    let directory = project();
    let root = directory.path();
    fs::create_dir(root.join("src")).unwrap();
    let original = root.join("src/file.txt");
    fs::write(&original, "original\n").unwrap();
    git_test(root, &["add", "."]);
    git_test(root, &["commit", "-m", "Initial"]);
    fs::write(&original, "working\n").unwrap();
    let cache = std::sync::Arc::new(tokio::sync::Mutex::new(BaselineCache::default()));
    for scope in [root.to_owned(), root.join("src")] {
        let status = working_status(scope.clone()).await.unwrap();
        for _ in 0..2 {
            let cached = working_file_cached(
                scope.clone(),
                original.clone(),
                working_snapshot(&status, &original).unwrap(),
                cache.clone(),
            )
            .await
            .unwrap();
            let direct = working_file(scope.clone(), original.clone()).await.unwrap();
            assert_eq!(cached.text, direct.text);
            assert_eq!(cached.previous, direct.previous);
            assert_eq!(cached.comparison_failed, direct.comparison_failed);
        }
    }
    let first_key = cache.lock().await.key.clone();
    git_test(root, &["mv", "src/file.txt", "src/moved.txt"]);
    let moved = root.join("src/moved.txt");
    let status = working_status(root.to_owned()).await.unwrap();
    let cached = working_file_cached(
        root.to_owned(),
        moved.clone(),
        working_snapshot(&status, &moved).unwrap(),
        cache.clone(),
    )
    .await
    .unwrap();
    assert_eq!(cached.previous.as_deref(), Some("original\n"));
    assert_eq!(cache.lock().await.key, first_key);
    git_test(root, &["add", "."]);
    git_test(root, &["commit", "-m", "Rename and update"]);
    let status = working_status(root.to_owned()).await.unwrap();
    let cached = working_file_cached(
        root.to_owned(),
        moved.clone(),
        working_snapshot(&status, &moved).unwrap(),
        cache.clone(),
    )
    .await
    .unwrap();
    assert_eq!(cached.previous.as_deref(), Some("working\n"));
    assert_ne!(cache.lock().await.key, first_key);
    fs::remove_file(&moved).unwrap();
    let status = working_status(root.to_owned()).await.unwrap();
    let cached = working_file_cached(
        root.to_owned(),
        moved.clone(),
        working_snapshot(&status, &moved).unwrap(),
        cache.clone(),
    )
    .await
    .unwrap();
    assert!(cached.deleted);
    assert_eq!(cached.text, "working\n");
    assert_eq!(cached.previous.as_deref(), Some("working\n"));
    let new = root.join("new.txt");
    fs::write(&new, "new\n").unwrap();
    let status = working_status(root.to_owned()).await.unwrap();
    let cached = working_file_cached(
        root.to_owned(),
        new.clone(),
        working_snapshot(&status, &new).unwrap(),
        cache,
    )
    .await
    .unwrap();
    assert_eq!(cached.previous.as_deref(), Some(""));
}

#[tokio::test]
async fn cached_baselines_keep_nested_repository_ownership_and_report_refresh_costs() {
    let parent = project();
    let child = parent.path().join("child");
    fs::create_dir(&child).unwrap();
    git_test(&child, &["init", "-b", "main"]);
    git_test(&child, &["config", "user.name", "Test"]);
    git_test(&child, &["config", "user.email", "test@example.invalid"]);
    let path = child.join("file.txt");
    fs::write(&path, "nested baseline\n").unwrap();
    git_test(&child, &["add", "."]);
    git_test(&child, &["commit", "-m", "Initial"]);
    fs::write(&path, "nested working\n").unwrap();
    let root = parent.path().to_owned();
    let status = working_status(root.clone()).await.unwrap();
    let cache = std::sync::Arc::new(tokio::sync::Mutex::new(BaselineCache::default()));
    let cached = working_file_cached(
        root.clone(),
        path.clone(),
        working_snapshot(&status, &path).unwrap(),
        cache.clone(),
    )
    .await
    .unwrap();
    assert_eq!(cached.previous.as_deref(), Some("nested baseline\n"));
    if std::env::var_os("IWE_PROFILE_COMPARISON").is_some() {
        let mut times = [Vec::new(), Vec::new()];
        for _ in 0..5 {
            let start = std::time::Instant::now();
            let direct = working_file(root.clone(), path.clone()).await.unwrap();
            times[0].push(start.elapsed().as_secs_f64() * 1000.);
            let start = std::time::Instant::now();
            let cached = working_file_cached(
                root.clone(),
                path.clone(),
                working_snapshot(&status, &path).unwrap(),
                cache.clone(),
            )
            .await
            .unwrap();
            times[1].push(start.elapsed().as_secs_f64() * 1000.);
            assert_eq!(direct.previous, cached.previous);
            assert_eq!(direct.text, cached.text);
        }
        for (name, mut values) in ["uncached", "cached"].into_iter().zip(times) {
            values.sort_by(f64::total_cmp);
            eprintln!("AUDIT git-refresh {name} median_ms={:.3}", values[2]);
        }
    }
}
