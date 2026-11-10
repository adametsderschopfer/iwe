use super::*;
use std::io::Cursor;

impl Prepared {
    pub(crate) fn fixture() -> Self {
        let directory = tempfile::tempdir().unwrap().keep();
        let lock = File::create(directory.join("running.lock")).unwrap();
        lock.lock().unwrap();
        Self {
            version: "0.1.12".into(),
            directory,
            _lock: lock,
            handed_off: AtomicBool::new(false),
        }
    }
}

fn archive(path: &Path, name: &str, data: &[u8], kind: tar::EntryType) {
    let file = File::create(path).unwrap();
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o755);
    header.set_entry_type(kind);
    if kind.is_symlink() {
        header.set_link_name("outside").unwrap();
    }
    archive
        .append_data(&mut header, name, Cursor::new(data))
        .unwrap();
    archive.into_inner().unwrap().finish().unwrap();
}

#[test]
fn signed_payload_extraction_preserves_the_executable_and_rejects_other_roots_and_links() {
    let temp = tempfile::tempdir().unwrap();
    let package = temp.path().join("package.tar.gz");
    archive(&package, "iwe", b"executable", tar::EntryType::Regular);
    let output = temp.path().join("ok");
    extract(&package, &output, "iwe").unwrap();
    assert_eq!(fs::read(output.join("iwe")).unwrap(), b"executable");
    archive(&package, "other", b"untrusted", tar::EntryType::Regular);
    assert_eq!(
        extract(&package, &temp.path().join("wrong"), "iwe"),
        Err(Error::Invalid)
    );
    archive(&package, "iwe", b"", tar::EntryType::Symlink);
    assert_eq!(
        extract(&package, &temp.path().join("link"), "iwe"),
        Err(Error::Invalid)
    );
    assert!(!temp.path().join("outside").exists());
}

#[test]
fn failed_replacement_and_restart_restore_the_previous_application() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("new");
    let target = temp.path().join("installed");
    let backup = temp.path().join("backup");
    fs::write(&source, b"new").unwrap();
    fs::write(&target, b"old").unwrap();
    assert_eq!(
        replace(&source, &target, &backup, || Err(Error::Install)),
        Err(Error::Install)
    );
    assert_eq!(fs::read(&target).unwrap(), b"old");
    assert_eq!(fs::read(&source).unwrap(), b"new");
    assert!(!backup.exists());
    replace(&source, &target, &backup, || Ok(())).unwrap();
    assert_eq!(fs::read(&target).unwrap(), b"new");
    assert_eq!(fs::read(&backup).unwrap(), b"old");
}

#[test]
fn macos_updates_require_a_complete_bundle_and_staging_stays_beside_installation() {
    assert_eq!(
        target_for(Path::new("/Applications/IWE.app/Contents/MacOS/iwe"), true).unwrap(),
        Path::new("/Applications/IWE.app")
    );
    assert_eq!(
        target_for(
            Path::new("/Applications/IWE Code.app/Contents/MacOS/iwe"),
            true
        )
        .unwrap(),
        Path::new("/Applications/IWE Code.app")
    );
    assert_eq!(
        target_for(Path::new("/tmp/iwe"), true),
        Err(Error::Unsupported)
    );
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("iwe");
    fs::write(&target, b"old").unwrap();
    let stage = stage_directory(&target).unwrap();
    assert!(valid_stage(stage.path(), &target));
    assert!(!valid_stage(temp.path(), &target));
}

#[test]
fn package_hash_checks_detect_tampering_before_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("package");
    fs::write(&archive, b"original").unwrap();
    let release = Release {
        version: "0.1.11".into(),
        tag: "0.1.11".into(),
        asset: "unused".into(),
        size: 8,
        sha256: format!("{:x}", Sha256::digest(b"original")),
        manifest: vec![],
        signature: vec![],
    };
    verify_package(&archive, &release).unwrap();
    fs::write(&archive, b"modified").unwrap();
    assert_eq!(verify_package(&archive, &release), Err(Error::Invalid));
}

#[test]
fn extraction_rejects_parent_paths_and_expansion_limits_before_writing_content() {
    let temp = tempfile::tempdir().unwrap();
    for (name, size, suffix) in [
        ("iwe/../../escape", 1_u64, "traversal"),
        ("iwe", MAX_PACKAGE * 3 + 1, "oversized"),
    ] {
        let package = temp.path().join(format!("{suffix}.tar.gz"));
        let encoder = flate2::write::GzEncoder::new(
            File::create(&package).unwrap(),
            flate2::Compression::default(),
        );
        let mut archive = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
        header.set_size(size);
        header.set_mode(0o755);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        // The oversized fixture deliberately omits its declared body.
        archive.append(&header, Cursor::new(b"x")).unwrap();
        archive.into_inner().unwrap().finish().unwrap();
        assert_eq!(
            extract(&package, &temp.path().join(suffix), "iwe"),
            Err(Error::Invalid)
        );
    }
    assert!(!temp.path().join("escape").exists());
}

