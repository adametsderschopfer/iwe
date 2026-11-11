use super::*;

#[test]
fn credentials_persist_across_reads_and_can_be_replaced_and_deleted() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("secrets");
    let provider = directory.join("provider");
    let deployment = directory.join("deployment");

    assert!(read(&provider).unwrap().is_none());
    write(&provider, b"original token").unwrap();
    write(&deployment, b"server password").unwrap();
    assert_eq!(
        read(&provider).unwrap().unwrap().as_slice(),
        b"original token"
    );
    write(&provider, b"refreshed token").unwrap();
    assert_eq!(
        read(&provider).unwrap().unwrap().as_slice(),
        b"refreshed token"
    );
    assert_eq!(
        read(&deployment).unwrap().unwrap().as_slice(),
        b"server password"
    );
    delete(&provider).unwrap();
    delete(&provider).unwrap();
    assert!(read(&provider).unwrap().is_none());
    assert!(read(&deployment).unwrap().is_some());
}

#[cfg(unix)]
#[test]
fn secrets_keep_private_modes_and_reject_insecure_reads() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("secrets");
    let path = directory.join("credential");
    write(&path, b"first").unwrap();
    write(&path, b"second").unwrap();

    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(read(&path).unwrap_err().kind(), ErrorKind::PermissionDenied);
    write(&path, b"replacement").unwrap();
    assert_eq!(read(&path).unwrap().unwrap().as_slice(), b"replacement");
}

#[cfg(unix)]
#[test]
fn secrets_do_not_follow_symbolic_links() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("secrets");
    let path = directory.join("credential");
    let outside = root.path().join("outside");
    fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, &directory).unwrap();
    assert_eq!(
        write(&path, b"secret").unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    assert!(!outside.join("credential").exists());

    fs::remove_file(&directory).unwrap();
    write(&path, b"secret").unwrap();
    let target = directory.join("target");
    fs::rename(&path, &target).unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert_eq!(read(&path).unwrap_err().kind(), ErrorKind::InvalidData);
    write(&path, b"replacement").unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"secret");
    assert_eq!(read(&path).unwrap().unwrap().as_slice(), b"replacement");
}
