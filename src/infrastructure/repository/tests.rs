use super::*;

fn long_repository(count: usize) -> tempfile::TempDir {
    use std::io::Write;
    use std::process::Stdio;

    let directory = tempfile::tempdir().unwrap();
    command(directory.path(), &["init", "-b", "main"]);
    let mut input = String::new();
    for number in 1..=count {
        let subject = format!("Commit {number}");
        input.push_str(&format!(
            "commit refs/heads/main\ncommitter Test Author <test@example.invalid> {} +0000\ndata {}\n{subject}\n\n",
            1_700_000_000 + number, subject.len(),
        ));
    }
    let mut child = std::process::Command::new("git")
        .args(["fast-import", "--quiet"])
        .current_dir(directory.path())
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
    directory
}

#[tokio::test]
async fn history_dates_include_hours_and_minutes_in_the_recorded_author_timezone() {
    let directory = long_repository(1);
    let root = directory.path();
    command(
        root,
        &[
            "-c",
            "user.name=Test Author",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--amend",
            "--allow-empty",
            "--no-edit",
            "--date=2026-11-09T14:37:26+0530",
        ],
    );
    let repo = load(root.to_owned(), None).await.unwrap();
    assert_eq!(repo.commits[0].date, "2026-11-09 14:37");
}

#[tokio::test]
async fn lightweight_and_annotated_tags_open_their_history_without_checkout() {
    let directory = long_repository(3);
    let root = directory.path();
    command(root, &["config", "user.name", "Test Author"]);
    command(root, &["config", "user.email", "test@example.invalid"]);
    command(root, &["tag", "v1", "HEAD~1"]);
    command(root, &["tag", "-a", "release/v2", "-m", "Release", "HEAD"]);
    let repo = load(root.to_owned(), None).await.unwrap();
    let tags: Vec<_> = repo
        .branches
        .iter()
        .filter(|branch| branch.kind == ReferenceKind::Tag)
        .collect();
    assert_eq!(tags.len(), 2);
    assert!(tags.iter().all(|tag| !tag.current));
    for (name, count) in [("v1", 2), ("release/v2", 3)] {
        let tag = tags.iter().find(|tag| tag.name == name).unwrap();
        let history = load(root.to_owned(), Some(tag.reference.clone()))
            .await
            .unwrap();
        assert_eq!(history.commits.len(), count);
        assert_eq!(history.commits[0].subject, format!("Commit {count}"));
        assert_eq!(command(root, &["branch", "--show-current"]), "main");
    }
}

