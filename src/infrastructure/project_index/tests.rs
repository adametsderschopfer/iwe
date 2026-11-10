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
fn cache_serialization_keeps_the_existing_format_and_preserves_previous_data_on_overflow() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let path = temp.path().join("cache.json");
    let tree = Tree::from([(root.clone(), vec![folder(&root, "src")])]);
    save_at(&path, &root, &tree).unwrap();
    let expected = serde_json::to_vec(&Stored {
        version: 1,
        root: root.clone(),
        directories: tree.clone(),
    })
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), expected);
    let oversized = Tree::from([(
        root.clone(),
        vec![FileEntry {
            name: "x".repeat(MAX_CACHE_BYTES as usize),
            path: root.join("large"),
            directory: false,
        }],
    )]);
    assert_eq!(save_at(&path, &root, &oversized), Err(Failure::TooLarge));
    assert_eq!(std::fs::read(&path).unwrap(), expected);
    assert_eq!(load_at(&path, &root), Some(tree));
}

#[test]
fn index_counts_follow_replacements_and_only_remove_obsolete_subtrees() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();
    let removed = root.join("src");
    let retained = root.join("src-other");
    std::fs::create_dir_all(removed.join("nested")).unwrap();
    std::fs::create_dir(&retained).unwrap();
    std::fs::write(retained.join("keep.rs"), "retained").unwrap();
    let tree = Tree::from([
        (
            root.clone(),
            vec![folder(&root, "src"), folder(&root, "src-other")],
        ),
        (removed.clone(), vec![folder(&removed, "nested")]),
        (
            removed.join("nested"),
            vec![folder(&removed.join("nested"), "old")],
        ),
        (retained.clone(), vec![]),
    ]);
    let (_handle, _events, mut worker) = session(root.clone(), tree);
    worker.storage = None;
    let mut queue = VecDeque::new();
    let mut queued = BTreeSet::new();
    std::fs::remove_dir_all(&removed).unwrap();
    assert!(worker.read(&root, &mut queue, &mut queued, true));
    assert!(!worker.tree.contains_key(&removed));
    assert!(!worker.tree.contains_key(&removed.join("nested")));
    assert!(worker.tree.contains_key(&retained));
    assert_eq!(worker.entry_count, 1);
    assert!(worker.read(&retained, &mut queue, &mut queued, true));
    assert_eq!(worker.entry_count, 2);
    assert!(!worker.read(&retained, &mut queue, &mut queued, true));
    assert_eq!(worker.entry_count, 2);
}

#[tokio::test]
async fn cancelled_index_stops_when_its_output_queue_is_full() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();
    let (handle, _events, mut worker) = session(root.clone(), Tree::new());
    worker.storage = None;
    for _ in 0..worker.events.max_capacity() {
        assert!(worker.send(Event::Directory(root.clone(), Ok(vec![]))));
    }
    let task = tokio::spawn(run(worker));
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(!task.is_finished());
    drop(handle);
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
}

#[test]
fn abandoned_index_receiver_cancels_further_traversal() {
    let temp = tempfile::tempdir().unwrap();
    let (handle, events, worker) = session(temp.path().into(), Tree::new());
    drop(events);
    assert!(!worker.send(Event::Directory(temp.path().into(), Ok(vec![]))));
    assert!(handle.cancelled.is_cancelled());
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
