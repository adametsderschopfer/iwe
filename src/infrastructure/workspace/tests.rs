use super::*;

#[test]
fn legacy_storage_is_backed_up_before_spaces_are_saved() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("workspace.json");
    let bytes = br#"{"projects":[{"name":"Team","directories":["frontend","backend"]}],"selected_project":0,"selected_directory":1,"preferences":{"language":"ru"}}"#;
    std::fs::write(&path, bytes).unwrap();
    let mut workspace = load_at(&path).unwrap();
    workspace.normalize();
    assert_eq!(workspace.directory(), Some(&PathBuf::from("backend")));
    let backup = path.with_extension("v1.json");
    assert_eq!(std::fs::read(&backup).unwrap(), bytes);
    save_at(&path, &workspace).unwrap();
    let restored = load_at(&path).unwrap();
    assert_eq!(restored.spaces[0].name, "Team");
    assert_eq!(restored.spaces[0].projects.len(), 2);
    assert_eq!(restored.preferences.language.as_deref(), Some("ru"));
    assert_eq!(std::fs::read(&backup).unwrap(), bytes);
    std::fs::write(&path, b"invalid json").unwrap();
    assert!(matches!(load_at(&path), Err(Failure::Storage)));
    assert_eq!(std::fs::read(&path).unwrap(), b"invalid json");
}

#[test]
fn storage_roundtrip_preserves_preferences_and_overwrites_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config/workspace.json");
    let mut workspace = Workspace::default();
    workspace.create_space("Work".to_owned(), vec![directory.path().to_path_buf()]);
    save_at(&path, &workspace).unwrap();
    workspace.preferences.language = Some("es".to_owned());
    workspace.preferences.code_size = 18;
    save_at(&path, &workspace).unwrap();
    let restored: Workspace = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(restored.preferences.language.as_deref(), Some("es"));
    assert_eq!(restored.preferences.code_size, 18);
    assert_eq!(restored.spaces.len(), 1);
    assert_eq!(restored.spaces[0].projects.len(), 1);
}

#[tokio::test]
async fn directory_reads_are_shallow_sorted_and_exclude_git_metadata() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::create_dir(root.path().join(".git")).unwrap();
    std::fs::write(root.path().join("a.rs"), "fn main() {}").unwrap();
    std::fs::write(root.path().join("src/nested.rs"), "nested").unwrap();
    let entries = entries(root.path().to_owned(), root.path().to_owned())
        .await
        .unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect::<Vec<_>>(),
        ["src", "a.rs"]
    );
    assert_eq!(
        read(root.path().to_owned(), root.path().join("a.rs"))
            .await
            .unwrap(),
        "fn main() {}"
    );
}

#[tokio::test]
async fn previews_reject_binary_large_missing_and_external_files() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    assert_eq!(
        read(root.path().to_owned(), outside.path().to_owned())
            .await
            .unwrap_err(),
        Failure::Read
    );
    assert_eq!(
        read(root.path().to_owned(), root.path().join("missing"))
            .await
            .unwrap_err(),
        Failure::Read
    );
    assert_eq!(decode(vec![0, 1]).unwrap_err(), Failure::Binary);
    assert_eq!(decode(vec![255]).unwrap_err(), Failure::Binary);
    assert_eq!(
        decode(vec![b'x'; PREVIEW_LIMIT as usize + 1]).unwrap_err(),
        Failure::TooLarge
    );
}

#[cfg(unix)]
#[tokio::test]
async fn symlinks_cannot_read_outside_the_linked_directory() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    let link = root.path().join("outside");
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    assert!(
        entries(root.path().to_owned(), root.path().to_owned())
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        read(root.path().to_owned(), link).await.unwrap_err(),
        Failure::Read
    );
}
