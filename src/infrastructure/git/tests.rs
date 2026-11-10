use super::*;

fn command(root: &Path, args: &[&str]) -> String {
    let result = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap().trim().to_owned()
}

fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    command(dir.path(), &["init", "-b", "main"]);
    command(dir.path(), &["config", "user.name", "Test"]);
    command(
        dir.path(),
        &["config", "user.email", "test@example.invalid"],
    );
    command(dir.path(), &["commit", "--allow-empty", "-m", "Initial"]);
    dir
}

fn remote(root: &Path) -> tempfile::TempDir {
    let remote = tempfile::tempdir().unwrap();
    command(remote.path(), &["init", "--bare", "-b", "main"]);
    command(
        root,
        &["remote", "add", "origin", remote.path().to_str().unwrap()],
    );
    command(root, &["push", "-u", "origin", "main"]);
    remote
}

#[tokio::test]
async fn create_checkout_rename_and_delete_keep_commits() {
    let dir = repository();
    let root = dir.path().to_path_buf();
    let head = command(&root, &["rev-parse", "HEAD"]);
    execute(
        root.clone(),
        Operation::Create {
            source: "refs/heads/main".into(),
            name: "feature/test".into(),
            checkout: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(
        command(&root, &["branch", "--show-current"]),
        "feature/test"
    );
    execute(
        root.clone(),
        Operation::Rename {
            source: "refs/heads/feature/test".into(),
            name: "feature/renamed".into(),
        },
    )
    .await
    .unwrap();
    execute(root.clone(), Operation::Checkout("refs/heads/main".into()))
        .await
        .unwrap();
    execute(
        root.clone(),
        Operation::Delete("refs/heads/feature/renamed".into()),
    )
    .await
    .unwrap();
    assert_eq!(command(&root, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        command(&root, &["branch", "--format=%(refname)"]),
        "refs/heads/main"
    );
}

#[tokio::test]
async fn dirty_worktrees_invalid_names_and_current_branch_deletion_are_rejected() {
    let dir = repository();
    let root = dir.path().to_path_buf();
    command(&root, &["branch", "other"]);
    std::fs::write(root.join("draft.txt"), "keep me").unwrap();
    assert_eq!(
        execute(root.clone(), Operation::Checkout("refs/heads/other".into())).await,
        Err(Error::Dirty)
    );
    for name in [
        "--force",
        "../escape",
        "bad name",
        "HEAD",
        "x.lock",
        "@{-1}",
    ] {
        assert!(
            execute(
                root.clone(),
                Operation::Create {
                    source: "refs/heads/main".into(),
                    name: name.into(),
                    checkout: false
                }
            )
            .await
            .is_err()
        );
    }
    assert_eq!(
        execute(root.clone(), Operation::Delete("refs/heads/main".into())).await,
        Err(Error::CurrentBranch)
    );
    assert_eq!(
        std::fs::read_to_string(root.join("draft.txt")).unwrap(),
        "keep me"
    );
    assert_eq!(command(&root, &["branch", "--show-current"]), "main");
}

#[tokio::test]
async fn checkout_does_not_overwrite_ignored_local_files() {
    let dir = repository();
    let root = dir.path().to_owned();
    std::fs::write(root.join(".gitignore"), "local.conf\n").unwrap();
    command(&root, &["add", ".gitignore"]);
    command(&root, &["commit", "-m", "Ignore local settings"]);
    command(&root, &["switch", "-c", "other"]);
    std::fs::write(root.join("local.conf"), "tracked settings").unwrap();
    command(&root, &["add", "-f", "local.conf"]);
    command(&root, &["commit", "-m", "Tracked settings"]);
    command(&root, &["switch", "main"]);
    std::fs::write(root.join("local.conf"), "keep local settings").unwrap();
    assert!(
        execute(root.clone(), Operation::Checkout("refs/heads/other".into()))
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(root.join("local.conf")).unwrap(),
        "keep local settings"
    );
    assert_eq!(command(&root, &["branch", "--show-current"]), "main");
}

#[tokio::test]
async fn push_fetch_and_remote_checkout_use_only_the_selected_branch() {
    let dir = repository();
    let root = dir.path().to_path_buf();
    let remote = remote(&root);
    command(&root, &["config", "push.followTags", "true"]);
    command(&root, &["config", "remote.origin.mirror", "true"]);
    command(&root, &["tag", "-a", "private-tag", "-m", "Keep local"]);
    execute(
        root.clone(),
        Operation::Create {
            source: "refs/heads/main".into(),
            name: "topic".into(),
            checkout: false,
        },
    )
    .await
    .unwrap();
    execute(
        root.clone(),
        Operation::Push {
            source: "refs/heads/topic".into(),
            remote: "origin".into(),
            name: "topic".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(
        command(remote.path(), &["rev-parse", "refs/heads/topic"]),
        command(&root, &["rev-parse", "topic"])
    );
    assert!(command(remote.path(), &["tag", "--list"]).is_empty());
    command(&root, &["branch", "-d", "topic"]);
    execute(root.clone(), Operation::Fetch).await.unwrap();
    execute(
        root.clone(),
        Operation::Checkout("refs/remotes/origin/topic".into()),
    )
    .await
    .unwrap();
    assert_eq!(command(&root, &["branch", "--show-current"]), "topic");
    execute(root.clone(), Operation::Checkout("refs/heads/main".into()))
        .await
        .unwrap();
    execute(
        root.clone(),
        Operation::Delete("refs/remotes/origin/topic".into()),
    )
    .await
    .unwrap();
    assert_eq!(
        command(
            remote.path(),
            &["for-each-ref", "--format=%(refname)", "refs/heads"]
        ),
        "refs/heads/main"
    );
}

#[tokio::test]
async fn update_fast_forwards_current_and_inactive_branches_without_switching() {
    for current in [true, false] {
        let dir = repository();
        let root = dir.path().to_path_buf();
        let remote = remote(&root);
        command(&root, &["branch", "other"]);
        command(&root, &["commit", "--allow-empty", "-m", "Remote change"]);
        let remote_head = command(&root, &["rev-parse", "HEAD"]);
        command(&root, &["push", "origin", "main"]);
        command(&root, &["reset", "--hard", "HEAD~1"]);
        if !current {
            command(&root, &["switch", "other"]);
        }
        execute(root.clone(), Operation::Update("refs/heads/main".into()))
            .await
            .unwrap();
        assert_eq!(command(&root, &["rev-parse", "main"]), remote_head);
        assert_eq!(
            command(&root, &["branch", "--show-current"]),
            if current { "main" } else { "other" }
        );
        assert_eq!(command(remote.path(), &["rev-parse", "main"]), remote_head);
    }
}

#[tokio::test]
async fn divergent_updates_and_unmerged_deletions_preserve_local_commits() {
    let dir = repository();
    let root = dir.path().to_path_buf();
    let _remote = remote(&root);
    command(&root, &["commit", "--allow-empty", "-m", "Remote"]);
    command(&root, &["push", "origin", "main"]);
    command(&root, &["reset", "--hard", "HEAD~1"]);
    command(&root, &["commit", "--allow-empty", "-m", "Local"]);
    let before = command(&root, &["rev-parse", "HEAD"]);
    assert!(
        execute(root.clone(), Operation::Update("refs/heads/main".into()))
            .await
            .is_err()
    );
    assert_eq!(command(&root, &["rev-parse", "HEAD"]), before);
    command(&root, &["switch", "-c", "other", "HEAD~1"]);
    assert!(
        execute(root.clone(), Operation::Update("refs/heads/main".into()))
            .await
            .is_err()
    );
    assert!(
        execute(root.clone(), Operation::Delete("refs/heads/main".into()))
            .await
            .is_err()
    );
    assert_eq!(command(&root, &["rev-parse", "main"]), before);
}

#[tokio::test]
async fn updating_a_branch_in_another_worktree_is_rejected() {
    let dir = repository();
    let root = dir.path().to_path_buf();
    let _remote = remote(&root);
    let other = tempfile::tempdir().unwrap();
    command(&root, &["switch", "-c", "other"]);
    command(
        &root,
        &["worktree", "add", other.path().to_str().unwrap(), "main"],
    );
    let before = command(&root, &["rev-parse", "main"]);
    assert!(
        execute(root.clone(), Operation::Update("refs/heads/main".into()))
            .await
            .is_err()
    );
    assert_eq!(command(&root, &["rev-parse", "main"]), before);
}

#[tokio::test]
async fn deleting_a_branch_reports_its_linked_worktree_and_succeeds_after_switching() {
    let dir = repository();
    let root = dir.path().to_path_buf();
    let other = tempfile::tempdir().unwrap();
    command(&root, &["branch", "feature/linked"]);
    command(&root, &["branch", "feature/other"]);
    command(
        &root,
        &[
            "worktree",
            "add",
            other.path().to_str().unwrap(),
            "feature/linked",
        ],
    );
    let operation = Operation::Delete("refs/heads/feature/linked".into());
    assert_eq!(
        execute(root.clone(), operation.clone()).await,
        Err(Error::BranchInWorktree)
    );
    assert_eq!(
        command(other.path(), &["branch", "--show-current"]),
        "feature/linked"
    );
    command(other.path(), &["switch", "feature/other"]);
    assert_eq!(execute(root.clone(), operation).await, Ok(()));
    assert!(
        !command(&root, &["branch", "--format=%(refname)"])
            .lines()
            .any(|line| line == "refs/heads/feature/linked")
    );
}