#[test]
fn temporary_windows_file_locks_are_retried_but_permanent_errors_stop() {
    let mut attempts = 0;
    retry_busy(Duration::from_secs(1), || {
        attempts += 1;
        if attempts < 3 {
            Err(std::io::ErrorKind::PermissionDenied.into())
        } else {
            Ok(())
        }
    })
    .unwrap();
    assert_eq!(attempts, 3);
    assert_eq!(
        retry_busy(Duration::ZERO, || Err(
            std::io::ErrorKind::PermissionDenied.into()
        )),
        Err(Error::Install)
    );
    assert_eq!(
        retry_busy(Duration::from_secs(1), || Err(
            std::io::ErrorKind::NotFound.into()
        )),
        Err(Error::Install)
    );
}

#[test]
#[ignore = "Subprocess fixture; started explicitly by helper handoff tests"]
fn helper_process_fixture() {
    let Some(directory) = std::env::var_os("IWE_TEST_HANDOFF").map(PathBuf::from) else {
        return;
    };
    if std::env::var_os("IWE_TEST_HANDOFF_EXIT").is_some() {
        return;
    }
    if std::env::var_os("IWE_TEST_HANDOFF_STALL").is_none() {
        fs::write(directory.join("helper.ready"), b"ready").unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while !directory.join("finish").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn handoff_requires_a_live_helper_confirmation_and_has_a_deadline() {
    for mode in ["ready", "exit", "stall"] {
        let temp = tempfile::tempdir().unwrap();
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "infrastructure::updates::install::tests::helper_process_fixture",
                "--ignored",
            ])
            .env("IWE_TEST_HANDOFF", temp.path())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if mode == "exit" {
            command.env("IWE_TEST_HANDOFF_EXIT", "1");
        }
        if mode == "stall" {
            command.env("IWE_TEST_HANDOFF_STALL", "1");
        }
        let mut child = command.spawn().unwrap();
        let result = wait_for_helper(&mut child, temp.path(), Duration::from_secs(1));
        fs::write(temp.path().join("finish"), b"done").unwrap();
        child.wait().unwrap();
        assert_eq!(
            result,
            if mode == "ready" {
                Ok(())
            } else {
                Err(Error::Install)
            }
        );
    }
}

#[test]
fn helper_paths_are_normalized_and_other_directories_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let helper = temp.path().join("helper.exe");
    fs::write(&helper, b"fixture").unwrap();
    let directory = temp.path().canonicalize().unwrap();
    assert_eq!(
        helper_directory(&helper, &temp.path().join(".")).unwrap(),
        directory
    );
    let other = tempfile::tempdir().unwrap();
    assert_eq!(helper_directory(&helper, other.path()), Err(Error::Invalid));
}

#[cfg(windows)]
#[test]
fn windows_sharing_and_lock_violations_are_retried() {
    for code in [32, 33] {
        let mut busy = true;
        retry_busy(Duration::from_secs(1), || {
            if std::mem::take(&mut busy) {
                Err(std::io::Error::from_raw_os_error(code))
            } else {
                Ok(())
            }
        })
        .unwrap();
    }
}

#[test]
fn failed_handoff_returns_without_terminating_the_process() {
    assert_eq!(exit_after_handoff(|| Err(Error::Install)), Error::Install);
}