#[tokio::test]
async fn default_history_follows_checkout_and_detached_head() {
    let directory = long_repository(3);
    let root = directory.path();
    command(root, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    command(root, &["checkout", "-b", "work/current", "HEAD~1"]);

    let repo = load(root.to_owned(), None).await.unwrap();
    assert_eq!(repo.head, "work/current");
    assert_eq!(repo.commits.len(), 2);
    assert_eq!(repo.commits[0].subject, "Commit 2");
    assert!(
        repo.branches
            .iter()
            .any(|branch| { branch.current && branch.reference == "refs/heads/work/current" })
    );

    for reference in ["refs/heads/main", "refs/remotes/origin/main"] {
        let selected = load(root.to_owned(), Some(reference.to_owned()))
            .await
            .unwrap();
        assert_eq!(selected.commits.len(), 3);
        assert_eq!(selected.commits[0].subject, "Commit 3");
        assert_eq!(command(root, &["branch", "--show-current"]), "work/current");
    }

    command(root, &["checkout", "--detach", "HEAD~1"]);
    let detached = load(root.to_owned(), None).await.unwrap();
    assert!(detached.head.is_empty());
    assert!(detached.branches.iter().all(|branch| !branch.current));
    assert_eq!(detached.commits.len(), 1);
    assert_eq!(detached.commits[0].subject, "Commit 1");
}

#[tokio::test]
async fn history_pages_reach_the_root_without_duplicates_when_refs_move() {
    let directory = long_repository(602);
    let root = directory.path();
    let expected = command(root, &["rev-list", "--topo-order", "HEAD"]);
    let repo = load(root.to_owned(), None).await.unwrap();
    assert_eq!(repo.commits.len(), HISTORY_PAGE_SIZE);
    let mut cursor = repo.next_history;
    let mut commits = repo.commits;
    command(
        root,
        &[
            "-c",
            "user.name=Test Author",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--allow-empty",
            "-m",
            "Later commit",
        ],
    );
    let mut pages = 1;
    while let Some(next) = cursor {
        let page = load_history(root.to_owned(), next).await.unwrap();
        assert!(!page.commits.is_empty() && page.commits.len() <= HISTORY_PAGE_SIZE);
        commits.extend(page.commits);
        cursor = page.next;
        pages += 1;
    }
    assert_eq!(pages, 3);
    assert_eq!(
        commits
            .iter()
            .map(|commit| commit.id.as_str())
            .collect::<Vec<_>>(),
        expected.lines().collect::<Vec<_>>()
    );
    assert!(commits.last().unwrap().parents.is_empty());
    let mut complete_graph = commits.clone();
    assign_graph(&mut complete_graph, Vec::new());
    for (paged, complete) in commits.iter().zip(complete_graph) {
        assert_eq!(paged.graph.lane, complete.graph.lane);
        assert_eq!(paged.graph.incoming, complete.graph.incoming);
        assert_eq!(paged.graph.outgoing, complete.graph.outgoing);
    }
    command(root, &["branch", "limited", "HEAD~353"]);
    let branch = load(root.to_owned(), Some("refs/heads/limited".to_owned()))
        .await
        .unwrap();
    assert_eq!(branch.commits.len(), HISTORY_PAGE_SIZE);
    assert!(branch.next_history.is_none());
    assert_eq!(command(root, &["branch", "--show-current"]), "main");
}

fn command(root: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

#[tokio::test]
async fn repository_reads_root_commits_branches_deleted_files_and_historical_content() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    command(root, &["init", "-b", "main"]);
    command(root, &["config", "user.email", "test@example.invalid"]);
    command(root, &["config", "user.name", "Test Author"]);
    command(root, &["config", "commit.gpgsign", "false"]);
    std::fs::write(root.join("hello world.rs"), "fn main() {}\n").unwrap();
    command(root, &["add", "--", "hello world.rs"]);
    command(root, &["commit", "-m", "Initial commit"]);
    let first = command(root, &["rev-parse", "HEAD"]);
    command(root, &["branch", "feature/test"]);
    command(root, &["rm", "--", "hello world.rs"]);
    command(root, &["commit", "-m", "Remove file"]);
    command(
        root,
        &["remote", "add", "origin", "git@github.com:example/demo.git"],
    );
    let repo = load(root.to_owned(), None).await.unwrap();
    assert_eq!(repo.commits.len(), 2);
    assert_eq!(repo.commits[0].subject, "Remove file");
    assert_eq!(repo.head, "main");
    assert_eq!(
        repo.remote.unwrap().provider,
        Some(crate::domain::repository::Provider::GitHub)
    );
    let files = changed_files(root.to_owned(), repo.commits[0].clone())
        .await
        .unwrap();
    assert_eq!(
        (files[0].status, files[0].path.as_str()),
        ('D', "hello world.rs")
    );
    let initial_files = changed_files(root.to_owned(), repo.commits[1].clone())
        .await
        .unwrap();
    assert_eq!(initial_files[0].status, 'A');
    assert_eq!(
        blob(root.to_owned(), first, files[0].path.clone())
            .await
            .unwrap(),
        "fn main() {}\n"
    );
    let branch = load(root.to_owned(), Some("refs/heads/feature/test".to_owned()))
        .await
        .unwrap();
    assert_eq!(branch.commits.len(), 1);
    assert_eq!(command(root, &["branch", "--show-current"]), "main");
    assert!(command(root, &["status", "--porcelain"]).is_empty());
}

#[tokio::test]
async fn empty_and_non_git_directories_are_distinct() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
        load(directory.path().to_owned(), None).await.unwrap_err(),
        Failure::NotRepository
    );
    command(directory.path(), &["init", "-b", "main"]);
    let repo = load(directory.path().to_owned(), None).await.unwrap();
    assert!(repo.commits.is_empty());
    assert!(repo.remote.is_none());
}

#[tokio::test]
async fn object_reads_reject_arbitrary_revision_arguments() {
    let directory = tempfile::tempdir().unwrap();
    assert_eq!(
        blob(
            directory.path().to_owned(),
            "--help".to_owned(),
            "x".to_owned()
        )
        .await
        .unwrap_err(),
        Failure::Read
    );
}
