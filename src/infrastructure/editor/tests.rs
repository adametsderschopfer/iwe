use super::*;

#[test]
fn save_preserves_line_endings_and_rejects_external_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.txt");
    fs::write(&path, "one\r\n").unwrap();
    save(&path, "one\r\n", "two\r\n").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"two\r\n");
    fs::write(&path, "external").unwrap();
    assert_eq!(save(&path, "two\r\n", "draft"), Err(Error::Changed));
    assert_eq!(fs::read(&path).unwrap(), b"external");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn failed_save_keeps_existing_file_and_does_not_create_missing_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("source.txt");
    assert_eq!(save(&path, "", "draft"), Err(Error::Access));
    assert!(!path.exists());
    fs::write(&path, "original").unwrap();
    assert_eq!(
        save(&path, "original", &"x".repeat(LIMIT + 1)),
        Err(Error::TooLarge)
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), "original");
}

#[cfg(unix)]
#[test]
fn save_preserves_permissions_and_refuses_symlink_replacement() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("run.sh");
    let link = directory.path().join("link");
    fs::write(&path, "old").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();
    symlink(&path, &link).unwrap();
    assert_eq!(save(&link, "old", "new"), Err(Error::Symlink));
    save(&path, "old", "new").unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o750
    );
}