#[cfg(unix)]
#[test]
fn confirmed_install_exits_with_blocked_workers_and_restarts_the_verified_payload() {
    use ed25519_dalek::{Signer, SigningKey};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let target = root.join(if cfg!(target_os = "macos") {
        "Installed.app"
    } else {
        "iwe"
    });
    let executable = if cfg!(target_os = "macos") {
        target.join("Contents/MacOS/iwe")
    } else {
        target.clone()
    };
    fs::create_dir_all(executable.parent().unwrap()).unwrap();
    fs::write(&executable, b"old executable").unwrap();
    if cfg!(target_os = "macos") {
        fs::write(target.join("Contents/Info.plist"), b"old bundle").unwrap();
    }
    let stage = stage_directory(&target).unwrap();
    let archive_path = stage.path().join("package.tar.gz");
    let file = File::create(&archive_path).unwrap();
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);
    let payload = payload_root(platform().unwrap());
    let script = b"#!/bin/sh\nprintf new > \"$IWE_TEST_RESTARTED\"\n";
    let path = if cfg!(target_os = "macos") {
        format!("{payload}/Contents/MacOS/iwe")
    } else {
        payload.to_owned()
    };
    for (path, body) in
        std::iter::once((path, script.as_slice())).chain(cfg!(target_os = "macos").then(|| {
            (
                format!("{payload}/Contents/Info.plist"),
                b"new bundle".as_slice(),
            )
        }))
    {
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        archive
            .append_data(&mut header, path, Cursor::new(body))
            .unwrap();
    }
    archive.into_inner().unwrap().finish().unwrap();
    let package = fs::read(&archive_path).unwrap();
    let platforms = if cfg!(target_os = "macos") {
        ["macos-arm64", "macos-x64"]
    } else {
        ["linux-x64", "linux-x86"]
    };
    let manifest = serde_json::to_vec(&serde_json::json!({
        "version": "99.0.0", "assets": platforms.map(|platform| serde_json::json!({
            "name": format!("iwe-99.0.0-{platform}-update.tar.gz"),
            "size": package.len(), "sha256": format!("{:x}", Sha256::digest(&package))
        }))
    }))
    .unwrap();
    let key = SigningKey::from_bytes(&[42; 32]);
    fs::write(stage.path().join("updates.json"), &manifest).unwrap();
    fs::write(
        stage.path().join("updates.sig"),
        key.sign(&manifest).to_bytes(),
    )
    .unwrap();
    fs::write(
        stage.path().join("plan.json"),
        serde_json::to_vec(&Plan {
            target: target.clone(),
            tag: format!("{}-v99.0.0", std::env::consts::OS),
        })
        .unwrap(),
    )
    .unwrap();
    fs::write(stage.path().join("running.lock"), b"").unwrap();
    let helper_path = stage.path().join("helper");
    fs::copy(std::env::current_exe().unwrap(), &helper_path).unwrap();
    let restarted = root.join("restarted");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "infrastructure::updates::install::tests::restart_process_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env("IWE_TEST_UPDATE_ROLE", "parent")
        .env("IWE_TEST_UPDATE_STAGE", stage.path())
        .env("IWE_TEST_RESTARTED", &restarted)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("updating process did not terminate");
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    assert!(status.success());
    while !restarted.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    assert_eq!(fs::read(&restarted).unwrap(), b"new");
    assert_eq!(fs::read(&executable).unwrap(), script);
    let previous = if cfg!(target_os = "macos") {
        stage.path().join("previous/Contents/MacOS/iwe")
    } else {
        stage.path().join("previous")
    };
    assert_eq!(fs::read(previous).unwrap(), b"old executable");
    assert!(stage.path().join("helper.ready").is_file());
}

#[cfg(unix)]
#[test]
#[ignore = "Subprocess fixture; invoked by the complete installation test"]
fn restart_process_fixture() {
    let directory = PathBuf::from(std::env::var_os("IWE_TEST_UPDATE_STAGE").unwrap());
    if std::env::var("IWE_TEST_UPDATE_ROLE").unwrap() == "helper" {
        let public = ed25519_dalek::SigningKey::from_bytes(&[42; 32])
            .verifying_key()
            .to_bytes();
        helper_with_key(&directory, &public).unwrap();
        return;
    }
    let lock = File::options()
        .read(true)
        .write(true)
        .open(directory.join("running.lock"))
        .unwrap();
    lock.lock().unwrap();
    let prepared = Prepared {
        version: "99.0.0".into(),
        directory: directory.clone(),
        _lock: lock,
        handed_off: AtomicBool::new(false),
    };
    // Runtime destruction would wait forever for this worker.
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (started, receiver) = std::sync::mpsc::channel();
    let _worker = runtime.spawn_blocking(move || {
        started.send(()).unwrap();
        std::thread::park();
    });
    receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    let mut command = Command::new(directory.join("helper"));
    command
        .args([
            "--exact",
            "infrastructure::updates::install::tests::restart_process_fixture",
            "--ignored",
            "--nocapture",
        ])
        .env("IWE_TEST_UPDATE_ROLE", "helper")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let error = runtime.block_on(async move {
        tokio::task::spawn_blocking(move || {
            exit_after_handoff(|| launch_helper(&prepared, command))
        })
        .await
        .unwrap()
    });
    panic!("handoff failed: {error:?}");
}
