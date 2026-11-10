use super::*;

fn connection(protocol: Protocol, port: u16) -> Connection {
    Connection {
        id: "fixture".into(),
        name: "Fixture".into(),
        protocol,
        host: "127.0.0.1".into(),
        port,
        username: "user".into(),
        ..Default::default()
    }
}

#[test]
fn ftp_and_both_ftps_modes_transfer_exact_bytes_over_loopback() {
    for protocol in [Protocol::Ftp, Protocol::Ftps, Protocol::FtpsImplicit] {
        let server = fixtures::Ftp::start(protocol);
        let connection = connection(protocol, server.port);
        let secret = Secret(Zeroizing::new("fixture".into()));
        let mut transport =
            ftp::Ftp::connect_with_roots(&connection, &secret, server.roots.clone()).unwrap();
        assert_eq!(transport.list("/").unwrap()[0].name, "hello.txt");
        let mut content = Vec::new();
        transport
            .read("/hello.txt", &mut content, &Progress::default())
            .unwrap();
        assert_eq!(content, b"server\n");
        let data = b"new\nbytes\0\xff";
        transport
            .write("/new.txt", &mut data.as_slice(), &Progress::default())
            .unwrap();
        assert_eq!(server.files.lock().unwrap()["/new.txt"], data);
        drop(transport);
    }
}

#[test]
fn ftps_rejects_untrusted_certificates() {
    let server = fixtures::Ftp::start(Protocol::Ftps);
    assert!(matches!(
        ftp::Ftp::connect(
            &connection(Protocol::Ftps, server.port),
            &Secret(Zeroizing::new("fixture".into()))
        ),
        Err(Error::Connection)
    ));
}

#[tokio::test]
async fn sftp_checks_host_keys_passwords_and_private_keys_over_loopback() {
    use base64::Engine;
    use sha2::Digest;
    let server = fixtures::Ssh::start().await;
    let port = server.port;
    let fingerprint = format!(
        "SHA256:{}",
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(sha2::Sha256::digest(
            server.key.public_key().to_bytes().unwrap()
        ))
    );
    let user_key =
        russh::keys::PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let key_path = temp.path().join("id_ed25519");
    std::fs::write(
        &key_path,
        user_key.to_openssh(Default::default()).unwrap().as_bytes(),
    )
    .unwrap();
    let encrypted_path = temp.path().join("encrypted");
    std::fs::write(
        &encrypted_path,
        user_key
            .encrypt(&mut rand::rng(), "fixture")
            .unwrap()
            .to_openssh(Default::default())
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    #[cfg(unix)]
    let agent_path = temp.path().join("agent.sock");
    #[cfg(unix)]
    let agent = fixtures::agent::serve(&agent_path);
    tokio::task::spawn_blocking(move || {
        let mut connection = connection(Protocol::Sftp, port);
        let secret = Secret(Zeroizing::new("fixture".into()));
        let mut settings = Connections::default();
        assert!(matches!(
            sftp::Sftp::connect(&connection, &settings, &secret),
            Err(Error::HostUnknown(_))
        ));
        connection.trusted_host_key = Some("wrong".into());
        assert!(matches!(
            sftp::Sftp::connect(&connection, &settings, &secret),
            Err(Error::HostChanged)
        ));
        connection.trusted_host_key = Some(fingerprint);
        assert!(matches!(
            sftp::Sftp::connect(&connection, &settings, &Secret::default()),
            Err(Error::Credentials)
        ));
        let mut transport = sftp::Sftp::connect(&connection, &settings, &secret).unwrap();
        assert_eq!(transport.list("/").unwrap()[0].name, "hello.txt");
        let mut content = Vec::new();
        transport
            .read("/hello.txt", &mut content, &Progress::default())
            .unwrap();
        assert_eq!(content, b"server\n");
        transport
            .write(
                "/new.txt",
                &mut b"uploaded".as_slice(),
                &Progress::default(),
            )
            .unwrap();
        connection.authentication = Authentication::Key("key".into());
        settings.keys.push(crate::domain::deployment::PrivateKey {
            id: "key".into(),
            name: "Test".into(),
            path: key_path,
        });
        let mut key_transport =
            sftp::Sftp::connect(&connection, &settings, &Secret::default()).unwrap();
        assert!(key_transport.stat("/new.txt").unwrap().is_some());
        settings.keys[0].path = encrypted_path;
        assert!(sftp::Sftp::connect(&connection, &settings, &secret).is_ok());
        #[cfg(unix)]
        {
            connection.authentication = Authentication::Agent;
            assert!(
                sftp::Sftp::connect_with_agent(
                    &connection,
                    &settings,
                    &Secret::default(),
                    Some(&agent_path)
                )
                .is_ok()
            );
        }
    })
    .await
    .unwrap();
    #[cfg(unix)]
    agent.join().unwrap();
    assert_eq!(server.files.lock().unwrap()["/new.txt"], b"uploaded");
}

#[test]
fn streaming_counts_bytes_and_honors_cancellation() {
    let progress = Progress::default();
    let bytes = vec![7; 150_000];
    let mut output = Vec::new();
    copy(&mut bytes.as_slice(), &mut output, &progress).unwrap();
    assert_eq!(bytes, output);
    assert_eq!(
        progress.bytes.load(std::sync::atomic::Ordering::Relaxed),
        150_000
    );
    progress.cancel.cancel();
    assert_eq!(
        copy(&mut bytes.as_slice(), &mut Vec::new(), &progress),
        Err(Error::Cancelled)
    );
}

#[test]
fn secrets_have_redacted_debug_output() {
    let secret = Secret(Zeroizing::new("password-never-log".into()));
    assert!(!format!("{secret:?}").contains("password-never-log"));
}

#[test]
fn directory_entries_cannot_escape_their_parent() {
    for name in ["..", "a/b", "a\\b", "file\nDELE", "."] {
        assert!(child_path("/srv", name).is_err());
    }
    assert_eq!(child_path("/srv", "тест.txt"), Ok("/srv/тест.txt".into()));
}
