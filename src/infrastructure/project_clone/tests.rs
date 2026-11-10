use super::*;

#[test]
fn names_support_https_ssh_and_scp_without_allowing_other_transports() {
    for remote in [
        "https://example.com/team/repo.git",
        "ssh://git@example.com/team/repo.git",
        "git@example.com:team/repo.git",
    ] {
        assert_eq!(repository_name(remote).as_deref(), Some("repo"));
    }
    for remote in [
        "",
        "-upload-pack=bad",
        "ext::bad",
        "file:///tmp/repo",
        "/tmp/repo",
        "http://example.com/a",
        "https://user:secret@example.com/a",
        "git@-host:a",
        "https://example.com/..",
        "https://example.com/a?token=secret",
    ] {
        assert!(repository_name(remote).is_none(), "{remote}");
    }
}

#[tokio::test]
async fn existing_directories_and_links_are_never_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("existing");
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(destination.join("keep"), b"unchanged").unwrap();
    assert_eq!(
        clone(
            "https://example.com/team/repo.git".into(),
            destination.clone()
        )
        .await,
        Err(Error::Exists)
    );
    assert_eq!(
        std::fs::read(destination.join("keep")).unwrap(),
        b"unchanged"
    );
    #[cfg(unix)]
    {
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&destination, &link).unwrap();
        assert_eq!(
            clone("https://example.com/team/repo.git".into(), link).await,
            Err(Error::Exists)
        );
    }
}

#[tokio::test]
async fn invalid_remote_and_relative_destination_do_not_create_files() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("missing");
    assert_eq!(
        clone("ext::bad".into(), target.clone()).await,
        Err(Error::InvalidRemote)
    );
    assert!(!target.exists());
    assert_eq!(
        clone("https://example.com/a.git".into(), "relative".into()).await,
        Err(Error::Destination)
    );
}

#[test]
fn successful_clone_creates_a_repository_without_network_access() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.git");
    let target = root.path().join("checkout");
    let config = root.path().join("empty-config");
    std::fs::write(&config, "").unwrap();
    let initialized = std::process::Command::new(process::executable("git"))
        .args(["init", "--bare", "--quiet"])
        .arg(&source)
        .status()
        .unwrap();
    assert!(initialized.success());
    // Isolate Git configuration in a child process. Rewrite only this fixture's URL locally.
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "infrastructure::project_clone::tests::local_clone_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env("IWE_CLONE_TEST_TARGET", &target)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", &config)
        .env("GIT_CONFIG_COUNT", "2")
        .env(
            "GIT_CONFIG_KEY_0",
            format!(
                "url.{}.insteadOf",
                source.to_string_lossy().replace('\\', "/")
            ),
        )
        .env("GIT_CONFIG_VALUE_0", "https://iwe.invalid/repo.git")
        .env("GIT_CONFIG_KEY_1", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_1", "always")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(target.join(".git").is_dir());
    assert!(
        std::fs::read_to_string(target.join(".git/config"))
            .unwrap()
            .contains("https://iwe.invalid/repo.git")
    );
}

#[tokio::test]
#[ignore = "Subprocess fixture; started explicitly by the local clone test"]
async fn local_clone_fixture() {
    let target = PathBuf::from(std::env::var_os("IWE_CLONE_TEST_TARGET").unwrap());
    let result = clone("https://iwe.invalid/repo.git".into(), target.clone())
        .await
        .unwrap();
    assert_eq!(result, target.canonicalize().unwrap());
}
