use super::*;
use ed25519_dalek::{Signer, SigningKey};

fn signed(version: &str) -> (Vec<u8>, Vec<u8>, [u8; 32]) {
    let key = SigningKey::from_bytes(&[42; 32]);
    let assets: Vec<_> = ["macos-arm64", "macos-x64", "windows-x64", "windows-x86", "linux-x64", "linux-x86"].into_iter().map(|platform| serde_json::json!({
        "name": format!("iwe-{version}-{platform}-update.tar.gz"), "size": 3, "sha256": "ab".repeat(32)
    })).collect();
    let bytes =
        serde_json::to_vec(&serde_json::json!({ "version":version, "assets": assets })).unwrap();
    let signature = key.sign(&bytes).to_bytes().to_vec();
    (bytes, signature, key.verifying_key().to_bytes())
}

#[test]
fn releases_require_new_stable_versions_and_safe_tags() {
    assert!(newer("macos-v0.1.11", "0.1.10").unwrap());
    assert!(newer("windows-v0.1.11", "0.1.10").unwrap());
    assert!(newer("v0.1.11", "0.1.10").unwrap());
    assert!(newer("0.2.0", "0.1.10").unwrap());
    assert!(!newer("0.1.10", "0.1.10").unwrap());
    assert!(!newer("0.1.9", "0.1.10").unwrap());
    for tag in ["0.1.11-rc.1", "../0.1.11", "0.1.11/evil", "garbage"] {
        assert!(newer(tag, "0.1.10").is_err());
    }
}

#[test]
fn platform_selection_ignores_other_systems_and_prefers_specific_tags() {
    let releases: Vec<_> = [
        "windows-v9.0.0",
        "linux-v8.0.0",
        "v0.1.12",
        "macos-v0.1.12",
        "garbage",
    ]
    .into_iter()
    .map(|tag| Latest {
        tag_name: tag.into(),
        draft: false,
        prerelease: false,
    })
    .collect();
    assert_eq!(
        select_latest(&releases, "macos-arm64", "0.1.10")
            .unwrap()
            .unwrap()
            .tag_name,
        "macos-v0.1.12"
    );
    assert!(
        select_latest(&releases, "macos-arm64", "0.1.12")
            .unwrap()
            .is_none()
    );
    assert_eq!(
        select_latest(&releases, "windows-x64", "0.1.10")
            .unwrap()
            .unwrap()
            .tag_name,
        "windows-v9.0.0"
    );
}

#[test]
fn platform_manifests_accept_only_their_own_two_architectures() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let assets: Vec<_> = ["linux-x64", "linux-x86"].into_iter().map(|platform| serde_json::json!({
        "name": format!("iwe-0.1.12-{platform}-update.tar.gz"), "size": 3, "sha256": "ab".repeat(32)
    })).collect();
    let bytes =
        serde_json::to_vec(&serde_json::json!({"version": "0.1.12", "assets": assets})).unwrap();
    let signature = key.sign(&bytes).to_bytes();
    let public = key.verifying_key().to_bytes();
    assert!(verified_release(&bytes, &signature, "linux-v0.1.12", "linux-x64", &public).is_ok());
    assert!(
        verified_release(
            &bytes,
            &signature,
            "windows-v0.1.12",
            "windows-x64",
            &public
        )
        .is_err()
    );
    assert!(verified_release(&bytes, &signature, "linux-v0.1.12", "windows-x64", &public).is_err());
    assert!(verified_release(&bytes, &signature, "v0.1.12", "linux-x64", &public).is_err());
}

#[test]
fn manifest_authenticates_version_platform_and_package_before_acceptance() {
    let (mut bytes, mut signature, key) = signed("0.1.11");
    let release = verified_release(&bytes, &signature, "v0.1.11", "linux-x64", &key).unwrap();
    assert_eq!(release.asset, "iwe-0.1.11-linux-x64-update.tar.gz");
    assert_eq!(release.version, "0.1.11");
    assert!(verified_release(&bytes, &signature, "0.1.12", "linux-x64", &key).is_err());
    assert!(verified_release(&bytes, &signature, "0.1.11", "linux-arm64", &key).is_err());
    signature[0] ^= 1;
    assert!(verified_release(&bytes, &signature, "0.1.11", "linux-x64", &key).is_err());
    signature[0] ^= 1;
    bytes[0] ^= 1;
    assert!(verified_release(&bytes, &signature, "0.1.11", "linux-x64", &key).is_err());
}

#[test]
fn update_redirects_cannot_leave_https_github_hosts() {
    for url in [
        "https://github.com/a",
        "https://release-assets.githubusercontent.com/file?token=test",
    ] {
        assert!(allowed_url(&url.parse().unwrap()));
    }
    for url in [
        "http://github.com/a",
        "https://github.com.evil.test/a",
        "https://example.org/a",
        "https://user:pass@github.com/a",
        "https://github.com:444/a",
    ] {
        assert!(!allowed_url(&url.parse().unwrap()));
    }
}

#[tokio::test]
async fn metadata_reads_reject_oversized_and_failed_responses() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    for (response, expected) in [
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\n12345",
            Error::Invalid,
        ),
        (
            "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            Error::Network,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            assert!(socket.read(&mut request).await.unwrap() > 0);
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        assert_eq!(small(&Client::new(), &url, 4).await.unwrap_err(), expected);
        server.await.unwrap();
    }
}
