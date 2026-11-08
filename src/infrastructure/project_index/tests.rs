use super::*;

fn folder(root: &Path, name: &str) -> FileEntry {
    FileEntry {
        path: root.join(name),
        name: name.to_owned(),
        directory: true,
    }
}

#[test]
fn disk_cache_roundtrip_rejects_corruption_and_paths_outside_the_project() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let storage = temp.path().join("cache/tree.json");
    let mut tree = Tree::from([(root.clone(), vec![folder(&root, "src")])]);
    save_at(&storage, &root, &tree).unwrap();
    assert_eq!(load_at(&storage, &root), Some(tree.clone()));
    assert!(load_at(&storage, &temp.path().join("other")).is_none());
    tree.insert(root.join("../outside"), vec![]);
    save_at(&storage, &root, &tree).unwrap();
    assert!(load_at(&storage, &root).is_none());
    std::fs::write(&storage, b"broken JSON").unwrap();
    assert!(load_at(&storage, &root).is_none());
    tree.remove(&root.join("../outside"));
    tree.get_mut(&root).unwrap()[0].path = temp.path().join("outside");
    save_at(&storage, &root, &tree).unwrap();
    assert!(load_at(&storage, &root).is_none());
    std::fs::File::create(&storage)
        .unwrap()
        .set_len(MAX_CACHE_BYTES + 1)
        .unwrap();
    assert!(load_at(&storage, &root).is_none());
}

#[test]
fn background_traversal_skips_generated_folders_and_stays_bounded() {
    let root = Path::new("project");
    assert!(prefetch(root, &root.join("src/screens")));
    for path in [
        ".git/objects",
        "node_modules/react",
        "target/debug",
        ".venv/lib",
        "build",
    ] {
        assert!(!prefetch(root, &root.join(path)));
    }
    assert!(!prefetch(
        root,
        &root.join(vec!["deep"; MAX_DEPTH + 1].join("/"))
    ));
    assert!(!valid_directory(root, &root.join("../outside")));
}

async fn next_directory(
    receiver: &mut events::Receiver<Event>,
    path: &Path,
) -> Result<Vec<FileEntry>, Failure> {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(Event::Directory(directory, entries)) = receiver.recv().await {
                if directory == path {
                    return entries;
                }
            } else if receiver.is_closed() {
                panic!("Index stopped before the expected directory");
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn indexing_restores_cache_prefetches_children_and_refreshes_opened_folders() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let src = root.join("src");
    let nested = src.join("nested");
    let dependencies = root.join("node_modules");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::create_dir_all(&dependencies).unwrap();
    std::fs::create_dir(root.join(".git")).unwrap();
    std::fs::write(nested.join("first.rs"), "contents are not cached").unwrap();
    std::fs::write(dependencies.join("package.json"), "{}").unwrap();
    let storage = temp.path().join("cache.json");
    let stale = Tree::from([(root.clone(), vec![folder(&root, "removed")])]);
    save_at(&storage, &root, &stale).unwrap();
    let (handle, mut events, mut worker) = session(root.clone(), Tree::new());
    worker.storage = Some(storage.clone());
    let task = tokio::spawn(run(worker));
    assert!(matches!(events.recv().await, Some(Event::Cached(tree)) if tree == stale));
    let entries = next_directory(&mut events, &root).await.unwrap();
    assert_eq!(
        entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["node_modules", "src"]
    );
    assert_eq!(
        next_directory(&mut events, &nested).await.unwrap()[0].name,
        "first.rs"
    );
    std::fs::remove_file(nested.join("first.rs")).unwrap();
    std::fs::write(nested.join("second.rs"), "new content").unwrap();
    handle.refresh(nested.clone());
    assert_eq!(
        next_directory(&mut events, &nested).await.unwrap()[0].name,
        "second.rs"
    );
    handle.refresh(dependencies.clone());
    assert_eq!(
        next_directory(&mut events, &dependencies).await.unwrap()[0].name,
        "package.json"
    );
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if load_at(&storage, &root).is_some_and(|tree| tree.contains_key(&dependencies)) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let stored = std::fs::read_to_string(&storage).unwrap();
    assert!(!stored.contains("contents are not cached") && !stored.contains("new content"));
    let cache = load_at(&storage, &root).unwrap();
    assert!(!cache.contains_key(&root.join("removed")));
    assert_eq!(cache[&nested][0].name, "second.rs");
    drop(handle);
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    let (handle, mut events, mut worker) = session(root.clone(), Tree::new());
    worker.storage = Some(storage);
    let task = tokio::spawn(run(worker));
    assert!(matches!(events.recv().await, Some(Event::Cached(tree)) if tree == cache));
    let missing = root.join("created-later");
    handle.refresh(missing.clone());
    assert_eq!(
        next_directory(&mut events, &missing).await.unwrap_err(),
        Failure::Read
    );
    std::fs::create_dir(&missing).unwrap();
    handle.refresh(missing.clone());
    assert!(
        next_directory(&mut events, &missing)
            .await
            .unwrap()
            .is_empty()
    );
    drop(handle);
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn index_does_not_follow_external_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let link = temp.path().join("outside");
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    let (handle, mut events, mut worker) = session(temp.path().to_path_buf(), Tree::new());
    worker.storage = None;
    let task = tokio::spawn(run(worker));
    assert!(
        next_directory(&mut events, temp.path())
            .await
            .unwrap()
            .is_empty()
    );
    handle.refresh(link.clone());
    assert_eq!(
        next_directory(&mut events, &link).await.unwrap_err(),
        Failure::Read
    );
    drop(handle);
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn memory_seed_is_persisted_even_when_disk_entries_are_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let storage = temp.path().join("cache/tree.json");
    let seed = Tree::from([(root.clone(), vec![])]);
    let (handle, mut events, mut worker) = session(root.clone(), seed.clone());
    worker.storage = Some(storage.clone());
    let task = tokio::spawn(run(worker));
    assert!(next_directory(&mut events, &root).await.unwrap().is_empty());
    tokio::time::timeout(Duration::from_secs(5), async {
        while load_at(&storage, &root).is_none() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(load_at(&storage, &root), Some(seed));
    drop(handle);
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
}
