use super::*;
use crate::domain::repository::Repository;

fn snapshot(root: &str) -> Snapshot {
    Snapshot {
        repository: Repository {
            root: PathBuf::from(root),
            branches: Vec::new(),
            commits: Vec::new(),
            next_history: None,
            remote: None,
            remotes: Vec::new(),
            head: "main".into(),
        }
        .into(),
        branch: Some(BranchKey {
            root: PathBuf::from(root),
            reference: "refs/heads/main".into(),
        }),
        folders: BTreeSet::from(["false".into()]),
    }
}

#[test]
fn cache_evicts_the_least_recent_project_and_isolates_returned_snapshots() {
    let mut cache = Cache::default();
    for index in 0..MAX_PROJECTS {
        cache.insert(
            PathBuf::from(index.to_string()),
            snapshot(&index.to_string()),
        );
    }
    let mut first = cache.get(Path::new("0")).unwrap();
    first.repository.repositories[0].head = "changed".into();
    cache.insert(PathBuf::from("new"), snapshot("new"));
    assert!(cache.get(Path::new("1")).is_none());
    assert_eq!(
        cache.get(Path::new("0")).unwrap().repository.repositories[0].head,
        "main"
    );
    assert_eq!(cache.entries.len(), MAX_PROJECTS);
}

#[test]
fn folder_state_only_updates_the_corresponding_branch_snapshot() {
    let mut cache = Cache::default();
    let expected = snapshot("project");
    let branch = expected.branch.clone();
    cache.insert(PathBuf::from("project"), expected);
    let folders = BTreeSet::from(["false/feature".into()]);
    cache.remember_folders(Path::new("project"), &None, &folders);
    assert!(
        cache
            .get(Path::new("project"))
            .unwrap()
            .folders
            .contains("false")
    );
    cache.remember_folders(Path::new("project"), &branch, &folders);
    let restored = cache.get(Path::new("project")).unwrap();
    assert_eq!(restored.folders, folders);
    assert_eq!(restored.branch, branch);
    cache.remove(Path::new("project"));
    assert!(cache.get(Path::new("project")).is_none());
}
